//! Deterministic repairs as a pipeline of pure steps over a draft document:
//! expand nodes around their labels, separate crowded nodes, reroute
//! connectors and place their labels, then grow the viewBox.

use std::cell::OnceCell;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, OnceLock};

use serde::Serialize;

use crate::arrow::{drawn_connectors, moved_head, shaft_route};
use crate::diagram::{
    Diagram, DiagramConnector, DiagramLabel, DiagramNode, ancestor_ids, child_element_paths, circle_bounds, holds_end,
    is_node_shape, node_group, non_empty, number_attribute, rect_bounds, translation,
};
use crate::geometry::{Bounds, Point, distance_to_route, enclosing, format_number, point, route_is_clear, segments};
use crate::inspect::{TEXT_PADDING, VIEWPORT_PADDING, parse_view_box, radius_enclosing};
use crate::route::{
    CLEARANCE, CLOSE_RUN, CROSSING_PENALTY, END_STUBS, LABEL_CLEAR_OF_OTHERS, LANE_SPACING, PortRequest, Ports,
    Pushable, Side, Sides, choose_sides, connector_ports, crossings, end_on_shapes, is_on_outline, label_landmarks,
    overlap, place_label, place_label_clear, route_connector, route_cost, routes_cost, runs_across, side_of,
    spread_ports, turns,
};
use crate::text::{label_text_index, move_label, text_bounds};
use crate::xml::{Document, Element, serialize};
use crate::{AnalysisReport, SvgInputError, parse, report_of};

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct FixChange {
    pub code: String,
    pub message: String,
    pub elements: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct FixResult {
    pub svg: String,
    pub changes: Vec<FixChange>,
    pub report: AnalysisReport,
}

/// The changes made to a draft, newest first, each sharing those before it
/// with the drafts it was made from: adding one copies none of them.
#[derive(Clone, Default)]
pub(crate) struct Changes(Option<Arc<ChangeLink>>);

struct ChangeLink {
    change: FixChange,
    before: Changes,
}

impl Changes {
    pub(crate) fn with(&self, change: FixChange) -> Changes {
        Changes(Some(Arc::new(ChangeLink {
            change,
            before: self.clone(),
        })))
    }

    /// Oldest first.
    fn to_vec(&self) -> Vec<FixChange> {
        let mut changes = Vec::new();
        let mut link = self.0.as_deref();
        while let Some(current) = link {
            changes.push(current.change.clone());
            link = current.before.0.as_deref();
        }
        changes.reverse();
        changes
    }
}

impl Drop for ChangeLink {
    /// Unlinks the changes before one at a time, so a long list does not
    /// drop recursively.
    fn drop(&mut self) {
        let mut before = self.before.0.take();
        while let Some(link) = before {
            match Arc::try_unwrap(link) {
                Ok(mut link) => before = link.before.0.take(),
                Err(_) => break,
            }
        }
    }
}

/// A document being repaired, with the changes made so far.
#[derive(Clone)]
pub(crate) struct Draft {
    pub(crate) document: Document,
    pub(crate) changes: Changes,
    /// Nodes just moved, whose connectors are rerouted to follow.
    moved_node_ids: Vec<String>,
    /// The report of `document`, read once; a changed draft starts without.
    report: OnceLock<Arc<AnalysisReport>>,
}

impl Draft {
    pub(crate) fn new(document: Document, changes: Changes, moved_node_ids: Vec<String>) -> Draft {
        Draft {
            document,
            changes,
            moved_node_ids,
            report: OnceLock::new(),
        }
    }

    #[tracing::instrument(name = "draft_report", skip_all)]
    pub(crate) fn report(&self) -> &Arc<AnalysisReport> {
        self.report.get_or_init(|| Arc::new(report_of(&self.document)))
    }

    /// The same document and changes, now following the nodes `moved`; the
    /// report already read is kept.
    pub(crate) fn moving(&self, moved: Vec<String>) -> Draft {
        Draft {
            moved_node_ids: moved,
            ..self.clone()
        }
    }

    /// The document written out, with its changes and report.
    pub(crate) fn written(&self) -> FixResult {
        FixResult {
            svg: serialize(&self.document),
            changes: self.changes.to_vec(),
            report: AnalysisReport::clone(self.report()),
        }
    }

    fn changed(self, document: Document, code: &str, message: String, elements: &[&str]) -> Draft {
        let change = FixChange {
            code: code.to_owned(),
            message,
            elements: elements.iter().map(|element| (*element).to_owned()).collect(),
        };
        Draft::new(document, self.changes.with(change), self.moved_node_ids)
    }
}

/// What pushing a label aside costs for each pixel its new spot lies nearer
/// another connector than a label keeps clear of them: touching one costs
/// what crossing it does.
const CROWDED_PUSH_COST: f64 = CROSSING_PENALTY / LABEL_CLEAR_OF_OTHERS;

/// Repairs what can be repaired, repeating passes while they leave fewer
/// issues (at most five).
#[tracing::instrument(skip_all)]
pub fn fix(svg: &str) -> Result<FixResult, SvgInputError> {
    fix_with_passes(svg, PASSES)
}

/// The passes `fix` gives a diagram, and arranging each arrangement.
pub(crate) const PASSES: usize = 5;

/// Later passes see earlier results: connectors routed early avoid labels
/// placed late, and labels left without a slot get another try. A pass is
/// kept only while it reduces the issues.
pub fn fix_with_passes(svg: &str, max_passes: usize) -> Result<FixResult, SvgInputError> {
    let draft = Draft::new(parse(svg)?, Changes::default(), Vec::new());
    Ok(fix_after_moves(draft, max_passes).written())
}

/// Fixes a draft, rerouting the connectors of the nodes it just moved to
/// follow them.
#[tracing::instrument(skip_all, fields(moved = draft.moved_node_ids.len()))]
pub(crate) fn fix_after_moves(draft: Draft, max_passes: usize) -> Draft {
    fix_further(fix_once(draft), max_passes.saturating_sub(1))
}

/// Runs up to `passes` more passes over a fixed draft, keeping each only
/// while it leaves fewer issues.
pub(crate) fn fix_further(best: Draft, passes: usize) -> Draft {
    if passes == 0 || best.report().issues.is_empty() {
        return best;
    }
    let next = fix_once(best.moving(Vec::new()));
    if next.report().issues.len() >= best.report().issues.len() {
        return best;
    }
    fix_further(next, passes - 1)
}

#[tracing::instrument(skip_all)]
fn fix_once(draft: Draft) -> Draft {
    let landmarks = connector_landmarks(&draft.report().diagram);
    expand_view_box(settle_labels(
        reroute_connectors(separate_nodes(expand_nodes(draft))),
        &landmarks,
    ))
}

/// Grows node shapes whose labels do not fit, leaving the labels in place.
#[tracing::instrument(skip_all)]
fn expand_nodes(draft: Draft) -> Draft {
    let report = draft.report().clone();
    report
        .issues
        .iter()
        .filter(|issue| issue.code == "text-overflow")
        .filter_map(|issue| issue.elements.first().cloned())
        .fold(draft, |draft, node_id| {
            let document = &draft.document;
            let Some(group) = node_group(document, &node_id) else {
                return draft;
            };
            let children = child_element_paths(document, &group);
            let shape = children
                .iter()
                .find(|child| is_node_shape(document.element(child)))
                .cloned();
            let labels: Vec<_> = children
                .into_iter()
                .filter(|child| document.element(child).name == "text")
                .collect();
            let (Some(shape), false) = (shape, labels.is_empty()) else {
                return draft;
            };
            let Some(measured) = enclosing(
                &labels
                    .iter()
                    .map(|label| text_bounds(document, label))
                    .collect::<Vec<_>>(),
            ) else {
                return draft;
            };
            if document.element(&shape).name == "circle" {
                let radius = number_attribute(document.element(&shape), "r");
                let needed = radius_enclosing(&measured, &circle_bounds(document, &shape), TEXT_PADDING).ceil();
                if needed <= radius {
                    return draft;
                }
                let updated = document.update(&shape, |circle| circle.with_attr("r", &format_number(needed)));
                let message = format!(
                    "Expanded node \"{node_id}\" from radius {} to {}.",
                    format_number(radius),
                    format_number(needed)
                );
                return draft.changed(updated, "expand-node", message, &[&node_id]);
            }
            let current = rect_bounds(document, &shape);
            let element = document.element(&shape);
            let offset = Point {
                x: current.x - number_attribute(element, "x"),
                y: current.y - number_attribute(element, "y"),
            };
            let left = current.x.min(measured.x - TEXT_PADDING);
            let top = current.y.min(measured.y - TEXT_PADDING);
            let right = current.right().max(measured.right() + TEXT_PADDING);
            let bottom = current.bottom().max(measured.bottom() + TEXT_PADDING);
            let expanded = Bounds {
                x: left,
                y: top,
                width: right - left,
                height: bottom - top,
            };
            let updated = document.update(&shape, |rect| {
                rect.with_attr("x", &format_number(expanded.x - offset.x))
                    .with_attr("y", &format_number(expanded.y - offset.y))
                    .with_attr("width", &format_number(expanded.width))
                    .with_attr("height", &format_number(expanded.height))
            });
            let message = format!(
                "Expanded node \"{node_id}\" from {} to {}.",
                current.format(),
                expanded.format()
            );
            draft.changed(updated, "expand-node", message, &[&node_id])
        })
}

/// Pushes the right node of each overlapping or too-close pair rightwards,
/// one pair at a time, until the row keeps its 20px gaps.
#[tracing::instrument(skip_all)]
fn separate_nodes(draft: Draft) -> Draft {
    fn step(draft: Draft, attempt: usize) -> Draft {
        if attempt >= 100 {
            return draft;
        }
        let report = draft.report().clone();
        let Some(issue) = report
            .issues
            .iter()
            .find(|issue| issue.code == "node-overlap" || issue.code == "node-gap")
        else {
            return draft;
        };
        let node = |index: usize| issue.elements.get(index).and_then(|id| report.diagram.node(id));
        let (Some(first), Some(second)) = (node(0), node(1)) else {
            return draft;
        };
        let (left, right) = if first.bounds.x <= second.bounds.x {
            (first, second)
        } else {
            (second, first)
        };
        let delta = left.bounds.right() + 20.0 - right.bounds.x;
        if delta <= 0.0 {
            return draft;
        }
        let Some(group) = node_group(&draft.document, &right.id) else {
            return draft;
        };
        let existing = draft.document.element(&group).attr("transform").trim().to_owned();
        let transform = format!(
            "{existing}{}translate({} 0)",
            if existing.is_empty() { "" } else { " " },
            format_number(delta)
        );
        let updated = draft.document.update(&group, |g| g.with_attr("transform", &transform));
        let message = format!(
            "Moved node \"{}\" {}px right to preserve a 20px gap.",
            right.id,
            format_number(delta)
        );
        let id = right.id.clone();
        let moved = draft.changed(updated, "move-node", message, &[&id]);
        let moved = Draft {
            moved_node_ids: [moved.moved_node_ids.clone(), vec![id]].concat(),
            ..moved
        };
        step(moved, attempt + 1)
    }
    step(draft, 0)
}

fn side_key(bounds: &Bounds, side: crate::route::Side) -> String {
    format!("{}:{}", bounds.format(), side.name())
}

/// Where the connectors that stay meet each node side, along the side: on
/// a box, the side the end lies on; on a circle, the side the connector
/// arrives at along its last segment.
fn settled_ports(
    diagram: &Diagram,
    nodes_by_id: &HashMap<&str, &DiagramNode>,
    rerouted_ids: &HashSet<&str>,
) -> HashMap<String, Vec<f64>> {
    diagram
        .connectors
        .iter()
        .filter(|connector| !rerouted_ids.contains(connector.id.as_str()) && connector.points.len() >= 2)
        .flat_map(|connector| {
            let points = &connector.points;
            [
                (&connector.from, points[0], points[1]),
                (&connector.to, points[points.len() - 1], points[points.len() - 2]),
            ]
        })
        .filter_map(|(node_id, end, inner)| {
            let bounds = nodes_by_id.get(node_id.as_str())?.bounds;
            let (side, port) = end_port(end, inner, &bounds);
            let along = if matches!(side, Side::Top | Side::Bottom) {
                port.x
            } else {
                port.y
            };
            Some((side_key(&bounds, side), along))
        })
        .fold(HashMap::new(), |mut ports: HashMap<String, Vec<f64>>, (key, along)| {
            ports.entry(key).or_default().push(along);
            ports
        })
}

/// The node side a connector meets at its `end`, coming from `inner`, and
/// the port there on the node's box: on a box, the side the end lies on;
/// on a circle, the side the connector arrives at along its last segment.
fn end_port(end: Point, inner: Point, bounds: &Bounds) -> (Side, Point) {
    let side = if is_on_outline(end, bounds) {
        side_of(end, bounds)
    } else if (end.x - inner.x).abs() >= (end.y - inner.y).abs() {
        if inner.x > end.x { Side::Right } else { Side::Left }
    } else if inner.y > end.y {
        Side::Bottom
    } else {
        Side::Top
    };
    let port = match side {
        Side::Top => point(end.x, bounds.y),
        Side::Bottom => point(end.x, bounds.bottom()),
        Side::Left => point(bounds.x, end.y),
        Side::Right => point(bounds.right(), end.y),
    };
    (side, port)
}

/// The node and side each end of a connector meets with these ports.
fn meets<'a>(reroute: &Reroute<'a>, ports: &Ports) -> [(&'a str, Side); 2] {
    [
        (reroute.source.id.as_str(), side_of(ports.start, &reroute.source.bounds)),
        (reroute.target.id.as_str(), side_of(ports.end, &reroute.target.bounds)),
    ]
}

struct Reroute<'a> {
    connector: &'a DiagramConnector,
    source: &'a DiagramNode,
    target: &'a DiagramNode,
    /// The point and side each end is pinned to by the port it names.
    pins: [Option<(Point, Side)>; 2],
}

