//! Arranging: after fixing, nodes move a little, keeping the input's rough
//! layout, so connectors bend, cross and crowd less.

mod support;

use nicevg::{AnalysisReport, FixResult};
use support::*;

/// Every node of the arranged diagram stays on the same side of the nodes
/// beside, above and below it as after fixing, moves at most 60px on each
/// axis, comes no
/// closer than 20px to a sibling it was further from, no issue is added
/// and no more connectors run within 10px of each other.
fn assert_layout_kept(svg: &str, result: &FixResult) {
    let fixed: AnalysisReport = fix(svg).report;
    let nodes = &fixed.diagram.nodes;
    for node in nodes {
        let (before, after) = (node.bounds, node_bounds(&result.report, &node.id));
        assert!(
            (after.x - before.x).abs() <= 60.0 && (after.y - before.y).abs() <= 60.0,
            "{} moved too far",
            node.id
        );
        for other in nodes.iter().filter(|other| other.id != node.id) {
            let (other_before, other_after) = (other.bounds, node_bounds(&result.report, &other.id));
            let left = |a: nicevg::Bounds, b: nicevg::Bounds| a.x + a.width <= b.x + 0.5;
            let above = |a: nicevg::Bounds, b: nicevg::Bounds| a.y + a.height <= b.y + 0.5;
            // Order matters between nodes side by side or stacked: sharing
            // some height or width, or nearly, within 20px.
            let share = |low: f64, size: f64, other_low: f64, other_size: f64| {
                (low + size).min(other_low + other_size) - low.max(other_low) > -20.0
            };
            if share(before.y, before.height, other_before.y, other_before.height) {
                assert!(
                    !left(before, other_before) || left(after, other_after),
                    "{} left of {}",
                    node.id,
                    other.id
                );
            }
            if share(before.x, before.width, other_before.x, other_before.width) {
                assert!(
                    !above(before, other_before) || above(after, other_after),
                    "{} above {}",
                    node.id,
                    other.id
                );
            }
        }
    }
    for node in nodes {
        for other in nodes
            .iter()
            .filter(|other| other.id != node.id && other.parent_id == node.parent_id)
        {
            let (a, b) = (
                node_bounds(&result.report, &node.id),
                node_bounds(&result.report, &other.id),
            );
            let (c, d) = (node.bounds, other.bounds);
            let gap = |a: nicevg::Bounds, b: nicevg::Bounds| {
                (b.x - (a.x + a.width))
                    .max(a.x - (b.x + b.width))
                    .max(b.y - (a.y + a.height))
                    .max(a.y - (b.y + b.height))
            };
            assert!(
                gap(a, b) >= gap(c, d).min(20.0),
                "{} and {} came closer than 20px",
                node.id,
                other.id
            );
        }
    }
    assert!(result.report.issues.len() <= fixed.issues.len());
    assert!(
        crowded_pairs(&result.report, 10.0) <= crowded_pairs(&fixed, 10.0),
        "connectors run closer than 10px"
    );
    assert!(!result.svg.contains("translate(0 0)"), "{}", result.svg);
}

/// Given two boxes side by side that overlap vertically by only 8px, too
///   little to run a connector straight between their facing sides, so the
///   connector between them has to bend twice
/// When the diagram is arranged
/// Then one box moves up or down so the connector runs straight, and the
///   other stays where it was
#[test]
fn moves_a_node_a_little_to_straighten_its_connector() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 440 220">
        <g data-node="notify">
          <rect x="20" y="100" width="130" height="56" />
          <text x="85" y="133" text-anchor="middle" font-size="13">Notify</text>
        </g>
        <g data-node="mail">
          <rect x="280" y="52" width="130" height="56" />
          <text x="345" y="85" text-anchor="middle" font-size="13">Email service</text>
        </g>
        <path id="email" data-from="notify" data-to="mail" d="M 150 128 L 220 128 L 220 80 L 280 80" />
      </svg>
    "#;

    let before = analyze(svg);
    let result = arrange(svg);
    let moved = |id: &str| node_bounds(&result.report, id).y - node_bounds(&before, id).y;

    assert_eq!(bend_count(&connector_points(&before, "email")), 2);
    assert_eq!(bend_count(&connector_points(&result.report, "email")), 0);
    assert!(
        (moved("mail") == 0.0) != (moved("notify") == 0.0),
        "exactly one box moves"
    );
    assert!(moved("mail").abs() <= 60.0 && moved("notify").abs() <= 60.0);
    assert_eq!(node_bounds(&result.report, "mail").x, node_bounds(&before, "mail").x);
    assert_eq!(
        node_bounds(&result.report, "notify").x,
        node_bounds(&before, "notify").x
    );
    assert!(result.report.issues.is_empty(), "{:?}", result.report.issues);
    assert_layout_kept(svg, &result);

    assert_fix_snapshots!(result);
}

