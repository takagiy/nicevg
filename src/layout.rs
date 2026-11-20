//! Layout: after fixing, nodes move a little so edges bend, cross
//! and crowd less, keeping the input's rough layout.
//!
//! The search is greedy. Each layout round tries small moves of single nodes —
//! lining a bent edge's ends up, or nudging the ends of edges
//! that cross, crowd or have issues — fixes the result, and keeps the move
//! that lowers the score most. Moves never add issues or jogs, never change which
//! side of another node a node is on, and stay within a few lanes of where
//! the node started.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Arc, LazyLock, Mutex, mpsc};

use regex::Regex;

use crate::diagram::{Diagram, DiagramEdge, DiagramNode, ancestor_ids, holds_end, node_group};
use crate::fix::{Changes, Draft, FIX_STEPS, FixChange, FixResult, fix_after_moves, fix_further, spread_parallel_runs};
use crate::geometry::{
    Bounds, Point, format_number, hypot, intersection, parse_number, point, points_bounds, segment_intersects_interior,
    segments,
};
use crate::route::{
    Fade, ROUTE_CLEARANCE, SHORT_END, SHORT_END_FADE, bends, chords_cross, crossings, route_cost, routes_cost,
};
use crate::xml::Element;
use crate::{InspectionReport, SvgInputError, parse};

/// How far a node may end up from where fixing left it, on each axis.
const MAX_SHIFT: f64 = 60.0;
const NUDGES: [f64; 2] = [20.0, 40.0];
const MAX_ROUNDS: usize = 12;
/// How far around a node and its edges a change makes moving the node
/// worth trying again. Measured on the example diagrams: 10px already
/// misses moves that 15px and 20px find.
const MOVE_REACH: f64 = 20.0;
/// Joins the ids of nodes whose edges a candidate reroutes together.
const TOGETHER: &str = "\u{1f}";

// Routes are scored as the route search prices them (its length, bends,
// crossings, close parallel runs and near misses, a crossing costing 300), so a
// route the search found cheaper also scores better. The costs below are
// what only layout weighs, in the same units.
const ISSUE_COST: f64 = 2000.0;
const MOVEMENT_COST: f64 = 1.0;
/// Nodes in a row or column joined by an edge running straight along
/// it show the flow with their alignment, so falling out of line costs
/// much more than for neighbours that only share a coordinate (or whose
/// edge detours).
const JOINED_ALIGNMENT_COST: f64 = 600.0;
const LOOSE_ALIGNMENT_COST: f64 = 100.0;
/// An inner node leaving a line its neighbours keep reads as an inner misalignment,
/// bending the eye twice where an end leaving bends it once; worse when
/// the node is joined to the line.
const JOINED_INNER_MISALIGNMENT_COST: f64 = 600.0;
const LOOSE_INNER_MISALIGNMENT_COST: f64 = 400.0;
/// Every node that leaves a line it was on.
const MISALIGNMENT_COST: f64 = 80.0;
/// An edge with any bend reads as a detour, on top of its bends.
const BENT_COST: f64 = 120.0;
/// A step this long or shorter between two runs heading the same way is a
/// jog: the line staggers aside instead of bending. No move may add one.
const JOG_LENGTH: f64 = 20.0;
/// What a step between two runs heading the same way costs: in full when
/// tiny, half at the jog limit above, so a step just past it still costs
/// something.
const JOG: Fade = Fade {
    start: 10.0,
    reach: 30.0,
    full: 300.0,
};
/// On top of the crossing itself, for a crossing near the node, half a
/// short end away.
const CROSSING_AT_END: Fade = Fade {
    start: 10.0,
    reach: 30.0,
    full: 200.0,
};
/// Edges sharing a node side read best spread evenly, with the same
/// gap between neighbours and to the side's corners; each even gap's worth
/// of unevenness costs this much.
const PORT_SPACING_COST: f64 = 80.0;
/// An edge alone on a node side reads best through its middle; each
/// even gap's worth of being off it costs this much.
const LONE_PORT_COST: f64 = 8.0;
/// Below this unevenness, in even gaps, a lone edge end counts as
/// centred: rerouting it in place is not worth trying.
const LONE_PORT_SLACK: f64 = 0.1;
/// A moved node keeps this far from other nodes, its parent's sides and
/// other nodes' labels, unless it was already closer.
const NODE_GAP: f64 = 20.0;
const LABEL_CLEARANCE: f64 = 4.0;

/// Fixes the diagram, then moves nodes a little where that leaves fewer
/// bends, crossings and crowded lanes.
#[tracing::instrument(skip_all)]
pub fn layout(svg: &str) -> Result<FixResult, SvgInputError> {
    let fixed = fix_after_moves(Draft::new(parse(svg)?, Changes::default(), Vec::new()), FIX_STEPS);
    let context = Context::new(fixed.report().diagram.clone());
    let start = State {
        score: context.score(fixed.report(), &HashMap::new()),
        result: fixed,
        offsets: HashMap::new(),
    };
    // The scope waits for the workers and the threads freeing old
    // candidates, so none outlives the call: a host that unloads the library
    // after it would crash them.
    let best = std::thread::scope(|scope| {
        let workers = Workers::new(scope);
        run_rounds(&context, start, MAX_ROUNDS, scope, &workers, HashMap::new(), Vec::new())
    });
    Ok(spread_parallel_runs(best.result).written())
}

