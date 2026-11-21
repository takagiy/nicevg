//! Deterministic repairs as a pipeline of pure functions over a draft document:
//! expand nodes around their labels, separate crowded nodes, reroute
//! edges and place their labels, then grow the viewBox.

use std::cell::OnceCell;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, OnceLock};

use serde::Serialize;

use crate::arrow::{drawn_edges, moved_arrowhead, shaft_route};
use crate::diagram::{
    Diagram, DiagramEdge, DiagramLabel, DiagramNode, ReadNodes, ancestor_ids, child_element_paths, circle_bounds,
    holds_end, is_node_shape, node_group, node_labels, node_shape, non_empty, number_attribute, read_nodes,
    rect_bounds, translation,
};
use crate::geometry::{
    Bounds, Point, distance_to_route, enclosing, format_number, intersection, point, route_is_clear, segments,
};
use crate::inspect::{
    EDGE_LABEL_CLEARANCE, NODE_GAP, TEXT_PADDING, VIEWPORT_PADDING, parse_view_box, radius_enclosing,
};
use crate::route::{
    CROSSING_COST, END_STUBS, LABEL_CLEAR_OF_OTHERS, LANE_SPACING, LabelSpot, PARALLEL_RUN, Placement, PortRequest,
    Ports, Pushable, ROUTE_CLEARANCE, Side, Sides, bends, choose_sides, crossings, edge_ports, end_on_shapes,
    is_on_outline, label_landmarks, overlap, place_label, route_cost, route_edge, routes_cost, runs_across, side_of,
    spread_ports,
};
use crate::text::{label_text_index, move_label, text_bounds};
use crate::xml::{Document, Element, Path, serialize};
use crate::{InspectionReport, SvgInputError, parse, report_around};

#[derive(Clone, Debug, PartialEq, Serialize, serde::Deserialize)]
pub struct FixChange {
    pub code: String,
    pub message: String,
    pub elements: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct FixResult {
    pub svg: String,
    pub changes: Vec<FixChange>,
    pub report: InspectionReport,
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
    /// Nodes just moved, whose edges are rerouted to follow.
    moved_node_ids: Vec<String>,
    /// The report of `document`, read once; a changed draft starts without.
    report: OnceLock<Arc<InspectionReport>>,
    /// The nodes of `document`, read once; a draft changed away from every
    /// node starts with those of the draft before it.
    nodes: OnceLock<Arc<ReadNodes>>,
    /// The nodes of the draft this one was made from by moving groups, with
    /// those groups: the nodes beside them are read from there.
    nodes_before: Option<(Arc<ReadNodes>, Vec<Path>)>,
}

impl Draft {
    pub(crate) fn new(document: Document, changes: Changes, moved_node_ids: Vec<String>) -> Draft {
        Draft {
            document,
            changes,
            moved_node_ids,
            report: OnceLock::new(),
            nodes: OnceLock::new(),
            nodes_before: None,
        }
    }

    /// A draft of this one's document with the groups at `groups` moved, as
    /// `document`: the nodes beside those groups are read as this draft's.
    pub(crate) fn with_groups_moved(
        &self,
        document: Document,
        changes: Changes,
        moved_node_ids: Vec<String>,
        groups: Vec<Path>,
    ) -> Draft {
        Draft {
            nodes_before: self.nodes.get().map(|read| (Arc::clone(read), groups)),
            ..Draft::new(document, changes, moved_node_ids)
        }
    }

    #[tracing::instrument(name = "draft_report", skip_all)]
    pub(crate) fn report(&self) -> &Arc<InspectionReport> {
        self.report.get_or_init(|| {
            let read = self.nodes.get_or_init(|| {
                let before = self
                    .nodes_before
                    .as_ref()
                    .map(|(read, groups)| (read.as_ref(), groups.as_slice()));
                Arc::new(read_nodes(&self.document, before))
            });
            Arc::new(report_around(&self.document, read.nodes.clone()))
        })
    }