impl<'a> Reroute<'a> {
    /// The places of the port each end names on its node, none for an end
    /// naming no port or one its node lacks.
    fn port_places(&self) -> [Vec<&crate::diagram::Port>; 2] {
        fn places<'n>(node: &'n DiagramNode, port: Option<&str>) -> Vec<&'n crate::diagram::Port> {
            port.map_or_else(Vec::new, |name| {
                node.ports.iter().filter(|place| place.name == name).collect()
            })
        }
        [
            places(self.source, self.connector.from_port.as_deref()),
            places(self.target, self.connector.to_port.as_deref()),
        ]
    }

    /// A connector to reroute between its nodes, each end pinned to the
    /// place of its port nearest where it ends now.
    fn new(connector: &'a DiagramConnector, source: &'a DiagramNode, target: &'a DiagramNode) -> Self {
        let mut reroute = Reroute {
            connector,
            source,
            target,
            pins: [None, None],
        };
        let ends = [connector.points.first(), connector.points.last()];
        reroute.pins = std::array::from_fn(|end| {
            let places = &reroute.port_places()[end];
            let nearest = |place: &&crate::diagram::Port| {
                ends[end].map_or(0.0, |at| (place.point.x - at.x).hypot(place.point.y - at.y))
            };
            places
                .iter()
                .min_by(|a, b| nearest(a).total_cmp(&nearest(b)))
                .map(|place| (place.point, place.side))
        });
        reroute
    }
}

/// What fixing reroutes: the ids of the tied labels that drifted from their
/// connector or cover a node, and the connectors to reroute. Those are the
/// ones with an issue rerouting fixes, those whose drifted label finds no
/// slot beside them as they run, and those following a moved node.
fn pick_reroutes<'a>(
    report: &'a AnalysisReport,
    moved_node_ids: &[String],
    nodes_by_id: &HashMap<&str, &'a DiagramNode>,
) -> (HashSet<&'a str>, Vec<Reroute<'a>>) {
    let diagram = &report.diagram;
    // A tied label that drifted from its connector or covers a node moves
    // beside its connector on its own when there is room there; otherwise
    // the connector is rerouted to give it a slot.
    let detached: Vec<(&str, &str)> = report
        .issues
        .iter()
        .filter_map(|issue| match issue.code.as_str() {
            "label-detached" => Some((issue.elements.first()?.as_str(), issue.elements.get(1)?.as_str())),
            "label-node-overlap" => {
                let label = diagram
                    .labels
                    .iter()
                    .find(|label| Some(&label.id) == issue.elements.first())?;
                Some((label.id.as_str(), label.connector.as_deref()?))
            }
            _ => None,
        })
        .collect();
    let detached_ids: HashSet<&str> = detached.iter().map(|(label, _)| *label).collect();
    let all_routes: Vec<Vec<Point>> = diagram.connectors.iter().map(|c| c.points.clone()).collect();
    let stuck_connectors: Vec<&str> = detached
        .iter()
        .filter(|(label_id, connector_id)| {
            let (Some(route), Some(label)) = (
                diagram.connector(connector_id),
                diagram.labels.iter().find(|label| label.id == *label_id),
            ) else {
                return false;
            };
            let blocked: Vec<Bounds> = diagram
                .labels
                .iter()
                .filter(|label| !detached_ids.contains(label.id.as_str()))
                .map(|label| label.bounds)
                .chain(label_blockers(diagram, connector_id))
                .collect();
            place_label(
                label.bounds.width,
                label.bounds.height,
                &route.points,
                &all_routes,
                &blocked,
                &label_containers(diagram, connector_id),
            )
            .is_none()
        })
        .map(|(_, connector_id)| *connector_id)
        .collect();

    let reroute_codes = [
        "connector-node-crossing",
        "connector-label-clearance",
        "connector-endpoint-inside",
        "connector-end-off-port",
        "connector-end-along-side",
        "connector-overlap",
        "connector-near-miss",
    ];
    let with_issues: HashSet<&str> = stuck_connectors
        .into_iter()
        .chain(
            report
                .issues
                .iter()
                .filter(|issue| reroute_codes.contains(&issue.code.as_str()))
                .flat_map(|issue| {
                    let take = if issue.code == "connector-overlap" || issue.code == "connector-near-miss" {
                        issue.elements.len()
                    } else {
                        1
                    };
                    issue.elements.iter().take(take).map(String::as_str)
                }),
        )
        .collect();
    let reroutes: Vec<Reroute> = diagram
        .connectors
        .iter()
        .filter(|connector| {
            with_issues.contains(connector.id.as_str())
                || moved_node_ids.contains(&connector.from)
                || moved_node_ids.contains(&connector.to)
        })
        .filter_map(|connector| {
            Some(Reroute::new(
                connector,
                nodes_by_id.get(connector.from.as_str())?,
                nodes_by_id.get(connector.to.as_str())?,
            ))
        })
        .collect();
    (detached_ids, reroutes)
}

/// A draft with routes written in, and every connector's route by id.
struct Routing {
    draft: Draft,
    routes: Vec<(String, Vec<Point>)>,
}

