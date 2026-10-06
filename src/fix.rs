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
use crate::geometry::{Bounds, Point, distance_to_route, enclosing, format_number, point, segments};
use crate::inspect::{TEXT_PADDING, VIEWPORT_PADDING, parse_view_box, radius_enclosing};
use crate::route::{
    LABEL_CLEAR_OF_OTHERS, LANE_SPACING, PortRequest, Ports, Side, Sides, choose_sides, chords_cross, connector_ports,
    end_on_shapes, is_on_outline, label_landmarks, place_label, place_label_clear, route_connector, runs_across,
    side_of, spread_ports,
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
    let landmarks = connector_landmarks(&draft.report().diagram);
    let draft = expand_view_box(settle_labels(
        reroute_connectors(separate_nodes(expand_nodes(draft))),
        &landmarks,
    ));
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

    let route_with = |reroute: &Reroute, ports: Ports, occupied: &[Vec<Point>]| {
        end_on_shapes(
            route_connector(
                &reroute.source.bounds,
                &reroute.target.bounds,
                &obstacles_for(reroute.connector),
                ports,
                occupied,
                &rails_for(reroute.connector),
            ),
            reroute.source,
            reroute.target,
        )
    };
    let first_ports: Vec<Ports> = reroutes
        .iter()
        .map(|reroute| {
            ports
                .get(&reroute.connector.id)
                .copied()
                .unwrap_or_else(|| connector_ports(&reroute.source.bounds, &reroute.target.bounds))
        })
        .collect();
    let (_, first_routes) = reroutes.iter().zip(&first_ports).fold(
        (settled.clone(), Vec::new()),
        |(occupied, routes): (Vec<Vec<Point>>, Vec<Vec<Point>>), (reroute, ports)| {
            let route = route_with(reroute, *ports, &occupied);
            ([occupied, vec![route.clone()]].concat(), [routes, vec![route]].concat())
        },
    );
    // A connector that stays but crosses a rerouted one where both meet the
    // same node side may trade ports with it, and is rerouted too if so.
    let partners: Vec<(Reroute, Ports)> = diagram
        .connectors
        .iter()
        .filter(|connector| !rerouted_ids.contains(connector.id.as_str()) && connector.points.len() >= 2)
        .filter_map(|connector| {
            let source = nodes_by_id.get(connector.from.as_str())?;
            let target = nodes_by_id.get(connector.to.as_str())?;
            let points = &connector.points;
            let (_, start) = end_port(points[0], points[1], &source.bounds);
            let (_, end) = end_port(points[points.len() - 1], points[points.len() - 2], &target.bounds);
            Some((
                Reroute {
                    connector,
                    source,
                    target,
                },
                Ports { start, end },
            ))
        })
        .filter(|(partner, partner_ports)| {
            let own = meets(partner, partner_ports);
            reroutes
                .iter()
                .zip(&first_ports)
                .zip(&first_routes)
                .any(|((reroute, ports), route)| {
                    meets(reroute, ports).iter().any(|end| own.contains(end))
                        && route_crossings(route, std::slice::from_ref(&partner.connector.points)) > 0
                })
        })
        .collect();
    let partner_ids: HashSet<&str> = partners
        .iter()
        .map(|(partner, _)| partner.connector.id.as_str())
        .collect();
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

    struct Routing {
        draft: Draft,
        routes: Vec<(String, Vec<Point>)>,
    }
    let initial = Routing {
        draft: draft.clone(),
        routes: diagram
            .connectors
            .iter()
            .map(|c| (c.id.clone(), c.points.clone()))
            .collect(),
    };
    let routed =
        reroutes
            .iter()
            .zip(final_routes)
            .fold(initial, |state, (reroute, route)| {
                let connector = reroute.connector;
                let routes = with_route(state.routes, &connector.id, route.clone());
                let document = &state.draft.document;
                let Some(drawn) = drawn_connectors(document).into_iter().find(|drawn| {
                    drawn.id == connector.id || (drawn.from == connector.from && drawn.to == connector.to)
                }) else {
                    return Routing { routes, ..state };
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
                    .fold(document.replace(shaft, replacement), |document, head| match moved_head(
                        &document, head, &route,
                    ) {
                        Some(polygon) => document.replace(&head.path, polygon),
                        None => document,
                    });
                let message = format!("Rerouted connector \"{}\" around diagram obstacles.", connector.id);
                Routing {
                    draft: state
                        .draft
                        .changed(updated, "route-connector", message, &[&connector.id]),
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
fn settle_labels(draft: Draft, before: &HashMap<String, Vec<Point>>) -> Draft {
    let report = draft.report();
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
    let (settled, _) = unsettled
        .into_iter()
        .fold((draft, others), |(draft, obstacles), (label, connector, stale)| {
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
                &texts,
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
fn improve_ports(
    reroutes: &[Reroute],
    settled: &[Vec<Point>],
    fixed: &HashMap<String, Vec<f64>>,
    ports: Vec<Ports>,
    routes: Vec<Vec<Point>>,
    route_with: &RouteWith,
) -> Vec<Vec<Point>> {
    // Reroutes the connectors at `changed`, in order, with their new ports,
    // keeping the change when it is better.
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
        let rerouted = changed
            .iter()
            .fold(Vec::new(), |rerouted: Vec<Vec<Point>>, (index, new)| {
                let occupied = [others.clone(), rerouted.clone()].concat();
                let route = route_with(&reroutes[*index], *new, &occupied);
                [rerouted, vec![route]].concat()
            });
        let current: Vec<Vec<Point>> = changed.iter().map(|(index, _)| routes[*index].clone()).collect();
        let cost = |set: &[Vec<Point>]| {
            let crossed: usize = set
                .iter()
                .enumerate()
                .map(|(index, route)| route_crossings(route, &others) + route_crossings(route, &set[index + 1..]))
                .sum();
            (crossed, set.iter().map(|route| turns(route)).sum::<usize>())
        };
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
    let with_port = |ports: Ports, end: usize, p: Point| {
        if end == 0 {
            Ports { start: p, ..ports }
        } else {
            Ports { end: p, ..ports }
        }
    };
    // Whether an end can take the coordinate `value` along its side: away
    // from the side's corners and a lane from every other port there.
    let free = |ports: &[Ports], index: usize, end: usize, value: f64| {
        let node = node_at(index, end);
        let side = side_of(port_at(&ports[index], end), &node.bounds);
        let along_y = matches!(side, Side::Left | Side::Right);
        let (low, length) = if along_y {
            (node.bounds.y, node.bounds.height)
        } else {
            (node.bounds.x, node.bounds.width)
        };
        // A port near a circle's tangent would meet it at a glancing angle.
        let margin = if node.shape.is_some() { length / 4.0 } else { 8.0 };
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
        value >= low + margin
            && value <= low + length - margin
            && taken.all(|other| (other - value).abs() >= LANE_SPACING)
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
        if facing && along(far) == along(old) && free(ports, index, opposite, along(new)) {
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
            if !shared || route_crossings(&routes[first], std::slice::from_ref(&routes[second])) == 0 {
                return (ports, routes);
            }
            let changed = [
                (first, carried(&ports, first, a, q)),
                (second, carried(&ports, second, b, p)),
            ];
            attempt((ports, routes), &changed)
        });
    let ends: Vec<(usize, usize)> = (0..reroutes.len()).flat_map(|index| [(index, 0), (index, 1)]).collect();
    let (_, slid) = ends.into_iter().fold(traded, |(ports, routes), (index, end)| {
        let port = port_at(&ports[index], end);
        let side = side_of(port, &node_at(index, end).bounds);
        let route: Vec<Point> = if end == 0 {
            routes[index].iter().rev().copied().collect()
        } else {
            routes[index].clone()
        };
        let Some(line) = stepped_end(&route) else {
            return (ports, routes);
        };
        if !free(&ports, index, end, line) {
            return (ports, routes);
        }
        let changed = [(index, with_port(ports[index], end, moved_to(port, side, line)))];
        attempt((ports, routes), &changed)
    });
    slid
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

/// How many times a route crosses the others.
fn route_crossings(route: &[Point], others: &[Vec<Point>]) -> usize {
    let own = segments(route);
    others
        .iter()
        .flat_map(|other| segments(other))
        .map(|(c, d)| own.iter().filter(|(a, b)| chords_cross(*a, *b, c, d)).count())
        .sum()
}

/// How many times a route changes direction.
fn turns(route: &[Point]) -> usize {
    route
        .windows(3)
        .filter(|w| {
            let (a, b, c) = (w[0], w[1], w[2]);
            (b.x - a.x) * (c.y - b.y) != (b.y - a.y) * (c.x - b.x)
        })
        .count()
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
