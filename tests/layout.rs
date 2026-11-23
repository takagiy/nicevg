//! Layout: after fixing, nodes move a little, keeping the input's rough
//! layout, so edges bend, cross and crowd less.

mod support;

use nicevg::{FixResult, InspectionReport, Point};
use support::*;

/// Every node of the laid out diagram stays on the same side of the nodes
/// beside, above and below it as after fixing, moves at most 60px on each
/// axis, comes no
/// closer than 20px to a sibling it was further from, no issue is added
/// and no more edges run within 10px of each other.
fn assert_layout_kept(svg: &str, result: &FixResult) {
    let fixed: InspectionReport = fix(svg).report;
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
        "edges run closer than 10px"
    );
    assert!(!result.svg.contains("translate(0 0)"), "{}", result.svg);
}

/// Given two boxes side by side that overlap vertically by only 8px, too
///   little to run an edge straight between their facing sides, so the
///   edge between them has to bend twice
/// When the diagram is laid out
/// Then one box moves up or down so the edge runs straight, and the
///   other stays where it was
#[test]
fn moves_a_node_a_little_to_straighten_its_edge() {
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

    let before = inspect(svg);
    let result = layout(svg);
    let moved = |id: &str| node_bounds(&result.report, id).y - node_bounds(&before, id).y;

    assert_eq!(bend_count(&edge_points(&before, "email")), 2);
    assert_eq!(bend_count(&edge_points(&result.report, "email")), 0);
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

/// Given the two boxes above, whose edge laying out straightens, and
///   elsewhere a box joined to a node drawn as a circle of radius 0
/// When the diagram is laid out
/// Then the edge between the two boxes comes out straight all the same
#[test]
fn lays_out_a_diagram_holding_a_node_of_no_size() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 700 420">
        <g data-node="notify">
          <rect x="20" y="100" width="130" height="56" />
          <text x="85" y="133" text-anchor="middle" font-size="13">Notify</text>
        </g>
        <g data-node="mail">
          <rect x="280" y="52" width="130" height="56" />
          <text x="345" y="85" text-anchor="middle" font-size="13">Email service</text>
        </g>
        <path id="email" data-from="notify" data-to="mail" d="M 150 128 L 220 128 L 220 80 L 280 80" />
        <g data-node="log">
          <rect x="500" y="240" width="130" height="56" />
          <text x="565" y="273" text-anchor="middle" font-size="13">Log</text>
        </g>
        <g data-node="dot"><circle cx="565" cy="370" r="0" /></g>
        <path id="logged" data-from="log" data-to="dot" d="M 565 296 L 565 370" />
      </svg>
    "#;

    let result = layout(svg);

    assert_eq!(bend_count(&edge_points(&result.report, "email")), 0);
}

/// Given two boxes of one height in a row, joined by a straight edge
///   that runs 12px above the middle of the sides it leaves and enters
/// When the diagram is laid out
/// Then the edge is rerouted in place through the middle of both
///   sides, still straight, and neither box moves
#[test]
fn reroutes_a_lone_edge_off_the_middle_of_its_sides_through_it() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 540 120">
        <g data-node="orders">
          <rect x="20" y="40" width="110" height="44" />
          <text x="75" y="67" text-anchor="middle" font-size="14">Orders</text>
        </g>
        <g data-node="db">
          <rect x="390" y="40" width="120" height="44" />
          <text x="450" y="67" text-anchor="middle" font-size="14">Orders DB</text>
        </g>
        <path id="save" data-from="orders" data-to="db" d="M 130 50 L 390 50" />
      </svg>
    "#;

    let before = inspect(svg);
    let result = layout(svg);
    let points = edge_points(&result.report, "save");

    assert!(before.issues.is_empty());
    assert_eq!(bend_count(&points), 0);
    assert!(points.iter().all(|p| p.y == 62.0), "{points:?}");
    for id in ["orders", "db"] {
        assert_eq!(node_bounds(&result.report, id), node_bounds(&before, id));
    }
    assert!(result.report.issues.is_empty(), "{:?}", result.report.issues);

    assert_fix_snapshots!(result);
}

