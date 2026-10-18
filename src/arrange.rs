//! Arranging: after fixing, nodes move a little so connectors bend, cross
//! and crowd less, keeping the input's rough layout.
//!
//! The search is greedy. Each step tries small moves of single nodes —
//! lining a bent connector's ends up, or nudging the ends of connectors
//! that cross, crowd or have issues — fixes the result, and keeps the move
//! that lowers the score most. Moves never add issues or jogs, never change which
//! side of another node a node is on, and stay within a few lanes of where
//! the node started.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Arc, LazyLock, Mutex, mpsc};

use regex::Regex;

use crate::diagram::{Diagram, DiagramConnector, DiagramNode, ancestor_ids, holds_end, node_group};
use crate::fix::{Changes, Draft, FixChange, FixResult, PASSES, fix_after_moves, fix_further, spread_close_runs};
use crate::geometry::{
    Bounds, Point, format_number, hypot, intersection, parse_number, point, points_bounds, segment_intersects_interior,
    segments,
};
use crate::route::{Fade, SHORT_END, SHORT_END_FADE, chords_cross, crossings, route_cost, routes_cost, turns};
use crate::xml::Element;
use crate::{AnalysisReport, SvgInputError, parse};

/// How far a node may end up from where fixing left it, on each axis.
const MAX_SHIFT: f64 = 60.0;
const NUDGES: [f64; 2] = [20.0, 40.0];
const MAX_STEPS: usize = 12;
/// How far around a node and its connectors a change makes moving the node
/// worth trying again. Measured on the example diagrams: 10px already
/// misses moves that 15px and 20px find.
const MOVE_REACH: f64 = 20.0;
/// Joins the ids of nodes whose connectors a candidate reroutes together.
const TOGETHER: &str = "\u{1f}";

// Routes are scored as the route search prices them (its length, bends,
// crossings, close runs and near misses, a crossing costing 300), so a
// route the search found cheaper also scores better. The costs below are
// what only arranging weighs, in the same units.
const ISSUE_COST: f64 = 2000.0;
const MOVEMENT_COST: f64 = 1.0;
/// Nodes in a row or column joined by a connector running straight along
/// it show the flow with their alignment, so falling out of line costs
/// much more than for neighbours that only share a coordinate (or whose
/// connector detours).
const JOINED_ALIGNMENT_COST: f64 = 600.0;
const LOOSE_ALIGNMENT_COST: f64 = 100.0;
/// An inner node leaving a line its neighbours keep reads as a dent,
/// turning the eye twice where an end leaving turns it once; worse when
/// the node is joined to the line.
const JOINED_DENT_COST: f64 = 600.0;
const LOOSE_DENT_COST: f64 = 400.0;
/// Every node that leaves a line it was on.
const OFF_LINE_COST: f64 = 80.0;
/// A connector with any bend reads as a detour, on top of its bends.
const BENT_COST: f64 = 120.0;
/// A step this long or shorter between two runs heading the same way is a
/// jog: the line staggers aside instead of turning. No move may add one.
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
/// Connectors sharing a node side read best spread evenly, with the same
/// gap between neighbours and to the side's corners; each even gap's worth
/// of unevenness costs this much.
const PORT_SPACING_COST: f64 = 80.0;
/// A connector alone on a node side reads best through its middle; each
/// even gap's worth of being off it costs this much.
const LONE_PORT_COST: f64 = 8.0;
/// Below this unevenness, in even gaps, a lone connector end counts as
/// centred: rerouting it in place is not worth trying.
const LONE_PORT_SLACK: f64 = 0.1;
/// How far connectors keep from nodes they pass.
const CLEARANCE: f64 = 8.0;
/// A moved node keeps this far from other nodes, its container's edges and
/// other nodes' labels, unless it was already closer.
const NODE_GAP: f64 = 20.0;
const CONTAINER_MARGIN: f64 = 12.0;
const LABEL_CLEARANCE: f64 = 4.0;

/// Fixes the diagram, then moves nodes a little where that leaves fewer
/// bends, crossings and crowded lanes.
#[tracing::instrument(skip_all)]
pub fn arrange(svg: &str) -> Result<FixResult, SvgInputError> {
    arranged(svg, |_| {})
}

/// Arranges as `arrange` does, with the context `prepare` has set up.
fn arranged(svg: &str, prepare: impl FnOnce(&mut Context)) -> Result<FixResult, SvgInputError> {
    let fixed = fix_after_moves(Draft::new(parse(svg)?, Changes::default(), Vec::new()), PASSES);
    #[allow(unused_mut)]
    let mut context = Context::new(fixed.report().diagram.clone());
    prepare(&mut context);
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
        improve(&context, start, MAX_STEPS, scope, &workers, HashMap::new(), Vec::new())
    });
    Ok(spread_close_runs(best.result).written())
}

struct Context {
    baseline: Diagram,
    alignments: Vec<Alignment>,
    containers: HashSet<String>,
    /// Pairs of nodes, either way round, joined by a connector running
    /// straight between them: along a row (`false`) or a column (`true`).
    joined: HashSet<(String, String, bool)>,
    #[cfg(feature = "trace-candidates")]
    step: trace::Step,
    /// The model screening the candidates, when arranging with one, and
    /// what the steps so far left for its features.
    #[cfg(feature = "screening")]
    screen: Option<screening::Screen>,
    #[cfg(feature = "screening")]
    history: std::sync::Mutex<screening::History>,
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

#[tracing::instrument(name = "arrange_step", skip_all, fields(steps_left = steps, candidates = tracing::field::Empty))]
fn improve<'scope>(
    context: &'scope Context,
    state: State,
    steps: usize,
    frees: &'scope std::thread::Scope<'scope, '_>,
    workers: &Workers<'scope>,
    tried: Tried,
    changed: Vec<Bounds>,
) -> State {
    if steps == 0 {
        return state;
    }
    let candidates = candidates(context, &state);
    tracing::Span::current().record("candidates", candidates.len());
    // The model's features need every candidate, the ones not tried again too.
    #[cfg(feature = "screening")]
    let all = context.screen.as_ref().map(|_| candidates.clone());
    let keeps = |next: &State| {
        next.result.report().issues.len() <= state.result.report().issues.len()
            && total_jogs(&next.result.report().diagram) <= total_jogs(&state.result.report().diagram)
    };
    // A move that did not help last step, away from everything the step
    // changed, would not help now either: it is not tried again.
    let diagram = &state.result.report().diagram;
    let key = |(id, step): &(String, Move)| (id.clone(), step.dx as i64, step.dy as i64);
    // Recording tries every candidate, so each gets its label.
    let (retried, skipped): (Vec<_>, Vec<_>) = candidates.into_iter().partition(|candidate| {
        cfg!(feature = "trace-candidates")
            || tried.get(&key(candidate)).is_none_or(|gain| *gain < 0.0)
            || changed_areas_touch(diagram, &candidate.0, &changed)
    });
    // With a model, only the ones it ranks first are evaluated.
    #[cfg(feature = "screening")]
    let retried = match (&context.screen, &all) {
        (Some(screen), Some(all)) => screening::kept(context, screen, &state, steps, all, retried, &changed),
        _ => retried,
    };
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
    // Candidates are compared after one pass of fixing; the passes that
    // follow go to the best one, and to the next ones only while those
    // passes leave it no better than now.
    #[cfg(any(feature = "trace-candidates", feature = "screening"))]
    let gains: Vec<Option<f64>> = evaluated
        .iter()
        .map(|next| {
            next.as_ref()
                .filter(|next| keeps(next))
                .map(|next| next.score - state.score)
        })
        .collect();
    // Each kept candidate with its place among those tried.
    let (mut compared, dropped): (Vec<_>, Vec<_>) = evaluated
        .into_iter()
        .enumerate()
        .filter_map(|(index, next)| Some((index, next?)))
        .partition(|(_, next)| keeps(next));
    compared.sort_by(|a, b| a.1.score.total_cmp(&b.1.score));
    let mut compared = compared.into_iter();
    // Each candidate given the further passes: its score after them and
    // whether it kept to the rules.
    #[cfg(feature = "trace-candidates")]
    let mut further = Vec::new();
    let best = compared
        .by_ref()
        .map(|(index, next)| {
            let result = fix_further(next.result, PASSES - 1);
            let next = State {
                score: context.score(result.report(), &next.offsets),
                result,
                offsets: next.offsets,
            };
            #[cfg(feature = "trace-candidates")]
            further.push((index, next.score, keeps(&next)));
            (index, next)
        })
        .find(|(_, next)| keeps(next) && next.score < state.score - 1e-9);
    #[cfg(feature = "trace-candidates")]
    trace::record(
        context,
        &state,
        steps,
        &retried,
        &gains,
        best.as_ref(),
        &changed,
        &further,
    );
    #[cfg(feature = "screening")]
    if context.screen.is_some() {
        screening::remember(context, steps, &retried, &gains, best.as_ref().map(|(index, _)| *index));
    }
    let best = best.map(|(_, next)| next);
    // The candidates not taken are many documents to free: let another
    // thread free them while the next step goes on.
    frees.spawn(move || drop((compared, dropped)));
    match best {
        Some(next) => {
            let changed = changed_areas(diagram, &next.result.report().diagram);
            improve(context, next, steps - 1, frees, workers, tried, changed)
        }
        None => state,
    }
}