/// Given two connectors whose detours cross: a reply running down from one
///   process and left into another, and a pick list running up from a third
///   and right into a fourth
/// When the diagram is arranged
/// Then the connectors no longer cross, rerouted or with a node moved a
///   little
#[test]
fn untangles_crossing_connectors() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 640 420">
        <g data-node="checkout"><circle cx="100" cy="200" r="50"/><text x="100" y="205" text-anchor="middle" font-size="13">Check out</text></g>
        <g data-node="authorize"><circle cx="360" cy="70" r="50"/><text x="360" y="75" text-anchor="middle" font-size="13">Authorize</text></g>
        <g data-node="allocate"><circle cx="360" cy="330" r="50"/><text x="360" y="335" text-anchor="middle" font-size="13">Allocate</text></g>
        <g data-node="ship"><circle cx="530" cy="190" r="50"/><text x="530" y="195" text-anchor="middle" font-size="13">Ship</text></g>
        <path id="approval" data-from="authorize" data-to="checkout" d="M 380 120 L 380 220 L 150 220"/>
        <path id="picklist" data-from="allocate" data-to="ship" d="M 340 280 L 340 190 L 480 190"/>
      </svg>
    "#;

    let before = analyze(svg);
    let result = arrange(svg);

    assert!(crosses_route(
        &connector_points(&before, "approval"),
        &connector_points(&before, "picklist")
    ));
    assert!(!crosses_route(
        &connector_points(&result.report, "approval"),
        &connector_points(&result.report, "picklist")
    ));
    assert!(result.report.issues.is_empty(), "{:?}", result.report.issues);
    assert_layout_kept(svg, &result);

    assert_fix_snapshots!(result);
}

/// Given a service inside a titled cluster, joined to an external system
///   whose centre is 30px higher, where the external system is held in
///   place between two others only 20px away
/// When the diagram is arranged
/// Then the service does not move up onto the cluster's title, and keeps
///   its distance from the cluster's border
#[test]
fn keeps_a_moved_node_off_its_container_s_title_and_border() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 -20 700 400">
        <g data-node="cluster">
          <rect x="20" y="40" width="300" height="200" />
          <text x="36" y="68" font-size="13">Kubernetes cluster (EKS)</text>
          <g data-node="auth">
            <rect x="60" y="90" width="128" height="48" />
            <text x="124" y="119" text-anchor="middle" font-size="13">Auth service</text>
          </g>
        </g>
        <g data-node="above">
          <rect x="460" y="4" width="160" height="48" />
          <text x="540" y="33" text-anchor="middle" font-size="13">Above</text>
        </g>
        <g data-node="idp">
          <rect x="460" y="72" width="160" height="48" />
          <text x="540" y="101" text-anchor="middle" font-size="13">Identity provider</text>
        </g>
        <g data-node="below">
          <rect x="460" y="140" width="160" height="48" />
          <text x="540" y="169" text-anchor="middle" font-size="13">Below</text>
        </g>
        <path id="oidc" data-from="auth" data-to="idp" d="M 188 114 L 400 114 L 400 96 L 460 96" />
      </svg>
    "#;

    let fixed = fix(svg);
    let result = arrange(svg);
    let title = node_bounds(&fixed.report, "cluster");
    let auth = node_bounds(&result.report, "auth");
    let auth_before = node_bounds(&fixed.report, "auth");

    assert!(auth.y >= auth_before.y.min(title.y + 36.0), "{auth:?}");
    assert!(auth.x - title.x >= (auth_before.x - title.x).min(12.0));
    assert!(fixed.report.issues.is_empty(), "{:?}", fixed.report.issues);
    assert!(result.report.issues.is_empty(), "{:?}", result.report.issues);
    assert_layout_kept(svg, &result);

    assert_fix_snapshots!(result);
}