/// Given two edges whose detours cross: a reply running down from one
///   process and left into another, and a pick list running up from a third
///   and right into a fourth
/// When the diagram is laid out
/// Then the edges no longer cross, rerouted or with a node moved a
///   little
#[test]
fn untangles_crossing_edges() {
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

    let before = inspect(svg);
    let result = layout(svg);

    assert!(crosses_route(
        &edge_points(&before, "approval"),
        &edge_points(&before, "picklist")
    ));
    assert!(!crosses_route(
        &edge_points(&result.report, "approval"),
        &edge_points(&result.report, "picklist")
    ));
    assert!(result.report.issues.is_empty(), "{:?}", result.report.issues);
    assert_layout_kept(svg, &result);

    assert_fix_snapshots!(result);
}

/// Given a service inside a cluster drawn as a custom node with its title
///   in the picture, pulled up by an edge to an external system 30px
///   higher that is held in place
/// When the diagram is laid out
/// Then the service stays 4px clear of the title, as it does of a built-in
///   parent's
#[test]
fn keeps_a_moved_node_off_a_custom_parent_s_title() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 -20 700 400">
        <g data-node="cluster" data-custom="true">
          <g>
            <rect data-bounds="true" x="20" y="40" width="300" height="200" />
            <text x="36" y="68" font-size="13">Kubernetes cluster (EKS)</text>
          </g>
          <g data-node="auth">
            <rect x="60" y="90" width="128" height="48" />
            <text x="124" y="119" text-anchor="middle" font-size="13">Auth service</text>
          </g>
        </g>
        <g data-node="above"><rect x="460" y="4" width="160" height="48" /><text x="540" y="33" text-anchor="middle" font-size="13">Above</text></g>
        <g data-node="idp"><rect x="460" y="72" width="160" height="48" /><text x="540" y="101" text-anchor="middle" font-size="13">Identity provider</text></g>
        <g data-node="below"><rect x="460" y="140" width="160" height="48" /><text x="540" y="169" text-anchor="middle" font-size="13">Below</text></g>
        <path id="oidc" data-from="auth" data-to="idp" d="M 188 114 L 400 114 L 400 96 L 460 96" />
      </svg>
    "#;

    let result = layout(svg);
    let title = result.report.diagram.node("cluster").expect("cluster").label_bounds[0];

    assert!(node_bounds(&result.report, "auth").y >= title.bottom() + 4.0);
}