/// Whether anything that changed lies within reach of moving the nodes `id`
/// names.
fn changed_areas_touch(diagram: &Diagram, id: &str, changed: &[Bounds]) -> bool {
    reach(diagram, id)
        .iter()
        .any(|reach| changed.iter().any(|area| touches_area(reach, area)))
}

/// What each move tried scored against the state it was tried from (lower
/// is better), infinite when it was ruled out; by node ids and rounded step.
type Tried = HashMap<(String, i64, i64), f64>;

/// Whether two areas overlap or touch; a straight segment's area is a line.
fn touches_area(first: &Bounds, second: &Bounds) -> bool {
    first.x <= second.right() && second.x <= first.right() && first.y <= second.bottom() && second.y <= first.bottom()
}

/// Where moving the nodes `id` names could change anything: around them
/// and each segment of their connectors.
fn reach(diagram: &Diagram, id: &str) -> Vec<Bounds> {
    let ids: HashSet<&str> = id.split(TOGETHER).collect();
    diagram
        .nodes
        .iter()
        .filter(|node| ids.contains(node.id.as_str()))
        .map(|node| node.bounds)
        .chain(
            diagram
                .connectors
                .iter()
                .filter(|c| ids.contains(c.from.as_str()) || ids.contains(c.to.as_str()))
                .flat_map(|c| segments(&c.points))
                .filter_map(|(a, b)| points_bounds(&[a, b])),
        )
        .map(|bounds| bounds.inflate(MOVE_REACH))
        .collect()
}

/// The nodes, labels and connector segments that differ between two
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
    let connectors = after.connectors.iter().filter_map(|connector| {
        let old = before
            .connector(&connector.id)
            .map_or_else(Vec::new, |old| segments(&old.points));
        let new = segments(&connector.points);
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
    nodes.chain(labels).chain(connectors).flatten().flatten().collect()
}

/// Threads kept for the whole arrangement, each taking the next job
/// queued as soon as it is free, so no step waits for threads to start.
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
                // The queue closes when the arrangement drops its workers. The
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
    // A move reroutes the connectors of the nodes it moves.
    let diagram = &state.result.report().diagram;
    let work = |(id, _): &(String, Move)| {
        let ids: HashSet<&str> = id.split(TOGETHER).collect();
        diagram
            .connectors
            .iter()
            .filter(|c| ids.contains(c.from.as_str()) || ids.contains(c.to.as_str()))
            .count()
    };
    let mut order: Vec<usize> = (0..candidates.len()).collect();
    order.sort_by_key(|index| std::cmp::Reverse(work(&candidates[*index])));
    let parent = tracing::Span::current();
    let shared = Arc::new(state.clone());
    let (results, received) = mpsc::channel();
    for index in order {
        let (state, results, parent) = (Arc::clone(&shared), results.clone(), parent.clone());
        let (id, step) = candidates[index].clone();
        let job: Job<'scope> = Box::new(move || {
            let _thread = tracing::info_span!(parent: &parent, "evaluate_thread").entered();
            #[cfg(feature = "trace-candidates")]
            let next = trace::timed_evaluate(context, &state, &id, step, index);
            #[cfg(not(feature = "trace-candidates"))]
            let next = evaluate(context, &state, &id, step);
            // The step only stops listening once every result is in.
            let _ = results.send((index, next));
        });
        workers.jobs.send(job).expect("the workers run while arranging");
    }
    drop(results);
    let mut evaluated: Vec<(usize, Option<State>)> = received.iter().collect();
    assert_eq!(evaluated.len(), candidates.len(), "every evaluation finishes");
    evaluated.sort_unstable_by_key(|(index, _)| *index);
    evaluated.into_iter().map(|(_, evaluated)| evaluated).collect()
}

/// Moves `id` by `step` and gives the result one pass of fixing, rerouting
/// the node's connectors. A zero step reroutes them in place, as the nodes
/// around them may have moved since they were last routed.
#[tracing::instrument(skip_all)]
fn evaluate(context: &Context, state: &State, id: &str, step: Move) -> Option<State> {
    let (draft, offsets) = moved(state, id, step)?;
    let result = fix_after_moves(draft, 1);
    Some(State {
        score: context.score(result.report(), &offsets),
        result,
        offsets,
    })
}

