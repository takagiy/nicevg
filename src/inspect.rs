//! Mechanical layout checks. Each check maps the diagram to the issues it
//! finds; `inspect` concatenates them in a fixed order.

use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

use serde::Serialize;
use serde_json::{Map, Value, json};

use crate::diagram::{Diagram, DiagramEdge, DiagramLabel, DiagramNode, holds_end};
use crate::geometry::{
    Bounds, Point, distance_to_route, distance_to_segment, enclosing, format_number, hypot, intersection, json_bounds,
    json_number, number_list, point_is_inside, round, routes_overlap, segment_intersects_interior, segments,
};

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct DiagramIssue {
    pub code: String,
    pub message: String,
    pub elements: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Map<String, Value>>,
}

fn issue(code: &str, message: String, elements: &[&str], details: Option<Value>) -> DiagramIssue {
    DiagramIssue {
        code: code.to_owned(),
        message,
        elements: elements.iter().map(|element| (*element).to_owned()).collect(),
        details: details.and_then(|value| match value {
            Value::Object(map) => Some(map),
            _ => None,
        }),
    }
}

pub const TEXT_PADDING: f64 = 12.0;
/// How far a child keeps inside its parent's outline.
pub const PARENT_PADDING: f64 = 12.0;
pub const VIEWPORT_PADDING: f64 = 20.0;
pub const NODE_GAP: f64 = 20.0;
/// Space an edge label keeps from routes and nodes.
pub const EDGE_LABEL_CLEARANCE: f64 = 8.0;
/// Labels are placed one clearance plus a pixel away from their edge,
/// so a tied label further than this no longer reads as belonging to it.
pub const DETACHED_LABEL_DISTANCE: f64 = 16.0;

#[tracing::instrument(skip_all, fields(nodes = diagram.nodes.len(), edges = diagram.edges.len(), labels = diagram.labels.len()))]
pub fn inspect(view_box: Option<Bounds>, diagram: &Diagram) -> Vec<DiagramIssue> {
    [
        viewport(view_box, diagram.diagram_bounds()),
        text_fit(&diagram.nodes),
        children_outside_parents(&diagram.nodes),
        children_on_parent_texts(&diagram.nodes),
        duplicate_node_ids(&diagram.nodes),
        node_overlaps(&diagram.nodes),
        node_gaps(diagram),
        edge_crossings(&diagram.nodes, &diagram.edges),
        label_overlaps(&diagram.labels),
        label_node_overlaps(&diagram.nodes, &diagram.labels),
        edge_label_clearance(&diagram.edges, &diagram.labels),
        edge_unknown_nodes(&diagram.nodes, &diagram.edges),
        edge_ends(&diagram.nodes, &diagram.edges),
        slanted_edges(&diagram.edges),
        edge_ends_off_ports(&diagram.nodes, &diagram.edges),
        edge_ends_along_sides(&diagram.nodes, &diagram.edges),
        edge_overlaps(&diagram.edges),
        edge_near_misses(&diagram.edges),
        detached_labels(&diagram.labels, &diagram.edges),
    ]
    .concat()
}

pub fn parse_view_box(value: &str) -> Option<Bounds> {
    match number_list(value)[..] {
        [x, y, width, height] if [x, y, width, height].iter().all(|v| v.is_finite()) => {
            Some(Bounds { x, y, width, height })
        }
        _ => None,
    }
}

pub(crate) fn viewport(view_box: Option<Bounds>, diagram: Option<Bounds>) -> Vec<DiagramIssue> {
    let (Some(view_box), Some(diagram)) = (view_box, diagram) else {
        return Vec::new();
    };
    let required = diagram.inflate(VIEWPORT_PADDING);
    let sides: Vec<&str> = [
        (required.x < view_box.x, "left"),
        (required.y < view_box.y, "top"),
        (required.right() > view_box.right(), "right"),
        (required.bottom() > view_box.bottom(), "bottom"),
    ]
    .into_iter()
    .filter_map(|(clipped, side)| clipped.then_some(side))
    .collect();
    if sides.is_empty() {
        return Vec::new();
    }
    vec![issue(
        "diagram-clipped",
        format!("The diagram exceeds the safe viewBox on the {} side.", sides.join(", ")),
        &["svg"],
        Some(json!({ "sides": sides, "requiredViewBox": json_bounds(&required) })),
    )]
}