struct Context {
    baseline: Diagram,
    alignments: Vec<Alignment>,
    parents: HashSet<String>,
    /// Pairs of nodes, either way round, joined by an edge running
    /// straight between them: along a row (`false`) or a column (`true`).
    joined: HashSet<(String, String, bool)>,
}

#[derive(Clone)]
struct State {
    result: Draft,
    /// How far each moved node is from where fixing left it.
    offsets: HashMap<String, Point>,
    score: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Move {
    dx: f64,
    dy: f64,
}

#[tracing::instrument(name = "layout_round", skip_all, fields(rounds_left = rounds, candidates = tracing::field::Empty))]
fn run_rounds<'scope>(
    context: &'scope Context,
    state: State,
    rounds: usize,
    frees: &'scope std::thread::Scope<'scope, '_>,
    workers: &Workers<'scope>,
    tried: Tried,
    changed: Vec<Bounds>,
) -> State {
    if rounds == 0 {
        return state;
    }
    let candidates = candidates(context, &state);
    tracing::Span::current().record("candidates", candidates.len());
    let keeps = |next: &State| {
        next.result.report().issues.len() <= state.result.report().issues.len()
            && total_jogs(&next.result.report().diagram) <= total_jogs(&state.result.report().diagram)
    };
    // A move that did not help last round, away from everything the round
    // changed, would not help now either: it is not tried again.
    let diagram = &state.result.report().diagram;
    let key = |(id, by): &(String, Move)| (id.clone(), by.dx as i64, by.dy as i64);
    let (retried, skipped): (Vec<_>, Vec<_>) = candidates.into_iter().partition(|candidate| {
        tried.get(&key(candidate)).is_none_or(|gain| *gain < 0.0)
            || reach(diagram, &candidate.0)
                .iter()
                .any(|reach| changed.iter().any(|area| touches_area(reach, area)))
    });
    let evaluated = evaluate_all(context, &state, &retried, workers);
    let tried: Tried = skipped
        .iter()
        .map(|candidate| (key(candidate), tried[&key(candidate)]))
        .chain(retried.iter().zip(&evaluated).map(|(candidate, next)| {
            let gain = next
                .as_ref()
                .filter(|next| keeps(next))
                .map_or(f64::INFINITY, |next| next.score - state.score);
            (key(candidate), gain)
        }))
        .collect();
    // Candidates are compared after one fix step; the fix steps that
    // follow go to the best one, and to the next ones only while those
    // fix steps leave it no better than now.
    let (mut compared, dropped): (Vec<State>, Vec<State>) =
        evaluated.into_iter().flatten().partition(|next| keeps(next));
    compared.sort_by(|a, b| a.score.total_cmp(&b.score));
    let mut compared = compared.into_iter();
    let best = compared
        .by_ref()
        .map(|next| {
            let result = fix_further(next.result, FIX_STEPS - 1);
            State {
                score: context.score(result.report(), &next.offsets),
                result,
                offsets: next.offsets,
            }
        })
        .find(|next| keeps(next) && next.score < state.score - 1e-9);
    // The candidates not taken are many documents to free: let another
    // thread free them while the next round goes on.
    frees.spawn(move || drop((compared, dropped)));
    match best {
        Some(next) => {
            let changed = changed_areas(diagram, &next.result.report().diagram);
            run_rounds(context, next, rounds - 1, frees, workers, tried, changed)
        }
        None => state,
    }
}

/// What each move tried scored against the state it was tried from (lower
/// is better), infinite when it was ruled out; by node ids and rounded move.
type Tried = HashMap<(String, i64, i64), f64>;

/// Whether two areas overlap or touch; a straight segment's area is a line.
fn touches_area(first: &Bounds, second: &Bounds) -> bool {
    first.x <= second.right() && second.x <= first.right() && first.y <= second.bottom() && second.y <= first.bottom()
}

/// Where moving the nodes `id` names could change anything: around them
/// and each segment of their edges.
fn reach(diagram: &Diagram, id: &str) -> Vec<Bounds> {
    let ids: HashSet<&str> = id.split(TOGETHER).collect();
    diagram
        .nodes
        .iter()
        .filter(|node| ids.contains(node.id.as_str()))
        .map(|node| node.bounds)
        .chain(
            diagram
                .edges
                .iter()
                .filter(|c| ids.contains(c.from.as_str()) || ids.contains(c.to.as_str()))
                .flat_map(|c| segments(&c.points))
                .filter_map(|(a, b)| points_bounds(&[a, b])),
        )
        .map(|bounds| bounds.inflate(MOVE_REACH))
        .collect()
}

/// The nodes, labels and edge segments that differ between two
/// diagrams, where they were and where they are.
fn changed_areas(before: &Diagram, after: &Diagram) -> Vec<Bounds> {
    let nodes = after.nodes.iter().filter_map(|node| {
        let old = before.node(&node.id).map(|old| old.bounds);
        (old != Some(node.bounds)).then(|| vec![old, Some(node.bounds)])
    });
    let labels = after.labels.iter().filter_map(|label| {
        let old = before
            .labels
            .iter()
            .find(|old| old.id == label.id)
            .map(|old| old.bounds);
        (old != Some(label.bounds)).then(|| vec![old, Some(label.bounds)])
    });
    let edges = after.edges.iter().filter_map(|edge| {
        let old = before.edge(&edge.id).map_or_else(Vec::new, |old| segments(&old.points));
        let new = segments(&edge.points);
        (old != new).then(|| {
            let only = |these: &[(Point, Point)], those: &[(Point, Point)]| -> Vec<Option<Bounds>> {
                these
                    .iter()
                    .filter(|segment| !those.contains(segment))
                    .map(|(a, b)| points_bounds(&[*a, *b]))
                    .collect()
            };
            [only(&old, &new), only(&new, &old)].concat()
        })
    });
    nodes.chain(labels).chain(edges).flatten().flatten().collect()
}