/// Given a service inside a titled cluster, joined to an external system
///   whose centre is 30px higher, where the external system is held in
///   place between two others only 20px away
/// When the diagram is laid out
/// Then the service does not move up onto the cluster's title, and keeps
///   its distance from the cluster's outline
#[test]
fn keeps_a_moved_node_off_its_parent_s_title_and_outline() {
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
    let result = layout(svg);
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
/// When the diagram is laid out
/// Then inventory moves out of the way so the charge runs straight, while
///   order and payment stay in line
#[test]
fn moves_a_node_out_of_a_edge_s_way_breaking_only_a_weak_alignment() {
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

    let before = inspect(svg);
    let result = layout(svg);

    assert_eq!(bend_count(&edge_points(&before, "charges")), 4);
    assert_eq!(bend_count(&edge_points(&result.report, "charges")), 0);
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
///   edge that detours over payment instead of running along the row
/// When the diagram is laid out
/// Then that joint does not hold inventory in line: inventory still moves
///   out of the charge's way and the charge runs straight
#[test]
fn treats_an_alignment_whose_edge_detours_as_weak() {
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

    let before = inspect(svg);
    let result = layout(svg);

    assert_eq!(bend_count(&edge_points(&result.report, "charges")), 0);
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
/// When the diagram is laid out
/// Then the edges entering PostgreSQL from the right are spread
///   evenly, none runs along a parent's outline, the
///   catalog service between auth and cart does not inner misalignment out of their row,
///   the gateway's call to the cart bends at most twice, and no edge
///   gains a jog shorter than 20px
#[test]
fn lays_out_the_storefront_architecture_without_denting_a_row() {
    let svg = include_str!("fixtures/storefront-architecture.svg");

    let fixed = fix(svg);
    let result = layout(svg);
    let centre_y = |id: &str| {
        let bounds = node_bounds(&result.report, id);
        bounds.y + bounds.height / 2.0
    };
    let jogs =
        |report: &InspectionReport| -> usize { report.diagram.edges.iter().map(|edge| jog_steps(&edge.points)).sum() };

    // Whatever edges enter PostgreSQL from the right are spread evenly,
    // keeping the same gap from each other and from the side's corners.
    let postgres = node_bounds(&result.report, "postgres");
    let mut right: Vec<f64> = result
        .report
        .diagram
        .edges
        .iter()
        .flat_map(|edge| [edge.points[0], edge.points[edge.points.len() - 1]])
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
    // No edge runs along a parent's outline, where it would read
    // as part of the outline.
    let parents: Vec<nicevg::Bounds> = result
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
    for edge in &result.report.diagram.edges {
        for (a, b) in segments(&edge.points) {
            for c in &parents {
                let along_x = a.y == b.y
                    && [c.y, c.y + c.height].iter().any(|y| (a.y - y).abs() < 8.0)
                    && a.x.max(b.x).min(c.x + c.width) - a.x.min(b.x).max(c.x) >= 20.0;
                let along_y = a.x == b.x
                    && [c.x, c.x + c.width].iter().any(|x| (a.x - x).abs() < 8.0)
                    && a.y.max(b.y).min(c.y + c.height) - a.y.min(b.y).max(c.y) >= 20.0;
                assert!(!along_x && !along_y, "{} runs along a container border", edge.id);
            }
        }
    }
    // The gateway's call to the cart bends into the cart from below once
    // the services around it have settled.
    assert!(bend_count(&edge_points(&result.report, "f10")) <= 2);
    // An inner misalignment: the neighbours still line up but the node between them
    // does not.
    let misaligned = centre_y("auth") == centre_y("cart") && centre_y("catalog") != centre_y("auth");
    assert!(!misaligned, "catalog is misaligned inside the row");
    assert!(jogs(&result.report) <= jogs(&fixed.report));
    assert!(result.report.issues.is_empty(), "{:?}", result.report.issues);
    assert_layout_kept(svg, &result);

    assert_fix_snapshots!(result);
}

/// Given the DFD of a plugin listing terminals in a sidebar, as its sketch
///   is drawn before layout, its nodes where the sketch puts them and its
///   edges straight between node centres: hc, an external entity 1490px
///   tall, beside a column of processes that read from and write to it from
///   top to bottom
/// When the diagram is laid out
/// Then nothing is reported, and every edge between hc and a process beside
///   it runs straight across
#[test]
fn lays_out_the_terminal_sidebar_dfd_with_straight_edges_into_its_tall_entity() {
    let svg = include_str!("fixtures/terminal-sidebar-dfd.svg");

    let result = layout(svg);

    assert!(result.report.issues.is_empty(), "{:?}", result.report.issues);
    for edge in result
        .report
        .diagram
        .edges
        .iter()
        .filter(|edge| edge.from == "hc" || edge.to == "hc")
    {
        assert_eq!(bend_count(&edge.points), 0, "{} bends: {:?}", edge.id, edge.points);
    }

    assert_fix_snapshots!(result);
}

/// Given the order checkout DFD draft, where the pick list from allocate to
///   ship has to get past the approval line
/// When the diagram is laid out
/// Then the pick list bends at most once, edges reach allocate and
///   manage cart without a short stub or a crossing right at the node, and
///   nothing is reported
#[test]
fn lays_out_the_order_checkout_dfd_with_the_pick_list_bending_once() {
    let svg = include_str!("fixtures/order-checkout-dfd.svg");

    let result = layout(svg);

    assert!(bend_count(&edge_points(&result.report, "f17")) <= 1);
    // Edges reach allocate and manage cart readably: no end stub
    // under 20px into allocate, no crossing within 20px of manage cart.
    assert!(shortest_end_segment(&result.report, "allocate") >= 20.0);
    assert!(nearest_crossing_at_end(&result.report, "cart") >= 20.0);
    assert!(result.report.issues.is_empty(), "{:?}", result.report.issues);
    assert_layout_kept(svg, &result);

    assert_fix_snapshots!(result);
}

/// Rows and columns of three or more sibling nodes sharing a centre line
///   after fixing, each listed along the line, with the line's position.
fn lines_of_three(report: &InspectionReport) -> Vec<(bool, f64, Vec<String>)> {
    let nodes = &report.diagram.nodes;
    let parents: Vec<&str> = nodes.iter().filter_map(|node| node.parent_id.as_deref()).collect();
    let movable: Vec<_> = nodes
        .iter()
        .filter(|node| !parents.contains(&node.id.as_str()))
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
///   stayed: inner misalignments, however many nodes wide.
fn inner_misalignments(fixed: &InspectionReport, laid_out: &InspectionReport) -> Vec<Vec<String>> {
    lines_of_three(fixed)
        .into_iter()
        .flat_map(|(column, at, members)| {
            let in_line = |id: &String| {
                let b = node_bounds(laid_out, id);
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
/// When the diagram is laid out
/// Then nothing is reported, no row or column has an inner misalignment, and the layout
///   is kept
#[test]
fn arranges_a_c4_container_diagram_without_issues_or_dents() {
    let svg = include_str!("fixtures/c4-food-delivery-containers.svg");

    let fixed = fix(svg);
    let result = layout(svg);

    assert!(result.report.issues.is_empty(), "{:?}", result.report.issues);
    assert_eq!(
        inner_misalignments(&fixed.report, &result.report),
        Vec::<Vec<String>>::new()
    );
    assert_layout_kept(svg, &result);

    assert_fix_snapshots!(result);
}

const SOCIAL_APP_ER: &str = include_str!("fixtures/social-app-er.svg");

/// The social app's ER diagram laid out, once for every test that reads it.
fn social_app_er_laid_out() -> &'static FixResult {
    static LAID_OUT: std::sync::OnceLock<FixResult> = std::sync::OnceLock::new();
    LAID_OUT.get_or_init(|| layout(SOCIAL_APP_ER))
}

/// Given a social app's ER diagram drafted with straight lines between
///   table centres: 24 tables drawn as custom nodes in five areas, and 32
///   foreign keys, each from its column's port to the port of the column
///   it refers to, nine of them to users, two pairs of tables joined twice
///   and a table referring to itself
/// When the diagram is laid out
/// Then the layout is kept, and its quality is recorded
#[test]
fn lays_out_a_social_app_er_diagram_keeping_its_layout() {
    let result = social_app_er_laid_out();

    assert_layout_kept(SOCIAL_APP_ER, result);
    assert_fix_snapshots!(result);
}

/// Given the social app's ER diagram
/// When it is laid out
/// Then every foreign key leaves the row of its column and reaches the row
///   of the column it refers to
#[test]
fn draws_each_foreign_key_of_the_social_app_from_its_column_to_the_one_it_refers_to() {
    let report = &social_app_er_laid_out().report;
    let at_port = |node: &str, port: Option<&str>, end: Point| {
        report
            .diagram
            .node(node)
            .expect(node)
            .ports
            .iter()
            .any(|place| Some(place.name.as_str()) == port && place.point == end)
    };

    let off: Vec<&str> = report
        .diagram
        .edges
        .iter()
        .filter(|edge| {
            let (start, end) = (edge.points[0], edge.points[edge.points.len() - 1]);
            !at_port(&edge.from, edge.from_port.as_deref(), start) || !at_port(&edge.to, edge.to_port.as_deref(), end)
        })
        .map(|edge| edge.id.as_str())
        .collect();

    assert_eq!(off, Vec::<&str>::new());
}

/// Given the social app's ER diagram, with two pairs of tables joined by
///   two foreign keys each
/// When it is laid out
/// Then no issue is left
#[test]
fn lays_out_the_social_app_er_diagram_without_issues() {
    let report = &social_app_er_laid_out().report;

    assert_eq!(issue_codes(report), Vec::<String>::new());
}

/// Given a tall loan origination DFD drafted with straight lines between
///   centres: a nested split and merge of checks, a manual review branch,
///   stores beside the processes, and flows skipping several processes
/// When the diagram is laid out
/// Then nothing is reported, the layout is kept, no edge steps aside
///   just before reaching a node or anywhere else, none arrives with a
///   last segment shorter than 20px, the two shortcuts leaving
///   "take application" side by side do not cross, neither do the two
///   edges entering "make offer" side by side from above, and no two
///   edges come so close that they read as touching
#[test]
fn lays_out_a_tall_loan_dfd_without_steps_at_the_ends() {
    let svg = include_str!("fixtures/loan-origination-dfd.svg");

    let result = layout(svg);

    assert!(result.report.issues.is_empty(), "{:?}", result.report.issues);
    assert_eq!(stepped_ends(&result.report), Vec::<String>::new());
    assert_eq!(jogged(&result.report), Vec::<String>::new());
    assert_eq!(short_arrivals(&result.report), Vec::<String>::new());
    assert_eq!(near_misses(&result.report), Vec::<(String, String)>::new());
    // f26 runs to "make offer" and f27 to "notify", both past every
    // process in between.
    assert!(!crosses_route(
        &edge_points(&result.report, "f26"),
        &edge_points(&result.report, "f27")
    ));
    // f20 comes straight down from "assess risk"; f26 joins it at the top
    // of "make offer" from the left.
    assert!(!crosses_route(
        &edge_points(&result.report, "f20"),
        &edge_points(&result.report, "f26")
    ));
    assert_layout_kept(svg, &result);

    assert_fix_snapshots!(result);
}

/// Steps of 20px or less between two runs of a route heading the same way.
fn jog_steps(points: &[Point]) -> usize {
    points
        .windows(4)
        .filter(|run| {
            let same_way =
                (run[1].x - run[0].x) * (run[3].x - run[2].x) + (run[1].y - run[0].y) * (run[3].y - run[2].y) > 0.0;
            same_way && (run[1].x - run[2].x).abs() + (run[1].y - run[2].y).abs() <= 20.0
        })
        .count()
}

/// Edges whose route steps aside anywhere.
fn jogged(report: &InspectionReport) -> Vec<String> {
    report
        .diagram
        .edges
        .iter()
        .filter(|edge| jog_steps(&edge.points) > 0)
        .map(|edge| edge.id.clone())
        .collect()
}

/// Bent edges whose last segment, into the node they arrive at, is
///   shorter than 20px.
fn short_arrivals(report: &InspectionReport) -> Vec<String> {
    report
        .diagram
        .edges
        .iter()
        .filter(|edge| {
            let points = &edge.points;
            points.len() > 2 && {
                let (a, b) = (points[points.len() - 2], points[points.len() - 1]);
                (a.x - b.x).abs() + (a.y - b.y).abs() < 20.0
            }
        })
        .map(|edge| edge.id.clone())
        .collect()
}

/// Edges whose route steps aside just before an end: a bend, a step
///   under 20px, and a bend back into the same direction as before.
fn stepped_ends(report: &InspectionReport) -> Vec<String> {
    report
        .diagram
        .edges
        .iter()
        .filter(|edge| {
            let points = &edge.points;
            let steps = |p: &[Point]| {
                p.len() >= 4 && {
                    let (a, b, c, d) = (p[0], p[1], p[2], p[3]);
                    let parallel = (a.x == b.x && c.x == d.x) || (a.y == b.y && c.y == d.y);
                    parallel && (b.x - c.x).abs() + (b.y - c.y).abs() < 20.0
                }
            };
            let reversed: Vec<Point> = points.iter().rev().copied().collect();
            steps(points) || steps(&reversed)
        })
        .map(|edge| edge.id.clone())
        .collect()
}

/// Given a diagram whose edges already run straight without crossing
/// When the diagram is laid out
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

    let result = layout(svg);

    assert_eq!(result.svg, fix(svg).svg);
    assert!(change_codes(&result).iter().all(|code| code != "layout-node"));
}

/// `article` with the port "author" on its right and `user` to its right
/// with the port "id" on its left 14px lower, joined port to port by a
/// edge stepping down between them.
const PORTS_A_LITTLE_APART: &str = r#"
  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 480 200">
    <g data-node="article" data-custom="true">
      <g transform="translate(20 20)">
        <rect data-bounds="true" width="120" height="96"/>
        <circle data-port="author" cx="120" cy="62" r="0"/>
      </g>
    </g>
    <g data-node="user" data-custom="true">
      <g transform="translate(300 20)">
        <rect data-bounds="true" width="120" height="96"/>
        <circle data-port="id" cx="0" cy="76" r="0"/>
      </g>
    </g>
    <path id="wrote" data-from="article" data-from-port="author" data-to="user" data-to-port="id"
      d="M 140 82 L 220 82 L 220 96 L 300 96"/>
  </svg>
"#;

/// Given an edge between two ports 14px apart in height
/// When the diagram is laid out and a node moves
/// Then the edge still leaves "author" and reaches "id" where they
///   are now
#[test]
fn keeps_edge_ends_at_their_ports_when_a_node_moves() {
    let result = layout(PORTS_A_LITTLE_APART);
    let report = &result.report;
    let port = |node: &str| report.diagram.node(node).expect(node).ports[0].point;
    let moved = ["article", "user"]
        .iter()
        .any(|id| node_bounds(report, id).y != node_bounds(&inspect(PORTS_A_LITTLE_APART), id).y);
    assert!(moved, "no node moved");

    let points = edge_points(report, "wrote");

    assert_eq!(
        (points[0], points[points.len() - 1]),
        (port("article"), port("user")),
        "{points:?}"
    );
}

/// Given a straight edge between two ports facing each other a
///   quarter of the way down their sides, the only edge on either
/// When the diagram is laid out
/// Then nothing changes: off the middle of their sides is where those
///   ports are
#[test]
fn leaves_a_edge_between_ports_off_the_middle_of_their_sides_as_it_is() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 480 200">
        <g data-node="article" data-custom="true">
          <g transform="translate(20 20)">
            <rect data-bounds="true" width="120" height="96"/>
            <circle data-port="id" cx="120" cy="24" r="0"/>
          </g>
        </g>
        <g data-node="user" data-custom="true">
          <g transform="translate(300 20)">
            <rect data-bounds="true" width="120" height="96"/>
            <circle data-port="id" cx="0" cy="24" r="0"/>
          </g>
        </g>
        <path id="same" data-from="article" data-from-port="id" data-to="user" data-to-port="id" d="M 140 44 L 300 44"/>
      </svg>
    "#;

    let result = layout(svg);

    assert_eq!(change_codes(&result), Vec::<String>::new());
}

/// Given an edge between two facing ports 14px apart in height
/// When the diagram is laid out
/// Then a node moves so the ports line up, and the edge runs straight
#[test]
fn lines_up_facing_ports_a_little_apart_to_straighten_their_edge() {
    let result = layout(PORTS_A_LITTLE_APART);

    let report = &result.report;
    let port = |node: &str| report.diagram.node(node).expect(node).ports[0].point;
    let points = edge_points(report, "wrote");
    assert_eq!(points, [port("article"), port("user")]);
    assert_eq!(points[0].y, points[1].y);
}

/// A child 40px from the bottom of its parent, joined to a node held still
///   inside a tight custom parent 32px lower; `below` puts a sibling under
///   the parent.
fn child_out_of_line(below: &str) -> String {
    format!(
        r#"
  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 460 260">
    <g data-node="group">
      <rect x="20" y="20" width="200" height="120" />
      <g data-node="child"><rect x="150" y="60" width="40" height="40" /></g>
    </g>
    <g data-node="frame" data-custom="true">
      <rect data-bounds="true" x="300" y="80" width="100" height="64" />
      <g data-node="held"><rect x="312" y="92" width="60" height="40" /></g>
    </g>
    {below}
    <path id="link" data-from="child" data-to="held" d="M 190 80 L 250 80 L 250 112 L 312 112" />
  </svg>
"#
    )
}

/// Given a child whose edge would run straight if it moved 32px down,
///   4px further than its parent leaves room for, to a node that cannot move
/// When the diagram is laid out
/// Then the child moves into line, and its parent grows to keep it 12px
///   inside
#[test]
fn moves_a_child_out_past_its_parent_s_padding_and_grows_the_parent() {
    let result = layout(&child_out_of_line(""));

    let child = node_bounds(&result.report, "child");
    assert_eq!(bend_count(&edge_points(&result.report, "link")), 0);
    assert!(node_bounds(&result.report, "group").bottom() >= child.bottom() + 12.0);
    assert!(result.report.issues.is_empty(), "{:?}", result.report.issues);
}

/// Given the same child out of line, and a sibling of its parent 20px below
///   the parent
/// When the diagram is laid out
/// Then the child is not moved into line: the parent would have to grow to
///   within 20px of the sibling, nearer than nodes beside each other may be
#[test]
fn keeps_a_child_where_growing_its_parent_would_crowd_a_sibling() {
    let result = layout(&child_out_of_line(
        r#"<g data-node="below"><rect x="20" y="160" width="200" height="60" /></g>"#,
    ));

    assert!(bend_count(&edge_points(&result.report, "link")) > 0);
    assert_eq!(node_bounds(&result.report, "group").bottom(), 140.0);
    assert_eq!(node_bounds(&result.report, "below").y, 160.0);
}