pub fn fits_in_box(label: &Bounds, box_: &Bounds, padding: f64) -> bool {
    label.x >= box_.x + padding
        && label.y >= box_.y + padding
        && label.right() <= box_.right() - padding
        && label.bottom() <= box_.bottom() - padding
}

/// Radius a circle centred like `box_` needs so that every corner of the
/// label keeps `padding` from its outline.
pub fn radius_enclosing(label: &Bounds, box_: &Bounds, padding: f64) -> f64 {
    let centre = box_.centre();
    let dx = (label.x - centre.x).abs().max((label.right() - centre.x).abs());
    let dy = (label.y - centre.y).abs().max((label.bottom() - centre.y).abs());
    hypot(dx, dy) + padding
}

fn text_fit(nodes: &[DiagramNode]) -> Vec<DiagramIssue> {
    nodes
        .iter()
        .filter(|node| !node.custom)
        .filter_map(|node| {
            let label = enclosing(&node.label_bounds)?;
            let fits = if node.is_circle() {
                radius_enclosing(&label, &node.bounds, TEXT_PADDING) <= node.bounds.width / 2.0
            } else {
                fits_in_box(&label, &node.bounds, TEXT_PADDING)
            };
            (!fits).then(|| {
                issue(
                    "node-label-overflows",
                    format!(
                        "Label does not fit inside node \"{}\" with {}px padding.",
                        node.id,
                        format_number(TEXT_PADDING)
                    ),
                    &[&node.id],
                    Some(json!({
                        "requiredWidth": json_number(label.width + TEXT_PADDING * 2.0),
                        "requiredHeight": json_number(label.height + TEXT_PADDING * 2.0),
                    })),
                )
            })
        })
        .collect()
}

fn ancestors<'a>(nodes: &'a [DiagramNode], node: &'a DiagramNode) -> impl Iterator<Item = &'a str> {
    std::iter::successors(node.parent_id.as_deref(), |parent| {
        nodes
            .iter()
            .find(|candidate| candidate.id == *parent)
            .and_then(|found| found.parent_id.as_deref())
    })
}

fn related(nodes: &[DiagramNode], first: &DiagramNode, second: &DiagramNode) -> bool {
    ancestors(nodes, first).any(|id| id == second.id) || ancestors(nodes, second).any(|id| id == first.id)
}

/// Pairs of sibling nodes that may not overlap.
fn comparable_pairs(nodes: &[DiagramNode]) -> impl Iterator<Item = (&DiagramNode, &DiagramNode)> {
    nodes.iter().enumerate().flat_map(move |(index, first)| {
        nodes[index + 1..]
            .iter()
            .filter(move |second| {
                !first.allow_overlap
                    && !second.allow_overlap
                    && !related(nodes, first, second)
                    && first.parent_id == second.parent_id
            })
            .map(move |second| (first, second))
    })
}

fn node_overlaps(nodes: &[DiagramNode]) -> Vec<DiagramIssue> {
    comparable_pairs(nodes)
        .filter_map(|(first, second)| {
            let overlap = intersection(&first.bounds, &second.bounds)?;
            Some(issue(
                "node-overlaps",
                format!("Nodes \"{}\" and \"{}\" overlap.", first.id, second.id),
                &[&first.id, &second.id],
                Some(json!({ "intersection": json_bounds(&overlap) })),
            ))
        })
        .collect()
}

/// Nodes side by side in a row closer than they may be: 20px, or wider
/// when an edge joining the two has a label, which must fit between
/// them beside the straight edge, a clearance off each node.
fn node_gaps(diagram: &Diagram) -> Vec<DiagramIssue> {
    comparable_pairs(&diagram.nodes)
        .filter_map(|(first, second)| {
            let vertical_overlap =
                first.bounds.bottom().min(second.bounds.bottom()) - first.bounds.y.max(second.bounds.y);
            if vertical_overlap <= 0.0 {
                return None;
            }
            let (left, right) = if first.bounds.x <= second.bounds.x {
                (first, second)
            } else {
                (second, first)
            };
            let gap = right.bounds.x - left.bounds.right();
            let required = label_gap(diagram, &left.id, &right.id);
            (0.0..required).contains(&gap).then(|| {
                issue(
                    "node-near-node",
                    format!(
                        "Nodes \"{}\" and \"{}\" have a {}px gap; {}px is required.",
                        left.id,
                        right.id,
                        format_number(gap),
                        format_number(required)
                    ),
                    &[&left.id, &right.id],
                    Some(json!({
                        "actualGap": json_number(gap),
                        "requiredGap": json_number(required),
                        "shortage": json_number(required - gap),
                    })),
                )
            })
        })
        .collect()
}