/// Writes each rerouted connector's new route into the document, moving its
/// arrowheads along, and gives every connector's route afterwards.
fn write_routes(draft: &Draft, diagram: &Diagram, reroutes: &[Reroute], final_routes: Vec<Vec<Point>>) -> Routing {
    let initial = Routing {
        draft: draft.clone(),
        routes: diagram
            .connectors
            .iter()
            .map(|c| (c.id.clone(), c.points.clone()))
            .collect(),
    };
    // Rewriting one connector leaves where the others are drawn as it was.
    let all_drawn = drawn_connectors(&draft.document);
    reroutes
        .iter()
        .zip(final_routes)
        .fold(initial, |state, (reroute, route)| {
            let connector = reroute.connector;
            let routes = with_route(state.routes, &connector.id, route.clone());
            let document = &state.draft.document;
            // Its own shaft by id, else the one joining the same nodes.
            let Some(drawn) = all_drawn.iter().find(|drawn| drawn.id == connector.id).or_else(|| {
                all_drawn
                    .iter()
                    .find(|drawn| drawn.from == connector.from && drawn.to == connector.to)
            }) else {
                return Routing { routes, ..state };
            };
            let shaft = &drawn.shaft;
            // Routes are in diagram coordinates; the shaft's own are inside its
            // ancestors' translations.
            let offset = translation(document, shaft);
            let local: Vec<Point> = shaft_route(drawn, &route)
                .into_iter()
                .map(|p| Point {
                    x: p.x - offset.x,
                    y: p.y - offset.y,
                })
                .collect();
            let replacement = rerouted_element(
                document.element(shaft),
                &local,
                document.needs_svg_namespace(&shaft[..shaft.len() - 1]),
            );
            let updated = drawn
                .heads
                .iter()
                .fold(
                    document.replace(shaft, replacement),
                    |document, head| match moved_head(&document, head, &route) {
                        Some(polygon) => document.replace(&head.path, polygon),
                        None => document,
                    },
                );
            let message = format!("Rerouted connector \"{}\" around diagram obstacles.", connector.id);
            Routing {
                draft: state
                    .draft
                    .changed(updated, "route-connector", message, &[&connector.id]),
                routes,
            }
        })
}

/// Reroutes connectors that cross nodes, overlap, end inside nodes, pass too
/// close to labels or follow moved nodes; then places the labels of rerouted
/// connectors and of labels that drifted from their connector.
#[tracing::instrument(skip_all, fields(reroutes = tracing::field::Empty, partners = tracing::field::Empty))]
fn reroute_connectors(draft: Draft) -> Draft {
    // A pushed label is sure of a spot only among the routes as they were;
    // one that finds none among the new routes stands as a wall instead.
    let (rerouted, stuck) = reroute_around(draft.clone(), &HashSet::new());
    if stuck.is_empty() {
        rerouted
    } else {
        reroute_around(draft, &stuck).0
    }
}