    /// A draft changed by replacing the elements at `edited`, none of them
    /// a group or by one: where they stand beside every node, the nodes
    /// read for this draft are those of the changed one too.
    fn changed_beside_nodes(
        self,
        document: Document,
        code: &str,
        message: String,
        elements: &[&str],
        edited: &[&[usize]],
    ) -> Draft {
        let kept = self.nodes.get().filter(|read| read.stand_beside(edited)).cloned();
        let draft = self.changed(document, code, message, elements);
        if let Some(kept) = kept {
            draft.nodes.set(kept).ok();
        }
        draft
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
            report: InspectionReport::clone(self.report()),
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
/// another edge than a label keeps clear of them: touching one costs
/// what crossing it does.
const CROWDED_PUSH_COST: f64 = CROSSING_COST / LABEL_CLEAR_OF_OTHERS;

/// Repairs what can be repaired, repeating fix steps while they leave fewer
/// issues (at most five).
#[tracing::instrument(skip_all)]
pub fn fix(svg: &str) -> Result<FixResult, SvgInputError> {
    fix_with_steps(svg, FIX_STEPS)
}

/// The fix steps `fix` gives a diagram, and layout each layout.
pub(crate) const FIX_STEPS: usize = 5;

/// Later fix steps see earlier results: edges routed early avoid labels
/// placed late, and labels left without a slot get another try. A fix step is
/// kept only while it reduces the issues.
pub fn fix_with_steps(svg: &str, max_steps: usize) -> Result<FixResult, SvgInputError> {
    let draft = Draft::new(parse(svg)?, Changes::default(), Vec::new());
    Ok(fix_after_moves(draft, max_steps).written())
}

/// Fixes a draft, rerouting the edges of the nodes it just moved to
/// follow them.
#[tracing::instrument(skip_all, fields(moved = draft.moved_node_ids.len()))]
pub(crate) fn fix_after_moves(draft: Draft, max_steps: usize) -> Draft {
    fix_further(fix_step(draft), max_steps.saturating_sub(1))
}

/// Runs up to `steps` more fix steps over a fixed draft, keeping each only
/// while it leaves fewer issues.
pub(crate) fn fix_further(best: Draft, steps: usize) -> Draft {
    if steps == 0 || best.report().issues.is_empty() {
        return best;
    }
    let next = fix_step(best.moving(Vec::new()));
    if next.report().issues.len() >= best.report().issues.len() {
        return best;
    }
    fix_further(next, steps - 1)
}

#[tracing::instrument(skip_all)]
fn fix_step(draft: Draft) -> Draft {
    let landmarks = edge_landmarks(&draft.report().diagram);
    expand_view_box(settle_labels(
        reroute_edges(separate_nodes(grow_parents(clear_parent_texts(expand_nodes(draft))))),
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
        .filter(|issue| issue.code == "node-label-overflows")
        .filter_map(|issue| issue.elements.first().cloned())
        .fold(draft, |draft, node_id| {
            let document = &draft.document;
            let Some(group) = node_group(document, &node_id) else {
                return draft;
            };
            let shape = node_shape(document, &group);
            let labels = node_labels(document, &group);
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

/// Grows each parent a child sticks out of, or comes nearer the outline of
/// than the parent's padding, on the sides it does, one child at a time: a
/// parent grown round a child then grows its own parent in bend.
#[tracing::instrument(skip_all)]
fn grow_parents(draft: Draft) -> Draft {
    fn step(draft: Draft, attempt: usize) -> Draft {
        if attempt >= 100 {
            return draft;
        }
        let report = draft.report().clone();
        let grown = report
            .issues
            .iter()
            .filter(|issue| issue.code == "node-overflows-parent")
            .find_map(|issue| {
                let node = |index: usize| issue.elements.get(index).and_then(|id| report.diagram.node(id));
                let (parent, child) = (node(1)?, node(0)?);
                let (updated, message) = grow_round(&draft.document, parent, child)?;
                Some((updated, message, [parent.id.clone(), child.id.clone()]))
            });
        match grown {
            Some((updated, message, [parent, child])) => step(
                draft.changed(updated, "grow-parent", message, &[&parent, &child]),
                attempt + 1,
            ),
            None => draft,
        }
    }
    step(draft, 0)
}

/// The document with `parent`'s rect grown just enough to hold `child` with
/// the parent's padding, and what changed; none for a parent nicevg cannot
/// grow: a custom one, or one drawn as a circle.
fn grow_round(document: &Document, parent: &DiagramNode, child: &DiagramNode) -> Option<(Document, String)> {
    if !parent.grows() {
        return None;
    }
    let group = node_group(document, &parent.id)?;
    // A picture's frame is the rect it marks; a node's own, its first shape.
    let (shape, element) = document
        .descendants(&group)
        .into_iter()
        .find(|(_, element)| is_node_shape(element) && (!parent.custom || element.has_attr("data-bounds")))?;
    if element.name != "rect" {
        return None;
    }
    let current = rect_bounds(document, &shape);
    let grown = parent.grown_round(&child.bounds);
    if grown == current {
        return None;
    }
    let offset = point(
        current.x - number_attribute(element, "x"),
        current.y - number_attribute(element, "y"),
    );
    let updated = document.update(&shape, |rect| {
        rect.with_attr("x", &format_number(grown.x - offset.x))
            .with_attr("y", &format_number(grown.y - offset.y))
            .with_attr("width", &format_number(grown.width))
            .with_attr("height", &format_number(grown.height))
    });
    let message = format!(
        "Grew node \"{}\" from {} to {} to hold \"{}\".",
        parent.id,
        current.format(),
        grown.format(),
        child.id
    );
    Some((updated, message))
}

/// Pushes the right node of each overlapping or too-close pair rightwards,
/// one pair at a time, until the row keeps the gaps its issues require:
/// 20px, or room for the label of an edge between the two.
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
            .find(|issue| issue.code == "node-overlaps" || issue.code == "node-near-node")
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
        // An overlap reports no gap; the gap it then leaves, if too small, is reported next.
        let required = issue
            .details
            .as_ref()
            .and_then(|details| details.get("requiredGap")?.as_f64())
            .unwrap_or(NODE_GAP);
        let delta = left.bounds.right() + required - right.bounds.x;
        if delta <= 0.0 {
            return draft;
        }
        let message = format!(
            "Moved node \"{}\" {}px right to preserve a {}px gap.",
            right.id,
            format_number(delta),
            format_number(required)
        );
        let id = right.id.clone();
        match moved(&draft, &id, point(delta, 0.0), message) {
            Some(moved) => step(moved, attempt + 1),
            None => draft,
        }
    }
    step(draft, 0)
}

/// Moves each child lying on a label of its parent the nearer way off it,
/// down or right, to the parent's padding from the text: the parent grows
/// round it after, as round any child. Never up or left, which would grow
/// the parent past its top left, where its texts start from: a style draws
/// them again from the new corner, onto the child again.
#[tracing::instrument(skip_all)]
fn clear_parent_texts(draft: Draft) -> Draft {
    fn step(draft: Draft, attempt: usize) -> Draft {
        if attempt >= 100 {
            return draft;
        }
        let report = draft.report().clone();
        let Some((id, by)) = report
            .issues
            .iter()
            .filter(|issue| issue.code == "node-overlaps-parent-label")
            .find_map(|issue| {
                let node = |index: usize| issue.elements.get(index).and_then(|id| report.diagram.node(id));
                let (child, parent) = (node(0)?, node(1)?);
                let padding = parent.child_padding();
                let lain_on: Vec<Bounds> = parent
                    .label_bounds
                    .iter()
                    .filter(|text| intersection(text, &child.bounds).is_some())
                    .copied()
                    .collect();
                let (text, at) = (enclosing(&lain_on)?, child.bounds);
                let ways = [
                    point(0.0, text.bottom() + padding - at.y),
                    point(text.right() + padding - at.x, 0.0),
                ];
                let nearest = ways
                    .into_iter()
                    .min_by(|a, b| (a.x.abs() + a.y.abs()).total_cmp(&(b.x.abs() + b.y.abs())))?;
                Some((child.id.clone(), nearest))
            })
        else {
            return draft;
        };
        let message = format!("Moved node \"{id}\" off a label of its parent.");
        match moved(&draft, &id, by, message) {
            Some(moved) => step(moved, attempt + 1),
            None => draft,
        }
    }
    step(draft, 0)
}

/// The draft with node `id` translated `by`, marked moved so that its
/// edges follow it; none when the node has no group to translate.
fn moved(draft: &Draft, id: &str, by: Point, message: String) -> Option<Draft> {
    let group = node_group(&draft.document, id)?;
    let existing = draft.document.element(&group).attr("transform").trim().to_owned();
    let transform = format!(
        "{existing}{}translate({} {})",
        if existing.is_empty() { "" } else { " " },
        format_number(by.x),
        format_number(by.y)
    );
    let updated = draft.document.update(&group, |g| g.with_attr("transform", &transform));
    let moved = draft.clone().changed(updated, "move-node", message, &[id]);
    Some(Draft {
        moved_node_ids: [moved.moved_node_ids.clone(), vec![id.to_owned()]].concat(),
        ..moved
    })
}

fn side_key(bounds: &Bounds, side: crate::route::Side) -> String {
    format!("{}:{}", bounds.format(), side.name())
}

/// Where the edges that stay meet each node side, along the side: on
/// a box, the side the end lies on; on a circle, the side the edge
/// arrives at along its last segment.
fn settled_ports(
    diagram: &Diagram,
    nodes_by_id: &HashMap<&str, &DiagramNode>,
    rerouted_ids: &HashSet<&str>,
) -> HashMap<String, Vec<f64>> {
    diagram
        .edges
        .iter()
        .filter(|edge| !rerouted_ids.contains(edge.id.as_str()) && edge.points.len() >= 2)
        .flat_map(|edge| {
            let points = &edge.points;
            [
                (&edge.from, points[0], points[1]),
                (&edge.to, points[points.len() - 1], points[points.len() - 2]),
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

/// The node side an edge meets at its `end`, coming from `inner`, and
/// the port there on the node's box: on a box, the side the end lies on;
/// on a circle, the side the edge arrives at along its last segment.
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

/// The node and side each end of an edge meets with these ports.
fn meets<'a>(reroute: &Reroute<'a>, ports: &Ports) -> [(&'a str, Side); 2] {
    [
        (reroute.source.id.as_str(), side_of(ports.start, &reroute.source.bounds)),
        (reroute.target.id.as_str(), side_of(ports.end, &reroute.target.bounds)),
    ]
}

struct Reroute<'a> {
    edge: &'a DiagramEdge,
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
            places(self.source, self.edge.from_port.as_deref()),
            places(self.target, self.edge.to_port.as_deref()),
        ]
    }

    /// An edge to reroute between its nodes, each end pinned to the
    /// place of its port nearest where it ends now.
    fn new(edge: &'a DiagramEdge, source: &'a DiagramNode, target: &'a DiagramNode) -> Self {
        let mut reroute = Reroute {
            edge,
            source,
            target,
            pins: [None, None],
        };
        let ends = [edge.points.first(), edge.points.last()];
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
/// edge or cover a node, and the edges to reroute. Those are the
/// ones with an issue rerouting fixes, those whose drifted label finds no
/// slot beside them as they run, and those following a moved node.
fn pick_reroutes<'a>(
    report: &'a InspectionReport,
    moved_node_ids: &[String],
    nodes_by_id: &HashMap<&str, &'a DiagramNode>,
) -> (HashSet<&'a str>, Vec<Reroute<'a>>) {
    let diagram = &report.diagram;
    // A tied label that drifted from its edge or covers a node moves
    // beside its edge on its own when there is room there; otherwise
    // the edge is rerouted to give it a slot.
    let detached: Vec<(&str, &str)> = report
        .issues
        .iter()
        .filter_map(|issue| match issue.code.as_str() {
            "edge-label-detached" => Some((issue.elements.first()?.as_str(), issue.elements.get(1)?.as_str())),
            "label-overlaps-node" => {
                let label = diagram
                    .labels
                    .iter()
                    .find(|label| Some(&label.id) == issue.elements.first())?;
                Some((label.id.as_str(), label.edge.as_deref()?))
            }
            _ => None,
        })
        .collect();
    let detached_ids: HashSet<&str> = detached.iter().map(|(label, _)| *label).collect();
    let all_routes: Vec<Vec<Point>> = diagram.edges.iter().map(|c| c.points.clone()).collect();
    let stuck_edges: Vec<&str> = detached
        .iter()
        .filter(|(label_id, edge_id)| {
            let (Some(route), Some(label)) = (
                diagram.edge(edge_id),
                diagram.labels.iter().find(|label| label.id == *label_id),
            ) else {
                return false;
            };
            let blocked: Vec<Bounds> = diagram
                .labels
                .iter()
                .filter(|label| !detached_ids.contains(label.id.as_str()))
                .map(|label| label.bounds)
                .chain(label_blockers(diagram, edge_id))
                .collect();
            place_label(
                label.bounds.width,
                label.bounds.height,
                &route.points,
                &all_routes,
                &blocked,
                &label_parents(diagram, edge_id),
                LabelSpot::Within,
            )
            .is_none()
        })
        .map(|(_, edge_id)| *edge_id)
        .collect();

    let reroute_codes = [
        "edge-overlaps-node",
        "edge-near-label",
        "edge-end-detached",
        "edge-slanted",
        "edge-end-detached-port",
        "edge-overlaps-node-outline",
        "edge-overlaps",
        "edge-bend-near-edge",
    ];
    let with_issues: HashSet<&str> = stuck_edges
        .into_iter()
        .chain(
            report
                .issues
                .iter()
                .filter(|issue| reroute_codes.contains(&issue.code.as_str()))
                .flat_map(|issue| {
                    let take = if issue.code == "edge-overlaps" || issue.code == "edge-bend-near-edge" {
                        issue.elements.len()
                    } else {
                        1
                    };
                    issue.elements.iter().take(take).map(String::as_str)
                }),
        )
        .collect();
    // A node moves with any parent around it.
    let moved = |id: &str| {
        std::iter::once(id)
            .chain(ancestor_ids(&diagram.nodes, id))
            .any(|id| moved_node_ids.iter().any(|moved| moved == id))
    };
    let reroutes: Vec<Reroute> = diagram
        .edges
        .iter()
        .filter(|edge| with_issues.contains(edge.id.as_str()) || moved(&edge.from) || moved(&edge.to))
        .filter_map(|edge| {
            Some(Reroute::new(
                edge,
                nodes_by_id.get(edge.from.as_str())?,
                nodes_by_id.get(edge.to.as_str())?,
            ))
        })
        .collect();
    (detached_ids, reroutes)
}

/// A draft with routes written in, and every edge's route by id.
struct Routing {
    draft: Draft,
    routes: Vec<(String, Vec<Point>)>,
}

/// Writes each rerouted edge's new route into the document, moving its
/// arrowheads along, and gives every edge's route afterwards.
fn write_routes(draft: &Draft, diagram: &Diagram, reroutes: &[Reroute], final_routes: Vec<Vec<Point>>) -> Routing {
    let initial = Routing {
        draft: draft.clone(),
        routes: diagram.edges.iter().map(|c| (c.id.clone(), c.points.clone())).collect(),
    };
    // Rewriting one edge leaves where the others are drawn as it was.
    let all_drawn = drawn_edges(&draft.document);
    reroutes
        .iter()
        .zip(final_routes)
        .fold(initial, |state, (reroute, route)| {
            let edge = reroute.edge;
            let routes = with_route(state.routes, &edge.id, route.clone());
            let document = &state.draft.document;
            // Its own shaft by id, else the one joining the same nodes.
            let Some(drawn) = all_drawn.iter().find(|drawn| drawn.id == edge.id).or_else(|| {
                all_drawn
                    .iter()
                    .find(|drawn| drawn.from == edge.from && drawn.to == edge.to)
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
                .arrowheads
                .iter()
                .fold(
                    document.replace(shaft, replacement),
                    |document, arrowhead| match moved_arrowhead(&document, arrowhead, &route) {
                        Some(polygon) => document.replace(&arrowhead.path, polygon),
                        None => document,
                    },
                );
            let message = format!("Rerouted edge \"{}\" around diagram obstacles.", edge.id);
            let edited: Vec<&[usize]> = std::iter::once(shaft.as_slice())
                .chain(drawn.arrowheads.iter().map(|arrowhead| arrowhead.path.as_slice()))
                .collect();
            Routing {
                draft: state
                    .draft
                    .changed_beside_nodes(updated, "route-edge", message, &[&edge.id], &edited),
                routes,
            }
        })
}

/// Reroutes edges that cross nodes, overlap, end inside nodes, pass too
/// close to labels or follow moved nodes; then places the labels of rerouted
/// edges and of labels that drifted from their edge.
#[tracing::instrument(skip_all, fields(reroutes = tracing::field::Empty, partners = tracing::field::Empty))]
fn reroute_edges(draft: Draft) -> Draft {
    // A pushed label is sure of a spot only among the routes as they were;
    // one that finds none among the new routes stands as a wall instead.
    let (rerouted, stuck) = reroute_around(draft.clone(), &HashSet::new());
    if stuck.is_empty() {
        rerouted
    } else {
        reroute_around(draft, &stuck).0
    }
}

/// Reroutes as `reroute_edges` does, with the labels named in `walls`
/// standing in the way instead of moving aside; also gives the labels it
/// pushed aside but found no spot for.
fn reroute_around(draft: Draft, walls: &HashSet<String>) -> (Draft, HashSet<String>) {
    let report = draft.report().clone();
    let diagram = &report.diagram;

    let nodes_by_id: HashMap<&str, &DiagramNode> = diagram.nodes.iter().map(|node| (node.id.as_str(), node)).collect();
    let (detached_ids, reroutes) = pick_reroutes(&report, &draft.moved_node_ids, &nodes_by_id);
    let rerouted_ids: HashSet<&str> = reroutes.iter().map(|reroute| reroute.edge.id.as_str()).collect();
    let settled: Vec<Vec<Point>> = diagram
        .edges
        .iter()
        .filter(|edge| !rerouted_ids.contains(edge.id.as_str()))
        .map(|edge| edge.points.clone())
        .collect();
    // The outlines of the parents around an edge's ends: it crosses
    // them to reach its ends but should not run along them.
    let rails_for = |edge: &DiagramEdge| -> Vec<(Point, Point)> {
        diagram
            .nodes
            .iter()
            .filter(|node| node.id != edge.from && node.id != edge.to)
            .filter(|node| holds_end(&diagram.nodes, edge, node))
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
    // An edge's own label moves beside its new route, and so may the
    // labels of the edges that stay: a route can push one aside to
    // another spot beside its edge, for free when that spot is clear
    // of every other edge and for what crowding them there costs when
    // it is not.
    let all_routes: Vec<Vec<Point>> = diagram.edges.iter().map(|c| c.points.clone()).collect();
    let push_cost = |label: &DiagramLabel| -> Option<f64> {
        let edge = diagram.edge(label.edge.as_deref()?)?;
        let blocked: Vec<Bounds> = diagram
            .labels
            .iter()
            .filter(|other| other.id != label.id)
            .map(|other| other.bounds)
            .chain(label_blockers(diagram, &edge.id))
            .chain([label.bounds.inflate(EDGE_LABEL_CLEARANCE)])
            .collect();
        let parents = label_parents(diagram, &edge.id);
        let (width, height) = (label.bounds.width, label.bounds.height);
        let place = |worst| place_label(width, height, &edge.points, &all_routes, &blocked, &parents, worst);
        if place(LabelSpot::Clear).is_some() {
            return Some(0.0);
        }
        let spot = place(LabelSpot::Within)?;
        let crowding: f64 = diagram
            .edges
            .iter()
            .filter(|other| other.id != edge.id)
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
                && label.edge.as_deref().is_some_and(|id| !rerouted_ids.contains(id))
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
    let obstacles_for = |edge: &DiagramEdge| -> Vec<Bounds> {
        diagram
            .nodes
            .iter()
            .filter(|node| !holds_end(&diagram.nodes, edge, node))
            .map(|node| node.bounds)
            .chain(
                diagram
                    .labels
                    .iter()
                    .filter(|label| {
                        !detached_ids.contains(label.id.as_str()) && (label.edge.is_none() || walls.contains(&label.id))
                    })
                    .map(|label| label.bounds),
            )
            .collect()
    };
    let pushable_for = |edge: &DiagramEdge| -> Vec<Pushable<'_>> {
        pushable_labels
            .iter()
            .zip(&pricing)
            .filter(|(label, _)| label.edge.as_deref() != Some(edge.id.as_str()))
            .map(|(label, cost)| (label.bounds, cost.as_ref()))
            .collect()
    };

    // Side loads: ends of the edges that stay, plus the sides
    // chosen for rerouted ones. Sides are chosen twice so every edge
    // sees where the others went, not only the ones before it.
    let settled_load: HashMap<String, f64> = diagram
        .edges
        .iter()
        .filter(|edge| !rerouted_ids.contains(edge.id.as_str()))
        .flat_map(|edge| {
            [(&edge.from, edge.points.first()), (&edge.to, edge.points.last())]
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
    // Each choice comes with the loads it saw on the sides of its ends.
    type Chosen<'a> = HashMap<&'a str, (Sides, [f64; 8])>;
    let choose_all = |previous: &Chosen| -> Chosen {
        let load = reroutes.iter().fold(settled_load.clone(), |mut load, reroute| {
            if let Some((chosen, _)) = previous.get(reroute.edge.id.as_str()) {
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
                let before = previous.get(reroute.edge.id.as_str());
                let own = before.map(|(sides, _)| sides);
                let load_of = |bounds: &Bounds, side: crate::route::Side| {
                    let key = side_key(bounds, side);
                    let mine = own.map_or(0.0, |own| {
                        f64::from(u8::from(key == side_key(&reroute.source.bounds, own.start)))
                            + f64::from(u8::from(key == side_key(&reroute.target.bounds, own.end)))
                    });
                    load.get(&key).copied().unwrap_or(0.0) - mine
                };
                let mut loads = [0.0; 8];
                for (index, side) in Side::ALL.iter().enumerate() {
                    loads[index] = load_of(&reroute.source.bounds, *side);
                    loads[index + 4] = load_of(&reroute.target.bounds, *side);
                }
                // Loads only grow, and each side's terminal costs more by
                // what its load grew: sides whose loads grew least of their
                // node's still make the cheapest route.
                if let Some((sides, seen)) = before {
                    let grown: Vec<f64> = loads.iter().zip(seen).map(|(now, then)| now - then).collect();
                    let least =
                        |range: std::ops::Range<usize>| grown[range].iter().copied().fold(f64::INFINITY, f64::min);
                    let index = |side: Side| Side::ALL.iter().position(|s| *s == side).expect("one of all sides");
                    if grown[index(sides.start)] <= least(0..4) && grown[4 + index(sides.end)] <= least(4..8) {
                        return (reroute.edge.id.as_str(), (*sides, loads));
                    }
                }
                let sides = choose_sides(
                    &reroute.source.bounds,
                    &reroute.target.bounds,
                    &obstacles_for(reroute.edge),
                    &pushable_for(reroute.edge),
                    &settled,
                    &rails_for(reroute.edge),
                    &load_of,
                );
                (reroute.edge.id.as_str(), (sides, loads))
            })
            .collect()
    };
    let route_with = |reroute: &Reroute, ports: Ports, occupied: &[Vec<Point>]| {
        // Routes into the same port as this one merge into it: their last
        // segment there is no route to keep clear of.
        let pinned: Vec<Point> = reroute.pins.iter().flatten().map(|(at, _)| *at).collect();
        let merging: Vec<Vec<Point>>;
        let occupied = if pinned.is_empty() {
            occupied
        } else {
            merging = occupied
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
            &merging
        };
        end_on_shapes(
            route_edge(
                &reroute.source.bounds,
                &reroute.target.bounds,
                &obstacles_for(reroute.edge),
                &pushable_for(reroute.edge),
                ports,
                occupied,
                &rails_for(reroute.edge),
            ),
            reroute.source,
            reroute.target,
        )
    };
    // An end naming a port takes its side from the port, whatever side is
    // chosen for it: when every end does, no chosen side is read, and none
    // is searched for (proofs/NicevgProofs/Pinned.lean).
    let all_pinned = reroutes
        .iter()
        .all(|reroute| reroute.port_places().iter().all(|places| !places.is_empty()));
    let chosen: HashMap<&str, Sides> = if all_pinned {
        reroutes
            .iter()
            .map(|reroute| {
                let ports = edge_ports(&reroute.source.bounds, &reroute.target.bounds);
                let sides = Sides {
                    start: side_of(ports.start, &reroute.source.bounds),
                    end: side_of(ports.end, &reroute.target.bounds),
                };
                (reroute.edge.id.as_str(), sides)
            })
            .collect()
    } else {
        choose_all(&choose_all(&HashMap::new()))
            .into_iter()
            .map(|(id, (sides, _))| (id, sides))
            .collect()
    };
    // An end whose port has several places takes the one its route costs
    // least from, among the edges that stay, with each free end at the
    // middle of the side chosen for it.
    let reroutes: Vec<Reroute> = reroutes
        .iter()
        .map(|reroute| {
            let places = reroute.port_places();
            if places.iter().all(|places| places.len() < 2) {
                return Reroute { ..*reroute };
            }
            let sides = chosen.get(reroute.edge.id.as_str()).copied();
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
            let default = edge_ports(&reroute.source.bounds, &reroute.target.bounds);
            let starts = options(0);
            let ends = options(1);
            let ports_of = |pins: [Option<(Point, Side)>; 2]| Ports {
                start: pins[0].map_or_else(
                    || free_end(&reroute.source.bounds, sides.map(|sides| sides.start), default.start),
                    |(at, _)| at,
                ),
                end: pins[1].map_or_else(
                    || free_end(&reroute.target.bounds, sides.map(|sides| sides.end), default.end),
                    |(at, _)| at,
                ),
            };
            let cost = |ports: Ports| {
                let route = route_with(reroute, ports, &settled);
                if !orthogonal(&route) {
                    return f64::INFINITY;
                }
                route_cost(&route) + settled.iter().map(|other| routes_cost(&route, other)).sum::<f64>()
            };
            // A route costs at least its length, and is no shorter than its
            // ports lie apart: places that far apart or more than the best
            // route so far costs are not routed, as only a cheaper route
            // would be taken (proofs/NicevgProofs/PrunedMin.lean). On a
            // circle the route ends off the port, so every place is routed.
            let floored = !reroute.source.is_circle() && !reroute.target.is_circle();
            // The nearest places are routed first, so the best cost falls
            // early; of routes that cost the same, the place listed first
            // still wins (proofs/NicevgProofs/AnyOrder.lean).
            type Pins = [Option<(Point, Side)>; 2];
            let mut options: Vec<(usize, Pins, Ports, f64)> = starts
                .iter()
                .flat_map(|start| ends.iter().map(move |end| [*start, *end]))
                .enumerate()
                .map(|(listed, pins)| {
                    let ports = ports_of(pins);
                    let apart = (ports.start.x - ports.end.x).abs() + (ports.start.y - ports.end.y).abs();
                    (listed, pins, ports, apart)
                })
                .collect();
            options.sort_by(|a, b| a.3.total_cmp(&b.3));
            let mut best: Option<(f64, usize, Pins)> = None;
            for (listed, pins, ports, apart) in options {
                // (The margin covers rounding in the sums.)
                if floored && best.is_some_and(|(least, ..)| apart >= least + 1e-6) {
                    continue;
                }
                let cost = cost(ports);
                if best.is_none_or(|(least, first, _)| cost.total_cmp(&least).then(listed.cmp(&first)).is_lt()) {
                    best = Some((cost, listed, pins));
                }
            }
            let pins = best.expect("a place for each end").2;
            Reroute { pins, ..*reroute }
        })
        .collect();
    // An end naming a port meets the side its port lies on.
    let chosen: HashMap<&str, Sides> = chosen
        .into_iter()
        .map(|(id, sides)| {
            let reroute = reroutes
                .iter()
                .find(|reroute| reroute.edge.id == id)
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
                    id: reroute.edge.id.clone(),
                    source_id: reroute.source.id.clone(),
                    target_id: reroute.target.id.clone(),
                    source: reroute.source.bounds,
                    target: reroute.target.bounds,
                    source_round: reroute.source.shape.is_some(),
                    target_round: reroute.target.shape.is_some(),
                    sides: *chosen.get(reroute.edge.id.as_str())?,
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
                .get(&reroute.edge.id)
                .copied()
                .unwrap_or_else(|| edge_ports(&reroute.source.bounds, &reroute.target.bounds));
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
    // An edge that stays but crosses another, rerouted or not, where
    // both meet the same node side may trade ports with it, and is rerouted
    // too if so.
    let staying_ends: Vec<(Reroute, Ports)> = diagram
        .edges
        .iter()
        .filter(|edge| !rerouted_ids.contains(edge.id.as_str()) && edge.points.len() >= 2)
        .filter_map(|edge| {
            let source = nodes_by_id.get(edge.from.as_str())?;
            let target = nodes_by_id.get(edge.to.as_str())?;
            let points = &edge.points;
            let (_, start) = end_port(points[0], points[1], &source.bounds);
            let (_, end) = end_port(points[points.len() - 1], points[points.len() - 2], &target.bounds);
            Some((Reroute::new(edge, source, target), Ports { start, end }))
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
                .map(|(staying, ports)| (staying, ports, &staying.edge.points)),
        )
        .collect();
    let partner_ids: HashSet<&str> = staying_ends
        .iter()
        .filter(|(partner, partner_ports)| {
            let own = meets(partner, partner_ports);
            all_ends.iter().any(|(other, ports, route)| {
                other.edge.id != partner.edge.id
                    && meets(other, ports).iter().any(|end| own.contains(end))
                    && crossings(route, &partner.edge.points) > 0
            })
        })
        .map(|(partner, _)| partner.edge.id.as_str())
        .collect();
    let partners: Vec<(Reroute, Ports)> = staying_ends
        .into_iter()
        .filter(|(partner, _)| partner_ids.contains(partner.edge.id.as_str()))
        .collect();
    tracing::Span::current()
        .record("reroutes", reroutes.len())
        .record("partners", partners.len());
    let staying: Vec<Vec<Point>> = diagram
        .edges
        .iter()
        .filter(|edge| !rerouted_ids.contains(edge.id.as_str()) && !partner_ids.contains(edge.id.as_str()))
        .map(|edge| edge.points.clone())
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
                .map(|(partner, _)| partner.edge.points.clone())
                .collect(),
        ]
        .concat(),
        &route_with,
    );
    // Partners whose ports stayed keep their route as drawn.
    let (reroutes, final_routes): (Vec<Reroute>, Vec<Vec<Point>>) = reroutes
        .into_iter()
        .zip(improved)
        .filter(|(reroute, route)| !partner_ids.contains(reroute.edge.id.as_str()) || *route != reroute.edge.points)
        .unzip();

    // Labels a new route runs over move out of its way.
    let pushed: HashSet<&str> = pushable_labels
        .iter()
        .filter(|label| {
            final_routes
                .iter()
                .any(|route| distance_to_route(&label.bounds, route) < EDGE_LABEL_CLEARANCE)
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
                || label.edge.as_deref().is_some_and(|id| rerouted_ids.contains(id))
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
            let edge_id = label.edge.clone().unwrap_or_default();
            let route = routed
                .routes
                .iter()
                .find(|(id, _)| *id == edge_id)
                .map(|(_, route)| route);
            let texts = label_texts.get(&label.id).map_or(&[][..], Vec::as_slice);
            let (Some(route), false) = (route, texts.is_empty()) else {
                return (draft, obstacles, stuck);
            };
            let blocked = [obstacles.clone(), label_blockers(diagram, &edge_id)].concat();
            let parents = label_parents(diagram, &edge_id);
            let (width, height) = (label.bounds.width, label.bounds.height);
            // A label pushed aside with nowhere inside or outside its parents
            // is stuck and its edge rerouted; its own edge's label
            // takes a spot across a parent's outline rather than none.
            let worst = if pushed.contains(label.id.as_str()) {
                LabelSpot::Within
            } else {
                LabelSpot::Across
            };
            let place = |blocked: &[Bounds]| place_label(width, height, route, &all_final, blocked, &parents, worst);
            let Some(placement) = off_parent_labels(diagram, &edge_id, &blocked, place) else {
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
            let message = format!("Moved label \"{}\" beside edge \"{edge_id}\".", label.id);
            let edited: Vec<&[usize]> = texts.iter().map(Vec::as_slice).collect();
            let draft = draft.changed_beside_nodes(updated, "move-label", message, &[&label.id, &edge_id], &edited);
            (draft, [obstacles, vec![placement.bounds]].concat(), stuck)
        },
    );
    (placed, stuck)
}

/// Once the layout is settled, moves a middle segment of an edge that
/// runs closer than two lanes beside another away from it, onto the line
/// where running alongside costs nothing, wherever the routes then cost
/// less and stay clear of the nodes and labels around them; then places
/// the labels of the moved edges again. A result that would be left
/// with more issues stays as it was.
#[tracing::instrument(skip_all)]
pub(crate) fn spread_parallel_runs(result: Draft) -> Draft {
    let draft = result.moving(Vec::new());
    let report = draft.report().clone();
    let diagram = &report.diagram;
    let obstacles_for = |edge: &DiagramEdge| -> Vec<Bounds> {
        diagram
            .nodes
            .iter()
            .filter(|node| !holds_end(&diagram.nodes, edge, node))
            .map(|node| node.bounds)
            .chain(
                diagram
                    .labels
                    .iter()
                    .filter(|label| label.edge.as_deref() != Some(edge.id.as_str()))
                    .map(|label| label.bounds),
            )
            .map(|bounds| bounds.inflate(ROUTE_CLEARANCE))
            .collect()
    };
    // What each edge keeps clear of: the nodes and labels around it,
    // and its own end nodes, which only its middle segments keep clear of.
    let obstacles: Vec<[Vec<Bounds>; 2]> = diagram
        .edges
        .iter()
        .map(|edge| {
            let ends = [&edge.from, &edge.to]
                .into_iter()
                .filter_map(|id| diagram.node(id))
                .map(|node| node.bounds.inflate(ROUTE_CLEARANCE))
                .collect();
            [obstacles_for(edge), ends]
        })
        .collect();
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
    let mut routes: Vec<Vec<Point>> = diagram.edges.iter().map(|c| c.points.clone()).collect();
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
                        if gap == 0.0 || gap >= PARALLEL_RUN.reach || overlap((a, b), (c, d)) <= 0.0 {
                            return None;
                        }
                        let away = (at - other).signum();
                        let mut trial = routes.clone();
                        let pushed = push(
                            &mut trial,
                            &obstacles,
                            index,
                            k,
                            other + away * PARALLEL_RUN.reach,
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
            let edge = &diagram.edges[*index];
            let reroute = Reroute::new(edge, diagram.node(&edge.from)?, diagram.node(&edge.to)?);
            Some((reroute, routes[*index].clone()))
        })
        .unzip();
    let routed = write_routes(&draft, diagram, &reroutes, finals);
    let draft = expand_view_box(settle_labels(routed.draft, &edge_landmarks(diagram)));
    if draft.report().issues.len() > result.report().issues.len() {
        result
    } else {
        draft
    }
}

/// Moves middle segment `k` of route `index` onto `line`, pushing on by two
/// lanes, in the same direction `away`, any other route's middle segment
/// beyond it that it would run within two lanes of. Fails when a route would
/// bend back on itself, run into the clearance of a node or label, run its
/// moved segment into the clearance of its own end nodes, or have to push
/// a route's first or last segment.
fn push(
    routes: &mut [Vec<Point>],
    obstacles: &[[Vec<Bounds>; 2]],
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
    if !keeps_headings
        || !route_is_clear(&moved[k - 1..=k + 2], &obstacles[index][0])
        || !route_is_clear(&moved[k..=k + 1], &obstacles[index][1])
    {
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
            if (at - line) * away < 0.0 || (at - line).abs() >= PARALLEL_RUN.reach || overlap(segment, (*c, *d)) <= 0.0
            {
                continue;
            }
            if m == 0
                || m + 1 == others.len()
                || !push(
                    routes,
                    obstacles,
                    other,
                    m,
                    line + away * PARALLEL_RUN.reach,
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

/// The crossings, bends and ends of every edge, by edge id.
fn edge_landmarks(diagram: &Diagram) -> HashMap<String, Vec<Point>> {
    let routes: Vec<Vec<Point>> = diagram.edges.iter().map(|c| c.points.clone()).collect();
    diagram
        .edges
        .iter()
        .map(|edge| (edge.id.clone(), label_landmarks(&edge.points, &routes)))
        .collect()
}

/// Places edge labels again where they may no longer be the best
/// spot: beside an edge whose crossings, bends or ends have changed
/// since `before`, wherever the label came from, and within a label
/// clearance of another edge, where it could be taken for that
/// one's label, when a spot clear of the others exists.
#[tracing::instrument(skip_all)]
fn settle_labels(draft: Draft, before: &HashMap<String, Vec<Point>>) -> Draft {
    let report = draft.report().clone();
    let diagram = &report.diagram;
    let routes: Vec<Vec<Point>> = diagram.edges.iter().map(|c| c.points.clone()).collect();
    let now = edge_landmarks(diagram);
    let unsettled: Vec<_> = diagram
        .labels
        .iter()
        .filter_map(|label| {
            let edge = diagram.edge(label.edge.as_deref()?)?;
            let stale = before
                .get(&edge.id)
                .is_some_and(|before| Some(before) != now.get(&edge.id));
            let beside = segments(&edge.points).into_iter().min_by(|a, b| {
                distance_to_route(&label.bounds, &[a.0, a.1]).total_cmp(&distance_to_route(&label.bounds, &[b.0, b.1]))
            })?;
            let nearest = diagram
                .edges
                .iter()
                .filter(|other| other.id != edge.id)
                .flat_map(|other| segments(&other.points))
                .filter(|other| !runs_across(beside, &label.bounds, *other))
                .map(|(c, d)| distance_to_route(&label.bounds, &[c, d]))
                .fold(f64::INFINITY, f64::min);
            (stale || nearest < LABEL_CLEAR_OF_OTHERS).then_some((label, edge, stale))
        })
        .collect();
    let others: Vec<Bounds> = diagram.labels.iter().map(|label| label.bounds).collect();
    // Moving a label rewrites positions only, so every text keeps its path.
    let label_texts = label_text_index(&draft.document);
    let (settled, _) = unsettled
        .into_iter()
        .fold((draft, others), |(draft, obstacles), (label, edge, stale)| {
            let texts = label_texts.get(&label.id).map_or(&[][..], Vec::as_slice);
            if texts.is_empty() {
                return (draft, obstacles);
            }
            let blocked: Vec<Bounds> = obstacles
                .iter()
                .filter(|bounds| **bounds != label.bounds)
                .copied()
                .chain(label_blockers(diagram, &edge.id))
                .collect();
            let parents = label_parents(diagram, &edge.id);
            let worst = if stale { LabelSpot::Within } else { LabelSpot::Clear };
            let (width, height) = (label.bounds.width, label.bounds.height);
            let place =
                |blocked: &[Bounds]| place_label(width, height, &edge.points, &routes, blocked, &parents, worst);
            let Some(placement) = off_parent_labels(diagram, &edge.id, &blocked, place)
                .filter(|placement| placement.bounds != label.bounds)
            else {
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
                    "Moved label \"{}\" away from where \"{}\" now crosses, bends or ends.",
                    label.id, edge.id
                )
            } else {
                format!(
                    "Moved label \"{}\" clear of edges other than \"{}\".",
                    label.id, edge.id
                )
            };
            let edited: Vec<&[usize]> = texts.iter().map(Vec::as_slice).collect();
            let draft = draft.changed_beside_nodes(updated, "move-label", message, &[&label.id, &edge.id], &edited);
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

/// Node boxes an edge's label keeps clear of: every node except the
/// parents around the edge's ends, inside which the label may sit.
fn label_blockers(diagram: &Diagram, edge_id: &str) -> Vec<Bounds> {
    let parents = end_parent_ids(diagram, edge_id);
    diagram
        .nodes
        .iter()
        .filter(|node| !parents.contains(&node.id.as_str()))
        .map(|node| node.bounds)
        .collect()
}

/// The spot `place` finds clear of `blocked` for the label of an edge: off
/// the labels of the parents around the edge's ends, such as their titles,
/// when there is one, and on them rather than nowhere.
fn off_parent_labels(
    diagram: &Diagram,
    edge_id: &str,
    blocked: &[Bounds],
    place: impl Fn(&[Bounds]) -> Option<Placement>,
) -> Option<Placement> {
    let parents = end_parent_ids(diagram, edge_id);
    let labels: Vec<Bounds> = diagram
        .nodes
        .iter()
        .filter(|node| parents.contains(&node.id.as_str()))
        .flat_map(|node| node.label_bounds.iter().copied())
        .collect();
    (!labels.is_empty())
        .then(|| place(&[blocked, &labels].concat()))
        .flatten()
        .or_else(|| place(blocked))
}

/// The parents around an edge's ends, which its label may sit in
/// but not across.
fn label_parents(diagram: &Diagram, edge_id: &str) -> Vec<Bounds> {
    let parents = end_parent_ids(diagram, edge_id);
    diagram
        .nodes
        .iter()
        .filter(|node| parents.contains(&node.id.as_str()))
        .map(|node| node.bounds)
        .collect()
}

fn end_parent_ids<'a>(diagram: &'a Diagram, edge_id: &str) -> Vec<&'a str> {
    diagram
        .edge(edge_id)
        .map(|edge| {
            [
                ancestor_ids(&diagram.nodes, &edge.from),
                ancestor_ids(&diagram.nodes, &edge.to),
            ]
            .concat()
        })
        .unwrap_or_default()
}

/// Routes an edge with the given ports around the routes occupying the
/// diagram.
type RouteWith<'a> = dyn Fn(&Reroute, Ports, &[Vec<Point>]) -> Vec<Point> + 'a;

/// Whether a route runs only across and down: one that found no way is
/// drawn straight across, slanted, and bends no corner.
fn orthogonal(route: &[Point]) -> bool {
    segments(route).iter().all(|(a, b)| a.x == b.x || a.y == b.y)
}

/// Port changes that spreading ports evenly cannot see, tried once routes
/// are drawn: two edges on the same node side whose routes cross
/// trade their ports there (an edge running straight takes its other
/// end along), and an end that steps aside just before its node slides
/// along the side to the line it arrives on. A change stays when the
/// edges it reroutes stay orthogonal and cross less without bending
/// more, or bend less without crossing more.
#[tracing::instrument(skip_all)]
fn improve_ports(
    reroutes: &[Reroute],
    settled: &[Vec<Point>],
    fixed: &HashMap<String, Vec<f64>>,
    ports: Vec<Ports>,
    routes: Vec<Vec<Point>>,
    route_with: &RouteWith,
) -> Vec<Vec<Point>> {
    // Reroutes the edges at `changed` with their new ports, keeping the
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
            (crossed, set.iter().map(|route| bends(route)).sum::<usize>())
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
        if new.0 <= old.0 && new.1 <= old.1 && new != old && rerouted.iter().all(|route| orthogonal(route)) {
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
    // The node at one end of an edge (0 for its start, 1 for its end)
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
    // The segment of a side a port may move to: its middle half, as a port
    // near a corner looks pushed aside, and one near a circle's tangent
    // would meet it at a glancing angle.
    let usable = |index: usize, end: usize, side: Side| {
        let node = node_at(index, end);
        let (low, length) = if matches!(side, Side::Left | Side::Right) {
            (node.bounds.y, node.bounds.height)
        } else {
            (node.bounds.x, node.bounds.width)
        };
        // At most half the side, so a side under 16px leaves its middle.
        let margin = (length / 4.0).max(8.0).min(length / 2.0);
        (low + margin, low + length - margin)
    };
    // Whether an end can take the coordinate `value` along `side`: within
    // its usable segment and a lane from every other port there.
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
    // An edge running straight between facing sides takes its other
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
        // Where the line before the step lies off the side's usable segment,
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
    // An end that bends onto a short last segment just before its node
    // moves to the side the route was heading into, where the route may run
    // straight in.
    let (_, bent) = ends.iter().fold(slid, |(ports, routes), &(index, end)| {
        if pinned(index, end) {
            return (ports, routes);
        }
        let Some((side, line)) = bent_end(&toward(&routes, index, end)) else {
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
    bent
}

/// Where a route bends onto a short last segment just before its last
/// point: the side of the node it would enter by running on along the
/// segment before the bend, and that segment's coordinate across it.
fn bent_end(route: &[Point]) -> Option<(Side, f64)> {
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
/// segments bend one way and back, so the end could lie on the line of the
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

/// Replaces or appends an edge's route, keeping the original order.
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

/// The path that replaces a rerouted edge, keeping its other
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

/// Grows the viewBox to the diagram plus its safe padding when it clips.
#[tracing::instrument(skip_all)]
fn expand_view_box(draft: Draft) -> Draft {
    let report = draft.report().clone();
    let nodes = draft.nodes.get().cloned();
    let root_path = vec![draft.document.root_index()];
    let view_box = parse_view_box(draft.document.root().attr("viewBox"));
    let clipped = report.issues.iter().any(|issue| issue.code == "diagram-clipped");
    let (Some(view_box), Some(diagram), true) = (view_box, report.diagram_bounds, clipped) else {
        return draft;
    };
    let required = diagram.inflate(VIEWPORT_PADDING);
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
    let draft = scale_size(draft, &view_box, &expanded);
    // The viewBox, the backgrounds and the size are read only by the check
    // that the diagram fits the viewBox: the grown draft's report is the one
    // before it with that check made again (proofs/NicevgProofs/Viewport.lean).
    let grown = parse_view_box(draft.document.root().attr("viewBox"));
    draft.report.set(Arc::new(report.with_view_box(grown))).ok();
    if let Some(nodes) = nodes {
        draft.nodes.set(nodes).ok();
    }
    draft
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
/// the parent.
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
        "Scaled the SVG {} to keep the diagram's scale.",
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
    let value = crate::geometry::finite_number(number).filter(|value| *value > 0.0)?;
    absolute.contains(&unit).then_some((value, unit))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inspect;

    /// Given routes ending on a node: one running left and then bending up
    ///   onto a 23px last segment, one doing the same onto a 30px segment,
    ///   and one running up and then bending right onto a 20px segment
    /// When the bend just before each end is looked for
    /// Then the first would run straight into the node's right side at the
    ///   height it came along, the third into its bottom side at the line
    ///   it came along, and the second, bending well before its node, is
    ///   left as it is
    #[test]
    fn finds_a_route_bending_onto_a_short_last_segment() {
        let bending_up = |rise: f64| vec![point(400.0, 350.0), point(100.0, 350.0), point(100.0, 350.0 - rise)];
        let bending_right = vec![point(100.0, 400.0), point(100.0, 100.0), point(120.0, 100.0)];

        assert_eq!(bent_end(&bending_up(23.0)), Some((Side::Right, 350.0)));
        assert_eq!(bent_end(&bending_up(30.0)), None);
        assert_eq!(bent_end(&bending_right), Some((Side::Bottom, 100.0)));
    }

    /// Given two edges whose middle segments run down side by side 8px
    ///   apart, with room on either side
    /// When their close parallel runs are spread once the layout is settled
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
        let points = |result: &FixResult, id: &str| result.report.diagram.edge(id).unwrap().points.clone();
        let (before_p, before_q) = (points(&fixed, "p"), points(&fixed, "q"));
        assert_eq!(before_q[1].x - before_p[1].x, 8.0);

        let spread =
            spread_parallel_runs(Draft::new(parse(&fixed.svg).unwrap(), Changes::default(), Vec::new())).written();

        let (p, q) = (points(&spread, "p"), points(&spread, "q"));
        assert!((q[1].x - p[1].x).abs() >= 16.0, "{p:?} {q:?}");
        assert_eq!((bends(&p), bends(&q)), (2, 2));
        assert_eq!(
            (p[0], p[3], q[0], q[3]),
            (before_p[0], before_p[3], before_q[0], before_q[3])
        );
    }

    /// Given three edges whose middle segments run down side by side,
    ///   8px and then 10px apart, with room on either side
    /// When their close parallel runs are spread once the layout is settled
    /// Then every two of them run at least two lanes apart: moving one away
    ///   from its neighbour pushes the next one on
    #[test]
    fn spreads_a_bundle_of_parallel_runs_pushing_each_on() {
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
        let x = |result: &FixResult, id: &str| result.report.diagram.edge(id).unwrap().points[1].x;
        assert_eq!((x(&fixed, "p"), x(&fixed, "q"), x(&fixed, "r")), (210.0, 202.0, 192.0));

        let spread =
            spread_parallel_runs(Draft::new(parse(&fixed.svg).unwrap(), Changes::default(), Vec::new())).written();

        let (p, q, r) = (x(&spread, "p"), x(&spread, "q"), x(&spread, "r"));
        assert!(
            (p - q).abs() >= 16.0 && (q - r).abs() >= 16.0 && (p - r).abs() >= 16.0,
            "{p} {q} {r}"
        );
    }

    /// Given `posts` with an edge from its port "reply_to_id" round to
    ///   its port "id", both on its left, bending 8px out, and a straight
    ///   edge running down 12px further left
    /// When their close parallel runs are spread once the layout is settled
    /// Then the loop still bends at least 8px clear of `posts`: it is not
    ///   pushed against its own node to get away from the other edge
    #[test]
    fn keeps_a_run_pushed_away_from_a_neighbour_clear_of_its_own_node() {
        let fixed = fix(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 320 340">
              <g data-node="posts" data-custom="true"><g transform="translate(100 100)">
                <rect data-bounds="true" width="120" height="96"/>
                <circle data-port="id" cx="0" cy="24" r="0"/>
                <circle data-port="reply_to_id" cx="0" cy="72" r="0"/>
              </g></g>
              <g data-node="above"><rect x="54" y="0" width="52" height="40"/></g>
              <g data-node="below"><rect x="54" y="260" width="52" height="40"/></g>
              <path id="reply" data-from="posts" data-from-port="reply_to_id" data-to="posts" data-to-port="id" d="M 100 172 L 92 172 L 92 124 L 100 124"/>
              <path id="down" data-from="above" data-to="below" d="M 80 40 L 80 260"/>
            </svg>"#)
        .unwrap();
        let reply = |result: &FixResult| result.report.diagram.edge("reply").unwrap().points.clone();
        assert_eq!(reply(&fixed)[1].x, 92.0, "{:?}", reply(&fixed));

        let spread =
            spread_parallel_runs(Draft::new(parse(&fixed.svg).unwrap(), Changes::default(), Vec::new())).written();

        let bend = reply(&spread)[1].x;
        assert!(100.0 - bend >= 8.0, "{:?}", reply(&spread));
    }

    /// Given two boxes joined by a straight edge with no issue
    /// When what to reroute is picked, with no node moved and with one of
    ///   its ends moved
    /// Then the edge is left alone until its node moves
    #[test]
    fn picks_a_edge_to_reroute_only_once_its_node_moves() {
        let report = inspect(
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