/// The gap two nodes need: the widest label of an edge joining them,
/// a clearance off each, or the node gap when that is wider.
fn label_gap(diagram: &Diagram, first: &str, second: &str) -> f64 {
    let joins =
        |edge: &DiagramEdge| (edge.from == first && edge.to == second) || (edge.from == second && edge.to == first);
    diagram
        .labels
        .iter()
        .filter(|label| label.edge.as_deref().and_then(|id| diagram.edge(id)).is_some_and(joins))
        .map(|label| label.bounds.width + 2.0 * EDGE_LABEL_CLEARANCE)
        .fold(NODE_GAP, f64::max)
}

/// Whether a segment passes through the open interior of a node's shape.
pub fn segment_enters_node(start: Point, end: Point, node: &DiagramNode) -> bool {
    if node.is_circle() {
        distance_to_segment(node.bounds.centre(), start, end) < node.bounds.width / 2.0 - 0.001
    } else {
        segment_intersects_interior(start, end, &node.bounds)
    }
}

fn edge_crossings(nodes: &[DiagramNode], edges: &[DiagramEdge]) -> Vec<DiagramIssue> {
    edges
        .iter()
        .flat_map(|edge| {
            nodes
                .iter()
                .filter(move |node| !holds_end(nodes, edge, node))
                .filter(move |node| {
                    segments(&edge.points)
                        .iter()
                        .any(|(from, to)| segment_enters_node(*from, *to, node))
                })
                .map(move |node| {
                    issue(
                        "edge-overlaps-node",
                        format!("Edge \"{}\" crosses unrelated node \"{}\".", edge.id, node.id),
                        &[&edge.id, &node.id],
                        None,
                    )
                })
        })
        .collect()
}

fn label_overlaps(labels: &[DiagramLabel]) -> Vec<DiagramIssue> {
    labels
        .iter()
        .enumerate()
        .flat_map(|(index, first)| {
            labels[index + 1..]
                .iter()
                .filter(move |second| intersection(&first.bounds, &second.bounds).is_some())
                .map(move |second| {
                    issue(
                        "label-overlaps",
                        format!("Labels \"{}\" and \"{}\" overlap.", first.id, second.id),
                        &[&first.id, &second.id],
                        None,
                    )
                })
        })
        .collect()
}

/// Whether a label's box covers part of a node's shape.
pub fn label_covers_node(label: &Bounds, node: &DiagramNode) -> bool {
    if node.is_circle() {
        let centre = node.bounds.centre();
        let nearest = Point {
            x: centre.x.clamp(label.x, label.right()),
            y: centre.y.clamp(label.y, label.bottom()),
        };
        hypot(nearest.x - centre.x, nearest.y - centre.y) < node.bounds.width / 2.0
    } else {
        intersection(label, &node.bounds).is_some()
    }
}

/// A parent's free space is a valid place for labels; only its children
/// and its outline are off limits.
fn lies_within_parent(nodes: &[DiagramNode], label: &Bounds, node: &DiagramNode) -> bool {
    nodes.iter().any(|child| child.parent_id.as_deref() == Some(&node.id))
        && intersection(label, &node.bounds) == Some(*label)
}

fn label_node_overlaps(nodes: &[DiagramNode], labels: &[DiagramLabel]) -> Vec<DiagramIssue> {
    labels
        .iter()
        .flat_map(|label| {
            nodes
                .iter()
                .filter(|node| {
                    label_covers_node(&label.bounds, node) && !lies_within_parent(nodes, &label.bounds, node)
                })
                .map(move |node| {
                    issue(
                        "label-overlaps-node",
                        format!("Label \"{}\" overlaps node \"{}\".", label.id, node.id),
                        &[&label.id, &node.id],
                        None,
                    )
                })
        })
        .collect()
}