/// Given a cluster with a row of order, payment and inventory services,
///   where order and payment are joined but inventory is joined to
///   neither, and a charge from payment to a provider outside the cluster,
///   beyond inventory, that detours around it
/// When the diagram is arranged
/// Then inventory moves out of the way so the charge runs straight, while
///   order and payment stay in line
#[test]
fn moves_a_node_out_of_a_connector_s_way_breaking_only_a_weak_alignment() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="-20 -20 920 250">
        <g data-node="cluster">
          <rect x="0" y="0" width="660" height="210"/>
          <text x="12" y="20" font-size="13">Cluster</text>
          <g data-node="order"><rect x="20" y="80" width="128" height="48"/><text x="84" y="109" text-anchor="middle" font-size="13">Order service</text></g>
          <g data-node="payment"><rect x="234" y="80" width="140" height="48"/><text x="304" y="109" text-anchor="middle" font-size="13">Payment service</text></g>
          <g data-node="inventory"><rect x="460" y="80" width="156" height="48"/><text x="538" y="109" text-anchor="middle" font-size="13">Inventory service</text></g>
        </g>
        <g data-node="psp"><rect x="730" y="80" width="150" height="48"/><text x="805" y="109" text-anchor="middle" font-size="13">Payment provider</text></g>
        <line id="grpc" data-from="order" data-to="payment" x1="148" y1="104" x2="234" y2="104"/>
        <path id="charges" data-from="payment" data-to="psp" d="M 374 104 L 420 104 L 420 60 L 680 60 L 680 104 L 730 104"/>
      </svg>
    "#;

    let before = analyze(svg);
    let result = arrange(svg);

    assert_eq!(bend_count(&connector_points(&before, "charges")), 4);
    assert_eq!(bend_count(&connector_points(&result.report, "charges")), 0);
    assert_ne!(
        node_bounds(&result.report, "inventory").y,
        node_bounds(&before, "inventory").y
    );
    assert_eq!(node_bounds(&result.report, "order"), node_bounds(&before, "order"));
    assert_eq!(node_bounds(&result.report, "payment"), node_bounds(&before, "payment"));
    assert!(result.report.issues.is_empty(), "{:?}", result.report.issues);
    assert_layout_kept(svg, &result);

    assert_fix_snapshots!(result);
}

/// Given the same row, where order is also joined to inventory by a
///   connector that detours over payment instead of running along the row
/// When the diagram is arranged
/// Then that joint does not hold inventory in line: inventory still moves
///   out of the charge's way and the charge runs straight
#[test]
fn treats_an_alignment_whose_connector_detours_as_weak() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="-20 -20 920 250">
        <g data-node="cluster">
          <rect x="0" y="0" width="660" height="210"/>
          <text x="12" y="20" font-size="13">Cluster</text>
          <g data-node="order"><rect x="20" y="80" width="128" height="48"/><text x="84" y="109" text-anchor="middle" font-size="13">Order service</text></g>
          <g data-node="payment"><rect x="234" y="80" width="140" height="48"/><text x="304" y="109" text-anchor="middle" font-size="13">Payment service</text></g>
          <g data-node="inventory"><rect x="460" y="80" width="156" height="48"/><text x="538" y="109" text-anchor="middle" font-size="13">Inventory service</text></g>
        </g>
        <g data-node="psp"><rect x="730" y="80" width="150" height="48"/><text x="805" y="109" text-anchor="middle" font-size="13">Payment provider</text></g>
        <line id="grpc" data-from="order" data-to="payment" x1="148" y1="104" x2="234" y2="104"/>
        <path id="stock" data-from="order" data-to="inventory" d="M 84 80 L 84 30 L 538 30 L 538 80"/>
        <path id="charges" data-from="payment" data-to="psp" d="M 374 104 L 420 104 L 420 60 L 680 60 L 680 104 L 730 104"/>
      </svg>
    "#;

    let before = analyze(svg);
    let result = arrange(svg);

    assert_eq!(bend_count(&connector_points(&result.report, "charges")), 0);
    assert_ne!(
        node_bounds(&result.report, "inventory").y,
        node_bounds(&before, "inventory").y
    );
    assert_eq!(node_bounds(&result.report, "payment"), node_bounds(&before, "payment"));
    assert!(result.report.issues.is_empty(), "{:?}", result.report.issues);
    assert_layout_kept(svg, &result);

    assert_fix_snapshots!(result);
}