/// Reroutes as `reroute_connectors` does, with the labels named in `walls`
/// standing in the way instead of moving aside; also gives the labels it
/// pushed aside but found no spot for.
fn reroute_around(draft: Draft, walls: &HashSet<String>) -> (Draft, HashSet<String>) {
    let report = draft.report().clone();
    let diagram = &report.diagram;

    let nodes_by_id: HashMap<&str, &DiagramNode> = diagram.nodes.iter().map(|node| (node.id.as_str(), node)).collect();
    let (detached_ids, reroutes) = pick_reroutes(&report, &draft.moved_node_ids, &nodes_by_id);
    let rerouted_ids: HashSet<&str> = reroutes.iter().map(|reroute| reroute.connector.id.as_str()).collect();
    let settled: Vec<Vec<Point>> = diagram
        .connectors
        .iter()
        .filter(|connector| !rerouted_ids.contains(connector.id.as_str()))
        .map(|connector| connector.points.clone())
        .collect();
    // The borders of the containers around a connector's ends: it crosses
    // them to reach its ends but should not run along them.
    let rails_for = |connector: &DiagramConnector| -> Vec<(Point, Point)> {
        diagram
            .nodes
            .iter()
            .filter(|node| node.id != connector.from && node.id != connector.to)
            .filter(|node| holds_end(&diagram.nodes, connector, node))
            .flat_map(|node| {
                let b = node.bounds;
                let corners = [
                    Point { x: b.x, y: b.y },
                    Point { x: b.right(), y: b.y },
                    Point {
                        x: b.right(),
                        y: b.bottom(),
                    },
                    Point { x: b.x, y: b.bottom() },
                ];
                (0..4).map(move |index| (corners[index], corners[(index + 1) % 4]))
            })
            .collect()
    };
    // A connector's own label moves beside its new route, and so may the
    // labels of the connectors that stay: a route can push one aside to
    // another spot beside its connector, for free when that spot is clear
    // of every other connector and for what crowding them there costs when
    // it is not.
    let all_routes: Vec<Vec<Point>> = diagram.connectors.iter().map(|c| c.points.clone()).collect();
    let push_cost = |label: &DiagramLabel| -> Option<f64> {
        let connector = diagram.connector(label.connector.as_deref()?)?;
        let blocked: Vec<Bounds> = diagram
            .labels
            .iter()
            .filter(|other| other.id != label.id)
            .map(|other| other.bounds)
            .chain(label_blockers(diagram, &connector.id))
            .chain([label.bounds.inflate(CLEARANCE)])
            .collect();
        let containers = label_containers(diagram, &connector.id);
        let (width, height) = (label.bounds.width, label.bounds.height);
        if place_label_clear(width, height, &connector.points, &all_routes, &blocked, &containers).is_some() {
            return Some(0.0);
        }
        let spot = place_label(width, height, &connector.points, &all_routes, &blocked, &containers)?;
        let crowding: f64 = diagram
            .connectors
            .iter()
            .filter(|other| other.id != connector.id)
            .map(|other| (LABEL_CLEAR_OF_OTHERS - distance_to_route(&spot.bounds, &other.points)).max(0.0))
            .sum();
        Some(crowding * CROWDED_PUSH_COST)
    };
    let pushable_labels: Vec<&DiagramLabel> = diagram
        .labels
        .iter()
        .filter(|label| {
            !detached_ids.contains(label.id.as_str())
                && !walls.contains(&label.id)
                && label.connector.as_deref().is_some_and(|id| !rerouted_ids.contains(id))
        })
        .collect();
    // Each label's cost is worked out once, when a route first reaches it.
    let push_costs: Vec<OnceCell<Option<f64>>> = pushable_labels.iter().map(|_| OnceCell::new()).collect();
    let pricing: Vec<Box<dyn Fn() -> Option<f64> + '_>> = pushable_labels
        .iter()
        .zip(&push_costs)
        .map(|(label, cost)| {
            let push_cost = &push_cost;
            Box::new(move || *cost.get_or_init(|| push_cost(label))) as Box<dyn Fn() -> Option<f64>>
        })
        .collect();
    let obstacles_for = |connector: &DiagramConnector| -> Vec<Bounds> {
        diagram
            .nodes
            .iter()
            .filter(|node| !holds_end(&diagram.nodes, connector, node))
            .map(|node| node.bounds)
            .chain(
                diagram
                    .labels
                    .iter()
                    .filter(|label| {
                        !detached_ids.contains(label.id.as_str())
                            && (label.connector.is_none() || walls.contains(&label.id))
                    })
                    .map(|label| label.bounds),
            )
            .collect()
    };
    let pushable_for = |connector: &DiagramConnector| -> Vec<Pushable<'_>> {
        pushable_labels
            .iter()
            .zip(&pricing)
            .filter(|(label, _)| label.connector.as_deref() != Some(connector.id.as_str()))
            .map(|(label, cost)| (label.bounds, cost.as_ref()))
            .collect()
    };

    // Side loads: endpoints of the connectors that stay, plus the sides
    // chosen for rerouted ones. Sides are chosen twice so every connector
    // sees where the others went, not only the ones before it.
    let settled_load: HashMap<String, f64> = diagram
        .connectors
        .iter()
        .filter(|connector| !rerouted_ids.contains(connector.id.as_str()))
        .flat_map(|connector| {
            [
                (&connector.from, connector.points.first()),
                (&connector.to, connector.points.last()),
            ]
            .into_iter()
            .filter_map(|(node_id, end)| {
                let bounds = nodes_by_id.get(node_id.as_str())?.bounds;
                let end = *end?;
                is_on_outline(end, &bounds).then(|| side_key(&bounds, side_of(end, &bounds)))
            })
        })
        .fold(HashMap::new(), |mut load, key| {
            *load.entry(key).or_insert(0.0) += 1.0;
            load
        });
    let choose_all = |previous: &HashMap<&str, Sides>| -> HashMap<&str, Sides> {
        let load = reroutes.iter().fold(settled_load.clone(), |mut load, reroute| {
            if let Some(chosen) = previous.get(reroute.connector.id.as_str()) {
                for key in [
                    side_key(&reroute.source.bounds, chosen.start),
                    side_key(&reroute.target.bounds, chosen.end),
                ] {
                    *load.entry(key).or_insert(0.0) += 1.0;
                }
            }
            load
        });
        reroutes
            .iter()
            .map(|reroute| {
                let own = previous.get(reroute.connector.id.as_str());
                let load_of = |bounds: &Bounds, side: crate::route::Side| {
                    let key = side_key(bounds, side);
                    let mine = own.map_or(0.0, |own| {
                        f64::from(u8::from(key == side_key(&reroute.source.bounds, own.start)))
                            + f64::from(u8::from(key == side_key(&reroute.target.bounds, own.end)))
                    });
                    load.get(&key).copied().unwrap_or(0.0) - mine
                };
                let sides = choose_sides(
                    &reroute.source.bounds,
                    &reroute.target.bounds,
                    &obstacles_for(reroute.connector),
                    &pushable_for(reroute.connector),
                    &settled,
                    &rails_for(reroute.connector),
                    &load_of,
                );
                (reroute.connector.id.as_str(), sides)
            })
            .collect()
    };
    let route_with = |reroute: &Reroute, ports: Ports, occupied: &[Vec<Point>]| {
        // Routes into the same port as this one merge into it: their last
        // stretch there is no route to keep clear of.
        let pinned: Vec<Point> = reroute.pins.iter().flatten().map(|(at, _)| *at).collect();
        let occupied: Vec<Vec<Point>> = occupied
            .iter()
            .map(|route| {
                let (mut first, mut last) = (0, route.len());
                if route.first().is_some_and(|start| pinned.contains(start)) {
                    first = 1;
                }
                if route.last().is_some_and(|end| pinned.contains(end)) {
                    last = last.saturating_sub(1);
                }
                route.get(first..last.max(first)).unwrap_or_default().to_vec()
            })
            .collect();
        end_on_shapes(
            route_connector(
                &reroute.source.bounds,
                &reroute.target.bounds,
                &obstacles_for(reroute.connector),
                &pushable_for(reroute.connector),
                ports,
                &occupied,
                &rails_for(reroute.connector),
            ),
            reroute.source,
            reroute.target,
        )
    };
    let chosen = choose_all(&choose_all(&HashMap::new()));
    // An end whose port has several places takes the one its route costs
    // least from, among the connectors that stay, with each free end at the
    // middle of the side chosen for it.
    let reroutes: Vec<Reroute> = reroutes
        .iter()
        .map(|reroute| {
            let places = reroute.port_places();
            if places.iter().all(|places| places.len() < 2) {
                return Reroute { ..*reroute };
            }
            let sides = chosen.get(reroute.connector.id.as_str()).copied();
            let options = |end: usize| -> Vec<Option<(Point, Side)>> {
                if places[end].is_empty() {
                    vec![None]
                } else {
                    places[end]
                        .iter()
                        .map(|place| Some((place.point, place.side)))
                        .collect()
                }
            };
            let free_end = |bounds: &Bounds, side: Option<Side>, fallback: Point| {
                side.map_or(fallback, |side| crate::route::port_on_side(bounds, side))
            };
            let default = connector_ports(&reroute.source.bounds, &reroute.target.bounds);
            let starts = options(0);
            let ends = options(1);
            let cost = |pins: [Option<(Point, Side)>; 2]| {
                let ports = Ports {
                    start: pins[0].map_or_else(
                        || free_end(&reroute.source.bounds, sides.map(|sides| sides.start), default.start),
                        |(at, _)| at,
                    ),
                    end: pins[1].map_or_else(
                        || free_end(&reroute.target.bounds, sides.map(|sides| sides.end), default.end),
                        |(at, _)| at,
                    ),
                };
                let route = route_with(reroute, ports, &settled);
                // A route that found no way is drawn straight across.
                let orthogonal = segments(&route).iter().all(|(a, b)| a.x == b.x || a.y == b.y);
                if !orthogonal {
                    return f64::INFINITY;
                }
                route_cost(&route) + settled.iter().map(|other| routes_cost(&route, other)).sum::<f64>()
            };
            let pins = starts
                .iter()
                .flat_map(|start| ends.iter().map(move |end| [*start, *end]))
                .min_by(|a, b| cost(*a).total_cmp(&cost(*b)))
                .expect("a place for each end");
            Reroute { pins, ..*reroute }
        })
        .collect();
    // An end naming a port meets the side its port lies on.
    let chosen: HashMap<&str, Sides> = chosen
        .into_iter()
        .map(|(id, sides)| {
            let reroute = reroutes
                .iter()
                .find(|reroute| reroute.connector.id == id)
                .expect("chosen for a reroute");
            let [start, end] = reroute.pins;
            (
                id,
                Sides {
                    start: start.map_or(sides.start, |(_, side)| side),
                    end: end.map_or(sides.end, |(_, side)| side),
                },
            )
        })
        .collect();
    let fixed = settled_ports(diagram, &nodes_by_id, &rerouted_ids);
    let ports = spread_ports(
        &reroutes
            .iter()
            .filter_map(|reroute| {
                Some(PortRequest {
                    id: reroute.connector.id.clone(),
                    source_id: reroute.source.id.clone(),
                    target_id: reroute.target.id.clone(),
                    source: reroute.source.bounds,
                    target: reroute.target.bounds,
                    source_round: reroute.source.shape.is_some(),
                    target_round: reroute.target.shape.is_some(),
                    sides: *chosen.get(reroute.connector.id.as_str())?,
                    pins: reroute.pins.map(|pin| pin.map(|(at, _)| at)),
                })
            })
            .collect::<Vec<_>>(),
        &fixed,
    );

    let first_ports: Vec<Ports> = reroutes
        .iter()
        .map(|reroute| {
            let spread = ports
                .get(&reroute.connector.id)
                .copied()
                .unwrap_or_else(|| connector_ports(&reroute.source.bounds, &reroute.target.bounds));
            let [start, end] = reroute.pins;
            Ports {
                start: start.map_or(spread.start, |(at, _)| at),
                end: end.map_or(spread.end, |(at, _)| at),
            }
        })
        .collect();
    let (_, first_routes) = reroutes.iter().zip(&first_ports).fold(
        (settled.clone(), Vec::new()),
        |(occupied, routes): (Vec<Vec<Point>>, Vec<Vec<Point>>), (reroute, ports)| {
            let route = route_with(reroute, *ports, &occupied);
            ([occupied, vec![route.clone()]].concat(), [routes, vec![route]].concat())
        },
    );
    // A connector that stays but crosses another, rerouted or not, where
    // both meet the same node side may trade ports with it, and is rerouted
    // too if so.
    let staying_ends: Vec<(Reroute, Ports)> = diagram
        .connectors
        .iter()
        .filter(|connector| !rerouted_ids.contains(connector.id.as_str()) && connector.points.len() >= 2)
        .filter_map(|connector| {
            let source = nodes_by_id.get(connector.from.as_str())?;
            let target = nodes_by_id.get(connector.to.as_str())?;
            let points = &connector.points;
            let (_, start) = end_port(points[0], points[1], &source.bounds);
            let (_, end) = end_port(points[points.len() - 1], points[points.len() - 2], &target.bounds);
            Some((Reroute::new(connector, source, target), Ports { start, end }))
        })
        .collect();
    let all_ends: Vec<(&Reroute, &Ports, &Vec<Point>)> = reroutes
        .iter()
        .zip(&first_ports)
        .zip(&first_routes)
        .map(|((reroute, ports), route)| (reroute, ports, route))
        .chain(
            staying_ends
                .iter()
                .map(|(staying, ports)| (staying, ports, &staying.connector.points)),
        )
        .collect();
    let partner_ids: HashSet<&str> = staying_ends
        .iter()
        .filter(|(partner, partner_ports)| {
            let own = meets(partner, partner_ports);
            all_ends.iter().any(|(other, ports, route)| {
                other.connector.id != partner.connector.id
                    && meets(other, ports).iter().any(|end| own.contains(end))
                    && crossings(route, &partner.connector.points) > 0
            })
        })
        .map(|(partner, _)| partner.connector.id.as_str())
        .collect();
    let partners: Vec<(Reroute, Ports)> = staying_ends
        .into_iter()
        .filter(|(partner, _)| partner_ids.contains(partner.connector.id.as_str()))
        .collect();
    tracing::Span::current()
        .record("reroutes", reroutes.len())
        .record("partners", partners.len());
    let staying: Vec<Vec<Point>> = diagram
        .connectors
        .iter()
        .filter(|connector| {
            !rerouted_ids.contains(connector.id.as_str()) && !partner_ids.contains(connector.id.as_str())
        })
        .map(|connector| connector.points.clone())
        .collect();
    let staying_ports = settled_ports(
        diagram,
        &nodes_by_id,
        &rerouted_ids.union(&partner_ids).copied().collect(),
    );
    let reroutes: Vec<Reroute> = reroutes
        .into_iter()
        .chain(partners.iter().map(|(partner, _)| Reroute { ..*partner }))
        .collect();
    let improved = improve_ports(
        &reroutes,
        &staying,
        &staying_ports,
        [first_ports, partners.iter().map(|(_, ports)| *ports).collect()].concat(),
        [
            first_routes,
            partners
                .iter()
                .map(|(partner, _)| partner.connector.points.clone())
                .collect(),
        ]
        .concat(),
        &route_with,
    );
    // Partners whose ports stayed keep their route as drawn.
    let (reroutes, final_routes): (Vec<Reroute>, Vec<Vec<Point>>) = reroutes
        .into_iter()
        .zip(improved)
        .filter(|(reroute, route)| {
            !partner_ids.contains(reroute.connector.id.as_str()) || *route != reroute.connector.points
        })
        .unzip();

    // Labels a new route runs over move out of its way.
    let pushed: HashSet<&str> = pushable_labels
        .iter()
        .filter(|label| {
            final_routes
                .iter()
                .any(|route| distance_to_route(&label.bounds, route) < crate::inspect::CLEARANCE)
        })
        .map(|label| label.id.as_str())
        .collect();
    let routed = write_routes(&draft, diagram, &reroutes, final_routes);

    let moving: Vec<_> = diagram
        .labels
        .iter()
        .enumerate()
        .filter(|(_, label)| {
            detached_ids.contains(label.id.as_str())
                || pushed.contains(label.id.as_str())
                || label.connector.as_deref().is_some_and(|id| rerouted_ids.contains(id))
        })
        .collect();
    let moving_indices: HashSet<usize> = moving.iter().map(|(index, _)| *index).collect();
    let still: Vec<Bounds> = diagram
        .labels
        .iter()
        .enumerate()
        .filter(|(index, _)| !moving_indices.contains(index))
        .map(|(_, label)| label.bounds)
        .collect();
    let all_final: Vec<Vec<Point>> = routed.routes.iter().map(|(_, route)| route.clone()).collect();
    // Moving a label rewrites positions only, so every text keeps its path.
    let label_texts = label_text_index(&routed.draft.document);
    let (placed, _, stuck) = moving.iter().fold(
        (routed.draft, still, HashSet::new()),
        |(draft, obstacles, mut stuck), (_, label)| {
            let connector_id = label.connector.clone().unwrap_or_default();
            let route = routed
                .routes
                .iter()
                .find(|(id, _)| *id == connector_id)
                .map(|(_, route)| route);
            let texts = label_texts.get(&label.id).map_or(&[][..], Vec::as_slice);
            let (Some(route), false) = (route, texts.is_empty()) else {
                return (draft, obstacles, stuck);
            };
            let blocked = [obstacles.clone(), label_blockers(diagram, &connector_id)].concat();
            let containers = label_containers(diagram, &connector_id);
            let (width, height) = (label.bounds.width, label.bounds.height);
            // A pushed label takes a spot clear of the other connectors when
            // there is one.
            let clear = pushed
                .contains(label.id.as_str())
                .then(|| place_label_clear(width, height, route, &all_final, &blocked, &containers))
                .flatten();
            let Some(placement) =
                clear.or_else(|| place_label(width, height, route, &all_final, &blocked, &containers))
            else {
                if pushed.contains(label.id.as_str()) {
                    stuck.insert(label.id.clone());
                }
                return (draft, obstacles, stuck);
            };
            let updated = move_label(
                draft.document.clone(),
                texts,
                label.bounds,
                placement.bounds,
                placement.anchor,
            );
            let message = format!("Moved label \"{}\" beside connector \"{connector_id}\".", label.id);
            let draft = draft.changed(updated, "move-label", message, &[&label.id, &connector_id]);
            (draft, [obstacles, vec![placement.bounds]].concat(), stuck)
        },
    );
    (placed, stuck)
}