/// Threads kept for the whole layout, each taking the next job
/// queued as soon as it is free, so no round waits for threads to start.
struct Workers<'scope> {
    jobs: mpsc::Sender<Job<'scope>>,
}

type Job<'scope> = Box<dyn FnOnce() + Send + 'scope>;

impl<'scope> Workers<'scope> {
    fn new(scope: &'scope std::thread::Scope<'scope, '_>) -> Workers<'scope> {
        let threads = std::thread::available_parallelism().map_or(1, |count| count.get());
        let (jobs, queue) = mpsc::channel::<Job<'scope>>();
        let queue = Arc::new(Mutex::new(queue));
        for _ in 0..threads {
            let queue = Arc::clone(&queue);
            scope.spawn(move || {
                // The queue closes when the layout drops its workers. The
                // lock is let go before the job runs, for others to take theirs.
                loop {
                    let job = queue.lock().expect("job queue lock").recv();
                    let Ok(job) = job else { break };
                    job();
                }
            });
        }
        Workers { jobs }
    }
}

/// Evaluates the candidates on the workers, the likely longest first,
/// since some take many times longer than others; the outcome does not
/// depend on which thread evaluates which.
#[tracing::instrument(skip_all)]
fn evaluate_all<'scope>(
    context: &'scope Context,
    state: &State,
    candidates: &[(String, Move)],
    workers: &Workers<'scope>,
) -> Vec<Option<State>> {
    // A move reroutes the edges of the nodes it moves.
    let diagram = &state.result.report().diagram;
    let work = |(id, _): &(String, Move)| {
        let ids: HashSet<&str> = id.split(TOGETHER).collect();
        diagram
            .edges
            .iter()
            .filter(|c| ids.contains(c.from.as_str()) || ids.contains(c.to.as_str()))
            .count()
    };
    // Rerouting in place depends on the nodes a candidate names only through
    // the edges it reroutes: candidates rerouting the same edges
    // share one evaluation (proofs/NicevgProofs/Shared.lean).
    let mut first_with: HashMap<Vec<usize>, usize> = HashMap::new();
    let same_as: Vec<usize> = candidates
        .iter()
        .enumerate()
        .map(|(index, (id, by))| {
            if by.dx != 0.0 || by.dy != 0.0 {
                return index;
            }
            *first_with.entry(rerouted_with(diagram, id)).or_insert(index)
        })
        .collect();
    let mut order: Vec<usize> = (0..candidates.len())
        .filter(|index| same_as[*index] == *index)
        .collect();
    order.sort_by_key(|index| std::cmp::Reverse(work(&candidates[*index])));
    let parent = tracing::Span::current();
    let shared = Arc::new(state.clone());
    let (results, received) = mpsc::channel();
    for index in order {
        let (state, results, parent) = (Arc::clone(&shared), results.clone(), parent.clone());
        let (id, by) = candidates[index].clone();
        let job: Job<'scope> = Box::new(move || {
            let _thread = tracing::info_span!(parent: &parent, "evaluate_thread").entered();
            // The round only stops listening once every result is in.
            let _ = results.send((index, evaluate(context, &state, &id, by)));
        });
        workers.jobs.send(job).expect("the workers run while arranging");
    }
    drop(results);
    let mut evaluated: Vec<Option<Option<State>>> = vec![None; candidates.len()];
    for (index, state) in received.iter() {
        evaluated[index] = Some(state);
    }
    same_as
        .iter()
        .map(|index| evaluated[*index].clone().expect("every evaluation finishes"))
        .collect()
}

/// The edges, by index, that rerouting the nodes `id` names in place
/// reroutes: those with an end at one of them or inside one.
fn rerouted_with(diagram: &Diagram, id: &str) -> Vec<usize> {
    let ids: HashSet<&str> = id.split(TOGETHER).collect();
    let moved = |node: &str| {
        std::iter::once(node)
            .chain(ancestor_ids(&diagram.nodes, node))
            .any(|id| ids.contains(id))
    };
    diagram
        .edges
        .iter()
        .enumerate()
        .filter(|(_, edge)| moved(&edge.from) || moved(&edge.to))
        .map(|(index, _)| index)
        .collect()
}