/// Given the storefront architecture draft: services in rows inside a
///   cluster, with the charge from payment to the payment provider
///   detouring around inventory, which is in the payment row but joined to
///   neither end
/// When the diagram is arranged
/// Then the charge bends less than after fixing, the connectors entering
///   PostgreSQL from the right are spread evenly, none runs along a
///   container's border, the
///   catalog service between auth and cart does not dent out of their row,
///   the gateway's call to the cart bends at most twice, and no connector
///   gains a jog shorter than 20px
#[test]
fn arranges_the_storefront_architecture_without_denting_a_row() {
    let svg = include_str!("fixtures/storefront-architecture.svg");

    let fixed = fix(svg);
    let result = arrange(svg);
    let centre_y = |id: &str| {
        let bounds = node_bounds(&result.report, id);
        bounds.y + bounds.height / 2.0
    };
    let jogs = |report: &AnalysisReport| -> usize {
        report
            .diagram
            .connectors
            .iter()
            .map(|connector| {
                let points = &connector.points;
                (1..points.len().saturating_sub(2))
                    .filter(|index| {
                        let (a, b) = (points[*index], points[index + 1]);
                        (a.x - b.x).abs() + (a.y - b.y).abs() < 20.0
                    })
                    .count()
            })
            .sum()
    };

    assert!(bend_count(&connector_points(&result.report, "f20")) < bend_count(&connector_points(&fixed.report, "f20")));
    // Whatever connectors enter PostgreSQL from the right are spread evenly,
    // keeping the same gap from each other and from the side's corners.
    let postgres = node_bounds(&result.report, "postgres");
    let mut right: Vec<f64> = result
        .report
        .diagram
        .connectors
        .iter()
        .flat_map(|connector| [connector.points[0], connector.points[connector.points.len() - 1]])
        .filter(|end| {
            end.x == postgres.x + postgres.width && end.y > postgres.y && end.y < postgres.y + postgres.height
        })
        .map(|end| end.y)
        .collect();
    right.sort_by(f64::total_cmp);
    let edges: Vec<f64> = [postgres.y]
        .into_iter()
        .chain(right.iter().copied())
        .chain([postgres.y + postgres.height])
        .collect();
    let gaps: Vec<f64> = edges.windows(2).map(|pair| pair[1] - pair[0]).collect();
    assert!(gaps.iter().all(|gap| (gap - gaps[0]).abs() <= 1.0), "{gaps:?}");
    // No connector runs along a container's border, where it would read
    // as part of the border.
    let containers: Vec<nicevg::Bounds> = result
        .report
        .diagram
        .nodes
        .iter()
        .filter(|node| {
            result
                .report
                .diagram
                .nodes
                .iter()
                .any(|child| child.parent_id.as_deref() == Some(&node.id))
        })
        .map(|node| node.bounds)
        .collect();
    for connector in &result.report.diagram.connectors {
        for (a, b) in segments(&connector.points) {
            for c in &containers {
                let along_x = a.y == b.y
                    && [c.y, c.y + c.height].iter().any(|y| (a.y - y).abs() < 8.0)
                    && a.x.max(b.x).min(c.x + c.width) - a.x.min(b.x).max(c.x) >= 20.0;
                let along_y = a.x == b.x
                    && [c.x, c.x + c.width].iter().any(|x| (a.x - x).abs() < 8.0)
                    && a.y.max(b.y).min(c.y + c.height) - a.y.min(b.y).max(c.y) >= 20.0;
                assert!(!along_x && !along_y, "{} runs along a container border", connector.id);
            }
        }
    }
    // The gateway's call to the cart turns into the cart from below once
    // the services around it have settled.
    assert!(bend_count(&connector_points(&result.report, "f10")) <= 2);
    // A dent: the neighbours still line up but the node between them
    // does not.
    let dented = centre_y("auth") == centre_y("cart") && centre_y("catalog") != centre_y("auth");
    assert!(!dented, "catalog dents out of the row");
    assert!(jogs(&result.report) <= jogs(&fixed.report));
    assert!(result.report.issues.is_empty(), "{:?}", result.report.issues);
    assert_layout_kept(svg, &result);

    assert_fix_snapshots!(result);
}

/// Given the order checkout DFD draft, where after fixing the pick list from
///   allocate to ship climbs in a staircase to stay off the approval line
/// When the diagram is arranged
/// Then a node moves so the pick list turns less, connectors reach allocate
///   and manage cart without a short stub or a crossing right at the node,
///   and nothing is reported
#[test]
fn arranges_the_order_checkout_dfd_so_the_pick_list_turns_less() {
    let svg = include_str!("fixtures/order-checkout-dfd.svg");

    let fixed = fix(svg);
    let result = arrange(svg);

    assert!(bend_count(&connector_points(&fixed.report, "f17")) >= 2);
    assert!(bend_count(&connector_points(&result.report, "f17")) < bend_count(&connector_points(&fixed.report, "f17")));
    // Connectors reach allocate and manage cart readably: no end stub
    // under 20px into allocate, no crossing within 20px of manage cart.
    assert!(shortest_end_segment(&result.report, "allocate") >= 20.0);
    assert!(nearest_crossing_at_end(&result.report, "cart") >= 20.0);
    assert!(result.report.issues.is_empty(), "{:?}", result.report.issues);
    assert_layout_kept(svg, &result);

    assert_fix_snapshots!(result);
}