/// Once the layout is settled, moves a middle segment of a connector that
/// runs closer than two lanes beside another away from it, onto the line
/// where running alongside costs nothing, wherever the routes then cost
/// less and stay clear of the nodes and labels around them; then places
/// the labels of the moved connectors again. A result that would be left
/// with more issues stays as it was.
#[tracing::instrument(skip_all)]
pub(crate) fn spread_close_runs(result: Draft) -> Draft {
    let draft = result.moving(Vec::new());
    let report = draft.report().clone();
    let diagram = &report.diagram;
    let obstacles_for = |connector: &DiagramConnector| -> Vec<Bounds> {
        diagram
            .nodes
            .iter()
            .filter(|node| !holds_end(&diagram.nodes, connector, node))
            .map(|node| node.bounds)
            .chain(
                diagram
                    .labels
                    .iter()
                    .filter(|label| label.connector.as_deref() != Some(connector.id.as_str()))
                    .map(|label| label.bounds),
            )
            .map(|bounds| bounds.inflate(CLEARANCE))
            .collect()
    };
    let obstacles: Vec<Vec<Bounds>> = diagram.connectors.iter().map(obstacles_for).collect();
    // How much a trial changes what the routes cost: only the routes it
    // moved cost differently, on their own and against the others.
    let change = |routes: &[Vec<Point>], trial: &[Vec<Point>]| -> f64 {
        let changed: Vec<bool> = routes.iter().zip(trial).map(|(old, new)| old != new).collect();
        (0..routes.len())
            .filter(|index| changed[*index])
            .map(|index| {
                route_cost(&trial[index]) - route_cost(&routes[index])
                    + (0..routes.len())
                        .filter(|other| *other != index && (!changed[*other] || *other > index))
                        .map(|other| {
                            routes_cost(&trial[index], &trial[other]) - routes_cost(&routes[index], &routes[other])
                        })
                        .sum::<f64>()
            })
            .sum()
    };
    let mut routes: Vec<Vec<Point>> = diagram.connectors.iter().map(|c| c.points.clone()).collect();
    let mut moved: HashSet<usize> = HashSet::new();
    for _ in 0..4 * routes.len() {
        // The first middle segment that runs too close beside another and
        // whose move away, pushing any it comes too close to, costs less.
        let better = (0..routes.len()).find_map(|index| {
            (1..routes[index].len().saturating_sub(2)).find_map(|k| {
                let (a, b) = (routes[index][k], routes[index][k + 1]);
                routes
                    .iter()
                    .enumerate()
                    .filter(|(other, _)| *other != index)
                    .flat_map(|(_, other)| segments(other))
                    .find_map(|(c, d)| {
                        let (at, other) = if a.x == b.x && c.x == d.x {
                            (a.x, c.x)
                        } else if a.y == b.y && c.y == d.y {
                            (a.y, c.y)
                        } else {
                            return None;
                        };
                        let gap = (at - other).abs();
                        if gap == 0.0 || gap >= CLOSE_RUN.reach || overlap((a, b), (c, d)) <= 0.0 {
                            return None;
                        }
                        let away = (at - other).signum();
                        let mut trial = routes.clone();
                        let pushed = push(
                            &mut trial,
                            &obstacles,
                            index,
                            k,
                            other + away * CLOSE_RUN.reach,
                            away,
                            0,
                        );
                        (pushed && change(&routes, &trial) < -1e-9).then_some(trial)
                    })
            })
        });
        let Some(trial) = better else { break };
        moved.extend((0..routes.len()).filter(|index| trial[*index] != routes[*index]));
        routes = trial;
    }
    if moved.is_empty() {
        return result;
    }
    let mut moved: Vec<usize> = moved.into_iter().collect();
    moved.sort_unstable();
    let (reroutes, finals): (Vec<Reroute>, Vec<Vec<Point>>) = moved
        .iter()
        .filter_map(|index| {
            let connector = &diagram.connectors[*index];
            let reroute = Reroute::new(connector, diagram.node(&connector.from)?, diagram.node(&connector.to)?);
            Some((reroute, routes[*index].clone()))
        })
        .unzip();
    let routed = write_routes(&draft, diagram, &reroutes, finals);
    let draft = expand_view_box(settle_labels(routed.draft, &connector_landmarks(diagram)));
    if draft.report().issues.len() > result.report().issues.len() {
        result
    } else {
        draft
    }
}

/// Moves middle segment `k` of route `index` onto `line`, pushing on by two
/// lanes, in the same direction `away`, any other route's middle segment
/// beyond it that it would run within two lanes of. Fails when a route would
/// turn back on itself, run into the clearance of a node or label, or have
/// to push a route's first or last segment.
fn push(
    routes: &mut [Vec<Point>],
    obstacles: &[Vec<Bounds>],
    index: usize,
    k: usize,
    line: f64,
    away: f64,
    depth: usize,
) -> bool {
    if depth > 4 {
        return false;
    }
    let route = &routes[index];
    let vertical = route[k].x == route[k + 1].x;
    let mut moved = route.clone();
    for point in &mut moved[k..=k + 1] {
        if vertical {
            point.x = line;
        } else {
            point.y = line;
        }
    }
    let heading = |p: Point, q: Point| ((q.x - p.x).signum(), (q.y - p.y).signum());
    let keeps_headings = route
        .windows(2)
        .zip(moved.windows(2))
        .all(|(old, new)| new[0] != new[1] && heading(old[0], old[1]) == heading(new[0], new[1]));
    if !keeps_headings || !route_is_clear(&moved[k - 1..=k + 2], &obstacles[index]) {
        return false;
    }
    let segment = (moved[k], moved[k + 1]);
    routes[index] = moved;
    for other in (0..routes.len()).filter(|other| *other != index) {
        let others = segments(&routes[other]);
        for (m, (c, d)) in others.iter().enumerate() {
            let at = if vertical && c.x == d.x {
                c.x
            } else if !vertical && c.y == d.y {
                c.y
            } else {
                continue;
            };
            if (at - line) * away < 0.0 || (at - line).abs() >= CLOSE_RUN.reach || overlap(segment, (*c, *d)) <= 0.0 {
                continue;
            }
            if m == 0
                || m + 1 == others.len()
                || !push(
                    routes,
                    obstacles,
                    other,
                    m,
                    line + away * CLOSE_RUN.reach,
                    away,
                    depth + 1,
                )
            {
                return false;
            }
        }
    }
    true
}

/// The crossings, bends and ends of every connector, by connector id.
fn connector_landmarks(diagram: &Diagram) -> HashMap<String, Vec<Point>> {
    let routes: Vec<Vec<Point>> = diagram.connectors.iter().map(|c| c.points.clone()).collect();
    diagram
        .connectors
        .iter()
        .map(|connector| (connector.id.clone(), label_landmarks(&connector.points, &routes)))
        .collect()
}

