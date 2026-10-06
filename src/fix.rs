//! Deterministic repairs as a pipeline of pure steps over a draft document:
//! expand nodes around their labels, separate crowded nodes, reroute
//! connectors and place their labels, then grow the viewBox.

use std::collections::{HashMap, HashSet};

use serde::Serialize;

use crate::arrow::{drawn_connectors, moved_head, shaft_route};
use crate::diagram::{
    Diagram, DiagramConnector, DiagramNode, ancestor_ids, child_element_paths, circle_bounds, holds_end, is_node_shape,
    node_group, number_attribute, rect_bounds, translation,
};
use crate::geometry::{Bounds, Point, distance_to_route, enclosing, format_number, segments};
use crate::inspect::{TEXT_PADDING, VIEWPORT_PADDING, parse_view_box, radius_enclosing};
use crate::route::{
    LABEL_CLEAR_OF_OTHERS, PortRequest, Sides, choose_sides, connector_ports, end_on_shapes, is_on_outline,
    place_label, place_label_clear, route_connector, runs_across, side_of, spread_ports,
};
use crate::text::{label_texts, move_label, text_bounds};
use crate::xml::{Document, Element, serialize};
use crate::{AnalysisReport, SvgInputError, analyze, parse, report_of};

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

/// A document being repaired, with the changes made so far.
#[derive(Clone)]
struct Draft {
    document: Document,
    changes: Vec<FixChange>,
    moved_node_ids: Vec<String>,
}

impl Draft {
    /// The report of the document as it would be written out.
    fn report(&self) -> AnalysisReport {
        let written = serialize(&self.document);
        report_of(&parse(&written).expect("a serialized draft parses again"))
    }

    fn changed(self, document: Document, code: &str, message: String, elements: &[&str]) -> Draft {
        let change = FixChange {
            code: code.to_owned(),
            message,
            elements: elements.iter().map(|element| (*element).to_owned()).collect(),
        };
        Draft {
            document,
            changes: [self.changes, vec![change]].concat(),
            ..self
        }
    }
}

/// Repairs what can be repaired, repeating passes while they leave fewer
/// issues (at most five).
pub fn fix(svg: &str) -> Result<FixResult, SvgInputError> {
    fix_with_passes(svg, 5)
}

/// Later passes see earlier results: connectors routed early avoid labels
/// placed late, and labels left without a slot get another try. A pass is
/// kept only while it reduces the issues.
pub fn fix_with_passes(svg: &str, max_passes: usize) -> Result<FixResult, SvgInputError> {
    fix_after_moves(svg, &[], max_passes)
}

/// Fixes a document whose nodes `moved` were just moved, so the connectors
/// attached to them are rerouted to follow.
pub(crate) fn fix_after_moves(svg: &str, moved: &[String], max_passes: usize) -> Result<FixResult, SvgInputError> {
    fn refine(best: FixResult, remaining: usize) -> Result<FixResult, SvgInputError> {
        if remaining == 0 || best.report.issues.is_empty() {
            return Ok(best);
        }
        let next = fix_once(&best.svg, &[])?;
        if next.report.issues.len() >= best.report.issues.len() {
            return Ok(best);
        }
        let changes = [best.changes, next.changes.clone()].concat();
        refine(FixResult { changes, ..next }, remaining - 1)
    }
    refine(fix_once(svg, moved)?, max_passes.saturating_sub(1))
}

fn fix_once(svg: &str, moved: &[String]) -> Result<FixResult, SvgInputError> {
    let draft = Draft {
        document: parse(svg)?,
        changes: Vec::new(),
        moved_node_ids: moved.to_vec(),
    };
    let draft = expand_view_box(clarify_labels(reroute_connectors(separate_nodes(expand_nodes(draft)))));
    let written = serialize(&draft.document);
    Ok(FixResult {
        report: analyze(&written)?,
        svg: written,
        changes: draft.changes,
    })
}

