//! Mechanical layout checks. Each check maps the diagram to the issues it
//! finds; `inspect` concatenates them in a fixed order.

use serde::Serialize;
use serde_json::{Map, Value, json};

use crate::diagram::{Diagram, DiagramConnector, DiagramLabel, DiagramNode, holds_end};
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
pub const VIEWPORT_PADDING: f64 = 20.0;
pub const NODE_GAP: f64 = 20.0;
pub const CLEARANCE: f64 = 8.0;
/// Labels are placed one clearance plus a pixel away from their connector,
/// so a tied label further than this no longer reads as belonging to it.
pub const DETACHED_LABEL_DISTANCE: f64 = 16.0;

pub fn inspect(view_box: Option<Bounds>, diagram: &Diagram) -> Vec<DiagramIssue> {
    [
        viewport(view_box, diagram.drawing_bounds()),
        text_fit(&diagram.nodes),
        node_overlaps(&diagram.nodes),
        node_gaps(&diagram.nodes),
        connector_crossings(&diagram.nodes, &diagram.connectors),
        label_overlaps(&diagram.labels),
        label_node_overlaps(&diagram.nodes, &diagram.labels),
        connector_label_clearance(&diagram.connectors, &diagram.labels),
        connector_endpoints(&diagram.nodes, &diagram.connectors),
        connector_ends_along_sides(&diagram.nodes, &diagram.connectors),
        connector_overlaps(&diagram.connectors),
        detached_labels(&diagram.labels, &diagram.connectors),
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

fn viewport(view_box: Option<Bounds>, drawing: Option<Bounds>) -> Vec<DiagramIssue> {
    let (Some(view_box), Some(drawing)) = (view_box, drawing) else {
        return Vec::new();
    };
    let required = drawing.inflate(VIEWPORT_PADDING);
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
        "viewport-clipping",
        format!("Drawing exceeds the safe viewBox on the {} side.", sides.join(", ")),
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
/// label keeps `padding` from its edge.
pub fn radius_enclosing(label: &Bounds, box_: &Bounds, padding: f64) -> f64 {
    let centre = box_.centre();
    let dx = (label.x - centre.x).abs().max((label.right() - centre.x).abs());
    let dy = (label.y - centre.y).abs().max((label.bottom() - centre.y).abs());
    hypot(dx, dy) + padding
}

fn text_fit(nodes: &[DiagramNode]) -> Vec<DiagramIssue> {
    nodes
        .iter()
        .filter_map(|node| {
            let label = enclosing(&node.label_bounds)?;
            let fits = if node.is_circle() {
                radius_enclosing(&label, &node.bounds, TEXT_PADDING) <= node.bounds.width / 2.0
            } else {
                fits_in_box(&label, &node.bounds, TEXT_PADDING)
            };
            (!fits).then(|| {
                issue(
                    "text-overflow",
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
                "node-overlap",
                format!("Nodes \"{}\" and \"{}\" overlap.", first.id, second.id),
                &[&first.id, &second.id],
                Some(json!({ "intersection": json_bounds(&overlap) })),
            ))
        })
        .collect()
}

fn node_gaps(nodes: &[DiagramNode]) -> Vec<DiagramIssue> {
    comparable_pairs(nodes)
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
            (0.0..NODE_GAP).contains(&gap).then(|| {
                issue(
                    "node-gap",
                    format!(
                        "Nodes \"{}\" and \"{}\" have a {}px gap; {}px is required.",
                        left.id,
                        right.id,
                        format_number(gap),
                        format_number(NODE_GAP)
                    ),
                    &[&left.id, &right.id],
                    Some(json!({
                        "actualGap": json_number(gap),
                        "requiredGap": json_number(NODE_GAP),
                        "shortage": json_number(NODE_GAP - gap),
                    })),
                )
            })
        })
        .collect()
}

/// Whether a segment passes through the open interior of a node's shape.
pub fn segment_enters_node(start: Point, end: Point, node: &DiagramNode) -> bool {
    if node.is_circle() {
        distance_to_segment(node.bounds.centre(), start, end) < node.bounds.width / 2.0 - 0.001
    } else {
        segment_intersects_interior(start, end, &node.bounds)
    }
}

fn connector_crossings(nodes: &[DiagramNode], connectors: &[DiagramConnector]) -> Vec<DiagramIssue> {
    connectors
        .iter()
        .flat_map(|connector| {
            nodes
                .iter()
                .filter(move |node| !holds_end(nodes, connector, node))
                .filter(move |node| {
                    segments(&connector.points)
                        .iter()
                        .any(|(from, to)| segment_enters_node(*from, *to, node))
                })
                .map(move |node| {
                    issue(
                        "connector-node-crossing",
                        format!("Connector \"{}\" crosses unrelated node \"{}\".", connector.id, node.id),
                        &[&connector.id, &node.id],
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
                        "label-overlap",
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

/// A container's free space is a valid place for labels; only its children
/// and its border are off limits.
fn lies_within_container(nodes: &[DiagramNode], label: &Bounds, node: &DiagramNode) -> bool {
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
                    label_covers_node(&label.bounds, node) && !lies_within_container(nodes, &label.bounds, node)
                })
                .map(move |node| {
                    issue(
                        "label-node-overlap",
                        format!("Label \"{}\" overlaps node \"{}\".", label.id, node.id),
                        &[&label.id, &node.id],
                        None,
                    )
                })
        })
        .collect()
}

fn connector_label_clearance(connectors: &[DiagramConnector], labels: &[DiagramLabel]) -> Vec<DiagramIssue> {
    connectors
        .iter()
        .flat_map(|connector| {
            labels
                .iter()
                .filter(move |label| {
                    let expanded = label.bounds.inflate(CLEARANCE);
                    segments(&connector.points)
                        .iter()
                        .any(|(from, to)| segment_intersects_interior(*from, *to, &expanded))
                })
                .map(move |label| {
                    issue(
                        "connector-label-clearance",
                        format!(
                            "Connector \"{}\" passes within {}px of label \"{}\".",
                            connector.id,
                            format_number(CLEARANCE),
                            label.id
                        ),
                        &[&connector.id, &label.id],
                        Some(json!({ "requiredClearance": json_number(CLEARANCE) })),
                    )
                })
        })
        .collect()
}

/// Endpoints on a circle are written with whole-unit coordinates, so a point
/// within half a unit of the circle counts as on it.
pub fn point_is_inside_node(target: Point, node: &DiagramNode) -> bool {
    if node.is_circle() {
        let centre = node.bounds.centre();
        hypot(target.x - centre.x, target.y - centre.y) < node.bounds.width / 2.0 - 0.5
    } else {
        point_is_inside(target, &node.bounds)
    }
}

fn connector_endpoints(nodes: &[DiagramNode], connectors: &[DiagramConnector]) -> Vec<DiagramIssue> {
    let node = |id: &str| nodes.iter().find(|node| node.id == id);
    connectors
        .iter()
        .flat_map(|connector| {
            let ends = [
                (connector.points.first(), node(&connector.from), "starts", "start"),
                (connector.points.last(), node(&connector.to), "ends", "end"),
            ];
            ends.into_iter().filter_map(move |(endpoint, owner, verb, which)| {
                let (endpoint, owner) = (endpoint?, owner?);
                point_is_inside_node(*endpoint, owner).then(|| {
                    issue(
                        "connector-endpoint-inside",
                        format!("Connector \"{}\" {verb} inside node \"{}\".", connector.id, owner.id),
                        &[&connector.id, &owner.id],
                        Some(json!({ "endpoint": which })),
                    )
                })
            })
        })
        .collect()
}

/// Ends whose last stretch lies on a side of the box they end on: the line
/// runs along the outline instead of meeting it, and its arrowhead points
/// along the side.
fn connector_ends_along_sides(nodes: &[DiagramNode], connectors: &[DiagramConnector]) -> Vec<DiagramIssue> {
    let node = |id: &str| nodes.iter().find(|node| node.id == id && !node.is_circle());
    let along = |end: Point, inner: Point, b: &Bounds| {
        let overlap = |low: f64, high: f64, a: f64, c: f64| high.min(a.max(c)) - low.max(a.min(c));
        (end.x == inner.x && (end.x == b.x || end.x == b.right()) && overlap(b.y, b.bottom(), end.y, inner.y) > 0.5)
            || (end.y == inner.y
                && (end.y == b.y || end.y == b.bottom())
                && overlap(b.x, b.right(), end.x, inner.x) > 0.5)
    };
    connectors
        .iter()
        .filter(|connector| connector.points.len() >= 2)
        .flat_map(|connector| {
            let points = &connector.points;
            let ends = [
                (points[0], points[1], node(&connector.from), "starts", "start"),
                (
                    points[points.len() - 1],
                    points[points.len() - 2],
                    node(&connector.to),
                    "ends",
                    "end",
                ),
            ];
            ends.into_iter().filter_map(move |(end, inner, owner, verb, which)| {
                let owner = owner?;
                along(end, inner, &owner.bounds).then(|| {
                    issue(
                        "connector-end-along-side",
                        format!(
                            "Connector \"{}\" {verb} running along a side of node \"{}\".",
                            connector.id, owner.id
                        ),
                        &[&connector.id, &owner.id],
                        Some(json!({ "endpoint": which })),
                    )
                })
            })
        })
        .collect()
}

fn connector_overlaps(connectors: &[DiagramConnector]) -> Vec<DiagramIssue> {
    connectors
        .iter()
        .enumerate()
        .flat_map(|(index, first)| {
            connectors[index + 1..]
                .iter()
                .filter(move |second| routes_overlap(&first.points, &second.points))
                .map(move |second| {
                    issue(
                        "connector-overlap",
                        format!(
                            "Connectors \"{}\" and \"{}\" overlap along a segment.",
                            first.id, second.id
                        ),
                        &[&first.id, &second.id],
                        None,
                    )
                })
        })
        .collect()
}

fn detached_labels(labels: &[DiagramLabel], connectors: &[DiagramConnector]) -> Vec<DiagramIssue> {
    labels
        .iter()
        .filter_map(|label| {
            let connector = connectors
                .iter()
                .find(|item| Some(&item.id) == label.connector.as_ref())?;
            let distance = distance_to_route(&label.bounds, &connector.points);
            (distance > DETACHED_LABEL_DISTANCE).then(|| {
                issue(
                    "label-detached",
                    format!(
                        "Label \"{}\" sits {}px from its connector \"{}\".",
                        label.id,
                        format_number(round(distance)),
                        connector.id
                    ),
                    &[&label.id, &connector.id],
                    Some(json!({
                        "distance": json_number(distance),
                        "maximumDistance": json_number(DETACHED_LABEL_DISTANCE),
                    })),
                )
            })
        })
        .collect()
}