fn edge_label_clearance(edges: &[DiagramEdge], labels: &[DiagramLabel]) -> Vec<DiagramIssue> {
    edges
        .iter()
        .flat_map(|edge| {
            labels
                .iter()
                .filter(move |label| {
                    let expanded = label.bounds.inflate(EDGE_LABEL_CLEARANCE);
                    segments(&edge.points)
                        .iter()
                        .any(|(from, to)| segment_intersects_interior(*from, *to, &expanded))
                })
                .map(move |label| {
                    issue(
                        "edge-near-label",
                        format!(
                            "Edge \"{}\" passes within {}px of label \"{}\".",
                            edge.id,
                            format_number(EDGE_LABEL_CLEARANCE),
                            label.id
                        ),
                        &[&edge.id, &label.id],
                        Some(json!({ "requiredClearance": json_number(EDGE_LABEL_CLEARANCE) })),
                    )
                })
        })
        .collect()
}

/// Ends on a circle are written with whole-unit coordinates, so a point
/// within half a unit of the circle counts as on it.
pub fn point_is_inside_node(target: Point, node: &DiagramNode) -> bool {
    if node.is_circle() {
        let centre = node.bounds.centre();
        hypot(target.x - centre.x, target.y - centre.y) < node.bounds.width / 2.0 - 0.5
    } else {
        point_is_inside(target, &node.bounds)
    }
}

/// How far short of its node an edge may stop, for an arrowhead drawn
/// in the gap.
pub const END_GAP: f64 = 16.0;

/// How far a point outside a node lies from the node's outline.
fn distance_to_node(target: Point, node: &DiagramNode) -> f64 {
    if node.is_circle() {
        let centre = node.bounds.centre();
        (hypot(target.x - centre.x, target.y - centre.y) - node.bounds.width / 2.0).max(0.0)
    } else {
        distance_to_route(&node.bounds, &[target, target])
    }
}

/// Edges with a segment neither horizontal nor vertical: nicevg routes
/// orthogonally, and a slanted line counts as straight to the bend count.
fn slanted_edges(edges: &[DiagramEdge]) -> Vec<DiagramIssue> {
    edges
        .iter()
        .filter(|edge| {
            edge.points
                .windows(2)
                .any(|pair| pair[0].x != pair[1].x && pair[0].y != pair[1].y)
        })
        .map(|edge| {
            issue(
                "edge-slanted",
                format!("Edge \"{}\" runs slanted.", edge.id),
                &[&edge.id],
                None,
            )
        })
        .collect()
}

fn edge_ends(nodes: &[DiagramNode], edges: &[DiagramEdge]) -> Vec<DiagramIssue> {
    let node = |id: &str| nodes.iter().find(|node| node.id == id);
    edges
        .iter()
        .flat_map(|edge| {
            let ends = [
                (
                    edge.points.first(),
                    node(&edge.from),
                    &edge.from_port,
                    "starts",
                    "start",
                ),
                (edge.points.last(), node(&edge.to), &edge.to_port, "ends", "end"),
            ];
            ends.into_iter().filter_map(move |(end, owner, port, verb, which)| {
                let (end, owner) = (end?, owner?);
                if point_is_inside_node(*end, owner) {
                    return Some(issue(
                        "edge-overlaps-node",
                        format!("Edge \"{}\" {verb} inside node \"{}\".", edge.id, owner.id),
                        &[&edge.id, &owner.id],
                        Some(json!({ "end": which })),
                    ));
                }
                // An end at a port may lie outside the node, where the port
                // is drawn; whether it meets the port is checked on its own.
                let distance = distance_to_node(*end, owner);
                (port.is_none() && distance > END_GAP).then(|| {
                    issue(
                        "edge-end-detached",
                        format!(
                            "Edge \"{}\" {verb} {}px away from node \"{}\".",
                            edge.id,
                            format_number(distance),
                            owner.id
                        ),
                        &[&edge.id, &owner.id],
                        Some(json!({ "end": which, "distance": json_number(distance) })),
                    )
                })
            })
        })
        .collect()
}

/// How far an end may lie from its port's point and still meet it.
pub const PORT_TOLERANCE: f64 = 0.5;