/// Rows and columns of three or more sibling nodes sharing a centre line
///   after fixing, each listed along the line, with the line's position.
fn lines_of_three(report: &AnalysisReport) -> Vec<(bool, f64, Vec<String>)> {
    let nodes = &report.diagram.nodes;
    let containers: Vec<&str> = nodes.iter().filter_map(|node| node.parent_id.as_deref()).collect();
    let movable: Vec<_> = nodes
        .iter()
        .filter(|node| !containers.contains(&node.id.as_str()))
        .collect();
    [false, true]
        .into_iter()
        .flat_map(|column| {
            let centre = move |b: &nicevg::Bounds| {
                if column {
                    b.x + b.width / 2.0
                } else {
                    b.y + b.height / 2.0
                }
            };
            let along = move |b: &nicevg::Bounds| if column { b.y } else { b.x };
            let mut lines: Vec<(bool, f64, Option<String>, Vec<&nicevg::DiagramNode>)> = Vec::new();
            for node in &movable {
                let value = centre(&node.bounds);
                match lines
                    .iter_mut()
                    .find(|(_, at, parent, _)| (at - value).abs() <= 0.5 && *parent == node.parent_id)
                {
                    Some((_, _, _, members)) => members.push(node),
                    None => lines.push((column, value, node.parent_id.clone(), vec![node])),
                }
            }
            lines.into_iter().filter(|(_, _, _, members)| members.len() >= 3).map(
                move |(column, at, _, mut members)| {
                    members.sort_by(|a, b| along(&a.bounds).total_cmp(&along(&b.bounds)));
                    (column, at, members.into_iter().map(|node| node.id.clone()).collect())
                },
            )
        })
        .collect()
}

/// Inner runs of a line's nodes that left it while the nodes either side
///   stayed: dents, however many nodes wide.
fn dents(fixed: &AnalysisReport, arranged: &AnalysisReport) -> Vec<Vec<String>> {
    lines_of_three(fixed)
        .into_iter()
        .flat_map(|(column, at, members)| {
            let in_line = |id: &String| {
                let b = node_bounds(arranged, id);
                let centre = if column {
                    b.x + b.width / 2.0
                } else {
                    b.y + b.height / 2.0
                };
                (centre - at).abs() <= 0.5
            };
            let kept: Vec<bool> = members.iter().map(in_line).collect();
            let mut found = Vec::new();
            let mut start = None;
            for (index, keep) in kept.iter().enumerate() {
                match (keep, start) {
                    (false, None) => start = Some(index),
                    (true, Some(first)) => {
                        if first > 0 {
                            found.push(members[first..index].to_vec());
                        }
                        start = None;
                    }
                    _ => {}
                }
            }
            found
        })
        .collect()
}

/// Given a C4 container diagram of a food delivery platform drafted with
///   straight lines between centres: people, apps, services and data
///   stores in rows inside the platform boundary, external systems beside
///   it, and multi-line labels on every element and relationship
/// When the diagram is arranged
/// Then nothing is reported, no row or column is dented, and the layout
///   is kept
#[test]
fn arranges_a_c4_container_diagram_without_issues_or_dents() {
    let svg = include_str!("fixtures/c4-food-delivery-containers.svg");

    let fixed = fix(svg);
    let result = arrange(svg);

    assert!(result.report.issues.is_empty(), "{:?}", result.report.issues);
    assert_eq!(dents(&fixed.report, &result.report), Vec::<Vec<String>>::new());
    assert_layout_kept(svg, &result);

    assert_fix_snapshots!(result);
}

/// Given a diagram whose connectors already run straight without crossing
/// When the diagram is arranged
/// Then nothing moves and the SVG is what fixing alone writes
#[test]
fn leaves_a_diagram_with_nothing_to_improve_unchanged() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 400 120">
        <g data-node="a"><rect x="20" y="20" width="100" height="56"/><text x="70" y="53" text-anchor="middle" font-size="14">A</text></g>
        <g data-node="b"><rect x="260" y="20" width="100" height="56"/><text x="310" y="53" text-anchor="middle" font-size="14">B</text></g>
        <line id="flow" data-from="a" data-to="b" x1="120" y1="48" x2="260" y2="48"/>
      </svg>
    "#;

    let result = arrange(svg);

    assert_eq!(result.svg, fix(svg).svg);
    assert!(change_codes(&result).iter().all(|code| code != "arrange-node"));
}