/// Moves `id` by `by` and gives the result one fix step, rerouting
/// the node's edges. A zero move reroutes them in place, as the nodes
/// around them may have moved since they were last routed.
#[tracing::instrument(skip_all)]
fn evaluate(context: &Context, state: &State, id: &str, by: Move) -> Option<State> {
    let in_place = by.dx == 0.0 && by.dy == 0.0;
    let ids: Vec<String> = id.split(TOGETHER).map(str::to_owned).collect();
    let mut document = state.result.document.clone();
    let mut offsets = state.offsets.clone();
    let mut groups = Vec::new();
    if !in_place {
        for member in &ids {
            let group = node_group(&document, member)?;
            let transform = translated(document.element(&group).attr("transform"), by);
            document = document.update(&group, |g| {
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
            groups.push(group);
            let offset = offsets.entry(member.clone()).or_insert(point(0.0, 0.0));
            *offset = point(offset.x + by.dx, offset.y + by.dy);
        }
    }
    let names = ids.iter().map(|id| format!("\"{id}\"")).collect::<Vec<_>>().join(", ");
    let change = FixChange {
        code: "layout-node".to_owned(),
        message: format!(
            "Moved {} {names} by ({}, {}) so edges bend, cross and crowd less.",
            if ids.len() > 1 { "nodes" } else { "node" },
            format_number(by.dx),
            format_number(by.dy)
        ),
        elements: ids.clone(),
    };
    let draft = if in_place {
        state.result.moving(ids)
    } else {
        state
            .result
            .with_groups_moved(document, state.result.changes.with(change), ids, groups)
    };
    let result = fix_after_moves(draft, 1);
    Some(State {
        score: context.score(result.report(), &offsets),
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
fn translated(existing: &str, by: Move) -> String {
    let existing = existing.trim();
    match TRAILING_TRANSLATE.captures(existing) {
        Some(captures) => {
            let x = parse_number(&captures[1]) + by.dx;
            let y = captures.get(2).map_or(0.0, |m| parse_number(m.as_str())) + by.dy;
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
            format_number(by.dx),
            format_number(by.dy)
        ),
    }
}

/// Moves worth trying: rerouting the edges of a bent edge's ends,
/// and of a node with an edge alone on a side off its middle, in place, lining up the ends of bent edges, moving nodes
/// off the straight line between a bent edge's lined-up ends, and
/// nudging the nodes at the ends of edges that cross, crowd or have
/// issues.
#[tracing::instrument(skip_all)]
fn candidates(context: &Context, state: &State) -> Vec<(String, Move)> {
    let diagram = &state.result.report().diagram;
    let node = |id: &str| diagram.node(id);
    let straighten = diagram.edges.iter().filter(|c| bends(&c.points) > 0).flat_map(|edge| {
        let (first, last) = (edge.points[0], edge.points[edge.points.len() - 1]);
        [(&edge.to, &edge.from, last, first), (&edge.from, &edge.to, first, last)]
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
                .map(|by| (moving.id.clone(), by))
            })
            .collect::<Vec<_>>()
    });
    let unblock = diagram.edges.iter().filter(|c| bends(&c.points) > 0).flat_map(|edge| {
        let (first, last) = (edge.points[0], edge.points[edge.points.len() - 1]);
        let reach = ROUTE_CLEARANCE + 1.0;
        diagram
            .nodes
            .iter()
            .filter(|node| !holds_end(&diagram.nodes, edge, node))
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
                moves.into_iter().map(|by| (node.id.clone(), by))
            })
            .collect::<Vec<_>>()
    });
    let troubled = troubled_nodes(state.result.report());
    // A row or column with a troubled node moves as one, across its line:
    // it keeps its alignment while making room above or beside it.
    let lines = context
        .alignments
        .iter()
        .filter(|alignment| alignment.members.iter().any(|id| troubled.contains(id)))
        .flat_map(|alignment| {
            let id = alignment.members.join(TOGETHER);
            let column = alignment.line.is_column();
            NUDGES.iter().flat_map(move |amount| {
                [*amount, -amount].map(|amount| {
                    let by = if column {
                        Move { dx: amount, dy: 0.0 }
                    } else {
                        Move { dx: 0.0, dy: amount }
                    };
                    (id.clone(), by)
                })
            })
        })
        .collect::<Vec<_>>();
    let nudges = troubled.into_iter().flat_map(|id| {
        NUDGES.iter().flat_map(move |amount| {
            [
                Move { dx: *amount, dy: 0.0 },
                Move { dx: -amount, dy: 0.0 },
                Move { dx: 0.0, dy: *amount },
                Move { dx: 0.0, dy: -amount },
            ]
            .map(|by| (id.clone(), by))
        })
    });
    // An edge alone on a side off its middle may be rerouted through it.
    let mut off_centre: Vec<String> = uneven_sides(diagram)
        .into_iter()
        .filter(|(_, ends, unevenness)| *ends == 1 && *unevenness > LONE_PORT_SLACK)
        .map(|(id, _, _)| id.to_owned())
        .collect();
    off_centre.dedup();
    let reroute = diagram
        .edges
        .iter()
        .filter(|c| bends(&c.points) > 0)
        .flat_map(|c| [c.from.clone(), c.to.clone()])
        .chain(off_centre)
        .map(|id| (id, Move { dx: 0.0, dy: 0.0 }));
    // Edges that cross or touch may each be stuck where the other
    // runs: rerouting all their ends at once lets both find new ways.
    let edges = &diagram.edges;
    let together = edges.iter().enumerate().flat_map(|(index, first)| {
        edges[index + 1..]
            .iter()
            .filter(move |second| crossings(&first.points, &second.points) + touches(&first.points, &second.points) > 0)
            .map(move |second| {
                let mut ends = vec![
                    first.from.clone(),
                    first.to.clone(),
                    second.from.clone(),
                    second.to.clone(),
                ];
                ends.sort();
                ends.dedup();
                (ends.join(TOGETHER), Move { dx: 0.0, dy: 0.0 })
            })
    });
    // Candidates in the order found, each once.
    let mut seen: Vec<(String, Move)> = Vec::new();
    let mut found: HashSet<(String, i64, i64)> = HashSet::new();
    for (id, by) in reroute
        .chain(together)
        .chain(straighten)
        .chain(unblock)
        .chain(nudges)
        .chain(lines)
    {
        let rounded = Move {
            dx: by.dx.round(),
            dy: by.dy.round(),
        };
        let in_place = rounded.dx == 0.0 && rounded.dy == 0.0;
        let key = (id.clone(), rounded.dx as i64, rounded.dy as i64);
        if found.insert(key) && (in_place || context.allows(state, &id, rounded)) {
            seen.push((id, rounded));
        }
    }
    seen
}

/// Nodes at the ends of edges that cross, crowd or touch another,
/// that bend twice or more, that meet the node with a very short stub or
/// are crossed right beside it, or that an issue names (directly, or
/// through its edge or label).
fn troubled_nodes(report: &InspectionReport) -> Vec<String> {
    let diagram = &report.diagram;
    let ends = |id: &str| {
        diagram
            .edge(id)
            .map(|edge| vec![edge.from.clone(), edge.to.clone()])
            .unwrap_or_default()
    };
    let edges = &diagram.edges;
    let tangled = edges.iter().enumerate().flat_map(|(index, first)| {
        edges[index + 1..]
            .iter()
            .filter(move |second| routes_cost(&first.points, &second.points) > 0.0)
            .flat_map(move |second| [ends(&first.id), ends(&second.id)].concat())
    });
    let named = report.issues.iter().flat_map(|issue| &issue.elements).flat_map(|id| {
        if diagram.node(id).is_some() {
            vec![id.clone()]
        } else if diagram.edge(id).is_some() {
            ends(id)
        } else {
            diagram
                .labels
                .iter()
                .find(|label| label.id == *id)
                .and_then(|label| label.edge.as_deref())
                .map(ends)
                .unwrap_or_default()
        }
    });
    // An edge that bends twice or more might bend less with one of its
    // ends a little further along.
    let winding = edges
        .iter()
        .filter(|edge| edge.points.len() > 3)
        .flat_map(|edge| [edge.from.clone(), edge.to.clone()]);
    // Where an edge meets its node with a stub too short to read, or
    // is crossed right beside the node, the node is worth nudging so the
    // edge arrives more clearly.
    let unclear = edges.iter().flat_map(|edge| {
        end_segments(edge)
            .into_iter()
            .filter(|(_, end, inner)| {
                let short = (end.x - inner.x).abs() + (end.y - inner.y).abs() < SHORT_END;
                short || crossed_near(edges, &edge.id, *end, *inner, SHORT_END).is_some()
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
            parents: baseline
                .nodes
                .iter()
                .filter_map(|node| node.parent_id.clone())
                .collect(),
            joined: baseline
                .edges
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

    /// Whether `id`, or every node of a group joined by `TOGETHER`, may move
    /// by `step`; nodes moving together are not checked against each other.
    fn allows(&self, state: &State, id: &str, by: Move) -> bool {
        let group: Vec<&str> = id.split(TOGETHER).collect();
        group.iter().all(|member| self.allows_one(state, member, &group, by))
    }

    /// Whether moving `id` by `step` keeps it within reach of where it
    /// started, inside its parent, and on the same side of every other
    /// node outside its `group` as in the input.
    fn allows_one(&self, state: &State, id: &str, group: &[&str], by: Move) -> bool {
        let diagram = &state.result.report().diagram;
        let (Some(node), Some(original)) = (diagram.node(id), self.baseline_node(id)) else {
            return false;
        };
        if self.parents.contains(id) {
            return false;
        }
        let offset = state.offsets.get(id).copied().unwrap_or(point(0.0, 0.0));
        if (offset.x + by.dx).abs() > MAX_SHIFT || (offset.y + by.dy).abs() > MAX_SHIFT {
            return false;
        }
        let moved = Bounds {
            x: node.bounds.x + by.dx,
            y: node.bounds.y + by.dy,
            ..node.bounds
        };
        let Some(grown) = grown_ancestors(diagram, node, moved) else {
            return false;
        };
        // A parent grown round the move keeps as clear of its siblings as a node moved does.
        let grown_clear = grown.iter().all(|(parent, bounds)| {
            diagram
                .nodes
                .iter()
                .filter(|other| other.parent_id == parent.parent_id && other.id != parent.id)
                .all(|other| gap(bounds, &other.bounds) >= gap(&parent.bounds, &other.bounds).min(NODE_GAP))
        });
        let ancestors = ancestor_ids(&diagram.nodes, id);
        // An edge the node lands on crosses it and is rerouted, but one
        // left grazing its outline, inside the clearance yet outside the node,
        // stays where it is: the node must not come to rest against one.
        let clear_of_edges = diagram
            .edges
            .iter()
            .filter(|edge| !holds_end(&diagram.nodes, edge, node))
            .all(|edge| {
                let grazes = |bounds: &Bounds| {
                    route_enters(&edge.points, &bounds.inflate(ROUTE_CLEARANCE - 0.5))
                        && !route_enters(&edge.points, bounds)
                };
                !grazes(&moved) || grazes(&node.bounds)
            });
        grown_clear
            && clear_of_edges
            && diagram
                .nodes
                .iter()
                .filter(|other| !group.contains(&other.id.as_str()))
                .all(|other| {
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

    #[tracing::instrument(skip_all)]
    fn score(&self, report: &InspectionReport, offsets: &HashMap<String, Point>) -> f64 {
        let edges = &report.diagram.edges;
        let pairs = edges
            .iter()
            .enumerate()
            .flat_map(|(index, first)| edges[index + 1..].iter().map(move |second| (first, second)));
        let routes: f64 = edges.iter().map(|c| route_cost(&c.points)).sum::<f64>()
            + pairs
                .map(|(first, second)| routes_cost(&first.points, &second.points))
                .sum::<f64>();
        let bent = edges.iter().filter(|c| bends(&c.points) > 0).count();
        let movement: f64 = offsets.values().map(|offset| offset.x.abs() + offset.y.abs()).sum();
        let sides = uneven_sides(&report.diagram);
        report.issues.len() as f64 * ISSUE_COST
            + routes
            + bent as f64 * BENT_COST
            + jog_cost(&report.diagram)
            + port_unevenness(&sides) * PORT_SPACING_COST
            + lone_port_offset(&sides) * LONE_PORT_COST
            + unclear_ends(edges)
            + movement * MOVEMENT_COST
            + self.alignment_cost(&report.diagram)
    }

    /// The cost of the alignments a diagram breaks: each pair in a line
    /// joined by an edge that fell out of it, each pair of unjoined
    /// neighbours that did, and an inner misalignment for each inner node whose
    /// neighbours still line up. A pair or node counts once per direction,
    /// whether it left the line's top, centre or bottom, or all three.
    fn alignment_cost(&self, diagram: &Diagram) -> f64 {
        let mut costs: HashMap<(String, String, bool), f64> = HashMap::new();
        let mut charge = |key: (String, String, bool), cost: f64| {
            let entry = costs.entry(key).or_insert(0.0);
            *entry = entry.max(cost);
        };
        for alignment in &self.alignments {
            let column = alignment.line.is_column();
            let joined = |a: &String, b: &String| self.joined.contains(&(a.clone(), b.clone(), column));
            let line = |id: &String| diagram.node(id).map(|node| alignment.line.of(&node.bounds));
            let in_line = |a: &String, b: &String| match (line(a), line(b)) {
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
            // An inner misalignment: an inner run of nodes, one or more wide, off the line
            // while the nodes either side of it stay on it.
            let on_line: Vec<bool> = members
                .iter()
                .map(|id| line(id).is_none_or(|value| (value - alignment.value).abs() <= 0.5))
                .collect();
            // Each node off the line counts too, so peeling a row off one
            // node at a time is not free after the first break.
            for (id, on) in members.iter().zip(&on_line) {
                if !on {
                    charge((id.clone(), "off".to_owned(), column), MISALIGNMENT_COST);
                }
            }
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
                    let cost = if tied {
                        JOINED_INNER_MISALIGNMENT_COST
                    } else {
                        LOOSE_INNER_MISALIGNMENT_COST
                    };
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

/// The ancestors that moving `node` to `moved` grows, each with the bounds it
/// grows to, as fixing grows them: a parent the moved node would come nearer
/// the outline of than its padding (unless it already was), then that
/// parent's own parent, and so on. None when one of them cannot grow.
fn grown_ancestors<'a>(
    diagram: &'a Diagram,
    node: &DiagramNode,
    moved: Bounds,
) -> Option<Vec<(&'a DiagramNode, Bounds)>> {
    let mut grown = Vec::new();
    let (mut inner, mut before) = (moved, node.bounds);
    let mut parent_id = node.parent_id.as_deref();
    while let Some(parent) = parent_id.and_then(|id| diagram.node(id)) {
        let padding = parent.child_padding();
        let fits = paddings(&inner, &parent.bounds)
            .iter()
            .zip(paddings(&before, &parent.bounds))
            .all(|(after, before)| *after >= before.min(padding));
        if fits {
            break;
        }
        if !parent.grows() {
            return None;
        }
        let bounds = parent.grown_round(&inner);
        grown.push((parent, bounds));
        (inner, before) = (bounds, parent.bounds);
        parent_id = parent.parent_id.as_deref();
    }
    Some(grown)
}

/// Distances from a node's sides to its parent's: left, top, right,
/// bottom.
fn paddings(node: &Bounds, parent: &Bounds) -> [f64; 4] {
    [
        node.x - parent.x,
        node.y - parent.y,
        parent.right() - node.right(),
        parent.bottom() - node.bottom(),
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
/// two read as one line bending instead of two lines meeting.
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

/// How unevenly the edge ends on each node side are spread, summed
/// over sides with two or more ends: the gaps between neighbours and to
/// the corners against an even split, in even gaps.
fn port_unevenness(sides: &[(&str, usize, f64)]) -> f64 {
    sides
        .iter()
        .filter(|(_, ends, _)| *ends >= 2)
        .map(|(_, _, unevenness)| unevenness)
        .sum()
}

/// How far the edge ends alone on their node sides are from the
/// middle, summed and measured the same way.
fn lone_port_offset(sides: &[(&str, usize, f64)]) -> f64 {
    sides
        .iter()
        .filter(|(_, ends, _)| *ends == 1)
        .map(|(_, _, unevenness)| unevenness)
        .sum()
}

/// Each node side edges meet: its node, how many ends it has and
/// how unevenly they are spread, in even gaps.
fn uneven_sides(diagram: &Diagram) -> Vec<(&str, usize, f64)> {
    let ends = diagram.edges.iter().filter(|c| c.points.len() >= 2).flat_map(|edge| {
        let points = &edge.points;
        [
            (edge.from.as_str(), points[0], points[1]),
            (edge.to.as_str(), points[points.len() - 1], points[points.len() - 2]),
        ]
    });
    // Ordered, so the sums over sides and the candidates come out the same every run.
    let mut sides: BTreeMap<(&str, u8), (f64, f64, Vec<f64>)> = BTreeMap::new();
    for (id, end, inner) in ends {
        let Some(node) = diagram.node(id) else { continue };
        let b = &node.bounds;
        // The side an edge meets is the one it arrives at, along its
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
        .into_iter()
        // A side of no length has no room to spread ends over.
        .filter(|(_, (low, high, _))| low < high)
        .map(|((id, _), (low, high, mut positions))| {
            positions.sort_by(f64::total_cmp);
            let even = (high - low) / (positions.len() + 1) as f64;
            let marks: Vec<f64> = [low]
                .into_iter()
                .chain(positions.iter().copied())
                .chain([high])
                .collect();
            let unevenness = marks
                .windows(2)
                .map(|pair| (pair[1] - pair[0] - even).abs())
                .sum::<f64>()
                / even;
            (id, positions.len(), unevenness)
        })
        .collect()
}

/// The cost of edge ends that are hard to see arrive: stubs shorter
/// than a short end, and crossings nearer the node than one.
fn unclear_ends(edges: &[DiagramEdge]) -> f64 {
    edges
        .iter()
        .flat_map(|edge| {
            end_segments(edge)
                .into_iter()
                .map(|(_, end, inner)| {
                    let length = (end.x - inner.x).abs() + (end.y - inner.y).abs();
                    SHORT_END_FADE.cost(length)
                        + crossed_near(edges, &edge.id, end, inner, CROSSING_AT_END.reach)
                            .map_or(0.0, |distance| CROSSING_AT_END.cost(distance))
                })
                .collect::<Vec<_>>()
        })
        .sum()
}

/// An edge's first and last segments, with the node each meets: the
/// node, the point on it and the segment's other end.
fn end_segments(edge: &DiagramEdge) -> Vec<(&str, Point, Point)> {
    let points = &edge.points;
    if points.len() < 2 {
        return Vec::new();
    }
    vec![
        (edge.from.as_str(), points[0], points[1]),
        (edge.to.as_str(), points[points.len() - 1], points[points.len() - 2]),
    ]
}

/// How close to `end` another edge crosses the segment from `end` to
/// `inner`, when that is nearer than `within`.
fn crossed_near(edges: &[DiagramEdge], id: &str, end: Point, inner: Point, within: f64) -> Option<f64> {
    edges
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
        .filter(|distance| *distance < within)
        .reduce(f64::min)
}

/// What the steps between runs heading the same way cost, each fading as
/// the step grows.
fn jog_cost(diagram: &Diagram) -> f64 {
    diagram
        .edges
        .iter()
        .flat_map(|c| c.points.windows(4))
        .filter(|run| heads_on(run))
        .map(|run| JOG.cost((run[1].x - run[2].x).abs() + (run[1].y - run[2].y).abs()))
        .sum()
}

/// Whether the first and last segments of four points head the same way.
fn heads_on(run: &[Point]) -> bool {
    (run[1].x - run[0].x) * (run[3].x - run[2].x) + (run[1].y - run[0].y) * (run[3].y - run[2].y) > 0.0
}

fn total_jogs(diagram: &Diagram) -> usize {
    diagram.edges.iter().map(|c| jogs(&c.points)).sum()
}

/// Segments no longer than a jog between two runs of a route heading the
/// same way; a short segment where the route bends back is no jog.
fn jogs(points: &[Point]) -> usize {
    points
        .windows(4)
        .filter(|run| heads_on(run) && (run[1].x - run[2].x).abs() + (run[1].y - run[2].y).abs() <= JOG_LENGTH)
        .count()
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Line {
    CentreX,
    CentreY,
    Left,
    Right,
    Top,
    Bottom,
}

impl Line {
    const ALL: [Line; 6] = [
        Line::CentreX,
        Line::CentreY,
        Line::Left,
        Line::Right,
        Line::Top,
        Line::Bottom,
    ];

    fn of(self, bounds: &Bounds) -> f64 {
        match self {
            Line::CentreX => bounds.x + bounds.width / 2.0,
            Line::CentreY => bounds.y + bounds.height / 2.0,
            Line::Left => bounds.x,
            Line::Right => bounds.right(),
            Line::Top => bounds.y,
            Line::Bottom => bounds.bottom(),
        }
    }

    fn is_centre(self) -> bool {
        matches!(self, Line::CentreX | Line::CentreY)
    }

    /// Whether nodes aligned on this line line up in a column.
    fn is_column(self) -> bool {
        matches!(self, Line::CentreX | Line::Left | Line::Right)
    }
}

/// Nodes sharing a side or centre line, listed along the line.
struct Alignment {
    line: Line,
    /// Where the line ran after fixing.
    value: f64,
    members: Vec<String>,
}

fn alignments(nodes: &[DiagramNode]) -> Vec<Alignment> {
    let parents: HashSet<&str> = nodes.iter().filter_map(|node| node.parent_id.as_deref()).collect();
    let movable: Vec<&DiagramNode> = nodes
        .iter()
        .filter(|node| !parents.contains(node.id.as_str()))
        .collect();
    Line::ALL
        .iter()
        .flat_map(|line| {
            // Lines form among siblings: nodes in different parents do
            // not read as one row even when they share a coordinate.
            // A circle shows no straight line, so only its centre lines up.
            let mut groups: Vec<(f64, Option<&str>, Vec<&DiagramNode>)> = Vec::new();
            for node in movable.iter().filter(|node| !node.is_circle() || line.is_centre()) {
                let value = line.of(&node.bounds);
                let parent = node.parent_id.as_deref();
                match groups
                    .iter_mut()
                    .find(|(existing, owner, _)| (existing - value).abs() <= 0.5 && *owner == parent)
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
                        if line.is_column() {
                            node.bounds.centre().y
                        } else {
                            node.bounds.centre().x
                        }
                    };
                    members.sort_by(|a, b| along(a).total_cmp(&along(b)));
                    Alignment {
                        line: *line,
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
            custom: false,
            ports: Vec::new(),
            padding: None,
            redrawn: false,
        }
    }

    fn diagram(nodes: Vec<DiagramNode>) -> Diagram {
        Diagram {
            nodes,
            edges: Vec::new(),
            labels: Vec::new(),
            unsupported_elements: Vec::new(),
        }
    }

    /// Given a row of three nodes sharing their top, centre and bottom
    /// When either the middle node or the last node drops out of line by
    ///   the same amount
    /// Then the middle one costs more, as it reads as an inner misalignment in the row
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
    ///   edge
    /// When either end of the row drops out of line by the same amount
    /// Then the joined end costs more, as its alignment is the stronger one
    #[test]
    fn breaking_a_joined_alignment_costs_more_than_an_unjoined_one() {
        let row = vec![node("a", 0.0, 0.0), node("b", 150.0, 0.0), node("c", 300.0, 0.0)];
        let joined = Diagram {
            edges: vec![crate::diagram::DiagramEdge {
                id: "ab".to_owned(),
                from: "a".to_owned(),
                to: "b".to_owned(),
                points: vec![point(100.0, 24.0), point(150.0, 24.0)],
                from_port: None,
                to_port: None,
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

    /// Given routes stepping 10px and 20px aside between two runs in the
    ///   same direction, and a route bending back 10px further on
    /// When their jogs are counted
    /// Then both steps are jogs and the bend back is not
    #[test]
    fn counts_only_a_step_between_runs_the_same_way_as_a_jog() {
        let by = |aside: f64| {
            [
                point(0.0, 0.0),
                point(100.0, 0.0),
                point(100.0, aside),
                point(200.0, aside),
            ]
        };
        let back = [point(0.0, 0.0), point(100.0, 0.0), point(100.0, 10.0), point(0.0, 10.0)];

        assert_eq!(jogs(&by(10.0)), 1);
        assert_eq!(jogs(&by(20.0)), 1);
        assert_eq!(jogs(&back), 0);
    }

    /// Given two routes, one bending at a point that lies on the other
    /// When their touches are counted
    /// Then the touch counts, while a clean crossing and routes that keep
    ///   apart do not
    #[test]
    fn counts_a_route_bending_on_another_as_a_touch() {
        let corner = [point(708.0, 154.0), point(779.0, 154.0), point(779.0, 122.0)];
        let through = [point(779.0, 193.0), point(779.0, 154.0), point(1020.0, 154.0)];
        let crossing = [point(750.0, 100.0), point(750.0, 200.0)];
        let apart = [point(0.0, 0.0), point(50.0, 0.0)];

        assert!(touches(&corner, &through) > 0);
        assert_eq!(touches(&corner, &crossing), 0);
        assert_eq!(touches(&corner, &apart), 0);
    }

    /// Given two edges entering a node's left side 8px apart, off its
    ///   centre, and the same two entering a third of the way from each corner
    /// When the spread of their ends is measured
    /// Then the even spread measures nothing and the bunched one more
    #[test]
    fn measures_how_unevenly_edges_share_a_side() {
        let side = |first: f64, second: f64| Diagram {
            edges: [("a", first), ("b", second)]
                .into_iter()
                .map(|(id, y)| crate::diagram::DiagramEdge {
                    id: id.to_owned(),
                    from: "source".to_owned(),
                    to: "target".to_owned(),
                    points: vec![point(0.0, y), point(150.0, y)],
                    from_port: None,
                    to_port: None,
                })
                .collect(),
            ..diagram(vec![node("source", -100.0, 100.0), node("target", 150.0, 100.0)])
        };

        assert!(port_unevenness(&uneven_sides(&side(116.0, 132.0))) < 1e-9);
        assert!(port_unevenness(&uneven_sides(&side(124.0, 132.0))) > 0.5);
    }

    /// Given a row of four nodes sharing their centre line
    /// When the two inner nodes drop out of line together, or the last
    ///   node alone
    /// Then the two inner ones cost an inner misalignment, more than an end leaving
    #[test]
    fn counts_a_inner_misalignment_two_nodes_wide() {
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

        assert!(
            context.alignment_cost(&moved(&["b", "c"])) >= 2.0 * LOOSE_ALIGNMENT_COST + LOOSE_INNER_MISALIGNMENT_COST
        );
        assert!(
            context.alignment_cost(&moved(&["b", "c"]))
                > context.alignment_cost(&moved(&["d"])) + LOOSE_INNER_MISALIGNMENT_COST
        );
    }

    /// Given a row of five nodes sharing their centre line
    /// When the last three drop out of line together, or the last alone
    /// Then peeling three off the end costs more than one, as the row
    ///   visibly splits in two
    #[test]
    fn peeling_more_nodes_off_a_row_costs_more() {
        let row: Vec<DiagramNode> = (0..5)
            .map(|index| node(&format!("n{index}"), 150.0 * index as f64, 0.0))
            .collect();
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

        assert!(context.alignment_cost(&moved(&["n2", "n3", "n4"])) > context.alignment_cost(&moved(&["n4"])));
    }
}
