//! Arranging: after fixing, nodes move a little so connectors bend, cross
//! and crowd less, keeping the input's rough layout.
//!
//! The search is greedy. Each step tries small moves of single nodes —
//! lining a bent connector's ends up, or nudging the ends of connectors
//! that cross, crowd or have issues — fixes the result, and keeps the move
//! that lowers the score most. Moves never add issues or jogs, never change which
//! side of another node a node is on, and stay within a few lanes of where
//! the node started.

use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use regex::Regex;

use crate::diagram::{Diagram, DiagramConnector, DiagramNode, ancestor_ids, holds_end, node_group};
use crate::fix::{FixChange, FixResult, fix, fix_after_moves};
use crate::geometry::{
    Bounds, Point, format_number, hypot, intersection, parse_number, point, segment_intersects_interior, segments,
};
use crate::route::chords_cross;
use crate::xml::{Element, serialize};
use crate::{AnalysisReport, SvgInputError, parse};

/// How far a node may end up from where fixing left it, on each axis.
const MAX_SHIFT: f64 = 60.0;
const NUDGES: [f64; 2] = [20.0, 40.0];
const MAX_STEPS: usize = 12;

const ISSUE_COST: f64 = 1000.0;
const CROSSING_COST: f64 = 150.0;
const BEND_COST: f64 = 30.0;
const CROWDING_COST: f64 = 60.0;
/// Parallel lines within two lanes still read as one band, less so.
const NEAR_CROWDING_COST: f64 = 20.0;
/// A route turning on another misreads as one line, worse than a crossing.
const TOUCH_COST: f64 = 150.0;
const MOVEMENT_COST: f64 = 0.5;
/// Nodes in a row or column joined by a connector running straight along
/// it show the flow with their alignment, so falling out of line costs
/// much more than for neighbours that only share a coordinate (or whose
/// connector detours).
const JOINED_ALIGNMENT_COST: f64 = 300.0;
const LOOSE_ALIGNMENT_COST: f64 = 50.0;
/// An inner node leaving a line its neighbours keep reads as a dent,
/// turning the eye twice where an end leaving turns it once; worse when
/// the node is joined to the line.
const JOINED_DENT_COST: f64 = 300.0;
const LOOSE_DENT_COST: f64 = 200.0;
/// A connector with any bend reads as a detour, on top of its bends.
const BENT_COST: f64 = 60.0;
/// A step shorter than this in the middle of a route is a jog: the line
/// staggers instead of turning.
const JOG_LENGTH: f64 = 20.0;
/// An end stub shorter than this, or a crossing nearer the node than this,
/// makes it hard to see the connector arrive.
const SHORT_END: f64 = 20.0;
const SHORT_END_COST: f64 = 60.0;
/// On top of the crossing itself.
const CROSSING_AT_END_COST: f64 = 100.0;
const JOG_COST: f64 = 60.0;
/// Every pixel of route counts a little, so long detours are not free.
const LENGTH_COST: f64 = 0.1;
/// Connectors sharing a node side read best spread evenly, with the same
/// gap between neighbours and to the side's corners; each even gap's worth
/// of unevenness costs this much.
const PORT_SPACING_COST: f64 = 40.0;
/// Parallel segments of different connectors this close, over this much
/// length, crowd each other.
const CROWDED_DISTANCE: f64 = 10.0;
const CROWDED_LENGTH: f64 = 20.0;
/// How far connectors keep from nodes they pass.
const CLEARANCE: f64 = 8.0;
/// A moved node keeps this far from other nodes, its container's edges and
/// other nodes' labels, unless it was already closer.
const NODE_GAP: f64 = 20.0;
const CONTAINER_MARGIN: f64 = 12.0;
const LABEL_CLEARANCE: f64 = 4.0;

/// Fixes the diagram, then moves nodes a little where that leaves fewer
/// bends, crossings and crowded lanes.
pub fn arrange(svg: &str) -> Result<FixResult, SvgInputError> {
    let fixed = fix(svg)?;
    let baseline = fixed.report.diagram.clone();
    let context = Context::new(baseline);
    let start = State {
        score: context.score(&fixed, &HashMap::new()),
        result: fixed,
        offsets: HashMap::new(),
    };
    Ok(improve(&context, start, MAX_STEPS).result)
}

struct Context {
    baseline: Diagram,
    alignments: Vec<Alignment>,
    containers: HashSet<String>,
    /// Pairs of nodes, either way round, joined by a connector running
    /// straight between them: along a row (`false`) or a column (`true`).
    joined: HashSet<(String, String, bool)>,
}