/// Moves `id` by `step` with nothing fixed yet: the draft fixing starts
/// from, and every node's offset after the move.
fn moved(state: &State, id: &str, step: Move) -> Option<(Draft, HashMap<String, Point>)> {
    let in_place = step.dx == 0.0 && step.dy == 0.0;
    let ids: Vec<String> = id.split(TOGETHER).map(str::to_owned).collect();
    let mut document = state.result.document.clone();
    let mut offsets = state.offsets.clone();
    if !in_place {
        for member in &ids {
            let group = node_group(&document, member)?;
            let transform = translated(document.element(&group).attr("transform"), step);
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
            let offset = offsets.entry(member.clone()).or_insert(point(0.0, 0.0));
            *offset = point(offset.x + step.dx, offset.y + step.dy);
        }
    }
    let names = ids.iter().map(|id| format!("\"{id}\"")).collect::<Vec<_>>().join(", ");
    let change = FixChange {
        code: "arrange-node".to_owned(),
        message: format!(
            "Moved {} {names} by ({}, {}) so connectors bend, cross and crowd less.",
            if ids.len() > 1 { "nodes" } else { "node" },
            format_number(step.dx),
            format_number(step.dy)
        ),
        elements: ids.clone(),
    };
    let draft = if in_place {
        state.result.moving(ids)
    } else {
        Draft::new(document, state.result.changes.with(change), ids)
    };
    Some((draft, offsets))
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

/// Moves worth trying: rerouting the connectors of a bent connector's ends,
/// and of a node with a connector alone on a side off its middle, in place, lining up the ends of bent connectors, moving nodes
/// off the straight line between a bent connector's lined-up ends, and
/// nudging the nodes at the ends of connectors that cross, crowd or have
/// issues.
#[tracing::instrument(skip_all)]
fn candidates(context: &Context, state: &State) -> Vec<(String, Move)> {
    let diagram = &state.result.report().diagram;
    let node = |id: &str| diagram.node(id);
    let straighten = diagram
        .connectors
        .iter()
        .filter(|c| turns(&c.points) > 0)
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
        .filter(|c| turns(&c.points) > 0)
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
    let troubled = troubled_nodes(state.result.report());
    // A row or column with a troubled node moves as one, across its line:
    // it keeps its alignment while making room above or beside it.
    let lines = context
        .alignments
        .iter()
        .filter(|alignment| alignment.members.iter().any(|id| troubled.contains(id)))
        .flat_map(|alignment| {
            let id = alignment.members.join(TOGETHER);
            let column = alignment.edge.is_column();
            NUDGES.iter().flat_map(move |amount| {
                [*amount, -amount].map(|amount| {
                    let step = if column {
                        Move { dx: amount, dy: 0.0 }
                    } else {
                        Move { dx: 0.0, dy: amount }
                    };
                    (id.clone(), step)
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
            .map(|step| (id.clone(), step))
        })
    });
    // A connector alone on a side off its middle may be rerouted through it.
    let mut off_centre: Vec<String> = uneven_sides(diagram)
        .into_iter()
        .filter(|(_, ends, unevenness)| *ends == 1 && *unevenness > LONE_PORT_SLACK)
        .map(|(id, _, _)| id.to_owned())
        .collect();
    off_centre.dedup();
    let reroute = diagram
        .connectors
        .iter()
        .filter(|c| turns(&c.points) > 0)
        .flat_map(|c| [c.from.clone(), c.to.clone()])
        .chain(off_centre)
        .map(|id| (id, Move { dx: 0.0, dy: 0.0 }));
    // Connectors that cross or touch may each be stuck where the other
    // runs: rerouting all their ends at once lets both find new ways.
    let connectors = &diagram.connectors;
    let together = connectors.iter().enumerate().flat_map(|(index, first)| {
        connectors[index + 1..]
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
    for (id, step) in reroute
        .chain(together)
        .chain(straighten)
        .chain(unblock)
        .chain(nudges)
        .chain(lines)
    {
        let rounded = Move {
            dx: step.dx.round(),
            dy: step.dy.round(),
        };
        let in_place = rounded.dx == 0.0 && rounded.dy == 0.0;
        let key = (id.clone(), rounded.dx as i64, rounded.dy as i64);
        if found.insert(key) && (in_place || context.allows(state, &id, rounded)) {
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
            .filter(move |second| routes_cost(&first.points, &second.points) > 0.0)
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
                short || crossed_near(connectors, &connector.id, *end, *inner, SHORT_END).is_some()
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
            #[cfg(feature = "trace-candidates")]
            step: trace::Step::default(),
            #[cfg(feature = "screening")]
            screen: None,
            #[cfg(feature = "screening")]
            history: std::sync::Mutex::default(),
        }
    }

    fn baseline_node(&self, id: &str) -> Option<&DiagramNode> {
        self.baseline.node(id)
    }

    /// Whether `id`, or every node of a group joined by `TOGETHER`, may move
    /// by `step`; nodes moving together are not checked against each other.
    fn allows(&self, state: &State, id: &str, step: Move) -> bool {
        let group: Vec<&str> = id.split(TOGETHER).collect();
        group.iter().all(|member| self.allows_one(state, member, &group, step))
    }

    /// Whether moving `id` by `step` keeps it within reach of where it
    /// started, inside its container, and on the same side of every other
    /// node outside its `group` as in the input.
    fn allows_one(&self, state: &State, id: &str, group: &[&str], step: Move) -> bool {
        let diagram = &state.result.report().diagram;
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

    fn score(&self, report: &AnalysisReport, offsets: &HashMap<String, Point>) -> f64 {
        // Added up in this order, as before the terms were named.
        let terms = self.score_terms(report, offsets);
        terms[1..].iter().fold(terms[0], |sum, term| sum + term)
    }

    /// What the score adds up: issues, routes, bent connectors, jogs, port
    /// spacing, lone ports off centre, unclear ends, movement, alignments.
    #[tracing::instrument(name = "score", skip_all)]
    fn score_terms(&self, report: &AnalysisReport, offsets: &HashMap<String, Point>) -> [f64; 9] {
        let connectors = &report.diagram.connectors;
        let pairs = connectors
            .iter()
            .enumerate()
            .flat_map(|(index, first)| connectors[index + 1..].iter().map(move |second| (first, second)));
        let routes: f64 = connectors.iter().map(|c| route_cost(&c.points)).sum::<f64>()
            + pairs
                .map(|(first, second)| routes_cost(&first.points, &second.points))
                .sum::<f64>();
        let bent = connectors.iter().filter(|c| turns(&c.points) > 0).count();
        let movement: f64 = offsets.values().map(|offset| offset.x.abs() + offset.y.abs()).sum();
        let sides = uneven_sides(&report.diagram);
        [
            report.issues.len() as f64 * ISSUE_COST,
            routes,
            bent as f64 * BENT_COST,
            jog_cost(&report.diagram),
            port_unevenness(&sides) * PORT_SPACING_COST,
            lone_port_offset(&sides) * LONE_PORT_COST,
            unclear_ends(connectors),
            movement * MOVEMENT_COST,
            self.alignment_cost(&report.diagram),
        ]
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
            // Each node off the line counts too, so peeling a row off one
            // node at a time is not free after the first break.
            for (id, on) in members.iter().zip(&on_line) {
                if !on {
                    charge((id.clone(), "off".to_owned(), column), OFF_LINE_COST);
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
fn port_unevenness(sides: &[(&str, usize, f64)]) -> f64 {
    sides
        .iter()
        .filter(|(_, ends, _)| *ends >= 2)
        .map(|(_, _, unevenness)| unevenness)
        .sum()
}

/// How far the connector ends alone on their node sides are from the
/// middle, summed and measured the same way.
fn lone_port_offset(sides: &[(&str, usize, f64)]) -> f64 {
    sides
        .iter()
        .filter(|(_, ends, _)| *ends == 1)
        .map(|(_, _, unevenness)| unevenness)
        .sum()
}

/// Each node side connectors meet: its node, how many ends it has and
/// how unevenly they are spread, in even gaps.
fn uneven_sides(diagram: &Diagram) -> Vec<(&str, usize, f64)> {
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
    // Ordered, so the sums over sides and the candidates come out the same every run.
    let mut sides: BTreeMap<(&str, u8), (f64, f64, Vec<f64>)> = BTreeMap::new();
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
        .into_iter()
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

/// The cost of connector ends that are hard to see arrive: stubs shorter
/// than a short end, and crossings nearer the node than one.
fn unclear_ends(connectors: &[DiagramConnector]) -> f64 {
    connectors
        .iter()
        .flat_map(|connector| {
            end_segments(connector)
                .into_iter()
                .map(|(_, end, inner)| {
                    let length = (end.x - inner.x).abs() + (end.y - inner.y).abs();
                    SHORT_END_FADE.cost(length)
                        + crossed_near(connectors, &connector.id, end, inner, CROSSING_AT_END.reach)
                            .map_or(0.0, |distance| CROSSING_AT_END.cost(distance))
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
/// `inner`, when that is nearer than `within`.
fn crossed_near(connectors: &[DiagramConnector], id: &str, end: Point, inner: Point, within: f64) -> Option<f64> {
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
        .filter(|distance| *distance < within)
        .reduce(f64::min)
}

/// What the steps between runs heading the same way cost, each fading as
/// the step grows.
fn jog_cost(diagram: &Diagram) -> f64 {
    diagram
        .connectors
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
    diagram.connectors.iter().map(|c| jogs(&c.points)).sum()
}

/// Segments no longer than a jog between two runs of a route heading the
/// same way; a short segment where the route turns back is no jog.
fn jogs(points: &[Point]) -> usize {
    points
        .windows(4)
        .filter(|run| heads_on(run) && (run[1].x - run[2].x).abs() + (run[1].y - run[2].y).abs() <= JOG_LENGTH)
        .count()
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
            custom: false,
            ports: Vec::new(),
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
    ///   same direction, and a route turning back 10px further on
    /// When their jogs are counted
    /// Then both steps are jogs and the turn back is not
    #[test]
    fn counts_only_a_step_between_runs_the_same_way_as_a_jog() {
        let step = |aside: f64| {
            [
                point(0.0, 0.0),
                point(100.0, 0.0),
                point(100.0, aside),
                point(200.0, aside),
            ]
        };
        let back = [point(0.0, 0.0), point(100.0, 0.0), point(100.0, 10.0), point(0.0, 10.0)];

        assert_eq!(jogs(&step(10.0)), 1);
        assert_eq!(jogs(&step(20.0)), 1);
        assert_eq!(jogs(&back), 0);
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

/// Recording, for learning to screen candidates: each candidate tried, what
/// could be known of it before trying it, and what trying it showed.
#[cfg(any(feature = "trace-candidates", feature = "screening"))]
pub mod trace {
    #[cfg(feature = "trace-candidates")]
    use std::cell::RefCell;
    use std::collections::HashMap;
    #[cfg(feature = "trace-candidates")]
    use std::sync::Mutex;

    use serde_json::json;

    #[cfg(feature = "trace-candidates")]
    use super::{Changes, Draft};
    use super::{
        Context, MAX_STEPS, Move, State, TOGETHER, changed_areas_touch, crossings, route_cost, routes_cost,
        segment_intersects_interior, segments, turns,
    };
    use crate::diagram::{Diagram, ancestor_ids};
    use crate::geometry::Point;
    use crate::geometry::{Bounds, hypot};

    // Arranging records on the thread that called it, so arrangements on
    // other threads keep their own records and logs.
    #[cfg(feature = "trace-candidates")]
    thread_local! {
        static RECORDS: RefCell<Vec<serde_json::Value>> = const { RefCell::new(Vec::new()) };
        /// What arranging went through, to replay it without arranging
        /// again: the document it started from, each step's candidates with
        /// how they came out and the move taken, and the document now.
        static LOG: RefCell<Log> = const {
            RefCell::new(Log {
                start: String::new(),
                now: String::new(),
                steps: Vec::new(),
            })
        };
    }

    #[cfg(feature = "trace-candidates")]
    struct Log {
        start: String,
        now: String,
        steps: Vec<serde_json::Value>,
    }

    /// What the workers find of the step under way, each candidate by its
    /// place; one per arrangement.
    #[cfg(feature = "trace-candidates")]
    #[derive(Default)]
    pub(super) struct Step {
        /// How long a trial rerouting of its lines took, how many searches
        /// that ran, and how long its real evaluation took.
        times: Mutex<Vec<(usize, u128, usize, u128)>>,
        /// What its trial rerouting found, before and after the move.
        trials: Mutex<Vec<(usize, [Trial; 2])>>,
        /// What its evaluation came to, none when it could not be made.
        outcomes: Mutex<Vec<(usize, serde_json::Value)>>,
    }

    /// What a trial rerouting of a candidate's lines found: their bends,
    /// crossings with the other lines, own cost (length, bends, short
    /// ends), cost against the other lines, and the lines it found no
    /// route for.
    #[derive(Clone, Copy, Default)]
    pub(super) struct Trial {
        bends: usize,
        crossings: usize,
        cost: f64,
        against: f64,
        unrouted: usize,
    }

    /// Evaluates a candidate as arrange does, after a trial rerouting of its
    /// lines, timing both on the same worker.
    #[cfg(feature = "trace-candidates")]
    pub(super) fn timed_evaluate(
        context: &Context,
        state: &State,
        id: &str,
        step: Move,
        index: usize,
    ) -> Option<State> {
        let started = std::time::Instant::now();
        let before = trial(state, id, Move { dx: 0.0, dy: 0.0 });
        let after = trial(state, id, step);
        let searches = 2 * after.1;
        let trial_ns = started.elapsed().as_nanos();
        context
            .step
            .trials
            .lock()
            .expect("trials lock")
            .push((index, [before.0, after.0]));
        let started = std::time::Instant::now();
        let next = super::evaluate(context, state, id, step);
        let eval_ns = started.elapsed().as_nanos();
        context
            .step
            .times
            .lock()
            .expect("times lock")
            .push((index, trial_ns, searches, eval_ns));
        let outcome = next
            .as_ref()
            .map_or(serde_json::Value::Null, |next| outcome(context, state, next));
        context
            .step
            .outcomes
            .lock()
            .expect("outcomes lock")
            .push((index, outcome));
        next
    }

    /// What one pass of fixing after a move came to, beyond its score: the
    /// score's terms, the issues and jogs left, and how many connectors and
    /// labels it changed.
    #[cfg(feature = "trace-candidates")]
    fn outcome(context: &Context, state: &State, next: &State) -> serde_json::Value {
        let (before, after) = (&state.result.report().diagram, &next.result.report().diagram);
        let rerouted = after
            .connectors
            .iter()
            .filter(|c| before.connector(&c.id).is_none_or(|old| old.points != c.points))
            .count();
        let labels_moved = after
            .labels
            .iter()
            .filter(|l| {
                before
                    .labels
                    .iter()
                    .find(|old| old.id == l.id)
                    .is_none_or(|old| old.bounds != l.bounds)
            })
            .count();
        json!({
            "score": next.score,
            "terms": context.score_terms(next.result.report(), &next.offsets),
            "issues": next.result.report().issues.len(),
            "jogs": super::total_jogs(after),
            "rerouted": rerouted,
            "labels_moved": labels_moved,
        })
    }

    /// A document cut after each `>`: a move changes some tags and keeps
    /// their number, so a step is told by the tags it changes.
    #[cfg(feature = "trace-candidates")]
    fn tags(svg: &str) -> Vec<&str> {
        svg.split_inclusive('>').collect()
    }

    /// How to get from one document to the next: the tags that differ by
    /// place, or the whole next document when the number of tags changed.
    #[cfg(feature = "trace-candidates")]
    fn difference(before: &str, after: &str) -> serde_json::Value {
        let (old, new) = (tags(before), tags(after));
        if old.len() != new.len() {
            return json!({ "svg": after });
        }
        let changed: Vec<_> = old
            .iter()
            .zip(&new)
            .enumerate()
            .filter(|(_, (old, new))| old != new)
            .map(|(index, (_, new))| json!([index, new]))
            .collect();
        json!({ "tags": changed })
    }

    #[cfg(feature = "trace-candidates")]
    fn applied(before: &str, difference: &serde_json::Value) -> Result<String, String> {
        if let Some(svg) = difference["svg"].as_str() {
            return Ok(svg.to_owned());
        }
        let mut tags = tags(before);
        for change in difference["tags"].as_array().ok_or("a difference without tags")? {
            let (Some(index), Some(tag)) = (change[0].as_u64(), change[1].as_str()) else {
                return Err(format!("a tag change that cannot be read: {change}"));
            };
            *tags.get_mut(index as usize).ok_or("a tag change past the end")? = tag;
        }
        Ok(tags.concat())
    }

    /// A trial rerouting: for each line of the moved nodes, the side choice
    /// arrange's rerouting runs (a coarse search, then a fine one along the
    /// coarse route), with the nodes where the move puts them and the other
    /// lines as they are. Gives what the routes it found come to, and how
    /// many searches it ran.
    pub(super) fn trial(state: &State, id: &str, step: Move) -> (Trial, usize) {
        let diagram = &state.result.report().diagram;
        let ids: Vec<&str> = id.split(TOGETHER).collect();
        let moved = |id: &str| ids.contains(&id);
        let at = |node: &crate::diagram::DiagramNode| {
            let shift = if moved(&node.id) {
                step
            } else {
                Move { dx: 0.0, dy: 0.0 }
            };
            Bounds {
                x: node.bounds.x + shift.dx,
                y: node.bounds.y + shift.dy,
                ..node.bounds
            }
        };
        let attached: Vec<_> = diagram
            .connectors
            .iter()
            .filter(|c| moved(&c.from) || moved(&c.to))
            .collect();
        let occupied: Vec<Vec<Point>> = diagram
            .connectors
            .iter()
            .filter(|c| !moved(&c.from) && !moved(&c.to))
            .map(|c| c.points.clone())
            .collect();
        let mut searches = 0;
        let mut found = Trial::default();
        for c in attached {
            let (Some(from), Some(to)) = (diagram.node(&c.from), diagram.node(&c.to)) else {
                continue;
            };
            let obstacles: Vec<Bounds> = diagram
                .nodes
                .iter()
                .filter(|n| !crate::diagram::holds_end(&diagram.nodes, c, n))
                .map(at)
                .collect();
            let (_, route) =
                crate::route::sides_and_route(&at(from), &at(to), &obstacles, &[], &occupied, &[], &|_, _| 0.0);
            searches += 1;
            let Some(route) = route else {
                found.unrouted += 1;
                continue;
            };
            found.bends += turns(&route);
            found.crossings += occupied.iter().map(|other| crossings(&route, other)).sum::<usize>();
            found.cost += route_cost(&route);
            found.against += occupied.iter().map(|other| routes_cost(&route, other)).sum::<f64>();
        }
        (found, searches)
    }

    /// The records made since the last call, one per candidate tried.
    #[cfg(feature = "trace-candidates")]
    pub fn take() -> Vec<serde_json::Value> {
        RECORDS.take()
    }

    /// What a candidate is, from how it was made: rerouting in place, the
    /// ends of crossing connectors together, a row or column moved as one,
    /// a nudge by one of the fixed amounts, or a move lining something up.
    fn kind(id: &str, step: Move) -> &'static str {
        let (ax, ay) = (step.dx.abs(), step.dy.abs());
        let nudge = |a: f64, b: f64| a == 0.0 && (b == 20.0 || b == 40.0);
        match (ax == 0.0 && ay == 0.0, id.contains(TOGETHER)) {
            (true, false) => "reroute",
            (true, true) => "together",
            (false, true) => "line",
            _ if nudge(ax, ay) || nudge(ay, ax) => "nudge",
            _ => "align",
        }
    }

    /// How far apart two boxes are, zero when they touch or overlap.
    fn gap(a: &Bounds, b: &Bounds) -> f64 {
        let dx = (b.x - a.right()).max(a.x - b.right()).max(0.0);
        let dy = (b.y - a.bottom()).max(a.y - b.bottom()).max(0.0);
        hypot(dx, dy)
    }

    /// What moving the nodes `ids` by `step` would do, known before the
    /// connectors are rerouted: the lines that would run straight between
    /// centres, or stop doing so; how much nearer the nodes at the other
    /// ends come; the closest any other node is, before and after, and
    /// how many it would overlap; the lines of alignment it would leave;
    /// and how many other connectors it would come to rest on.
    fn destination(diagram: &Diagram, context: &Context, ids: &[&str], step: Move) -> serde_json::Value {
        let moved = |id: &str| ids.contains(&id);
        let at = |id: &str, after: bool| {
            let node = diagram.node(id)?;
            let shift = if after && moved(id) {
                step
            } else {
                Move { dx: 0.0, dy: 0.0 }
            };
            Some(Bounds {
                x: node.bounds.x + shift.dx,
                y: node.bounds.y + shift.dy,
                ..node.bounds
            })
        };
        let (mut straightened, mut unstraightened, mut nearer) = (0, 0, 0.0);
        for c in diagram.connectors.iter().filter(|c| moved(&c.from) != moved(&c.to)) {
            let (Some(a0), Some(b0), Some(a1), Some(b1)) =
                (at(&c.from, false), at(&c.to, false), at(&c.from, true), at(&c.to, true))
            else {
                continue;
            };
            let lined = |a: &Bounds, b: &Bounds| {
                let (p, q) = (a.centre(), b.centre());
                (p.x - q.x).abs() < 1.0 || (p.y - q.y).abs() < 1.0
            };
            let span = |a: &Bounds, b: &Bounds| {
                let (p, q) = (a.centre(), b.centre());
                (p.x - q.x).abs() + (p.y - q.y).abs()
            };
            match (lined(&a0, &b0), lined(&a1, &b1)) {
                (false, true) => straightened += 1,
                (true, false) => unstraightened += 1,
                _ => {}
            }
            nearer += span(&a0, &b0) - span(&a1, &b1);
        }
        // Other nodes, leaving out the containers round the moved ones and
        // the nodes inside them.
        let around: Vec<&str> = ids.iter().flat_map(|id| ancestor_ids(&diagram.nodes, id)).collect();
        let others: Vec<&Bounds> = diagram
            .nodes
            .iter()
            .filter(|n| !moved(&n.id) && !around.contains(&n.id.as_str()))
            .filter(|n| !ancestor_ids(&diagram.nodes, &n.id).iter().any(|a| moved(a)))
            .map(|n| &n.bounds)
            .collect();
        let closest = |after: bool| {
            ids.iter()
                .filter_map(|id| at(id, after))
                .flat_map(|b| others.iter().map(move |o| gap(&b, o)))
                .fold(1000.0, f64::min)
        };
        let overlaps = ids
            .iter()
            .filter_map(|id| at(id, true))
            .map(|b| others.iter().filter(|o| gap(&b, o) == 0.0).count())
            .sum::<usize>();
        let leaves = context
            .alignments
            .iter()
            // A line moved as one keeps its own alignment.
            .filter(|a| a.members.iter().any(|m| moved(m)) && !a.members.iter().all(|m| moved(m)))
            .filter(|a| {
                if a.edge.is_column() {
                    step.dx != 0.0
                } else {
                    step.dy != 0.0
                }
            })
            .count();
        let lands_on = diagram
            .connectors
            .iter()
            .filter(|c| !moved(&c.from) && !moved(&c.to))
            .filter(|c| {
                ids.iter().filter_map(|id| at(id, true)).any(|b| {
                    segments(&c.points)
                        .iter()
                        .any(|(p, q)| segment_intersects_interior(*p, *q, &b))
                })
            })
            .count();
        let size: f64 = ids
            .iter()
            .filter_map(|id| diagram.node(id))
            .map(|n| n.bounds.width.min(n.bounds.height))
            .fold(0.0, f64::max)
            .max(1.0);
        json!({
            "straightened": straightened,
            "unstraightened": unstraightened,
            "nearer": nearer,
            "gap_before": closest(false),
            "gap_after": closest(true),
            "overlaps_after": overlaps,
            "leaves_alignments": leaves,
            "lands_on": lands_on,
            "move_per_size": (step.dx.abs() + step.dy.abs()) / size,
        })
    }

    /// More of what a move would do, for a finer ranking: how far each
    /// line's ends stay off a straight line, which way the move runs
    /// against the lines' end segments, the room ahead, the change in the
    /// nodes' offset from where fixing left them, the connectors the nodes
    /// sit on before and after, a rough rerouting of their lines as L
    /// shapes before and after, what their lines cost against the others,
    /// how far the last step's changes are, and the diagram's own bends and
    /// crossings.
    fn finer(diagram: &Diagram, state: &State, ids: &[&str], step: Move, changed: &[Bounds]) -> serde_json::Value {
        let moved = |id: &str| ids.contains(&id);
        let at = |id: &str, after: bool| {
            let node = diagram.node(id)?;
            let shift = if after && moved(id) {
                step
            } else {
                Move { dx: 0.0, dy: 0.0 }
            };
            Some(Bounds {
                x: node.bounds.x + shift.dx,
                y: node.bounds.y + shift.dy,
                ..node.bounds
            })
        };
        let attached: Vec<_> = diagram
            .connectors
            .iter()
            .filter(|c| moved(&c.from) || moved(&c.to))
            .collect();
        let others: Vec<_> = diagram
            .connectors
            .iter()
            .filter(|c| !moved(&c.from) && !moved(&c.to))
            .collect();
        // How far a line between two boxes' centres is off running straight.
        let off = |a: &Bounds, b: &Bounds| {
            let (p, q) = (a.centre(), b.centre());
            (p.x - q.x).abs().min((p.y - q.y).abs())
        };
        let mut off_before = Vec::new();
        let mut off_after = Vec::new();
        let (mut along, mut across) = (0, 0);
        // A rough rerouting: an L from centre to centre, turning once
        // unless the centres line up.
        let (mut l_bends, mut l_crossings, mut l_hits) = ([0usize; 2], [0usize; 2], [0usize; 2]);
        for c in &attached {
            // Its ends' nodes and the containers round them it runs inside.
            let holding: Vec<&str> = [c.from.as_str(), c.to.as_str()]
                .into_iter()
                .chain(ancestor_ids(&diagram.nodes, &c.from))
                .chain(ancestor_ids(&diagram.nodes, &c.to))
                .collect();
            let node_boxes: Vec<&Bounds> = diagram
                .nodes
                .iter()
                .filter(|n| !holding.contains(&n.id.as_str()))
                .map(|n| &n.bounds)
                .collect();
            let (Some(a0), Some(b0), Some(a1), Some(b1)) =
                (at(&c.from, false), at(&c.to, false), at(&c.from, true), at(&c.to, true))
            else {
                continue;
            };
            off_before.push(off(&a0, &b0));
            off_after.push(off(&a1, &b1));
            // The end segment at the moved node: along it or across it.
            let points = &c.points;
            if points.len() >= 2 && (step.dx != 0.0 || step.dy != 0.0) {
                let (p, q) = if moved(&c.from) {
                    (points[0], points[1])
                } else {
                    (points[points.len() - 1], points[points.len() - 2])
                };
                let horizontal = p.y == q.y;
                if horizontal == (step.dx != 0.0) {
                    along += 1;
                } else {
                    across += 1;
                }
            }
            for (i, (a, b)) in [(a0, b0), (a1, b1)].into_iter().enumerate() {
                let (p, q) = (a.centre(), b.centre());
                let route: Vec<Point> = if off(&a, &b) < 1.0 {
                    vec![p, q]
                } else {
                    vec![p, Point { x: p.x, y: q.y }, q]
                };
                l_bends[i] += route.len() - 2;
                l_crossings[i] += others.iter().map(|o| crossings(&route, &o.points)).sum::<usize>();
                l_hits[i] += node_boxes
                    .iter()
                    .filter(|n| {
                        segments(&route)
                            .iter()
                            .any(|(s, t)| segment_intersects_interior(*s, *t, n))
                    })
                    .count();
            }
        }
        let sum = |v: &[f64]| v.iter().sum::<f64>();
        let least = |v: &[f64]| v.iter().copied().fold(1000.0, f64::min);
        // Room ahead: the nearest other node in the direction of the move,
        // over the length of the move.
        let length = step.dx.abs() + step.dy.abs();
        let room = if length == 0.0 {
            0.0
        } else {
            ids.iter()
                .filter_map(|id| at(id, false))
                .flat_map(|b| {
                    diagram.nodes.iter().filter(|n| !moved(&n.id)).filter_map(move |n| {
                        let o = &n.bounds;
                        if step.dx > 0.0 {
                            (o.y < b.bottom() && o.bottom() > b.y && o.x >= b.right()).then(|| o.x - b.right())
                        } else if step.dx < 0.0 {
                            (o.y < b.bottom() && o.bottom() > b.y && o.right() <= b.x).then(|| b.x - o.right())
                        } else if step.dy > 0.0 {
                            (o.x < b.right() && o.right() > b.x && o.y >= b.bottom()).then(|| o.y - b.bottom())
                        } else {
                            (o.x < b.right() && o.right() > b.x && o.bottom() <= b.y).then(|| b.y - o.bottom())
                        }
                    })
                })
                .fold(1000.0, f64::min)
                / length
        };
        let offset = |after: bool| -> f64 {
            ids.iter()
                .map(|id| {
                    let o = state.offsets.get(*id).copied().unwrap_or(Point { x: 0.0, y: 0.0 });
                    let (dx, dy) = if after { (step.dx, step.dy) } else { (0.0, 0.0) };
                    (o.x + dx).abs() + (o.y + dy).abs()
                })
                .sum()
        };
        let sitting_on = |after: bool| {
            others
                .iter()
                .filter(|c| {
                    ids.iter().filter_map(|id| at(id, after)).any(|b| {
                        segments(&c.points)
                            .iter()
                            .any(|(p, q)| segment_intersects_interior(*p, *q, &b))
                    })
                })
                .count()
        };
        let internal: usize = attached
            .iter()
            .enumerate()
            .flat_map(|(i, a)| attached[i + 1..].iter().map(move |b| crossings(&a.points, &b.points)))
            .sum();
        let against: f64 = attached
            .iter()
            .map(|a| others.iter().map(|o| routes_cost(&a.points, &o.points)).sum::<f64>())
            .sum();
        let centre = ids
            .iter()
            .filter_map(|id| at(id, false))
            .map(|b| b.centre())
            .next()
            .unwrap_or(Point { x: 0.0, y: 0.0 });
        let to_change = changed
            .iter()
            .map(|area| {
                let dx = (area.x - centre.x).max(centre.x - area.right()).max(0.0);
                let dy = (area.y - centre.y).max(centre.y - area.bottom()).max(0.0);
                hypot(dx, dy)
            })
            .fold(10000.0, f64::min);
        let diagram_bends: usize = diagram.connectors.iter().map(|c| turns(&c.points)).sum();
        let diagram_crossings: usize = diagram
            .connectors
            .iter()
            .enumerate()
            .flat_map(|(i, a)| {
                diagram.connectors[i + 1..]
                    .iter()
                    .map(move |b| crossings(&a.points, &b.points))
            })
            .sum();
        json!({
            "off_before_sum": sum(&off_before),
            "off_after_sum": sum(&off_after),
            "off_before_min": least(&off_before),
            "off_after_min": least(&off_after),
            "along_ends": along,
            "across_ends": across,
            "room_ahead": room,
            "offset_change": offset(true) - offset(false),
            "sitting_before": sitting_on(false),
            "sitting_after": sitting_on(true),
            "internal_crossings": internal,
            "against_cost": against,
            "to_change": to_change,
            "l_bends_before": l_bends[0],
            "l_bends_after": l_bends[1],
            "l_crossings_before": l_crossings[0],
            "l_crossings_after": l_crossings[1],
            "l_hits_before": l_hits[0],
            "l_hits_after": l_hits[1],
            "diagram_bends": diagram_bends,
            "diagram_crossings": diagram_crossings,
        })
    }

    /// Records a step: a line per candidate of what could be known of it
    /// before trying it and the gain trying it showed, and the step in the
    /// log, with the move taken.
    #[cfg(feature = "trace-candidates")]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn record(
        context: &Context,
        state: &State,
        steps: usize,
        candidates: &[(String, Move)],
        gains: &[Option<f64>],
        best: Option<&(usize, State)>,
        changed: &[Bounds],
        further: &[(usize, f64, bool)],
    ) {
        let trials: Trials = std::mem::take(&mut *context.step.trials.lock().expect("trials lock"))
            .into_iter()
            .collect();
        let times: Times = std::mem::take(&mut *context.step.times.lock().expect("times lock"))
            .into_iter()
            .map(|(index, trial, searches, eval)| (index, (trial, searches, eval)))
            .collect();
        let mut outcomes: HashMap<usize, serde_json::Value> =
            std::mem::take(&mut *context.step.outcomes.lock().expect("outcomes lock"))
                .into_iter()
                .collect();
        let chosen = best.map(|(index, _)| *index);
        let records = table(
            context, state, steps, candidates, gains, chosen, changed, &trials, &times,
        );
        RECORDS.with_borrow_mut(|all| all.extend(records));

        let results: Vec<_> = gains
            .iter()
            .enumerate()
            .map(|(index, gain)| {
                let (trial_ns, trial_searches, eval_ns) = times[&index];
                json!({
                    "gain": gain,
                    "trial_ns": trial_ns,
                    "trial_searches": trial_searches,
                    "eval_ns": eval_ns,
                    "outcome": outcomes.remove(&index).unwrap_or_default(),
                })
            })
            .collect();
        LOG.with_borrow_mut(|log| {
            if steps == MAX_STEPS {
                log.start = crate::xml::serialize(&state.result.document);
                log.now = log.start.clone();
                log.steps.clear();
            }
            let next = best.map(|(_, next)| crate::xml::serialize(&next.result.document));
            let difference = next.as_deref().map(|next| difference(&log.now, next));
            log.steps.push(json!({
                "score": state.score,
                "candidates": candidates.iter().map(|(id, step)| json!([id, step.dx, step.dy])).collect::<Vec<_>>(),
                "results": results,
                "further": further.iter().map(|(index, score, kept)| json!([index, score, kept])).collect::<Vec<_>>(),
                "chosen": chosen,
                "next": difference,
            }));
            if let Some(next) = next {
                log.now = next;
            }
        });
    }

    pub(super) type Trials = HashMap<usize, [Trial; 2]>;
    pub(super) type Times = HashMap<usize, (u128, usize, u128)>;

    /// The log of the arrangement just made: the document arranging started
    /// from (after fixing), and its steps.
    #[cfg(feature = "trace-candidates")]
    pub fn take_log() -> serde_json::Value {
        LOG.with_borrow_mut(|log| {
            log.now.clear();
            json!({
                "version": env!("CARGO_PKG_VERSION"),
                "start": std::mem::take(&mut log.start),
                "steps": std::mem::take(&mut log.steps),
            })
        })
    }

    /// The records arranging would have made, from its log: each step's
    /// document is rebuilt from the start and the tags each step changed,
    /// and what could be known of each candidate is worked out again from
    /// it; the gains, the move taken and the times are the log's. Fails
    /// when a rebuilt document does not score what the log says it did.
    #[cfg(feature = "trace-candidates")]
    pub fn replay(log: &serde_json::Value) -> Result<Vec<serde_json::Value>, String> {
        let mut records = Vec::new();
        walk(log, |context, state, steps_left, candidates, changed, step| {
            let results = step["results"].as_array().ok_or("a step without results")?;
            let gains: Vec<Option<f64>> = results.iter().map(|r| r["gain"].as_f64()).collect();
            let times: Times = results
                .iter()
                .enumerate()
                .map(|(i, r)| {
                    let number = |key: &str| r[key].as_u64().unwrap_or_default();
                    (
                        i,
                        (
                            number("trial_ns").into(),
                            number("trial_searches") as usize,
                            number("eval_ns").into(),
                        ),
                    )
                })
                .collect();
            let trials: Trials = candidates
                .iter()
                .enumerate()
                .map(|(i, (id, step))| {
                    (
                        i,
                        [trial(state, id, Move { dx: 0.0, dy: 0.0 }).0, trial(state, id, *step).0],
                    )
                })
                .collect();
            let chosen = step["chosen"].as_u64().map(|c| c as usize);
            records.extend(table(
                context, state, steps_left, candidates, &gains, chosen, changed, &trials, &times,
            ));
            Ok(())
        })?;
        Ok(records)
    }

    /// Walks a log's steps, rebuilding each step's document from the start
    /// and the tags each step changed, and hands `visit` each step: the
    /// arrangement's context, the state, the steps left, the candidates,
    /// the areas the step before changed, and the step as logged. Fails
    /// when a rebuilt document does not score what the log says it did.
    #[cfg(feature = "trace-candidates")]
    pub(super) fn walk(
        log: &serde_json::Value,
        mut visit: impl FnMut(
            &Context,
            &State,
            usize,
            &[(String, Move)],
            &[Bounds],
            &serde_json::Value,
        ) -> Result<(), String>,
    ) -> Result<(), String> {
        let mut now = log["start"].as_str().ok_or("a log without its start")?.to_owned();
        let read = |svg: &str| crate::parse(svg).map_err(|e| e.0);
        let draft = Draft::new(read(&now)?, Changes::default(), Vec::new());
        let context = Context::new(draft.report().diagram.clone());
        let mut state = State {
            score: context.score(draft.report(), &HashMap::new()),
            result: draft,
            offsets: HashMap::new(),
        };
        let mut changed = Vec::new();
        let steps = log["steps"].as_array().ok_or("a log without steps")?;
        for (index, step) in steps.iter().enumerate() {
            if step["score"].as_f64() != Some(state.score) {
                return Err(format!(
                    "step {index}: the rebuilt document scores {}, the log {}",
                    state.score, step["score"]
                ));
            }
            let candidates: Vec<(String, Move)> = step["candidates"]
                .as_array()
                .ok_or("a step without candidates")?
                .iter()
                .map(|c| {
                    let (Some(id), Some(dx), Some(dy)) = (c[0].as_str(), c[1].as_f64(), c[2].as_f64()) else {
                        return Err(format!("a candidate that cannot be read: {c}"));
                    };
                    Ok((id.to_owned(), Move { dx, dy }))
                })
                .collect::<Result<_, String>>()?;
            visit(&context, &state, MAX_STEPS - index, &candidates, &changed, step)?;
            let Some(chosen) = step["chosen"].as_u64().map(|c| c as usize) else {
                break;
            };
            let (id, taken) = candidates.get(chosen).ok_or("a move taken past the candidates")?;
            let (_, offsets) = super::moved(&state, id, *taken).ok_or("a move taken that cannot be made")?;
            now = applied(&now, &step["next"])?;
            let result = Draft::new(read(&now)?, Changes::default(), Vec::new());
            changed = super::changed_areas(&state.result.report().diagram, &result.report().diagram);
            state = State {
                score: context.score(result.report(), &offsets),
                result,
                offsets,
            };
        }
        Ok(())
    }

    /// A line per candidate: what could be known of it before trying it,
    /// with the gain trying it showed and whether arrange took it.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn table(
        context: &Context,
        state: &State,
        steps: usize,
        candidates: &[(String, Move)],
        gains: &[Option<f64>],
        chosen: Option<usize>,
        changed: &[Bounds],
        trials: &Trials,
        times: &Times,
    ) -> Vec<serde_json::Value> {
        let report = state.result.report();
        let diagram = &report.diagram;
        candidates
            .iter()
            .zip(gains)
            .enumerate()
            .map(|(index, ((id, step), gain))| {
                let ids: Vec<&str> = id.split(TOGETHER).collect();
                let attached: Vec<_> = diagram
                    .connectors
                    .iter()
                    .filter(|c| ids.contains(&c.from.as_str()) || ids.contains(&c.to.as_str()))
                    .collect();
                let names: Vec<&str> = ids
                    .iter()
                    .copied()
                    .chain(attached.iter().map(|c| c.id.as_str()))
                    .collect();
                let crossed: usize = attached
                    .iter()
                    .map(|c| {
                        diagram
                            .connectors
                            .iter()
                            .map(|other| crossings(&c.points, &other.points))
                            .sum::<usize>()
                    })
                    .sum();
                let issues = report
                    .issues
                    .iter()
                    .filter(|issue| issue.elements.iter().any(|e| names.contains(&e.as_str())))
                    .count();
                let offset: f64 = ids
                    .iter()
                    .filter_map(|id| state.offsets.get(*id))
                    .map(|p| p.x.abs() + p.y.abs())
                    .sum();
                let aligned = context
                    .alignments
                    .iter()
                    .filter(|a| a.members.iter().any(|m| ids.contains(&m.as_str())))
                    .count();
                let area: f64 = diagram
                    .nodes
                    .iter()
                    .filter(|n| ids.contains(&n.id.as_str()))
                    .map(|n| n.bounds.width * n.bounds.height)
                    .sum();
                let mut record = json!({
                    "step": MAX_STEPS - steps,
                    "candidates": candidates.len(),
                    "id": id.replace(TOGETHER, "+"),
                "kind": kind(id, *step),
                    "dx": step.dx,
                    "dy": step.dy,
                    "nodes_moved": ids.len(),
                    "attached": attached.len(),
                    "attached_bends": attached.iter().map(|c| turns(&c.points)).sum::<usize>(),
                    "attached_crossings": crossed,
                    "attached_cost": attached.iter().map(|c| route_cost(&c.points)).sum::<f64>(),
                    "issues_here": issues,
                    "offset": offset,
                    "alignments": aligned,
                    "area": area,
                    "touched": changed_areas_touch(diagram, id, changed),
                    "diagram_nodes": diagram.nodes.len(),
                    "diagram_connectors": diagram.connectors.len(),
                    "diagram_issues": report.issues.len(),
                    "score": state.score,
                    "gain": gain,
                    "chosen": chosen == Some(index),
                });
                if let (Some(record), serde_json::Value::Object(after)) =
                    (record.as_object_mut(), destination(diagram, context, &ids, *step))
                {
                    record.extend(after);
                }
                if let (Some(record), Some((trial, searches, eval))) = (record.as_object_mut(), times.get(&index)) {
                    record.insert("trial_ns".into(), json!(trial));
                    record.insert("trial_searches".into(), json!(searches));
                    record.insert("eval_ns".into(), json!(eval));
                }
                if let (Some(record), Some([before, after])) = (record.as_object_mut(), trials.get(&index)) {
                    for (when, t) in [("before", before), ("after", after)] {
                        record.insert(format!("trial_bends_{when}"), json!(t.bends));
                        record.insert(format!("trial_crossings_{when}"), json!(t.crossings));
                        record.insert(format!("trial_cost_{when}"), json!(t.cost));
                        record.insert(format!("trial_against_{when}"), json!(t.against));
                        record.insert(format!("trial_unrouted_{when}"), json!(t.unrouted));
                    }
                }
                if let (Some(record), serde_json::Value::Object(finer)) =
                    (record.as_object_mut(), finer(diagram, state, &ids, *step, changed))
                {
                    record.extend(finer);
                }
                record
            })
            .collect()
    }

    #[cfg(all(test, feature = "trace-candidates"))]
    mod tests {
        use super::{applied, difference};

        #[test]
        fn a_difference_applied_gives_the_next_document() {
            // Given a step that changes a tag in place, and one that adds a tag
            let start = r#"<svg><g transform="translate(1 2)"><rect/></g></svg>"#;
            let moved = r#"<svg><g transform="translate(5 2)"><rect/></g></svg>"#;
            let grown = r#"<svg><g transform="translate(5 2)"><rect/><text>a</text></g></svg>"#;

            // When each is told as a difference and applied
            let first = applied(start, &difference(start, moved)).expect("applies");
            let second = applied(&first, &difference(&first, grown)).expect("applies");

            // Then each gives the next document, the first by its changed tag alone
            assert_eq!((first.as_str(), second.as_str()), (moved, grown));
            assert_eq!(difference(start, moved)["tags"].as_array().map(Vec::len), Some(1));
        }
    }
}

/// Screening arrange's candidates with a learned model: each step, of the
/// moves arrange would try, only the ones the model ranks first are
/// evaluated. The model is an XGBoost forest saved as JSON, trained in
/// nicevg-lab on the features `FEATURES` names, as arrange's records and
/// the steps before give them.
#[cfg(feature = "screening")]
pub mod screening {
    use std::collections::{HashMap, HashSet};
    use std::sync::Arc;

    use serde_json::Value;

    use super::trace::{Times, Trials, table, trial};
    use super::{Context, MAX_STEPS, Move, State, TOGETHER};
    use crate::geometry::Bounds;
    use crate::{FixResult, SvgInputError};

    /// The features a model may use. Those ending in `~` are left missing
    /// where nothing is known (the first step, a move not tried before).
    pub const FEATURES: &[&str] = &[
        "prior_gain~",
        "prior_tried",
        "node_prior_best~",
        "node_prior_mean~",
        "node_prior_count",
        "last_taken_gain~",
        "last_taken_share~",
        "steps_since_moved~",
        "to_change",
        "against_cost",
        "leaves_alignments",
        "trial_total_change",
        "trial_cost_change",
        "touched",
        "off_after_sum",
        "off_after_min",
        "area",
        "attached_bends",
        "offset_change",
        "gap_before",
        "nearer",
        "candidates",
        "issues_here_rank",
        "attached_bends_rank",
        "straightened_rank",
        "share_nudge",
        "share_reroute",
        "share_line",
        "share_align",
        "share_together",
        "nodes_with_moves",
        "connectors_per_node",
        "bends_per_connector",
        "crossings_per_connector",
    ];
    const KINDS: [&str; 5] = ["nudge", "reroute", "line", "align", "together"];

    /// A model and how many candidates a step evaluates with it.
    #[derive(Clone)]
    pub struct Screen {
        forest: Arc<Forest>,
        keep: usize,
    }

    impl Screen {
        /// From an XGBoost model saved as JSON (`Booster.save_model` to a
        /// `.json` file) whose features are all among `FEATURES`; `keep`
        /// candidates a step are evaluated.
        pub fn from_xgboost_json(text: &str, keep: usize) -> Result<Screen, String> {
            let model: Value = serde_json::from_str(text).map_err(|error| error.to_string())?;
            let learner = &model["learner"];
            let names: Vec<String> = learner["feature_names"]
                .as_array()
                .ok_or("a model without feature names")?
                .iter()
                .map(|name| name.as_str().map(str::to_owned).ok_or("a feature name that is no text"))
                .collect::<Result<_, _>>()?;
            if let Some(name) = names.iter().find(|name| !FEATURES.contains(&name.as_str())) {
                return Err(format!("the model uses \"{name}\", which screening does not compute"));
            }
            let trees = learner["gradient_booster"]["model"]["trees"]
                .as_array()
                .ok_or("a model without trees")?
                .iter()
                .map(Tree::from_json)
                .collect::<Result<_, _>>()?;
            // The base score as a margin: a logistic model keeps it as a probability.
            let base = learner["learner_model_param"]["base_score"]
                .as_str()
                .map(|text| text.trim_matches(|c| c == '[' || c == ']'))
                .and_then(|text| text.parse::<f64>().ok())
                .unwrap_or(0.5);
            let logistic = learner["objective"]["name"].as_str() == Some("binary:logistic");
            let base = if logistic { (base / (1.0 - base)).ln() } else { base };
            Ok(Screen {
                forest: Arc::new(Forest { names, trees, base }),
                keep: keep.max(1),
            })
        }

        /// The model's margin for each row of features, given in the order
        /// `names` lists them; a missing value is NaN.
        pub fn margins(&self, rows: &[Vec<f64>]) -> Vec<f64> {
            rows.iter().map(|row| self.forest.margin(row)).collect()
        }

        /// The model's features, in the order `margins` takes them.
        pub fn names(&self) -> &[String] {
            &self.forest.names
        }
    }

    struct Forest {
        names: Vec<String>,
        trees: Vec<Tree>,
        base: f64,
    }

    impl Forest {
        fn margin(&self, row: &[f64]) -> f64 {
            self.base + self.trees.iter().map(|tree| tree.leaf(row)).sum::<f64>()
        }
    }

    /// A tree as XGBoost saves it: per node, its children (-1 at a leaf),
    /// the feature it splits on, the split value (its output at a leaf),
    /// and which way a missing value goes.
    struct Tree {
        left: Vec<i64>,
        right: Vec<i64>,
        feature: Vec<usize>,
        condition: Vec<f32>,
        default_left: Vec<bool>,
    }

    impl Tree {
        fn from_json(tree: &Value) -> Result<Tree, String> {
            let numbers = |key: &str| -> Result<Vec<f64>, String> {
                tree[key]
                    .as_array()
                    .ok_or(format!("a tree without {key}"))?
                    .iter()
                    .map(|v| {
                        v.as_f64()
                            .or_else(|| v.as_bool().map(f64::from))
                            .ok_or(format!("{key} that are not numbers"))
                    })
                    .collect()
            };
            Ok(Tree {
                left: numbers("left_children")?.into_iter().map(|v| v as i64).collect(),
                right: numbers("right_children")?.into_iter().map(|v| v as i64).collect(),
                feature: numbers("split_indices")?.into_iter().map(|v| v as usize).collect(),
                condition: numbers("split_conditions")?.into_iter().map(|v| v as f32).collect(),
                default_left: numbers("default_left")?.into_iter().map(|v| v != 0.0).collect(),
            })
        }

        /// As XGBoost walks it: left when the value, as a 32-bit float, is
        /// less than the split; a missing value goes its default way.
        fn leaf(&self, row: &[f64]) -> f64 {
            let mut node = 0;
            while self.left[node] >= 0 {
                let value = row[self.feature[node]];
                let left = if value.is_nan() {
                    self.default_left[node]
                } else {
                    (value as f32) < self.condition[node]
                };
                node = if left { self.left[node] } else { self.right[node] } as usize;
            }
            f64::from(self.condition[node])
        }
    }

    /// What a screened arrangement keeps from one step to the next for the
    /// features.
    #[derive(Default)]
    pub(super) struct History {
        /// The moves evaluated at the step before, with the gain each
        /// showed, none when it was ruled out.
        evaluated: HashMap<(String, u64, u64), Option<f64>>,
        /// The gain of the move taken at the step before (after one pass of
        /// fixing), none before the first.
        last_taken: Option<f64>,
        /// The step each node was last moved at.
        moved_at: HashMap<String, usize>,
    }

    fn key(id: &str, step: Move) -> (String, u64, u64) {
        // Adding 0 makes -0 the same move as 0.
        (id.to_owned(), (step.dx + 0.0).to_bits(), (step.dy + 0.0).to_bits())
    }

    /// Of the candidates arrange would try again (`retried`), the ones the
    /// model ranks first, at most `keep` of them, best first.
    pub(super) fn kept(
        context: &Context,
        screen: &Screen,
        state: &State,
        steps: usize,
        all: &[(String, Move)],
        mut retried: Vec<(String, Move)>,
        changed: &[Bounds],
    ) -> Vec<(String, Move)> {
        if retried.len() <= screen.keep {
            return retried;
        }
        let rows = {
            let history = context.history.lock().expect("history lock");
            features(context, state, steps, all, changed, &history, screen.names())
        };
        let scores: HashMap<_, f64> = all
            .iter()
            .zip(screen.margins(&rows))
            .map(|((id, step), score)| (key(id, *step), score))
            .collect();
        let score = |(id, step): &(String, Move)| scores[&key(id, *step)];
        retried.sort_by(|a, b| score(b).total_cmp(&score(a)));
        retried.truncate(screen.keep);
        retried
    }

    /// After a step: the moves it evaluated with their gains, and the move
    /// it took, if any.
    pub(super) fn remember(
        context: &Context,
        steps: usize,
        evaluated: &[(String, Move)],
        gains: &[Option<f64>],
        taken: Option<usize>,
    ) {
        let mut history = context.history.lock().expect("history lock");
        history.evaluated = evaluated
            .iter()
            .zip(gains)
            .map(|((id, step), gain)| (key(id, *step), *gain))
            .collect();
        history.last_taken = taken.and_then(|index| gains[index]);
        if let Some(index) = taken {
            for node in evaluated[index].0.split(TOGETHER) {
                history.moved_at.insert(node.to_owned(), MAX_STEPS - steps);
            }
        }
    }

    /// The step's records as arrange's trace makes them, with a trial
    /// rerouting of every candidate's lines, on as many threads as there
    /// are cores. Each part's records count only its own candidates, so
    /// `features` takes the count from the whole.
    fn records(
        context: &Context,
        state: &State,
        steps: usize,
        candidates: &[(String, Move)],
        changed: &[Bounds],
    ) -> Vec<Value> {
        if candidates.is_empty() {
            return Vec::new();
        }
        let threads = std::thread::available_parallelism().map_or(1, |count| count.get());
        let size = candidates.len().div_ceil(threads).max(1);
        std::thread::scope(|scope| {
            let parts: Vec<_> = candidates
                .chunks(size)
                .map(|chunk| {
                    scope.spawn(move || {
                        let trials: Trials = chunk
                            .iter()
                            .enumerate()
                            .map(|(index, (id, step))| {
                                let still = Move { dx: 0.0, dy: 0.0 };
                                (index, [trial(state, id, still).0, trial(state, id, *step).0])
                            })
                            .collect();
                        let gains = vec![None; chunk.len()];
                        table(
                            context,
                            state,
                            steps,
                            chunk,
                            &gains,
                            None,
                            changed,
                            &trials,
                            &Times::new(),
                        )
                    })
                })
                .collect();
            parts
                .into_iter()
                .flat_map(|part| part.join().expect("a features thread"))
                .collect()
        })
    }

    /// Percent ranks within the step, ties sharing their average place, as
    /// pandas' `rank(pct=True)` gives them.
    fn ranks(values: &[f64]) -> Vec<f64> {
        let mut order: Vec<usize> = (0..values.len()).collect();
        order.sort_by(|&a, &b| values[a].total_cmp(&values[b]));
        let mut out = vec![0.0; values.len()];
        let mut start = 0;
        while start < order.len() {
            let mut end = start + 1;
            while end < order.len() && values[order[end]] == values[order[start]] {
                end += 1;
            }
            let place = (start + 1 + end) as f64 / 2.0;
            for &index in &order[start..end] {
                out[index] = place / values.len() as f64;
            }
            start = end;
        }
        out
    }

    /// Each candidate's features, in the order `names` gives: from the
    /// step's records, and from what the steps before left.
    fn features(
        context: &Context,
        state: &State,
        steps: usize,
        candidates: &[(String, Move)],
        changed: &[Bounds],
        history: &History,
        names: &[String],
    ) -> Vec<Vec<f64>> {
        let records = records(context, state, steps, candidates, changed);
        let count = candidates.len() as f64;
        let number = |record: &Value, field: &str| match &record[field] {
            Value::Bool(flag) => f64::from(u8::from(*flag)),
            value => value.as_f64().unwrap_or(f64::NAN),
        };
        let column = |field: &str| records.iter().map(|r| number(r, field)).collect::<Vec<f64>>();
        let issues_rank = ranks(&column("issues_here"));
        let bends_rank = ranks(&column("attached_bends"));
        let straightened_rank = ranks(&column("straightened"));
        let shares: HashMap<&str, f64> = KINDS
            .iter()
            .map(|kind| {
                let of_kind = records.iter().filter(|r| r["kind"].as_str() == Some(*kind)).count();
                (*kind, of_kind as f64 / count)
            })
            .collect();
        let nodes_with_moves = candidates
            .iter()
            .flat_map(|(id, _)| id.split(TOGETHER))
            .collect::<HashSet<_>>()
            .len() as f64;
        let step = MAX_STEPS - steps;
        // What each move showed a step before, when it was evaluated then.
        let known: Vec<Option<Option<f64>>> = candidates
            .iter()
            .map(|(id, step)| history.evaluated.get(&key(id, *step)).copied())
            .collect();
        let mut by_node: HashMap<&str, (usize, Vec<f64>)> = HashMap::new();
        for ((id, _), seen) in candidates.iter().zip(&known) {
            if let Some(gain) = seen {
                let entry = by_node.entry(id.as_str()).or_default();
                entry.0 += 1;
                entry.1.extend(*gain);
            }
        }
        records
            .iter()
            .enumerate()
            .map(|(index, record)| {
                let id = candidates[index].0.as_str();
                let (node_count, node_gains) = by_node.get(id).cloned().unwrap_or_default();
                let since = id
                    .split(TOGETHER)
                    .filter_map(|node| history.moved_at.get(node))
                    .max()
                    .map_or(f64::NAN, |at| (step - at) as f64);
                let score = number(record, "score");
                let connectors = number(record, "diagram_connectors").max(1.0);
                let change =
                    |what: &str| number(record, &format!("{what}_after")) - number(record, &format!("{what}_before"));
                names
                    .iter()
                    .map(|name| match name.as_str() {
                        "prior_gain~" => known[index].flatten().unwrap_or(f64::NAN),
                        "prior_tried" => f64::from(u8::from(known[index].is_some())),
                        "node_prior_best~" => node_gains.iter().copied().reduce(f64::min).unwrap_or(f64::NAN),
                        "node_prior_mean~" if node_gains.is_empty() => f64::NAN,
                        "node_prior_mean~" => node_gains.iter().sum::<f64>() / node_gains.len() as f64,
                        "node_prior_count" => node_count as f64,
                        "last_taken_gain~" => history.last_taken.unwrap_or(f64::NAN),
                        "last_taken_share~" => history
                            .last_taken
                            .map_or(f64::NAN, |gain| gain / if score != 0.0 { score } else { 1.0 }),
                        "steps_since_moved~" => since,
                        "trial_total_change" => change("trial_cost") + change("trial_against"),
                        "trial_cost_change" => change("trial_cost"),
                        "candidates" => count,
                        "issues_here_rank" => issues_rank[index],
                        "attached_bends_rank" => bends_rank[index],
                        "straightened_rank" => straightened_rank[index],
                        "nodes_with_moves" => nodes_with_moves,
                        "connectors_per_node" => {
                            number(record, "diagram_connectors") / number(record, "diagram_nodes").max(1.0)
                        }
                        "bends_per_connector" => number(record, "diagram_bends") / connectors,
                        "crossings_per_connector" => number(record, "diagram_crossings") / connectors,
                        share if share.starts_with("share_") => shares[&share["share_".len()..]],
                        field => number(record, field),
                    })
                    .collect()
            })
            .collect()
    }

    /// Each candidate's features at each step of a recorded arrangement
    /// (a log of the record example), in the order `screen.names()` gives
    /// them, as screening computes them when every candidate was evaluated:
    /// to check them against the ones learning computes from the records,
    /// and the model's margin on them. A missing value is null.
    #[cfg(feature = "trace-candidates")]
    pub fn replay_features(log: &Value, screen: &Screen) -> Result<Vec<Value>, String> {
        let mut rows = Vec::new();
        let mut history = History::default();
        super::trace::walk(log, |context, state, steps, candidates, changed, step| {
            let features = features(context, state, steps, candidates, changed, &history, screen.names());
            let margins = screen.margins(&features);
            for (((id, step), row), margin) in candidates.iter().zip(features).zip(margins) {
                rows.push(serde_json::json!({
                    "step": MAX_STEPS - steps,
                    "id": id.replace(TOGETHER, "+"),
                    "dx": step.dx,
                    "dy": step.dy,
                    "margin": margin,
                    "features": row.iter().map(|v| if v.is_nan() { Value::Null } else { serde_json::json!(v) }).collect::<Vec<_>>(),
                }));
            }
            let gains: Vec<Option<f64>> = step["results"]
                .as_array()
                .ok_or("a step without results")?
                .iter()
                .map(|r| r["gain"].as_f64())
                .collect();
            let taken = step["chosen"].as_u64().map(|c| c as usize);
            history.evaluated = candidates
                .iter()
                .zip(&gains)
                .map(|((id, m), gain)| (key(id, *m), *gain))
                .collect();
            history.last_taken = taken.and_then(|index| gains[index]);
            if let Some(index) = taken {
                for node in candidates[index].0.split(TOGETHER) {
                    history.moved_at.insert(node.to_owned(), MAX_STEPS - steps);
                }
            }
            Ok(())
        })?;
        Ok(rows)
    }

    /// Arranges as `arrange` does, evaluating at each step only the moves
    /// `screen` ranks first.
    pub fn arrange_screened(svg: &str, screen: &Screen) -> Result<FixResult, SvgInputError> {
        super::arranged(svg, |context| context.screen = Some(screen.clone()))
    }
}