/// Each end of an edge, as (its point, the node it meets, the port it
/// names, its verb, which end), for the ends that name a port.
fn port_ends<'a>(
    nodes: &'a [DiagramNode],
    edge: &'a DiagramEdge,
) -> impl Iterator<Item = (Point, &'a DiagramNode, &'a str, &'static str, &'static str)> {
    let node = |id: &str| nodes.iter().find(|node| node.id == id);
    [
        (
            edge.points.first(),
            node(&edge.from),
            edge.from_port.as_deref(),
            "starts",
            "start",
        ),
        (
            edge.points.last(),
            node(&edge.to),
            edge.to_port.as_deref(),
            "ends",
            "end",
        ),
    ]
    .into_iter()
    .filter_map(|(point, owner, port, verb, which)| Some((*point?, owner?, port?, verb, which)))
}

/// Ends naming a port their node does not declare, and ends naming one of
/// its ports that meet none of its points.
/// Children out of their parent, or nearer its outline than its padding.
fn children_outside_parents(nodes: &[DiagramNode]) -> Vec<DiagramIssue> {
    nodes
        .iter()
        .filter_map(|child| {
            let parent = nodes.iter().find(|node| Some(&node.id) == child.parent_id.as_ref())?;
            let (inner, outer) = (&child.bounds, &parent.bounds);
            let room = [
                inner.x - outer.x,
                inner.y - outer.y,
                outer.right() - inner.right(),
                outer.bottom() - inner.bottom(),
            ];
            let padding = parent.child_padding();
            room.iter().any(|room| *room < padding).then(|| {
                issue(
                    "node-overflows-parent",
                    format!(
                        "Node \"{}\" does not fit inside its parent \"{}\" with {}px padding.",
                        child.id,
                        parent.id,
                        format_number(padding)
                    ),
                    &[&child.id, &parent.id],
                    None,
                )
            })
        })
        .collect()
}

/// Children lying on a text of their parent, such as its title.
fn children_on_parent_texts(nodes: &[DiagramNode]) -> Vec<DiagramIssue> {
    nodes
        .iter()
        .filter_map(|child| {
            let parent = nodes.iter().find(|node| Some(&node.id) == child.parent_id.as_ref())?;
            parent
                .label_bounds
                .iter()
                .any(|text| intersection(text, &child.bounds).is_some())
                .then(|| {
                    issue(
                        "node-overlaps-parent-label",
                        format!("Node \"{}\" lies on a label of its parent \"{}\".", child.id, parent.id),
                        &[&child.id, &parent.id],
                        None,
                    )
                })
        })
        .collect()
}

/// Ids more than one node has, once each: an edge naming one meets only
/// the first of them.
fn duplicate_node_ids(nodes: &[DiagramNode]) -> Vec<DiagramIssue> {
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for node in nodes {
        *counts.entry(node.id.as_str()).or_default() += 1;
    }
    let mut reported = HashSet::new();
    nodes
        .iter()
        .map(|node| (node.id.as_str(), counts[node.id.as_str()]))
        .filter(|(id, count)| *count > 1 && reported.insert(*id))
        .map(|(id, count)| {
            issue(
                "node-id-duplicated",
                format!("{count} nodes share the id \"{id}\"."),
                &[id],
                Some(json!({ "count": count })),
            )
        })
        .collect()
}