/// Places connector labels again where they may no longer be the best
/// spot: beside a connector whose crossings, bends or ends have changed
/// since `before`, wherever the label came from, and within a label
/// clearance of another connector, where it could be taken for that
/// one's label, when a spot clear of the others exists.
#[tracing::instrument(skip_all)]
fn settle_labels(draft: Draft, before: &HashMap<String, Vec<Point>>) -> Draft {
    let report = draft.report().clone();
    let diagram = &report.diagram;
    let routes: Vec<Vec<Point>> = diagram.connectors.iter().map(|c| c.points.clone()).collect();
    let now = connector_landmarks(diagram);
    let unsettled: Vec<_> = diagram
        .labels
        .iter()
        .filter_map(|label| {
            let connector = diagram.connector(label.connector.as_deref()?)?;
            let stale = before
                .get(&connector.id)
                .is_some_and(|before| Some(before) != now.get(&connector.id));
            let beside = segments(&connector.points).into_iter().min_by(|a, b| {
                distance_to_route(&label.bounds, &[a.0, a.1]).total_cmp(&distance_to_route(&label.bounds, &[b.0, b.1]))
            })?;
            let nearest = diagram
                .connectors
                .iter()
                .filter(|other| other.id != connector.id)
                .flat_map(|other| segments(&other.points))
                .filter(|other| !runs_across(beside, &label.bounds, *other))
                .map(|(c, d)| distance_to_route(&label.bounds, &[c, d]))
                .fold(f64::INFINITY, f64::min);
            (stale || nearest < LABEL_CLEAR_OF_OTHERS).then_some((label, connector, stale))
        })
        .collect();
    let others: Vec<Bounds> = diagram.labels.iter().map(|label| label.bounds).collect();
    // Moving a label rewrites positions only, so every text keeps its path.
    let label_texts = label_text_index(&draft.document);
    let (settled, _) = unsettled
        .into_iter()
        .fold((draft, others), |(draft, obstacles), (label, connector, stale)| {
            let texts = label_texts.get(&label.id).map_or(&[][..], Vec::as_slice);
            if texts.is_empty() {
                return (draft, obstacles);
            }
            let blocked: Vec<Bounds> = obstacles
                .iter()
                .filter(|bounds| **bounds != label.bounds)
                .copied()
                .chain(label_blockers(diagram, &connector.id))
                .collect();
            let place = if stale { place_label } else { place_label_clear };
            let Some(placement) = place(
                label.bounds.width,
                label.bounds.height,
                &connector.points,
                &routes,
                &blocked,
                &label_containers(diagram, &connector.id),
            )
            .filter(|placement| placement.bounds != label.bounds) else {
                return (draft, obstacles);
            };
            let updated = move_label(
                draft.document.clone(),
                texts,
                label.bounds,
                placement.bounds,
                placement.anchor,
            );
            let message = if stale {
                format!(
                    "Moved label \"{}\" away from where \"{}\" now crosses, turns or ends.",
                    label.id, connector.id
                )
            } else {
                format!(
                    "Moved label \"{}\" clear of connectors other than \"{}\".",
                    label.id, connector.id
                )
            };
            let draft = draft.changed(updated, "move-label", message, &[&label.id, &connector.id]);
            let obstacles = obstacles
                .into_iter()
                .map(|bounds| {
                    if bounds == label.bounds {
                        placement.bounds
                    } else {
                        bounds
                    }
                })
                .collect();
            (draft, obstacles)
        });
    settled
}

/// Node boxes a connector's label keeps clear of: every node except the
/// containers around the connector's ends, inside which the label may sit.
fn label_blockers(diagram: &Diagram, connector_id: &str) -> Vec<Bounds> {
    let containers = end_container_ids(diagram, connector_id);
    diagram
        .nodes
        .iter()
        .filter(|node| !containers.contains(&node.id.as_str()))
        .map(|node| node.bounds)
        .collect()
}

/// The containers around a connector's ends, which its label may sit in
/// but not across.
fn label_containers(diagram: &Diagram, connector_id: &str) -> Vec<Bounds> {
    let containers = end_container_ids(diagram, connector_id);
    diagram
        .nodes
        .iter()
        .filter(|node| containers.contains(&node.id.as_str()))
        .map(|node| node.bounds)
        .collect()
}

fn end_container_ids<'a>(diagram: &'a Diagram, connector_id: &str) -> Vec<&'a str> {
    diagram
        .connector(connector_id)
        .map(|connector| {
            [
                ancestor_ids(&diagram.nodes, &connector.from),
                ancestor_ids(&diagram.nodes, &connector.to),
            ]
            .concat()
        })
        .unwrap_or_default()
}

/// Replaces or appends a connector's route, keeping the original order.
/// Routes a connector with the given ports around the routes occupying the
/// diagram.
type RouteWith<'a> = dyn Fn(&Reroute, Ports, &[Vec<Point>]) -> Vec<Point> + 'a;

/// Port changes that spreading ports evenly cannot see, tried once routes
/// are drawn: two connectors on the same node side whose routes cross
/// trade their ports there (a connector running straight takes its other
/// end along), and an end that steps aside just before its node slides
/// along the side to the line it arrives on. A change stays
/// when the connectors it reroutes cross less without turning more, or
/// turn less without crossing more.
#[tracing::instrument(skip_all)]
fn improve_ports(
    reroutes: &[Reroute],
    settled: &[Vec<Point>],
    fixed: &HashMap<String, Vec<f64>>,
    ports: Vec<Ports>,
    routes: Vec<Vec<Point>>,
    route_with: &RouteWith,
) -> Vec<Vec<Point>> {
    // Reroutes the connectors at `changed` with their new ports, keeping the
    // change when it is better. The first one rerouted takes the lane it
    // likes and the next goes around it, so each order is tried.
    let attempt = |(ports, routes): (Vec<Ports>, Vec<Vec<Point>>), changed: &[(usize, Ports)]| {
        let others: Vec<Vec<Point>> = settled
            .iter()
            .cloned()
            .chain(
                routes
                    .iter()
                    .enumerate()
                    .filter(|(index, _)| changed.iter().all(|(changed, _)| changed != index))
                    .map(|(_, route)| route.clone()),
            )
            .collect();
        let cost = |set: &[Vec<Point>]| {
            let crossed: usize = set
                .iter()
                .enumerate()
                .map(|(index, route)| {
                    others
                        .iter()
                        .chain(&set[index + 1..])
                        .map(|other| crossings(route, other))
                        .sum::<usize>()
                })
                .sum();
            (crossed, set.iter().map(|route| turns(route)).sum::<usize>())
        };
        // Routes in the order of `changed`, rerouting them in `order`.
        let reroute_in = |order: &[usize]| {
            let mut rerouted: Vec<Option<Vec<Point>>> = vec![None; changed.len()];
            for position in order {
                let (index, new) = changed[*position];
                let occupied: Vec<Vec<Point>> = others
                    .iter()
                    .cloned()
                    .chain(rerouted.iter().flatten().cloned())
                    .collect();
                rerouted[*position] = Some(route_with(&reroutes[index], new, &occupied));
            }
            rerouted.into_iter().flatten().collect::<Vec<_>>()
        };
        let forward: Vec<usize> = (0..changed.len()).collect();
        let backward: Vec<usize> = forward.iter().rev().copied().collect();
        let orders = if changed.len() > 1 {
            vec![forward, backward]
        } else {
            vec![forward]
        };
        let rerouted = orders
            .iter()
            .map(|order| reroute_in(order))
            .min_by_key(|set| cost(set))
            .expect("at least one order");
        let current: Vec<Vec<Point>> = changed.iter().map(|(index, _)| routes[*index].clone()).collect();
        let (old, new) = (cost(&current), cost(&rerouted));
        if new.0 <= old.0 && new.1 <= old.1 && new != old {
            changed
                .iter()
                .zip(rerouted)
                .fold((ports, routes), |(mut ports, mut routes), ((index, port), route)| {
                    ports[*index] = *port;
                    routes[*index] = route;
                    (ports, routes)
                })
        } else {
            (ports, routes)
        }
    };
    // The node at one end of a connector (0 for its start, 1 for its end)
    // and that end's port.
    let node_at = |index: usize, end: usize| {
        if end == 0 {
            reroutes[index].source
        } else {
            reroutes[index].target
        }
    };
    let port_at = |ports: &Ports, end: usize| if end == 0 { ports.start } else { ports.end };
    // An end its port pins stays where it is.
    let pinned = |index: usize, end: usize| reroutes[index].pins[end].is_some();
    let with_port = |ports: Ports, end: usize, p: Point| {
        if end == 0 {
            Ports { start: p, ..ports }
        } else {
            Ports { end: p, ..ports }
        }
    };
    // The stretch of a side a port may move to: its middle half, as a port
    // near a corner looks pushed aside, and one near a circle's tangent
    // would meet it at a glancing angle.
    let usable = |index: usize, end: usize, side: Side| {
        let node = node_at(index, end);
        let (low, length) = if matches!(side, Side::Left | Side::Right) {
            (node.bounds.y, node.bounds.height)
        } else {
            (node.bounds.x, node.bounds.width)
        };
        let margin = (length / 4.0).max(8.0);
        (low + margin, low + length - margin)
    };
    // Whether an end can take the coordinate `value` along `side`: within
    // its usable stretch and a lane from every other port there.
    let free = |ports: &[Ports], index: usize, end: usize, side: Side, value: f64| {
        let node = node_at(index, end);
        let along_y = matches!(side, Side::Left | Side::Right);
        let (low, high) = usable(index, end, side);
        let mut taken = fixed
            .get(&side_key(&node.bounds, side))
            .into_iter()
            .flatten()
            .copied()
            .chain((0..reroutes.len()).flat_map(|other| {
                (0..2).filter_map(move |other_end| {
                    if (other, other_end) == (index, end) {
                        return None;
                    }
                    let owner = node_at(other, other_end);
                    let p = port_at(&ports[other], other_end);
                    (owner.id == node.id && side_of(p, &owner.bounds) == side).then_some(if along_y {
                        p.y
                    } else {
                        p.x
                    })
                })
            }));
        value >= low && value <= high && taken.all(|other| (other - value).abs() >= LANE_SPACING)
    };
    let moved_to = |port: Point, side: Side, value: f64| {
        if matches!(side, Side::Left | Side::Right) {
            point(port.x, value)
        } else {
            point(value, port.y)
        }
    };
    // A connector running straight between facing sides takes its other
    // end along when one end moves, so it stays straight, if that end is
    // free to move.
    let carried = |ports: &[Ports], index: usize, end: usize, new: Point| {
        let moved = with_port(ports[index], end, new);
        let old = port_at(&ports[index], end);
        let side = side_of(old, &node_at(index, end).bounds);
        let opposite = 1 - end;
        let far = port_at(&ports[index], opposite);
        let far_side = side_of(far, &node_at(index, opposite).bounds);
        let facing = matches!(
            (side, far_side),
            (Side::Top, Side::Bottom)
                | (Side::Bottom, Side::Top)
                | (Side::Left, Side::Right)
                | (Side::Right, Side::Left)
        );
        let along = |p: Point| {
            if matches!(side, Side::Left | Side::Right) {
                p.y
            } else {
                p.x
            }
        };
        if facing
            && along(far) == along(old)
            && !pinned(index, opposite)
            && free(ports, index, opposite, far_side, along(new))
        {
            with_port(moved, opposite, moved_to(far, far_side, along(new)))
        } else {
            moved
        }
    };
    let pairs: Vec<(usize, usize, usize, usize)> = (0..reroutes.len())
        .flat_map(|first| ((first + 1)..reroutes.len()).map(move |second| (first, second)))
        .flat_map(|(first, second)| (0..2).flat_map(move |a| (0..2).map(move |b| (first, second, a, b))))
        .collect();
    let traded = pairs
        .into_iter()
        .fold((ports, routes), |(ports, routes), (first, second, a, b)| {
            let (one, other) = (node_at(first, a), node_at(second, b));
            let (p, q) = (port_at(&ports[first], a), port_at(&ports[second], b));
            let shared = one.id == other.id && side_of(p, &one.bounds) == side_of(q, &other.bounds);
            if !shared || pinned(first, a) || pinned(second, b) || crossings(&routes[first], &routes[second]) == 0 {
                return (ports, routes);
            }
            let changed = [
                (first, carried(&ports, first, a, q)),
                (second, carried(&ports, second, b, p)),
            ];
            attempt((ports, routes), &changed)
        });
    let ends: Vec<(usize, usize)> = (0..reroutes.len()).flat_map(|index| [(index, 0), (index, 1)]).collect();
    // A route with the end at `end` last.
    let toward = |routes: &[Vec<Point>], index: usize, end: usize| -> Vec<Point> {
        if end == 0 {
            routes[index].iter().rev().copied().collect()
        } else {
            routes[index].clone()
        }
    };
    let slid = ends.iter().fold(traded, |(ports, routes), &(index, end)| {
        if pinned(index, end) {
            return (ports, routes);
        }
        let port = port_at(&ports[index], end);
        let side = side_of(port, &node_at(index, end).bounds);
        let route = toward(&routes, index, end);
        let Some(line) = stepped_end(&route) else {
            return (ports, routes);
        };
        // Where the line before the step lies off the side's usable stretch,
        // the end moves as near it as it may; the route rerouted to there
        // may then run straight in.
        let (low, high) = usable(index, end, side);
        let line = line.clamp(low, high);
        if moved_to(port, side, line) == port || !free(&ports, index, end, side, line) {
            return (ports, routes);
        }
        let changed = [(index, with_port(ports[index], end, moved_to(port, side, line)))];
        attempt((ports, routes), &changed)
    });
    // An end that turns onto a short last segment just before its node
    // moves to the side the route was heading into, where the route may run
    // straight in.
    let (_, turned) = ends.iter().fold(slid, |(ports, routes), &(index, end)| {
        if pinned(index, end) {
            return (ports, routes);
        }
        let Some((side, line)) = turned_end(&toward(&routes, index, end)) else {
            return (ports, routes);
        };
        let bounds = node_at(index, end).bounds;
        let (low, high) = usable(index, end, side);
        let line = line.clamp(low, high);
        if side == side_of(port_at(&ports[index], end), &bounds) || !free(&ports, index, end, side, line) {
            return (ports, routes);
        }
        let port = match side {
            Side::Left => point(bounds.x, line),
            Side::Right => point(bounds.right(), line),
            Side::Top => point(line, bounds.y),
            Side::Bottom => point(line, bounds.bottom()),
        };
        let changed = [(index, with_port(ports[index], end, port))];
        attempt((ports, routes), &changed)
    });
    turned
}