/// Grows node shapes whose labels do not fit, leaving the labels in place.
fn expand_nodes(draft: Draft) -> Draft {
    let report = draft.report();
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
fn separate_nodes(draft: Draft) -> Draft {
    fn step(draft: Draft, attempt: usize) -> Draft {
        if attempt >= 100 {
            return draft;
        }
        let report = draft.report();
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
    use crate::route::Side;
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
            let side = if is_on_outline(end, &bounds) {
                side_of(end, &bounds)
            } else if (end.x - inner.x).abs() >= (end.y - inner.y).abs() {
                if inner.x > end.x { Side::Right } else { Side::Left }
            } else if inner.y > end.y {
                Side::Bottom
            } else {
                Side::Top
            };
            let along = if matches!(side, Side::Top | Side::Bottom) {
                end.x
            } else {
                end.y
            };
            Some((side_key(&bounds, side), along))
        })
        .fold(HashMap::new(), |mut ports: HashMap<String, Vec<f64>>, (key, along)| {
            ports.entry(key).or_default().push(along);
            ports
        })
}

struct Reroute<'a> {
    connector: &'a DiagramConnector,
    source: &'a DiagramNode,
    target: &'a DiagramNode,
}

/// Reroutes connectors that cross nodes, overlap, end inside nodes, pass too
/// close to labels or follow moved nodes; then places the labels of rerouted
/// connectors and of labels that drifted from their connector.
fn reroute_connectors(draft: Draft) -> Draft {
    let report = draft.report();
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
        "connector-end-along-side",
        "connector-overlap",
    ];
    let with_issues: HashSet<&str> = stuck_connectors
        .into_iter()
        .chain(
            report
                .issues
                .iter()
                .filter(|issue| reroute_codes.contains(&issue.code.as_str()))
                .flat_map(|issue| {
                    let take = if issue.code == "connector-overlap" {
                        issue.elements.len()
                    } else {
                        1
                    };
                    issue.elements.iter().take(take).map(String::as_str)
                }),
        )
        .collect();
    let nodes_by_id: HashMap<&str, &DiagramNode> = diagram.nodes.iter().map(|node| (node.id.as_str(), node)).collect();
    let reroutes: Vec<Reroute> = diagram
        .connectors
        .iter()
        .filter(|connector| {
            with_issues.contains(connector.id.as_str())
                || draft.moved_node_ids.contains(&connector.from)
                || draft.moved_node_ids.contains(&connector.to)
        })
        .filter_map(|connector| {
            Some(Reroute {
                connector,
                source: nodes_by_id.get(connector.from.as_str())?,
                target: nodes_by_id.get(connector.to.as_str())?,
            })
        })
        .collect();
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
                            && label.connector.as_deref().is_none_or(|id| !rerouted_ids.contains(id))
                    })
                    .map(|label| label.bounds),
            )
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
                    &settled,
                    &rails_for(reroute.connector),
                    &load_of,
                );
                (reroute.connector.id.as_str(), sides)
            })
            .collect()
    };
    let chosen = choose_all(&choose_all(&HashMap::new()));
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
                    sides: *chosen.get(reroute.connector.id.as_str())?,
                })
            })
            .collect::<Vec<_>>(),
        &fixed,
    );

    struct Routing {
        draft: Draft,
        settled: Vec<Vec<Point>>,
        routes: Vec<(String, Vec<Point>)>,
    }
    let initial = Routing {
        draft: draft.clone(),
        settled,
        routes: diagram
            .connectors
            .iter()
            .map(|c| (c.id.clone(), c.points.clone()))
            .collect(),
    };
    let routed = reroutes.iter().fold(initial, |state, reroute| {
        let connector = reroute.connector;
        let route = end_on_shapes(
            route_connector(
                &reroute.source.bounds,
                &reroute.target.bounds,
                &obstacles_for(connector),
                ports
                    .get(&connector.id)
                    .copied()
                    .unwrap_or_else(|| connector_ports(&reroute.source.bounds, &reroute.target.bounds)),
                &state.settled,
                &rails_for(connector),
            ),
            reroute.source,
            reroute.target,
        );
        let settled = [state.settled, vec![route.clone()]].concat();
        let routes = with_route(state.routes, &connector.id, route.clone());
        let document = &state.draft.document;
        let Some(drawn) = drawn_connectors(document)
            .into_iter()
            .find(|drawn| drawn.id == connector.id || (drawn.from == connector.from && drawn.to == connector.to))
        else {
            return Routing {
                settled,
                routes,
                ..state
            };
        };
        let shaft = &drawn.shaft;
        // Routes are in diagram coordinates; the shaft's own are inside its
        // ancestors' translations.
        let offset = translation(document, shaft);
        let local: Vec<Point> = shaft_route(&drawn, &route)
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
            settled,
            routes,
        }
    });

    let moving: Vec<_> = diagram
        .labels
        .iter()
        .enumerate()
        .filter(|(_, label)| {
            detached_ids.contains(label.id.as_str())
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
    let (placed, _) = moving
        .iter()
        .fold((routed.draft, still), |(draft, obstacles), (_, label)| {
            let connector_id = label.connector.clone().unwrap_or_default();
            let route = routed
                .routes
                .iter()
                .find(|(id, _)| *id == connector_id)
                .map(|(_, route)| route);
            let texts = label_texts(&draft.document, &label.id);
            let (Some(route), false) = (route, texts.is_empty()) else {
                return (draft, obstacles);
            };
            let blocked = [obstacles.clone(), label_blockers(diagram, &connector_id)].concat();
            let Some(placement) = place_label(
                label.bounds.width,
                label.bounds.height,
                route,
                &all_final,
                &blocked,
                &label_containers(diagram, &connector_id),
            ) else {
                return (draft, obstacles);
            };
            let updated = move_label(
                draft.document.clone(),
                &texts,
                label.bounds,
                placement.bounds,
                placement.anchor,
            );
            let message = format!("Moved label \"{}\" beside connector \"{connector_id}\".", label.id);
            let draft = draft.changed(updated, "move-label", message, &[&label.id, &connector_id]);
            (draft, [obstacles, vec![placement.bounds]].concat())
        });
    placed
}

/// Moves connector labels that sit within a label clearance of another
/// connector, where they could be taken for its label, to a spot beside
/// their own connector clear of the others, when there is one.
fn clarify_labels(draft: Draft) -> Draft {
    let report = draft.report();
    let diagram = &report.diagram;
    let routes: Vec<Vec<Point>> = diagram.connectors.iter().map(|c| c.points.clone()).collect();
    let unclear: Vec<_> = diagram
        .labels
        .iter()
        .filter_map(|label| {
            let connector = diagram.connector(label.connector.as_deref()?)?;
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
            (nearest < LABEL_CLEAR_OF_OTHERS).then_some((label, connector))
        })
        .collect();
    let others: Vec<Bounds> = diagram.labels.iter().map(|label| label.bounds).collect();
    let (clarified, _) = unclear
        .into_iter()
        .fold((draft, others), |(draft, obstacles), (label, connector)| {
            let texts = label_texts(&draft.document, &label.id);
            if texts.is_empty() {
                return (draft, obstacles);
            }
            let blocked: Vec<Bounds> = obstacles
                .iter()
                .filter(|bounds| **bounds != label.bounds)
                .copied()
                .chain(label_blockers(diagram, &connector.id))
                .collect();
            let Some(placement) = place_label_clear(
                label.bounds.width,
                label.bounds.height,
                &connector.points,
                &routes,
                &blocked,
                &label_containers(diagram, &connector.id),
            ) else {
                return (draft, obstacles);
            };
            let updated = move_label(
                draft.document.clone(),
                &texts,
                label.bounds,
                placement.bounds,
                placement.anchor,
            );
            let message = format!(
                "Moved label \"{}\" clear of connectors other than \"{}\".",
                label.id, connector.id
            );
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
    clarified
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
fn expand_view_box(draft: Draft) -> Draft {
    let report = draft.report();
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
    scale_size(
        draft.changed(updated, "expand-viewbox", message, &["svg"]),
        &view_box,
        &expanded,
    )
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