/// Edge ends naming a node the diagram does not have, such as a
/// misspelt id: nothing checks or reroutes them.
fn edge_unknown_nodes(nodes: &[DiagramNode], edges: &[DiagramEdge]) -> Vec<DiagramIssue> {
    edges
        .iter()
        .flat_map(|edge| {
            [(&edge.from, "starts", "start"), (&edge.to, "ends", "end")]
                .into_iter()
                .filter(|(id, _, _)| !id.is_empty() && !nodes.iter().any(|node| node.id == **id))
                .map(move |(id, verb, which)| {
                    issue(
                        "unknown-node",
                        format!(
                            "Edge \"{}\" {verb} at node \"{id}\", which the diagram does not have.",
                            edge.id
                        ),
                        &[&edge.id],
                        Some(json!({ "end": which, "node": id })),
                    )
                })
        })
        .collect()
}
fn edge_ends_off_ports(nodes: &[DiagramNode], edges: &[DiagramEdge]) -> Vec<DiagramIssue> {
    edges
        .iter()
        .flat_map(|edge| {
            port_ends(nodes, edge).filter_map(move |(end, owner, port, verb, which)| {
                let places: Vec<Point> = owner
                    .ports
                    .iter()
                    .filter(|place| place.name == port)
                    .map(|place| place.point)
                    .collect();
                let details = Some(json!({ "end": which, "port": port }));
                let elements: [&str; 2] = [&edge.id, &owner.id];
                if places.is_empty() {
                    return Some(issue(
                        "unknown-port",
                        format!(
                            "Edge \"{}\" {verb} at port \"{port}\", which node \"{}\" does not have.",
                            edge.id, owner.id
                        ),
                        &elements,
                        details,
                    ));
                }
                places
                    .iter()
                    .all(|place| hypot(end.x - place.x, end.y - place.y) > PORT_TOLERANCE)
                    .then(|| {
                        issue(
                            "edge-end-detached-port",
                            format!(
                                "Edge \"{}\" {verb} off port \"{port}\" of node \"{}\".",
                                edge.id, owner.id
                            ),
                            &elements,
                            details,
                        )
                    })
            })
        })
        .collect()
}

/// Ends whose last segment lies on a side of the box they end on: the line
/// runs along the outline instead of meeting it, and its arrowhead points
/// along the side.
fn edge_ends_along_sides(nodes: &[DiagramNode], edges: &[DiagramEdge]) -> Vec<DiagramIssue> {
    let node = |id: &str| nodes.iter().find(|node| node.id == id && !node.is_circle());
    let along = |end: Point, inner: Point, b: &Bounds| {
        let overlap = |low: f64, high: f64, a: f64, c: f64| high.min(a.max(c)) - low.max(a.min(c));
        (end.x == inner.x && (end.x == b.x || end.x == b.right()) && overlap(b.y, b.bottom(), end.y, inner.y) > 0.5)
            || (end.y == inner.y
                && (end.y == b.y || end.y == b.bottom())
                && overlap(b.x, b.right(), end.x, inner.x) > 0.5)
    };
    edges
        .iter()
        .filter(|edge| edge.points.len() >= 2)
        .flat_map(|edge| {
            let points = &edge.points;
            let ends = [
                (points[0], points[1], node(&edge.from), "starts", "start"),
                (
                    points[points.len() - 1],
                    points[points.len() - 2],
                    node(&edge.to),
                    "ends",
                    "end",
                ),
            ];
            ends.into_iter().filter_map(move |(end, inner, owner, verb, which)| {
                let owner = owner?;
                along(end, inner, &owner.bounds).then(|| {
                    issue(
                        "edge-overlaps-node-outline",
                        format!(
                            "Edge \"{}\" {verb} running along a side of node \"{}\".",
                            edge.id, owner.id
                        ),
                        &[&edge.id, &owner.id],
                        Some(json!({ "end": which })),
                    )
                })
            })
        })
        .collect()
}

/// An edge's ends as (node, port, point, whether it is the last point).
fn named_ends(edge: &DiagramEdge) -> [(&str, Option<&str>, Option<&Point>, bool); 2] {
    [
        (
            edge.from.as_str(),
            edge.from_port.as_deref(),
            edge.points.first(),
            false,
        ),
        (edge.to.as_str(), edge.to_port.as_deref(), edge.points.last(), true),
    ]
}