/// Where a route turns onto a short last segment just before its last
/// point: the side of the node it would enter by running on along the
/// segment before the turn, and that segment's coordinate across it.
fn turned_end(route: &[Point]) -> Option<(Side, f64)> {
    let n = route.len();
    if n < 3 {
        return None;
    }
    let (b, c, d) = (route[n - 3], route[n - 2], route[n - 1]);
    if (d.x - c.x).abs() + (d.y - c.y).abs() > END_STUBS[0] {
        return None;
    }
    if b.y == c.y && c.x == d.x && b.x != c.x {
        Some((if c.x < b.x { Side::Right } else { Side::Left }, c.y))
    } else if b.x == c.x && c.y == d.y && b.y != c.y {
        Some((if c.y < b.y { Side::Bottom } else { Side::Top }, c.x))
    } else {
        None
    }
}

/// Where a route steps aside just before its last point: its last three
/// segments turn one way and back, so the end could lie on the line of the
/// segment before the step instead. Gives that line's coordinate across
/// the last segment.
fn stepped_end(route: &[Point]) -> Option<f64> {
    let n = route.len();
    if n < 4 {
        return None;
    }
    let (a, b, c, d) = (route[n - 4], route[n - 3], route[n - 2], route[n - 1]);
    let direction = |p: Point, q: Point| ((q.x - p.x).signum(), (q.y - p.y).signum());
    if direction(a, b) != direction(c, d) {
        return None;
    }
    if c.y == d.y && a.y == b.y {
        Some(b.y)
    } else if c.x == d.x && a.x == b.x {
        Some(b.x)
    } else {
        None
    }
}

fn with_route(routes: Vec<(String, Vec<Point>)>, id: &str, route: Vec<Point>) -> Vec<(String, Vec<Point>)> {
    if routes.iter().any(|(existing, _)| existing == id) {
        routes
            .into_iter()
            .map(|(existing, points)| {
                if existing == id {
                    (existing, route.clone())
                } else {
                    (existing, points)
                }
            })
            .collect()
    } else {
        [routes, vec![(id.to_owned(), route)]].concat()
    }
}

/// The path that replaces a rerouted connector, keeping its other
/// attributes. A line is never filled, but a path is filled black by
/// default, so an unfilled line stays unfilled.
fn rerouted_element(original: &Element, route: &[Point], namespace_undeclared: bool) -> Element {
    let geometry = ["x1", "y1", "x2", "y2", "points", "d"];
    let copied = Element {
        attributes: original
            .attributes
            .iter()
            .filter(|(name, _)| !geometry.contains(&name.as_str()))
            .cloned()
            .collect(),
        ..Element::new("path")
    };
    let fill_style = regex_fill(copied.attr("style"));
    let filled = if original.name == "line" && !copied.has_attr("fill") && !fill_style {
        copied.with_attr("fill", "none")
    } else {
        copied
    };
    let data = route
        .iter()
        .enumerate()
        .map(|(index, p)| {
            format!(
                "{} {} {}",
                if index == 0 { "M" } else { "L" },
                format_number(p.x),
                format_number(p.y)
            )
        })
        .collect::<Vec<_>>()
        .join(" ");
    let path = filled.with_attr("d", &data);
    Element {
        declares_svg_namespace: namespace_undeclared && !path.has_attr("xmlns"),
        ..path
    }
}

/// Whether a style attribute sets `fill`.
fn regex_fill(style: &str) -> bool {
    static FILL: std::sync::LazyLock<regex::Regex> =
        std::sync::LazyLock::new(|| regex::Regex::new(r"(^|;)\s*fill\s*:").expect("valid pattern"));
    FILL.is_match(style)
}

/// Grows the viewBox to the drawing plus its safe padding when it clips.
#[tracing::instrument(skip_all)]
fn expand_view_box(draft: Draft) -> Draft {
    let report = draft.report().clone();
    let root_path = vec![draft.document.root_index()];
    let view_box = parse_view_box(draft.document.root().attr("viewBox"));
    let clipped = report.issues.iter().any(|issue| issue.code == "viewport-clipping");
    let (Some(view_box), Some(drawing), true) = (view_box, report.drawing_bounds, clipped) else {
        return draft;
    };
    let required = drawing.inflate(VIEWPORT_PADDING);
    let left = view_box.x.min(required.x);
    let top = view_box.y.min(required.y);
    let right = view_box.right().max(required.right());
    let bottom = view_box.bottom().max(required.bottom());
    let expanded = Bounds {
        x: left,
        y: top,
        width: right - left,
        height: bottom - top,
    };
    let updated = draft
        .document
        .update(&root_path, |root| root.with_attr("viewBox", &expanded.format()));
    let message = format!("Expanded viewBox from {} to {}.", view_box.format(), expanded.format());
    let draft = grow_backgrounds(
        draft.changed(updated, "expand-viewbox", message, &["svg"]),
        &view_box,
        &expanded,
    );
    scale_size(draft, &view_box, &expanded)
}