#[derive(Clone)]
struct State {
    result: FixResult,
    /// How far each moved node is from where fixing left it.
    offsets: HashMap<String, Point>,
    score: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Move {
    dx: f64,
    dy: f64,
}

fn improve(context: &Context, state: State, steps: usize) -> State {
    if steps == 0 {
        return state;
    }
    let candidates = candidates(context, &state);
    let best = evaluate_all(context, &state, &candidates)
        .into_iter()
        .flatten()
        .filter(|next| next.result.report.issues.len() <= state.result.report.issues.len())
        .filter(|next| total_jogs(&next.result.report.diagram) <= total_jogs(&state.result.report.diagram))
        .reduce(|best, next| if next.score < best.score { next } else { best });
    match best {
        Some(next) if next.score < state.score - 1e-9 => improve(context, next, steps - 1),
        _ => state,
    }
}

/// Evaluates the candidates on as many threads as are available; the
/// outcome does not depend on how they are split.
fn evaluate_all(context: &Context, state: &State, candidates: &[(String, Move)]) -> Vec<Option<State>> {
    let threads = std::thread::available_parallelism().map_or(1, |count| count.get());
    let chunk = candidates.len().div_ceil(threads).max(1);
    std::thread::scope(|scope| {
        let handles: Vec<_> = candidates
            .chunks(chunk)
            .map(|part| {
                scope.spawn(move || {
                    part.iter()
                        .map(|(id, step)| evaluate(context, state, id, *step))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|handle| handle.join().expect("an evaluation thread finishes"))
            .collect()
    })
}

/// Moves `id` by `step` and fixes the result, rerouting the node's
/// connectors. A zero step reroutes them in place, as the nodes around
/// them may have moved since they were last routed.
fn evaluate(context: &Context, state: &State, id: &str, step: Move) -> Option<State> {
    let in_place = step.dx == 0.0 && step.dy == 0.0;
    let mut document = parse(&state.result.svg).ok()?;
    let group = node_group(&document, id)?;
    let transform = translated(document.element(&group).attr("transform"), step);
    document = document.update(&group, |g| {
        if in_place {
            return g;
        }
        if transform.is_empty() {
            Element {
                attributes: g
                    .attributes
                    .into_iter()
                    .filter(|(name, _)| name != "transform")
                    .collect(),
                ..g
            }
        } else {
            g.with_attr("transform", &transform)
        }
    });
    let moved = fix_after_moves(&serialize(&document), &[id.to_owned()], 5).ok()?;
    let mut offsets = state.offsets.clone();
    let offset = offsets.entry(id.to_owned()).or_insert(point(0.0, 0.0));
    *offset = point(offset.x + step.dx, offset.y + step.dy);
    let change = FixChange {
        code: "arrange-node".to_owned(),
        message: format!(
            "Moved node \"{id}\" by ({}, {}) so connectors bend, cross and crowd less.",
            format_number(step.dx),
            format_number(step.dy)
        ),
        elements: vec![id.to_owned()],
    };
    let change = (!in_place).then_some(change);
    let result = FixResult {
        changes: [
            state.result.changes.clone(),
            change.into_iter().collect(),
            moved.changes.clone(),
        ]
        .concat(),
        ..moved
    };
    Some(State {
        score: context.score(&result, &offsets),
        result,
        offsets,
    })
}

static TRAILING_TRANSLATE: LazyLock<Regex> = LazyLock::new(|| {
    let number = r"[-+]?(?:\d*\.?\d+)(?:[eE][-+]?\d+)?";
    Regex::new(&format!(r"translate\(\s*({number})(?:[\s,]+({number}))?\s*\)\s*$")).expect("valid pattern")
});

/// A transform moved by `step`, folding it into a trailing translate and
/// dropping that translate when it comes to nothing.
fn translated(existing: &str, step: Move) -> String {
    let existing = existing.trim();
    match TRAILING_TRANSLATE.captures(existing) {
        Some(captures) => {
            let x = parse_number(&captures[1]) + step.dx;
            let y = captures.get(2).map_or(0.0, |m| parse_number(m.as_str())) + step.dy;
            let rest = existing[..captures.get(0).expect("whole match").start()].trim_end();
            if x == 0.0 && y == 0.0 {
                rest.to_owned()
            } else {
                format!(
                    "{rest}{}translate({} {})",
                    if rest.is_empty() { "" } else { " " },
                    format_number(x),
                    format_number(y)
                )
            }
        }
        None => format!(
            "{existing}{}translate({} {})",
            if existing.is_empty() { "" } else { " " },
            format_number(step.dx),
            format_number(step.dy)
        ),
    }
}

/// Moves worth trying: rerouting the connectors of a bent connector's ends
/// in place, lining up the ends of bent connectors, moving nodes
/// off the straight line between a bent connector's lined-up ends, and
/// nudging the nodes at the ends of connectors that cross, crowd or have
/// issues.
fn candidates(context: &Context, state: &State) -> Vec<(String, Move)> {
    let diagram = &state.result.report.diagram;
    let node = |id: &str| diagram.node(id);
    let straighten = diagram
        .connectors
        .iter()
        .filter(|c| c.points.len() > 2)
        .flat_map(|connector| {
            let (first, last) = (connector.points[0], connector.points[connector.points.len() - 1]);
            [
                (&connector.to, &connector.from, last, first),
                (&connector.from, &connector.to, first, last),
            ]
            .into_iter()
            .filter_map(|(moving, other, own_end, other_end)| Some((node(moving)?, node(other)?, own_end, other_end)))
            .flat_map(|(moving, other, own_end, other_end)| {
                let (centre, other_centre) = (moving.bounds.centre(), other.bounds.centre());
                [
                    Move {
                        dx: 0.0,
                        dy: other_centre.y - centre.y,
                    },
                    Move {
                        dx: 0.0,
                        dy: other_end.y - own_end.y,
                    },
                    Move {
                        dx: other_centre.x - centre.x,
                        dy: 0.0,
                    },
                    Move {
                        dx: other_end.x - own_end.x,
                        dy: 0.0,
                    },
                ]
                .map(|step| (moving.id.clone(), step))
            })
            .collect::<Vec<_>>()
        });
    let unblock = diagram
        .connectors
        .iter()
        .filter(|c| c.points.len() > 2)
        .flat_map(|connector| {
            let (first, last) = (connector.points[0], connector.points[connector.points.len() - 1]);
            let reach = CLEARANCE + 1.0;
            diagram
                .nodes
                .iter()
                .filter(|node| !holds_end(&diagram.nodes, connector, node))
                .flat_map(|node| {
                    let b = &node.bounds;
                    let blocks_row = first.y == last.y
                        && b.y < first.y + reach
                        && b.bottom() > first.y - reach
                        && b.right() > first.x.min(last.x)
                        && b.x < first.x.max(last.x);
                    let blocks_column = first.x == last.x
                        && b.x < first.x + reach
                        && b.right() > first.x - reach
                        && b.bottom() > first.y.min(last.y)
                        && b.y < first.y.max(last.y);
                    let moves = if blocks_row {
                        vec![
                            Move {
                                dx: 0.0,
                                dy: first.y + reach - b.y,
                            },
                            Move {
                                dx: 0.0,
                                dy: first.y - reach - b.bottom(),
                            },
                        ]
                    } else if blocks_column {
                        vec![
                            Move {
                                dx: first.x + reach - b.x,
                                dy: 0.0,
                            },
                            Move {
                                dx: first.x - reach - b.right(),
                                dy: 0.0,
                            },
                        ]
                    } else {
                        Vec::new()
                    };
                    moves.into_iter().map(|step| (node.id.clone(), step))
                })
                .collect::<Vec<_>>()
        });
    let troubled = troubled_nodes(&state.result.report);
    let nudges = troubled.into_iter().flat_map(|id| {
        NUDGES.iter().flat_map(move |amount| {
            [
                Move { dx: *amount, dy: 0.0 },
                Move { dx: -amount, dy: 0.0 },
                Move { dx: 0.0, dy: *amount },
                Move { dx: 0.0, dy: -amount },
            ]
            .map(|step| (id.clone(), step))
        })
    });
    let reroute = diagram
        .connectors
        .iter()
        .filter(|c| c.points.len() > 2)
        .flat_map(|c| [c.from.clone(), c.to.clone()])
        .map(|id| (id, Move { dx: 0.0, dy: 0.0 }));
    let mut seen: Vec<(String, Move)> = Vec::new();
    for (id, step) in reroute.chain(straighten).chain(unblock).chain(nudges) {
        let rounded = Move {
            dx: step.dx.round(),
            dy: step.dy.round(),
        };
        if !seen.contains(&(id.clone(), rounded)) && context.allows(state, &id, rounded) {
            seen.push((id, rounded));
        }
    }
    seen
}

/// Nodes at the ends of connectors that cross, crowd or touch another,
/// that turn twice or more, that meet the node with a very short stub or
/// are crossed right beside it, or that an issue names (directly, or
/// through its connector or label).
fn troubled_nodes(report: &AnalysisReport) -> Vec<String> {
    let diagram = &report.diagram;
    let ends = |id: &str| {
        diagram
            .connector(id)
            .map(|connector| vec![connector.from.clone(), connector.to.clone()])
            .unwrap_or_default()
    };
    let connectors = &diagram.connectors;
    let tangled = connectors.iter().enumerate().flat_map(|(index, first)| {
        connectors[index + 1..]
            .iter()
            .filter(move |second| {
                crossings(&first.points, &second.points) + touches(&first.points, &second.points) > 0
                    || crowding(&first.points, &second.points) > 0.0
            })
            .flat_map(move |second| [ends(&first.id), ends(&second.id)].concat())
    });
    let named = report.issues.iter().flat_map(|issue| &issue.elements).flat_map(|id| {
        if diagram.node(id).is_some() {
            vec![id.clone()]
        } else if diagram.connector(id).is_some() {
            ends(id)
        } else {
            diagram
                .labels
                .iter()
                .find(|label| label.id == *id)
                .and_then(|label| label.connector.as_deref())
                .map(ends)
                .unwrap_or_default()
        }
    });
    // A connector that turns twice or more might turn less with one of its
    // ends a little further along.
    let winding = connectors
        .iter()
        .filter(|connector| connector.points.len() > 3)
        .flat_map(|connector| [connector.from.clone(), connector.to.clone()]);
    // Where a connector meets its node with a stub too short to read, or
    // is crossed right beside the node, the node is worth nudging so the
    // connector arrives more clearly.
    let unclear = connectors.iter().flat_map(|connector| {
        end_segments(connector)
            .into_iter()
            .filter(|(_, end, inner)| {
                let short = (end.x - inner.x).abs() + (end.y - inner.y).abs() < SHORT_END;
                short || crossed_near(connectors, &connector.id, *end, *inner).is_some()
            })
            .map(|(node, _, _)| node.to_owned())
            .collect::<Vec<_>>()
    });
    let mut ids: Vec<String> = Vec::new();
    for id in tangled.chain(named).chain(winding).chain(unclear) {
        if !ids.contains(&id) {
            ids.push(id);
        }
    }
    ids
}

impl Context {
    fn new(baseline: Diagram) -> Context {
        Context {
            alignments: alignments(&baseline.nodes),
            containers: baseline
                .nodes
                .iter()
                .filter_map(|node| node.parent_id.clone())
                .collect(),
            joined: baseline
                .connectors
                .iter()
                .filter_map(|c| match c.points.as_slice() {
                    [start, end] if start.y == end.y => Some((c, false)),
                    [start, end] if start.x == end.x => Some((c, true)),
                    _ => None,
                })
                .flat_map(|(c, column)| {
                    [
                        (c.from.clone(), c.to.clone(), column),
                        (c.to.clone(), c.from.clone(), column),
                    ]
                })
                .collect(),
            baseline,
        }
    }

    fn baseline_node(&self, id: &str) -> Option<&DiagramNode> {
        self.baseline.node(id)
    }

    /// Whether moving `id` by `step` keeps it within reach of where it
    /// started, inside its container, and on the same side of every other
    /// node as in the input.
    fn allows(&self, state: &State, id: &str, step: Move) -> bool {
        let diagram = &state.result.report.diagram;
        let (Some(node), Some(original)) = (diagram.node(id), self.baseline_node(id)) else {
            return false;
        };
        if self.containers.contains(id) {
            return false;
        }
        let offset = state.offsets.get(id).copied().unwrap_or(point(0.0, 0.0));
        if (offset.x + step.dx).abs() > MAX_SHIFT || (offset.y + step.dy).abs() > MAX_SHIFT {
            return false;
        }
        let moved = Bounds {
            x: node.bounds.x + step.dx,
            y: node.bounds.y + step.dy,
            ..node.bounds
        };
        let parent = node.parent_id.as_deref().and_then(|parent| diagram.node(parent));
        let parent_fits = parent.is_none_or(|parent| {
            margins(&moved, &parent.bounds)
                .iter()
                .zip(margins(&node.bounds, &parent.bounds))
                .all(|(after, before)| *after >= before.min(CONTAINER_MARGIN))
        });
        let ancestors = ancestor_ids(&diagram.nodes, id);
        // A connector the node lands on crosses it and is rerouted, but one
        // left grazing its edge, inside the clearance yet outside the node,
        // stays where it is: the node must not come to rest against one.
        let clear_of_connectors = diagram
            .connectors
            .iter()
            .filter(|connector| !holds_end(&diagram.nodes, connector, node))
            .all(|connector| {
                let grazes = |bounds: &Bounds| {
                    route_enters(&connector.points, &bounds.inflate(CLEARANCE - 0.5))
                        && !route_enters(&connector.points, bounds)
                };
                !grazes(&moved) || grazes(&node.bounds)
            });
        parent_fits
            && clear_of_connectors
            && diagram.nodes.iter().filter(|other| other.id != id).all(|other| {
                let related = ancestors.contains(&other.id.as_str());
                let clear_of_labels = other.label_bounds.iter().all(|label| {
                    let label = label.inflate(LABEL_CLEARANCE);
                    intersection(&moved, &label).is_none() || intersection(&node.bounds, &label).is_some()
                });
                let Some(other_original) = self.baseline_node(&other.id) else {
                    return clear_of_labels;
                };
                clear_of_labels
                    && (related
                        || (keeps_sides(&original.bounds, &other_original.bounds, &moved, &other.bounds)
                            && gap(&moved, &other.bounds) >= gap(&node.bounds, &other.bounds).min(NODE_GAP)))
            })
    }

    fn score(&self, result: &FixResult, offsets: &HashMap<String, Point>) -> f64 {
        let report = &result.report;
        let connectors = &report.diagram.connectors;
        let pairs = connectors
            .iter()
            .enumerate()
            .flat_map(|(index, first)| connectors[index + 1..].iter().map(move |second| (first, second)));
        let (crossed, crowded, touched) = pairs.fold((0, 0.0, 0), |(crossed, crowded, touched), (first, second)| {
            (
                crossed + crossings(&first.points, &second.points),
                crowded + crowding(&first.points, &second.points),
                touched + touches(&first.points, &second.points),
            )
        });
        let bends: usize = connectors.iter().map(|c| c.points.len().saturating_sub(2)).sum();
        let bent = connectors.iter().filter(|c| c.points.len() > 2).count();
        let jogs = total_jogs(&report.diagram);
        let length: f64 = connectors
            .iter()
            .flat_map(|c| segments(&c.points))
            .map(|(a, b)| (a.x - b.x).abs() + (a.y - b.y).abs())
            .sum();
        let movement: f64 = offsets.values().map(|offset| offset.x.abs() + offset.y.abs()).sum();
        report.issues.len() as f64 * ISSUE_COST
            + crossed as f64 * CROSSING_COST
            + bends as f64 * BEND_COST
            + bent as f64 * BENT_COST
            + jogs as f64 * JOG_COST
            + length * LENGTH_COST
            + port_unevenness(&report.diagram) * PORT_SPACING_COST
            + unclear_ends(connectors)
            + crowded
            + touched as f64 * TOUCH_COST
            + movement * MOVEMENT_COST
            + self.alignment_cost(&report.diagram)
    }

    /// The cost of the alignments a diagram breaks: each pair in a line
    /// joined by a connector that fell out of it, each pair of unjoined
    /// neighbours that did, and a dent for each inner node whose
    /// neighbours still line up. A pair or node counts once per direction,
    /// whether it left the line's top, centre or bottom, or all three.
    fn alignment_cost(&self, diagram: &Diagram) -> f64 {
        let mut costs: HashMap<(String, String, bool), f64> = HashMap::new();
        let mut charge = |key: (String, String, bool), cost: f64| {
            let entry = costs.entry(key).or_insert(0.0);
            *entry = entry.max(cost);
        };
        for alignment in &self.alignments {
            let column = alignment.edge.is_column();
            let joined = |a: &String, b: &String| self.joined.contains(&(a.clone(), b.clone(), column));
            let edge = |id: &String| diagram.node(id).map(|node| alignment.edge.of(&node.bounds));
            let in_line = |a: &String, b: &String| match (edge(a), edge(b)) {
                (Some(a), Some(b)) => (a - b).abs() <= 0.5,
                _ => true,
            };
            let members = &alignment.members;
            for (index, first) in members.iter().enumerate() {
                for second in &members[index + 1..] {
                    if joined(first, second) && !in_line(first, second) {
                        charge((first.clone(), second.clone(), column), JOINED_ALIGNMENT_COST);
                    }
                }
            }
            for pair in members.windows(2) {
                if !joined(&pair[0], &pair[1]) && !in_line(&pair[0], &pair[1]) {
                    charge((pair[0].clone(), pair[1].clone(), column), LOOSE_ALIGNMENT_COST);
                }
            }
            // A dent: an inner run of nodes, one or more wide, off the line
            // while the nodes either side of it stay on it.
            let on_line: Vec<bool> = members
                .iter()
                .map(|id| edge(id).is_none_or(|value| (value - alignment.value).abs() <= 0.5))
                .collect();
            let mut index = 1;
            while index + 1 < members.len() {
                if on_line[index] {
                    index += 1;
                    continue;
                }
                let start = index;
                while index + 1 < members.len() && !on_line[index] {
                    index += 1;
                }
                if on_line[start - 1] && on_line[index] {
                    let run = &members[start..index];
                    let tied = run.iter().any(|node| members.iter().any(|other| joined(node, other)));
                    let cost = if tied { JOINED_DENT_COST } else { LOOSE_DENT_COST };
                    charge((run[0].clone(), run[run.len() - 1].clone(), column), cost);
                }
            }
        }
        costs.values().sum()
    }
}

/// Whether any segment of a route reaches into a box's interior.
fn route_enters(points: &[Point], bounds: &Bounds) -> bool {
    segments(points)
        .iter()
        .any(|(start, end)| segment_intersects_interior(*start, *end, bounds))
}

/// Distances from a node's edges to its container's: left, top, right,
/// bottom.
fn margins(node: &Bounds, container: &Bounds) -> [f64; 4] {
    [
        node.x - container.x,
        node.y - container.y,
        container.right() - node.right(),
        container.bottom() - node.bottom(),
    ]
}

/// The distance between two boxes along the axis that separates them most;
/// negative when they overlap.
fn gap(first: &Bounds, second: &Bounds) -> f64 {
    (second.x - first.right())
        .max(first.x - second.right())
        .max(second.y - first.bottom())
        .max(first.y - second.bottom())
}

/// Whether a moved node stays on the same side of another as the two were
/// in the input: left or right of it where they share some height, above
/// or below it where they share some width (or nearly, within a gap).
fn keeps_sides(original: &Bounds, other_original: &Bounds, moved: &Bounds, other: &Bounds) -> bool {
    let before = |a: f64, b: f64| a <= b + 0.5;
    let near =
        |low: f64, high: f64, other_low: f64, other_high: f64| high.min(other_high) - low.max(other_low) > -NODE_GAP;
    let side_by_side = near(original.y, original.bottom(), other_original.y, other_original.bottom());
    let stacked = near(original.x, original.right(), other_original.x, other_original.right());
    (!side_by_side
        || ((!before(original.right(), other_original.x) || before(moved.right(), other.x))
            && (!before(other_original.right(), original.x) || before(other.right(), moved.x))))
        && (!stacked
            || ((!before(original.bottom(), other_original.y) || before(moved.bottom(), other.y))
                && (!before(other_original.bottom(), original.y) || before(other.bottom(), moved.y))))
}

/// Corners of one route that sit on the other, either way round: there the
/// two read as one line turning instead of two lines meeting.
fn touches(first: &[Point], second: &[Point]) -> usize {
    let corners_on = |route: &[Point], other: &[Point]| {
        let others = segments(other);
        route[1..route.len().saturating_sub(1).max(1)]
            .iter()
            .filter(|corner| others.iter().any(|(a, b)| lies_on(**corner, *a, *b)))
            .count()
    };
    if first.len() < 2 || second.len() < 2 {
        return 0;
    }
    corners_on(first, second) + corners_on(second, first)
}

fn lies_on(point: Point, start: Point, end: Point) -> bool {
    let cross = (end.x - start.x) * (point.y - start.y) - (end.y - start.y) * (point.x - start.x);
    cross.abs() < 1e-9
        && point.x >= start.x.min(end.x)
        && point.x <= start.x.max(end.x)
        && point.y >= start.y.min(end.y)
        && point.y <= start.y.max(end.y)
}

/// How unevenly the connector ends on each node side are spread, summed
/// over sides with two or more ends: the gaps between neighbours and to
/// the corners against an even split, in even gaps.
fn port_unevenness(diagram: &Diagram) -> f64 {
    let ends = diagram
        .connectors
        .iter()
        .filter(|c| c.points.len() >= 2)
        .flat_map(|connector| {
            let points = &connector.points;
            [
                (connector.from.as_str(), points[0], points[1]),
                (
                    connector.to.as_str(),
                    points[points.len() - 1],
                    points[points.len() - 2],
                ),
            ]
        });
    let mut sides: HashMap<(&str, u8), (f64, f64, Vec<f64>)> = HashMap::new();
    for (id, end, inner) in ends {
        let Some(node) = diagram.node(id) else { continue };
        let b = &node.bounds;
        // The side a connector meets is the one it arrives at, along its
        // last segment.
        let horizontal = (end.x - inner.x).abs() >= (end.y - inner.y).abs();
        let (side, position, low, high) = if horizontal {
            (if inner.x > end.x { 1 } else { 3 }, end.y, b.y, b.bottom())
        } else {
            (if inner.y > end.y { 2 } else { 0 }, end.x, b.x, b.right())
        };
        let entry = sides.entry((node.id.as_str(), side)).or_insert((low, high, Vec::new()));
        entry.2.push(position);
    }
    sides
        .into_values()
        .filter(|(_, _, positions)| positions.len() >= 2)
        .map(|(low, high, mut positions)| {
            positions.sort_by(f64::total_cmp);
            let even = (high - low) / (positions.len() + 1) as f64;
            let marks: Vec<f64> = [low].into_iter().chain(positions).chain([high]).collect();
            marks
                .windows(2)
                .map(|pair| (pair[1] - pair[0] - even).abs())
                .sum::<f64>()
                / even
        })
        .sum()
}

/// The cost of connector ends that are hard to see arrive: stubs shorter
/// than a short end, and crossings nearer the node than one.
fn unclear_ends(connectors: &[DiagramConnector]) -> f64 {
    connectors
        .iter()
        .flat_map(|connector| {
            end_segments(connector)
                .into_iter()
                .map(|(_, end, inner)| {
                    let short = (end.x - inner.x).abs() + (end.y - inner.y).abs() < SHORT_END;
                    let crossed = crossed_near(connectors, &connector.id, end, inner).is_some();
                    f64::from(u8::from(short)) * SHORT_END_COST + f64::from(u8::from(crossed)) * CROSSING_AT_END_COST
                })
                .collect::<Vec<_>>()
        })
        .sum()
}

/// A connector's first and last segments, with the node each meets: the
/// node, the point on it and the segment's other end.
fn end_segments(connector: &DiagramConnector) -> Vec<(&str, Point, Point)> {
    let points = &connector.points;
    if points.len() < 2 {
        return Vec::new();
    }
    vec![
        (connector.from.as_str(), points[0], points[1]),
        (
            connector.to.as_str(),
            points[points.len() - 1],
            points[points.len() - 2],
        ),
    ]
}

/// How close to `end` another connector crosses the segment from `end` to
/// `inner`, when that is nearer than a short stub.
fn crossed_near(connectors: &[DiagramConnector], id: &str, end: Point, inner: Point) -> Option<f64> {
    connectors
        .iter()
        .filter(|other| other.id != id)
        .flat_map(|other| segments(&other.points))
        .filter(|(a, b)| chords_cross(end, inner, *a, *b))
        .map(|(a, b)| {
            if a.x == b.x {
                (a.x - end.x).abs()
            } else if a.y == b.y {
                (a.y - end.y).abs()
            } else {
                hypot(a.x - end.x, a.y - end.y)
            }
        })
        .filter(|distance| *distance < SHORT_END)
        .reduce(f64::min)
}

fn total_jogs(diagram: &Diagram) -> usize {
    diagram.connectors.iter().map(|c| jogs(&c.points)).sum()
}

/// Segments shorter than a jog between two turns of a route.
fn jogs(points: &[Point]) -> usize {
    segments(points)
        .iter()
        .enumerate()
        .filter(|(index, (a, b))| {
            *index > 0 && index + 1 < points.len() - 1 && (a.x - b.x).abs() + (a.y - b.y).abs() < JOG_LENGTH
        })
        .count()
}

fn crossings(first: &[Point], second: &[Point]) -> usize {
    let others = segments(second);
    segments(first)
        .iter()
        .map(|(a, b)| others.iter().filter(|(c, d)| chords_cross(*a, *b, *c, *d)).count())
        .sum()
}

/// The cost of parallel segments, one from each route, running within a
/// lane (or two) of each other for a stretch.
fn crowding(first: &[Point], second: &[Point]) -> f64 {
    let others = segments(second);
    segments(first)
        .iter()
        .flat_map(|(a, b)| {
            others.iter().map(move |(c, d)| {
                let horizontal = a.y == b.y && c.y == d.y;
                let vertical = a.x == b.x && c.x == d.x;
                let (distance, (low, high), (other_low, other_high)) = if horizontal {
                    ((a.y - c.y).abs(), span(a.x, b.x), span(c.x, d.x))
                } else if vertical {
                    ((a.x - c.x).abs(), span(a.y, b.y), span(c.y, d.y))
                } else {
                    return 0.0;
                };
                if distance <= 0.0 || high.min(other_high) - low.max(other_low) < CROWDED_LENGTH {
                    0.0
                } else if distance <= CROWDED_DISTANCE {
                    CROWDING_COST
                } else if distance < 2.0 * CROWDED_DISTANCE {
                    NEAR_CROWDING_COST
                } else {
                    0.0
                }
            })
        })
        .sum()
}

fn span(a: f64, b: f64) -> (f64, f64) {
    (a.min(b), a.max(b))
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Edge {
    CentreX,
    CentreY,
    Left,
    Right,
    Top,
    Bottom,
}

impl Edge {
    const ALL: [Edge; 6] = [
        Edge::CentreX,
        Edge::CentreY,
        Edge::Left,
        Edge::Right,
        Edge::Top,
        Edge::Bottom,
    ];

    fn of(self, bounds: &Bounds) -> f64 {
        match self {
            Edge::CentreX => bounds.x + bounds.width / 2.0,
            Edge::CentreY => bounds.y + bounds.height / 2.0,
            Edge::Left => bounds.x,
            Edge::Right => bounds.right(),
            Edge::Top => bounds.y,
            Edge::Bottom => bounds.bottom(),
        }
    }

    fn is_centre(self) -> bool {
        matches!(self, Edge::CentreX | Edge::CentreY)
    }

    /// Whether nodes aligned on this edge line up in a column.
    fn is_column(self) -> bool {
        matches!(self, Edge::CentreX | Edge::Left | Edge::Right)
    }
}

/// Nodes sharing an edge or centre line, listed along the line.
struct Alignment {
    edge: Edge,
    /// Where the line ran after fixing.
    value: f64,
    members: Vec<String>,
}

fn alignments(nodes: &[DiagramNode]) -> Vec<Alignment> {
    let containers: HashSet<&str> = nodes.iter().filter_map(|node| node.parent_id.as_deref()).collect();
    let movable: Vec<&DiagramNode> = nodes
        .iter()
        .filter(|node| !containers.contains(node.id.as_str()))
        .collect();
    Edge::ALL
        .iter()
        .flat_map(|edge| {
            // Lines form among siblings: nodes in different containers do
            // not read as one row even when they share a coordinate.
            // A circle shows no straight edge, so only its centre lines up.
            let mut groups: Vec<(f64, Option<&str>, Vec<&DiagramNode>)> = Vec::new();
            for node in movable.iter().filter(|node| !node.is_circle() || edge.is_centre()) {
                let value = edge.of(&node.bounds);
                let parent = node.parent_id.as_deref();
                match groups
                    .iter_mut()
                    .find(|(existing, container, _)| (existing - value).abs() <= 0.5 && *container == parent)
                {
                    Some((_, _, members)) => members.push(node),
                    None => groups.push((value, parent, vec![node])),
                }
            }
            groups
                .into_iter()
                .filter(|(_, _, members)| members.len() > 1)
                .map(move |(value, _, mut members)| {
                    let along = |node: &DiagramNode| {
                        if edge.is_column() {
                            node.bounds.centre().y
                        } else {
                            node.bounds.centre().x
                        }
                    };
                    members.sort_by(|a, b| along(a).total_cmp(&along(b)));
                    Alignment {
                        edge: *edge,
                        value,
                        members: members.into_iter().map(|node| node.id.clone()).collect(),
                    }
                })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: &str, x: f64, y: f64) -> DiagramNode {
        DiagramNode {
            id: id.to_owned(),
            shape: None,
            bounds: Bounds {
                x,
                y,
                width: 100.0,
                height: 48.0,
            },
            label_bounds: Vec::new(),
            parent_id: None,
            allow_overlap: false,
        }
    }

    fn diagram(nodes: Vec<DiagramNode>) -> Diagram {
        Diagram {
            nodes,
            connectors: Vec::new(),
            labels: Vec::new(),
            unsupported_elements: Vec::new(),
        }
    }

    /// Given a row of three nodes sharing their top, centre and bottom
    /// When either the middle node or the last node drops out of line by
    ///   the same amount
    /// Then the middle one costs more, as it reads as a dent in the row
    #[test]
    fn breaking_an_inner_alignment_costs_more_than_an_outer_one() {
        let row = vec![node("a", 0.0, 0.0), node("b", 150.0, 0.0), node("c", 300.0, 0.0)];
        let context = Context::new(diagram(row.clone()));
        let moved = |id: &str| {
            diagram(
                row.iter()
                    .map(|n| {
                        if n.id == id {
                            node(id, n.bounds.x, 24.0)
                        } else {
                            n.clone()
                        }
                    })
                    .collect(),
            )
        };

        assert_eq!(context.alignment_cost(&diagram(row.clone())), 0.0);
        assert!(context.alignment_cost(&moved("b")) > context.alignment_cost(&moved("c")));
        assert!(context.alignment_cost(&moved("c")) > 0.0);
    }
    /// Given a row of three nodes where only the first two are joined by a
    ///   connector
    /// When either end of the row drops out of line by the same amount
    /// Then the joined end costs more, as its alignment is the stronger one
    #[test]
    fn breaking_a_joined_alignment_costs_more_than_an_unjoined_one() {
        let row = vec![node("a", 0.0, 0.0), node("b", 150.0, 0.0), node("c", 300.0, 0.0)];
        let joined = Diagram {
            connectors: vec![crate::diagram::DiagramConnector {
                id: "ab".to_owned(),
                from: "a".to_owned(),
                to: "b".to_owned(),
                points: vec![point(100.0, 24.0), point(150.0, 24.0)],
            }],
            ..diagram(row.clone())
        };
        let context = Context::new(joined);
        let moved = |id: &str| {
            diagram(
                row.iter()
                    .map(|n| {
                        if n.id == id {
                            node(id, n.bounds.x, 24.0)
                        } else {
                            n.clone()
                        }
                    })
                    .collect(),
            )
        };

        assert!(context.alignment_cost(&moved("a")) > context.alignment_cost(&moved("c")));
        assert!(context.alignment_cost(&moved("c")) > 0.0);
    }

    /// Given two routes, one turning at a point that lies on the other
    /// When their touches are counted
    /// Then the touch counts, while a clean crossing and routes that keep
    ///   apart do not
    #[test]
    fn counts_a_route_turning_on_another_as_a_touch() {
        let corner = [point(708.0, 154.0), point(779.0, 154.0), point(779.0, 122.0)];
        let through = [point(779.0, 193.0), point(779.0, 154.0), point(1020.0, 154.0)];
        let crossing = [point(750.0, 100.0), point(750.0, 200.0)];
        let apart = [point(0.0, 0.0), point(50.0, 0.0)];

        assert!(touches(&corner, &through) > 0);
        assert_eq!(touches(&corner, &crossing), 0);
        assert_eq!(touches(&corner, &apart), 0);
    }

    /// Given two connectors entering a node's left side 8px apart, off its
    ///   centre, and the same two entering a third of the way from each corner
    /// When the spread of their ends is measured
    /// Then the even spread measures nothing and the bunched one more
    #[test]
    fn measures_how_unevenly_connectors_share_a_side() {
        let side = |first: f64, second: f64| Diagram {
            connectors: [("a", first), ("b", second)]
                .into_iter()
                .map(|(id, y)| crate::diagram::DiagramConnector {
                    id: id.to_owned(),
                    from: "source".to_owned(),
                    to: "target".to_owned(),
                    points: vec![point(0.0, y), point(150.0, y)],
                })
                .collect(),
            ..diagram(vec![node("source", -100.0, 100.0), node("target", 150.0, 100.0)])
        };

        assert!(port_unevenness(&side(116.0, 132.0)) < 1e-9);
        assert!(port_unevenness(&side(124.0, 132.0)) > 0.5);
    }

    /// Given a row of four nodes sharing their centre line
    /// When the two inner nodes drop out of line together, or the last
    ///   node alone
    /// Then the two inner ones cost a dent, more than an end leaving
    #[test]
    fn counts_a_dent_two_nodes_wide() {
        let row = vec![
            node("a", 0.0, 0.0),
            node("b", 150.0, 0.0),
            node("c", 300.0, 0.0),
            node("d", 450.0, 0.0),
        ];
        let context = Context::new(diagram(row.clone()));
        let moved = |ids: &[&str]| {
            diagram(
                row.iter()
                    .map(|n| {
                        if ids.contains(&n.id.as_str()) {
                            node(&n.id, n.bounds.x, 24.0)
                        } else {
                            n.clone()
                        }
                    })
                    .collect(),
            )
        };

        assert!(context.alignment_cost(&moved(&["b", "c"])) >= 2.0 * LOOSE_ALIGNMENT_COST + LOOSE_DENT_COST);
        assert!(context.alignment_cost(&moved(&["b", "c"])) > context.alignment_cost(&moved(&["d"])) + LOOSE_DENT_COST);
    }
}