/// The routes of two edges with the run they share into a port they
/// both end at cut off, so that merging there does not read as running
/// along each other; both routes as they are when they share no port end.
fn before_merging<'a>(first: &'a DiagramEdge, second: &'a DiagramEdge) -> (Cow<'a, [Point]>, Cow<'a, [Point]>) {
    let shared = named_ends(first).into_iter().find_map(|(node, port, point, last)| {
        let (port, point) = (port?, point?);
        named_ends(second)
            .into_iter()
            .find_map(|(other_node, other_port, other_point, other_last)| {
                let other_point = other_point?;
                (other_node == node
                    && other_port == Some(port)
                    && hypot(point.x - other_point.x, point.y - other_point.y) <= PORT_TOLERANCE)
                    .then_some((last, other_last))
            })
    });
    let Some((a_last, b_last)) = shared else {
        return (Cow::Borrowed(&first.points), Cow::Borrowed(&second.points));
    };
    let (mut a, mut b) = (first.points.clone(), second.points.clone());
    // Both run into the port at their last point while the run is cut.
    if !a_last {
        a.reverse();
    }
    if !b_last {
        b.reverse();
    }
    while a.len() >= 2 && b.len() >= 2 {
        let end = a[a.len() - 1];
        let (next_a, next_b) = (a[a.len() - 2], b[b.len() - 2]);
        let (da, db) = (
            (next_a.x - end.x, next_a.y - end.y),
            (next_b.x - end.x, next_b.y - end.y),
        );
        let (la, lb) = (hypot(da.0, da.1), hypot(db.0, db.1));
        let together =
            la > 0.0 && lb > 0.0 && (da.0 * db.1 - da.1 * db.0).abs() < 1e-6 && da.0 * db.0 + da.1 * db.1 > 0.0;
        if !together {
            break;
        }
        let run = la.min(lb);
        let cut = Point {
            x: end.x + da.0 / la * run,
            y: end.y + da.1 / la * run,
        };
        for (route, length) in [(&mut a, la), (&mut b, lb)] {
            route.pop();
            if length > run {
                route.push(cut);
            }
        }
    }
    if !a_last {
        a.reverse();
    }
    if !b_last {
        b.reverse();
    }
    (Cow::Owned(a), Cow::Owned(b))
}

fn edge_overlaps(edges: &[DiagramEdge]) -> Vec<DiagramIssue> {
    edges
        .iter()
        .enumerate()
        .flat_map(|(index, first)| {
            edges[index + 1..]
                .iter()
                .filter(move |second| {
                    let (a, b) = before_merging(first, second);
                    routes_overlap(&a, &b)
                })
                .map(move |second| {
                    issue(
                        "edge-overlaps",
                        format!("Edges \"{}\" and \"{}\" overlap along a segment.", first.id, second.id),
                        &[&first.id, &second.id],
                        None,
                    )
                })
        })
        .collect()
}

/// How close an edge may bend to another before the two read as one
/// line bending.
pub const NEAR_MISS_DISTANCE: f64 = 5.0;

/// How many bends of `route` lie within the near-miss distance of the
/// segments of another route.
pub(crate) fn bends_beside(route: &[Point], other: &[(Point, Point)]) -> usize {
    route
        .get(1..route.len().saturating_sub(1))
        .unwrap_or_default()
        .iter()
        .filter(|corner| {
            other
                .iter()
                .any(|(a, b)| distance_to_segment(**corner, *a, *b) < NEAR_MISS_DISTANCE)
        })
        .count()
}

fn edge_near_misses(edges: &[DiagramEdge]) -> Vec<DiagramIssue> {
    edges
        .iter()
        .enumerate()
        .flat_map(|(index, first)| {
            edges[index + 1..]
                .iter()
                .filter(move |second| {
                    let (a, b) = before_merging(first, second);
                    bends_beside(&a, &segments(&b)) + bends_beside(&b, &segments(&a)) > 0
                })
                .map(move |second| {
                    issue(
                        "edge-bend-near-edge",
                        format!(
                            "Edges \"{}\" and \"{}\" come within {}px of each other where one bends.",
                            first.id,
                            second.id,
                            format_number(NEAR_MISS_DISTANCE)
                        ),
                        &[&first.id, &second.id],
                        None,
                    )
                })
        })
        .collect()
}

fn detached_labels(labels: &[DiagramLabel], edges: &[DiagramEdge]) -> Vec<DiagramIssue> {
    labels
        .iter()
        .filter_map(|label| {
            let edge = edges.iter().find(|item| Some(&item.id) == label.edge.as_ref())?;
            let distance = distance_to_route(&label.bounds, &edge.points);
            (distance > DETACHED_LABEL_DISTANCE).then(|| {
                issue(
                    "edge-label-detached",
                    format!(
                        "Label \"{}\" sits {}px from its edge \"{}\".",
                        label.id,
                        format_number(round(distance)),
                        edge.id
                    ),
                    &[&label.id, &edge.id],
                    Some(json!({
                        "distance": json_number(distance),
                        "maximumDistance": json_number(DETACHED_LABEL_DISTANCE),
                    })),
                )
            })
        })
        .collect()
}