/// Grows each rect drawn straight on the root that covered the whole old
/// viewBox, such as a background fill, to cover the expanded one too. A
/// rect sized in percentages already follows the viewBox's size, so one at
/// the old origin only moves to the new origin.
fn grow_backgrounds(draft: Draft, before: &Bounds, after: &Bounds) -> Draft {
    let root_path = vec![draft.document.root_index()];
    let relative = |element: &Element| ["width", "height"].iter().any(|name| element.attr(name).contains('%'));
    let backgrounds: Vec<Vec<usize>> = child_element_paths(&draft.document, &root_path)
        .into_iter()
        .filter(|path| {
            let element = draft.document.element(path);
            let current = rect_bounds(&draft.document, path);
            element.name == "rect"
                && if relative(element) {
                    (current.x, current.y) == (before.x, before.y)
                } else {
                    current.x <= before.x
                        && current.y <= before.y
                        && current.right() >= before.right()
                        && current.bottom() >= before.bottom()
                }
        })
        .collect();
    backgrounds.into_iter().fold(draft, |draft, path| {
        let document = &draft.document;
        let element = document.element(&path);
        let current = rect_bounds(document, &path);
        let offset = point(
            current.x - number_attribute(element, "x"),
            current.y - number_attribute(element, "y"),
        );
        let (left, top) = (current.x.min(after.x), current.y.min(after.y));
        let grown = Bounds {
            x: left,
            y: top,
            width: current.right().max(after.right()) - left,
            height: current.bottom().max(after.bottom()) - top,
        };
        if relative(element) && (left, top) == (current.x, current.y) {
            return draft;
        }
        let id = non_empty(element.attr("id")).unwrap_or("background").to_owned();
        let relative = relative(element);
        let updated = document.update(&path, |rect| {
            let moved = rect
                .with_attr("x", &format_number(grown.x - offset.x))
                .with_attr("y", &format_number(grown.y - offset.y));
            if relative {
                moved
            } else {
                moved
                    .with_attr("width", &format_number(grown.width))
                    .with_attr("height", &format_number(grown.height))
            }
        });
        let message = format!("Expanded background \"{id}\" to cover the viewBox {}.", after.format());
        draft.changed(updated, "expand-background", message, &[&id])
    })
}

/// Grows absolute `width` and `height` by the same ratio as the viewBox, so
/// the drawing keeps its scale. Percentages and missing sizes are left to
/// the container.
fn scale_size(draft: Draft, before: &Bounds, after: &Bounds) -> Draft {
    let root_path = vec![draft.document.root_index()];
    let root = draft.document.root();
    let scaled: Vec<(&str, String, String)> = [
        ("width", before.width, after.width),
        ("height", before.height, after.height),
    ]
    .into_iter()
    .filter(|(_, from, to)| from != to && *from > 0.0)
    .filter_map(|(name, from, to)| {
        let (value, unit) = absolute_length(root.attr(name))?;
        Some((
            name,
            root.attr(name).to_owned(),
            format!("{}{unit}", format_number(value * to / from)),
        ))
    })
    .collect();
    if scaled.is_empty() {
        return draft;
    }
    let updated = draft.document.update(&root_path, |root| {
        scaled
            .iter()
            .fold(root, |root, (name, _, value)| root.with_attr(name, value))
    });
    let message = format!(
        "Scaled the SVG {} to keep the drawing scale.",
        scaled
            .iter()
            .map(|(name, from, to)| format!("{name} from {from} to {to}"))
            .collect::<Vec<_>>()
            .join(" and ")
    );
    draft.changed(updated, "scale-size", message, &["svg"])
}

/// A length with an absolute unit (or none), split into number and unit.
fn absolute_length(text: &str) -> Option<(f64, &str)> {
    let trimmed = text.trim();
    let unit_at = trimmed
        .find(|c: char| c.is_ascii_alphabetic() || c == '%')
        .unwrap_or(trimmed.len());
    let (number, unit) = trimmed.split_at(unit_at);
    let absolute = ["", "px", "pt", "pc", "mm", "cm", "in", "em", "ex"];
    let value = number
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite() && *value > 0.0)?;
    absolute.contains(&unit).then_some((value, unit))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyze;

    /// Given routes ending on a node: one running left and then turning up
    ///   onto a 23px last segment, one doing the same onto a 30px segment,
    ///   and one running up and then turning right onto a 20px segment
    /// When the turn just before each end is looked for
    /// Then the first would run straight into the node's right side at the
    ///   height it came along, the third into its bottom side at the line
    ///   it came along, and the second, turning well before its node, is
    ///   left as it is
    #[test]
    fn finds_a_route_turning_onto_a_short_last_segment() {
        let turning_up = |rise: f64| vec![point(400.0, 350.0), point(100.0, 350.0), point(100.0, 350.0 - rise)];
        let turning_right = vec![point(100.0, 400.0), point(100.0, 100.0), point(120.0, 100.0)];

        assert_eq!(turned_end(&turning_up(23.0)), Some((Side::Right, 350.0)));
        assert_eq!(turned_end(&turning_up(30.0)), None);
        assert_eq!(turned_end(&turning_right), Some((Side::Bottom, 100.0)));
    }

    /// Given two connectors whose middle segments run down side by side 8px
    ///   apart, with room on either side
    /// When their close runs are spread once the layout is settled
    /// Then the two run at least two lanes apart, each keeps its bends, and
    ///   both still start and end where they did
    #[test]
    fn spreads_middle_segments_running_too_close_apart() {
        let fixed = fix(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 400 480">
              <g data-node="a"><rect x="0" y="0" width="60" height="40"/></g>
              <g data-node="b"><rect x="300" y="300" width="60" height="40"/></g>
              <g data-node="c"><rect x="0" y="100" width="60" height="40"/></g>
              <g data-node="d"><rect x="300" y="400" width="60" height="40"/></g>
              <path id="p" data-from="a" data-to="b" fill="none" d="M 60 20 L 200 20 L 200 320 L 300 320"/>
              <path id="q" data-from="c" data-to="d" fill="none" d="M 60 120 L 208 120 L 208 420 L 300 420"/>
            </svg>"#)
        .unwrap();
        let points = |result: &FixResult, id: &str| result.report.diagram.connector(id).unwrap().points.clone();
        let (before_p, before_q) = (points(&fixed, "p"), points(&fixed, "q"));
        assert_eq!(before_q[1].x - before_p[1].x, 8.0);

        let spread =
            spread_close_runs(Draft::new(parse(&fixed.svg).unwrap(), Changes::default(), Vec::new())).written();

        let (p, q) = (points(&spread, "p"), points(&spread, "q"));
        assert!((q[1].x - p[1].x).abs() >= 16.0, "{p:?} {q:?}");
        assert_eq!((turns(&p), turns(&q)), (2, 2));
        assert_eq!(
            (p[0], p[3], q[0], q[3]),
            (before_p[0], before_p[3], before_q[0], before_q[3])
        );
    }

    /// Given three connectors whose middle segments run down side by side,
    ///   8px and then 10px apart, with room on either side
    /// When their close runs are spread once the layout is settled
    /// Then every two of them run at least two lanes apart: moving one away
    ///   from its neighbour pushes the next one on
    #[test]
    fn spreads_a_bundle_of_close_runs_pushing_each_on() {
        let fixed = fix(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 400 580">
              <g data-node="a"><rect x="0" y="0" width="60" height="40"/></g>
              <g data-node="b"><rect x="300" y="300" width="60" height="40"/></g>
              <g data-node="c"><rect x="0" y="100" width="60" height="40"/></g>
              <g data-node="d"><rect x="300" y="400" width="60" height="40"/></g>
              <g data-node="e"><rect x="0" y="200" width="60" height="40"/></g>
              <g data-node="f"><rect x="300" y="500" width="60" height="40"/></g>
              <path id="p" data-from="a" data-to="b" fill="none" d="M 60 20 L 210 20 L 210 320 L 300 320"/>
              <path id="q" data-from="c" data-to="d" fill="none" d="M 60 120 L 202 120 L 202 420 L 300 420"/>
              <path id="r" data-from="e" data-to="f" fill="none" d="M 60 220 L 192 220 L 192 520 L 300 520"/>
            </svg>"#)
        .unwrap();
        let x = |result: &FixResult, id: &str| result.report.diagram.connector(id).unwrap().points[1].x;
        assert_eq!((x(&fixed, "p"), x(&fixed, "q"), x(&fixed, "r")), (210.0, 202.0, 192.0));

        let spread =
            spread_close_runs(Draft::new(parse(&fixed.svg).unwrap(), Changes::default(), Vec::new())).written();

        let (p, q, r) = (x(&spread, "p"), x(&spread, "q"), x(&spread, "r"));
        assert!(
            (p - q).abs() >= 16.0 && (q - r).abs() >= 16.0 && (p - r).abs() >= 16.0,
            "{p} {q} {r}"
        );
    }

    /// Given two boxes joined by a straight connector with no issue
    /// When what to reroute is picked, with no node moved and with one of
    ///   its ends moved
    /// Then the connector is left alone until its node moves
    #[test]
    fn picks_a_connector_to_reroute_only_once_its_node_moves() {
        let report = analyze(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 400 100">
              <g data-node="a"><rect x="0" y="20" width="100" height="60"/><text x="50" y="55">A</text></g>
              <g data-node="b"><rect x="300" y="20" width="100" height="60"/><text x="350" y="55">B</text></g>
              <line data-from="a" data-to="b" x1="100" y1="50" x2="300" y2="50"/>
            </svg>"#,
        )
        .unwrap();
        let nodes_by_id: HashMap<&str, &DiagramNode> = report
            .diagram
            .nodes
            .iter()
            .map(|node| (node.id.as_str(), node))
            .collect();
        let picked = |moved: &[String]| pick_reroutes(&report, moved, &nodes_by_id).1.len();

        assert_eq!(picked(&[]), 0);
        assert_eq!(picked(&["a".to_owned()]), 1);
    }
}
