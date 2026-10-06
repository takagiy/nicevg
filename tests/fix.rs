//! Repairs, each asserted by what its Given/When/Then promises, plus
//! snapshots of the fixed diagram's quality and written SVG to catch any
//! change in shape.

mod support;

use nicevg::{FixResult, Point};
use support::*;

const RETRY_LABEL: &str = r#"<text data-label="retry" data-label-for="flow"
        x="190" y="42" text-anchor="middle" font-size="14">Retry</text>"#;

const NOTE_AND_RETRY_LABEL: &str = r#"<text data-label="note" x="190" y="0" text-anchor="middle"
          font-size="14">Note</text>
        <text data-label="retry" data-label-for="flow"
          x="190" y="42" text-anchor="middle" font-size="14">Retry</text>"#;

/// Source, an unrelated block and a target in one row, joined by a
/// connector that runs straight through the block.
fn blocked_row(extra: &str) -> String {
    [
        r#"
  <svg xmlns="http://www.w3.org/2000/svg" viewBox="-100 -100 600 300">
    <g data-node="source">
      <rect x="20" y="20" width="100" height="56" />
      <text x="70" y="53" text-anchor="middle" font-size="14">Source</text>
    </g>
    <g data-node="obstacle">
      <rect x="150" y="20" width="80" height="56" />
      <text x="190" y="53" text-anchor="middle" font-size="14">Block</text>
    </g>
    <g data-node="target">
      <rect x="260" y="20" width="100" height="56" />
      <text x="310" y="53" text-anchor="middle" font-size="14">Target</text>
    </g>
    "#,
        extra,
        r#"
    <line id="flow" data-from="source" data-to="target"
      x1="120" y1="48" x2="260" y2="48" />
  </svg>
"#,
    ]
    .concat()
}

/// Given a diagram clipped on the right side
/// When the diagram is fixed
/// Then the viewBox expands to include safe padding and no clipping remains
#[test]
fn expands_a_clipping_viewbox_without_shrinking_its_existing_extent() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 120">
        <g data-node="checkout">
          <rect x="20" y="20" width="100" height="56" />
          <text x="70" y="53" text-anchor="middle" font-size="14">Checkout</text>
        </g>
      </svg>
    "#;

    let result = fix(svg);
    let written = view_box(Written::parse(&result.svg).root());

    assert!(contains(&written, &view_box(Written::parse(svg).root())));
    let drawing = result.report.drawing_bounds.expect("drawing bounds");
    assert!(contains(&written, &inflate(&drawing, 20.0)));
    assert!(!has_issue(&result.report, "viewport-clipping"));
    assert_eq!(change_codes(&result), ["expand-viewbox"]);

    assert_fix_snapshots!(result);
}

/// Given a diagram clipped on the right and bottom whose width and height
///   are absolute lengths with units
/// When the diagram is fixed
/// Then width and height grow with the viewBox, keeping their units, so the
///   drawing keeps its scale
#[test]
fn scales_absolute_width_and_height_with_the_viewbox() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 60" width="200px" height="6cm">
        <g data-node="checkout">
          <rect x="20" y="20" width="100" height="56" />
          <text x="70" y="53" text-anchor="middle" font-size="14">Checkout</text>
        </g>
      </svg>
    "#;

    let result = fix(svg);
    let before = Written::parse(svg);
    let after = Written::parse(&result.svg);
    let scale = |root: &std::collections::HashMap<String, String>, name: &str, extent: f64| {
        let (value, unit) = split_length(attr(root, name));
        (value / extent, unit)
    };

    let (old_box, new_box) = (view_box(before.root()), view_box(after.root()));
    assert!(new_box.width > old_box.width && new_box.height > old_box.height);
    for (name, old_extent, new_extent) in [
        ("width", old_box.width, new_box.width),
        ("height", old_box.height, new_box.height),
    ] {
        let (old_scale, old_unit) = scale(before.root(), name, old_extent);
        let (new_scale, new_unit) = scale(after.root(), name, new_extent);
        assert_eq!(new_unit, old_unit);
        assert!(
            (new_scale - old_scale).abs() <= old_scale * 1e-9,
            "{name}: {new_scale} vs {old_scale}"
        );
    }
    assert!(result.report.issues.is_empty());

    assert_fix_snapshots!(result);
}

/// Given a clipped diagram sized to its container with a percentage width
///   and no height
/// When the diagram is fixed
/// Then the viewBox grows but the width stays relative and no height is added
#[test]
fn leaves_relative_and_missing_sizes_to_the_container() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 120" width="100%">
        <g data-node="checkout">
          <rect x="20" y="20" width="100" height="56" />
          <text x="70" y="53" text-anchor="middle" font-size="14">Checkout</text>
        </g>
      </svg>
    "#;

    let result = fix(svg);
    let root = Written::parse(&result.svg);

    assert!(view_box(root.root()).width > 100.0);
    assert_eq!(attr(root.root(), "width"), "100%");
    assert!(!root.root().contains_key("height"));
    assert_eq!(change_codes(&result), ["expand-viewbox"]);

    assert_fix_snapshots!(result);
}

/// Given a label that violates its node's 12px inner padding
/// When the diagram is fixed
/// Then only the box expands and the text coordinates remain unchanged
#[test]
fn expands_a_node_box_around_its_label_without_moving_the_label() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="-100 -100 400 300">
        <g data-node="confirm">
          <rect x="20" y="20" width="70" height="40" />
          <text x="55" y="52" text-anchor="middle" font-size="14">Confirm payment</text>
        </g>
      </svg>
    "#;

    let result = fix(svg);
    let written = Written::parse(&result.svg);
    let text = written.nth("text", 0);
    let box_ = node_bounds(&result.report, "confirm");
    let label = result.report.diagram.nodes[0].label_bounds[0];

    assert!(contains(&box_, &bounds(20.0, 20.0, 70.0, 40.0)));
    assert!(contains(&box_, &inflate(&label, 12.0)));
    assert_eq!((attr(text, "x"), attr(text, "y")), ("55", "52"));
    assert!(!has_issue(&result.report, "text-overflow"));

    assert_fix_snapshots!(result);
}

/// Given a circular node whose label reaches within 12px of its circle
/// When the diagram is fixed
/// Then the radius grows until the label fits, while the centre and the
///   text coordinates stay put
#[test]
fn grows_a_circular_node_around_its_label_without_moving_either() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 240 200">
        <g data-node="confirm">
          <circle cx="120" cy="100" r="40" />
          <text x="120" y="105" text-anchor="middle" font-size="14">Confirmed</text>
        </g>
      </svg>
    "#;

    let result = fix(svg);
    let written = Written::parse(&result.svg);
    let circle = written.nth("circle", 0);
    let text = written.nth("text", 0);

    assert_eq!((attr(circle, "cx"), attr(circle, "cy")), ("120", "100"));
    assert!(attr(circle, "r").parse::<f64>().expect("numeric radius") > 40.0);
    assert_eq!((attr(text, "x"), attr(text, "y")), ("120", "105"));
    assert!(!has_issue(&result.report, "text-overflow"));

    assert_fix_snapshots!(result);
}

/// Given two same-row nodes that overlap by 30px
/// When the diagram is fixed
/// Then the right node is shifted to leave a 20px gap
#[test]
fn pushes_overlapping_nodes_apart_while_preserving_their_order() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="-100 -100 600 300">
        <g data-node="first">
          <rect x="20" y="20" width="100" height="56" />
          <text x="70" y="53" text-anchor="middle" font-size="14">First</text>
        </g>
        <g data-node="second">
          <rect x="90" y="20" width="100" height="56" />
          <text x="140" y="53" text-anchor="middle" font-size="14">Second</text>
        </g>
      </svg>
    "#;

    let result = fix(svg);
    let first = node_bounds(&result.report, "first");
    let second = node_bounds(&result.report, "second");

    assert_eq!(first, bounds(20.0, 20.0, 100.0, 56.0));
    assert!(second.x > first.x);
    assert!(gap_between(&first, &second) >= 20.0);
    assert!(!has_issue(&result.report, "node-overlap"));
    assert!(!has_issue(&result.report, "node-gap"));

    assert_fix_snapshots!(result);
}

/// Given a horizontal connector that crosses an unrelated node
/// When the diagram is fixed
/// Then it follows an orthogonal path with 8px obstacle clearance
#[test]
fn reroutes_a_connector_around_an_unrelated_node() {
    let svg = &blocked_row("");

    let result = fix(svg);
    let points = connector_points(&result.report, "flow");

    assert!(is_orthogonal(&points));
    assert!(!enters_box(
        &points,
        &inflate(&node_bounds(&result.report, "obstacle"), 8.0)
    ));
    assert!(!has_issue(&result.report, "connector-node-crossing"));

    assert_fix_snapshots!(result);
}

/// Given a horizontal connector that has to detour around an unrelated node
/// When the diagram is fixed
/// Then the detour's last bend stays at least 16px from the target, so an
///   arrowhead at the end does not sit on the bend, and the detour still
///   clears the node by 8px
#[test]
fn keeps_the_last_bend_clear_of_the_arrowhead() {
    let svg = &blocked_row("");

    let result = fix(svg);
    let points = connector_points(&result.report, "flow");
    let last = segments(&points).pop().expect("a segment");

    assert!(bend_count(&points) > 0);
    assert!(segment_length(&last) >= 16.0, "{points:?}");
    assert!(!enters_box(
        &points,
        &inflate(&node_bounds(&result.report, "obstacle"), 8.0)
    ));
    assert!(result.report.issues.is_empty());

    assert_fix_snapshots!(result);
}

/// Given an arrow drawn as a group carrying data-from and data-to around a
///   path that stops at the tail of a 10px polygon head, straight through
///   an unrelated node
/// When the diagram is fixed
/// Then the connector detours, the head moves to the new end on the target
///   and points along the last segment, and the path stops 10px before it
#[test]
fn moves_a_polygon_arrowhead_with_its_rerouted_connector() {
    let svg = &blocked_row("").replace(
        r#"<line id="flow" data-from="source" data-to="target"
      x1="120" y1="48" x2="260" y2="48" />"#,
        r#"<g id="flow" data-from="source" data-to="target">
      <path d="M 120 48 L 250 48" />
      <polygon points="250,43 260,48 250,53" />
    </g>"#,
    );

    let result = fix(svg);
    let points = connector_points(&result.report, "flow");
    let (end, before) = (points[points.len() - 1], points[points.len() - 2]);
    let written = Written::parse(&result.svg);
    let head: Vec<f64> = attr(written.nth("polygon", 0), "points")
        .split([' ', ','])
        .map(|value| value.parse().expect("a number"))
        .collect();
    let tip = point(head[2], head[3]);
    let base = point((head[0] + head[4]) / 2.0, (head[1] + head[5]) / 2.0);
    let shaft = attr(written.nth("path", 0), "d");

    assert!(bend_count(&points) > 0);
    assert_eq!(tip, end);
    assert!(side_of(end, &node_bounds(&result.report, "target")).is_some());
    assert_eq!((tip.x - base.x, tip.y - base.y), {
        let length = ((end.x - before.x).powi(2) + (end.y - before.y).powi(2)).sqrt();
        ((end.x - before.x) / length * 10.0, (end.y - before.y) / length * 10.0)
    });
    assert!(shaft.ends_with(&format!("{} {}", base.x, base.y)), "{shaft}");
    assert!(result.report.issues.is_empty(), "{:?}", result.report.issues);

    assert_fix_snapshots!(result);
}

/// Given a connector drawn inside a group translated by (-10, 5), crossing
///   an unrelated node
/// When the diagram is fixed
/// Then the detour, read back through the translation, clears the node and
///   ends on the target's outline
#[test]
fn writes_a_rerouted_connector_in_its_translated_group_s_coordinates() {
    let svg = &blocked_row("").replace(
        r#"<line id="flow" data-from="source" data-to="target"
      x1="120" y1="48" x2="260" y2="48" />"#,
        r#"<g transform="translate(-10 5)">
      <line id="flow" data-from="source" data-to="target" x1="130" y1="43" x2="270" y2="43" />
    </g>"#,
    );

    let result = fix(svg);
    let points = connector_points(&result.report, "flow");

    assert_eq!(
        connector_points(&analyze(svg), "flow"),
        [point(120.0, 48.0), point(260.0, 48.0)]
    );
    assert!(!enters_box(
        &points,
        &inflate(&node_bounds(&result.report, "obstacle"), 8.0)
    ));
    assert!(side_of(points[points.len() - 1], &node_bounds(&result.report, "target")).is_some());
    assert!(result.report.issues.is_empty(), "{:?}", result.report.issues);

    assert_fix_snapshots!(result);
}

/// Asserts the written arrowhead (a triangle listed base corner, tip, base
///   corner) sits with its tip on the connector's end and points along the
///   segment arriving there, with the shaft ending at its base 10px back.
fn assert_head_points_along_its_end(result: &FixResult, at_start: bool) {
    let points = connector_points(&result.report, "flow");
    let (end, inner) = if at_start {
        (points[0], points[1])
    } else {
        (points[points.len() - 1], points[points.len() - 2])
    };
    let length = ((end.x - inner.x).powi(2) + (end.y - inner.y).powi(2)).sqrt();
    let direction = ((end.x - inner.x) / length, (end.y - inner.y) / length);
    let written = Written::parse(&result.svg);
    let head: Vec<f64> = attr(written.nth("polygon", 0), "points")
        .split([' ', ','])
        .map(|value| value.parse().expect("a number"))
        .collect();
    let tip = point(head[2], head[3]);
    let base = point((head[0] + head[4]) / 2.0, (head[1] + head[5]) / 2.0);
    let shaft = attr(written.nth("path", 0), "d");
    let base_text = format!("{} {}", base.x, base.y);

    assert_eq!(tip, end);
    assert_eq!(
        (tip.x - base.x, tip.y - base.y),
        (direction.0 * 10.0, direction.1 * 10.0)
    );
    assert!(
        if at_start {
            shaft.starts_with(&format!("M {base_text}"))
        } else {
            shaft.ends_with(&base_text)
        },
        "{shaft}"
    );
}

/// Given an arrow whose head enters the target's right side heading left,
///   drawn around the top through an unrelated node
/// When the diagram is fixed
/// Then the connector enters the facing left side instead, and the head
///   turns round to point right along it
#[test]
fn turns_a_polygon_arrowhead_round_when_it_enters_from_the_other_side() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="-100 -100 700 300">
        <g data-node="source">
          <rect x="20" y="20" width="100" height="56" />
          <text x="70" y="53" text-anchor="middle" font-size="14">Source</text>
        </g>
        <g data-node="obstacle">
          <rect x="200" y="-30" width="60" height="40" />
          <text x="230" y="-5" text-anchor="middle" font-size="14">Block</text>
        </g>
        <g data-node="target">
          <rect x="300" y="20" width="100" height="56" />
          <text x="350" y="53" text-anchor="middle" font-size="14">Target</text>
        </g>
        <g id="flow" data-from="source" data-to="target">
          <path d="M 120 48 L 140 48 L 140 0 L 420 0 L 420 48 L 410 48" />
          <polygon points="410,53 400,48 410,43" />
        </g>
      </svg>
    "#;

    let result = fix(svg);
    let points = connector_points(&result.report, "flow");

    assert!(points[points.len() - 1].x > points[points.len() - 2].x, "{points:?}");
    assert_head_points_along_its_end(&result, false);
    assert!(result.report.issues.is_empty(), "{:?}", result.report.issues);

    assert_fix_snapshots!(result);
}

/// Given an arrow whose head is at the start of its path, pointing back
///   into the source, with the path running straight through a node
/// When the diagram is fixed
/// Then the head moves to the new start and points into the source along
///   the first segment
#[test]
fn turns_a_polygon_arrowhead_at_the_start_of_its_connector() {
    let svg = &blocked_row("").replace(
        r#"<line id="flow" data-from="source" data-to="target"
      x1="120" y1="48" x2="260" y2="48" />"#,
        r#"<g id="flow" data-from="source" data-to="target">
      <polygon points="130,53 120,48 130,43" />
      <path d="M 130 48 L 260 48" />
    </g>"#,
    );

    let result = fix(svg);

    assert!(bend_count(&connector_points(&result.report, "flow")) > 0);
    assert_head_points_along_its_end(&result, true);
    assert!(result.report.issues.is_empty(), "{:?}", result.report.issues);

    assert_fix_snapshots!(result);
}

/// Given a connector from a circle that comes straight down onto the middle
///   of a box's left side, its last stretch lying on that side
/// When the diagram is fixed
/// Then the connector is rerouted to meet the box at a right angle, and
///   nothing is reported
#[test]
fn reroutes_a_connector_end_that_runs_along_its_node_s_side() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 400 300">
        <g data-node="checkout"><circle cx="100" cy="70" r="50"/><text x="100" y="75" text-anchor="middle" font-size="13">Check out</text></g>
        <g data-node="orders"><rect x="100" y="200" width="120" height="40"/><text x="160" y="225" text-anchor="middle" font-size="13">Orders</text></g>
        <path id="order" data-from="checkout" data-to="orders" d="M 100 120 L 100 220"/>
      </svg>
    "#;

    let result = fix(svg);
    let points = connector_points(&result.report, "order");
    let orders = node_bounds(&result.report, "orders");

    assert_eq!(issue_codes(&analyze(svg)), ["connector-end-along-side"]);
    assert!(enters_perpendicularly(&points, &orders), "{points:?}");
    assert!(result.report.issues.is_empty(), "{:?}", result.report.issues);

    assert_fix_snapshots!(result);
}

/// Given a line without a fill, which SVG never fills, that has to detour
/// When the diagram is fixed
/// Then the bent path that replaces it is not filled either, while
///   its other attributes carry over
#[test]
fn keeps_a_rerouted_line_unfilled_once_it_becomes_a_path() {
    let svg = &blocked_row("");

    let result = fix(svg);
    let written = Written::parse(&result.svg);
    let (tag, path) = written.by_id("flow");

    assert_eq!(tag, "path");
    assert_eq!(attr(path, "fill"), "none");
    assert_eq!(attr(path, "data-from"), "source");

    assert_fix_snapshots!(result);
}

/// Given two services inside a cluster container joined by a straight line,
///   and a straight line from one of them to a node outside the cluster
/// When the diagram is fixed
/// Then both become orthogonal routes meeting their ends at right angles,
///   without treating the cluster around them as an obstacle
#[test]
fn routes_connectors_inside_and_out_of_a_container() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 620 260">
        <g data-node="cluster">
          <rect x="20" y="20" width="420" height="200" />
          <text x="40" y="45" font-size="14">Cluster</text>
          <g data-node="a">
            <rect x="50" y="80" width="100" height="56" />
            <text x="100" y="113" text-anchor="middle" font-size="14">A</text>
          </g>
          <g data-node="b">
            <rect x="300" y="140" width="100" height="56" />
            <text x="350" y="173" text-anchor="middle" font-size="14">B</text>
          </g>
        </g>
        <g data-node="ext">
          <rect x="480" y="80" width="100" height="56" />
          <text x="530" y="113" text-anchor="middle" font-size="14">Ext</text>
        </g>
        <line id="ab" data-from="a" data-to="b" x1="100" y1="108" x2="350" y2="168" />
        <line id="bx" data-from="b" data-to="ext" x1="350" y1="168" x2="530" y2="108" />
      </svg>
    "#;

    let result = fix(svg);

    for (id, from, to) in [("ab", "a", "b"), ("bx", "b", "ext")] {
        let points = connector_points(&result.report, id);
        assert!(is_orthogonal(&points), "{id} is orthogonal");
        assert!(
            leaves_perpendicularly(&points, &node_bounds(&result.report, from)),
            "{id} leaves"
        );
        assert!(
            enters_perpendicularly(&points, &node_bounds(&result.report, to)),
            "{id} enters"
        );
    }
    assert!(result.report.issues.is_empty());

    assert_fix_snapshots!(result);
}

/// Given two services inside a cluster container joined by a straight line
///   whose tied label sits at the line's midpoint
/// When the diagram is fixed
/// Then the label moves beside the rerouted connector, inside the cluster,
///   and nothing is reported
#[test]
fn places_labels_of_connectors_inside_a_container() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 480 260">
        <g data-node="cluster">
          <rect x="20" y="20" width="420" height="200" />
          <text x="40" y="45" font-size="14">Cluster</text>
          <g data-node="a">
            <rect x="50" y="80" width="100" height="56" />
            <text x="100" y="113" text-anchor="middle" font-size="14">A</text>
          </g>
          <g data-node="b">
            <rect x="300" y="140" width="100" height="56" />
            <text x="350" y="173" text-anchor="middle" font-size="14">B</text>
          </g>
        </g>
        <line id="ab" data-from="a" data-to="b" x1="100" y1="108" x2="350" y2="168" />
        <text data-label="calls" data-label-for="ab" x="225" y="142" text-anchor="middle"
          font-size="12">gRPC</text>
      </svg>
    "#;

    let result = fix(svg);
    let label = label_bounds(&result.report, "calls");

    assert!(distance_to_route(&label, &connector_points(&result.report, "ab")) <= 16.0);
    assert!(contains(&node_bounds(&result.report, "cluster"), &label));
    assert!(result.report.issues.is_empty());

    assert_fix_snapshots!(result);
}

/// Given a connector that must detour around a node between its ends
/// When the diagram is fixed
/// Then it leaves the source side and enters the target side at right
///   angles, and no segment runs along a side of either node
#[test]
fn leaves_and_enters_node_sides_perpendicularly_when_detouring() {
    let svg = &blocked_row("");

    let result = fix(svg);
    let points = connector_points(&result.report, "flow");
    let source = node_bounds(&result.report, "source");
    let target = node_bounds(&result.report, "target");

    assert!(leaves_perpendicularly(&points, &source));
    assert!(enters_perpendicularly(&points, &target));
    assert!(!runs_along(&points, &source));
    assert!(!runs_along(&points, &target));
    assert!(result.report.issues.is_empty());

    assert_fix_snapshots!(result);
}

/// Given a connector from inside a circular node to a facing box whose
///   middle sits so much lower than the circle's centre that the port on
///   the circle's side moves down to meet it
/// When the diagram is fixed
/// Then the connector stays orthogonal and starts on the circle itself,
///   not on its bounding box
#[test]
fn ends_a_rerouted_connector_on_a_circular_node_s_circle() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 400 200">
        <g data-node="start">
          <circle cx="70" cy="80" r="30" />
          <text x="70" y="85" text-anchor="middle" font-size="14">Go</text>
        </g>
        <g data-node="target">
          <rect x="240" y="73" width="100" height="56" />
          <text x="290" y="106" text-anchor="middle" font-size="14">Target</text>
        </g>
        <line id="flow" data-from="start" data-to="target"
          x1="70" y1="80" x2="290" y2="101" />
      </svg>
    "#;

    let result = fix(svg);
    let points = connector_points(&result.report, "flow");

    assert!(is_orthogonal(&points));
    assert!(distance_from_circle(points[0], &node_bounds(&result.report, "start")) <= 0.5);
    assert!(enters_perpendicularly(&points, &node_bounds(&result.report, "target")));
    assert!(result.report.issues.is_empty());

    assert_fix_snapshots!(result);
}

/// Given two connectors from inside a circular node to two boxes stacked
///   on its right
/// When the diagram is fixed
/// Then both leave the circle's right half from separate points on the
///   circle, without overlapping
#[test]
fn spreads_connectors_on_one_side_of_a_circle_each_ending_on_it() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 400 240">
        <g data-node="hub">
          <circle cx="80" cy="120" r="40" />
          <text x="80" y="125" text-anchor="middle" font-size="14">Hub</text>
        </g>
        <g data-node="upper">
          <rect x="240" y="60" width="100" height="56" />
          <text x="290" y="93" text-anchor="middle" font-size="14">Upper</text>
        </g>
        <g data-node="lower">
          <rect x="240" y="136" width="100" height="56" />
          <text x="290" y="169" text-anchor="middle" font-size="14">Lower</text>
        </g>
        <line id="to-upper" data-from="hub" data-to="upper"
          x1="80" y1="120" x2="290" y2="88" />
        <line id="to-lower" data-from="hub" data-to="lower"
          x1="80" y1="120" x2="290" y2="164" />
      </svg>
    "#;

    let result = fix(svg);
    let hub = node_bounds(&result.report, "hub");
    let upper = connector_points(&result.report, "to-upper");
    let lower = connector_points(&result.report, "to-lower");

    for route in [&upper, &lower] {
        assert!(is_orthogonal(route));
        assert!(distance_from_circle(route[0], &hub) <= 0.5);
        assert!(route[0].x > hub.x + hub.width / 2.0);
    }
    assert_ne!(upper[0], lower[0]);
    assert!(!overlaps_route(&upper, &lower));
    assert!(result.report.issues.is_empty());

    assert_fix_snapshots!(result);
}

/// Given a connector whose ends sit inside two nodes too far apart in
///   height to share a port coordinate
/// When the diagram is fixed
/// Then it becomes an orthogonal path meeting both nodes at right angles
#[test]
fn reroutes_between_offset_ports_orthogonally_instead_of_diagonally() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="-100 -100 600 300">
        <g data-node="source">
          <rect x="20" y="20" width="100" height="56" />
          <text x="70" y="53" text-anchor="middle" font-size="14">Source</text>
        </g>
        <g data-node="target">
          <rect x="260" y="60" width="100" height="56" />
          <text x="310" y="93" text-anchor="middle" font-size="14">Target</text>
        </g>
        <line id="flow" data-from="source" data-to="target"
          x1="100" y1="48" x2="280" y2="88" />
      </svg>
    "#;

    let result = fix(svg);
    let points = connector_points(&result.report, "flow");

    assert!(is_orthogonal(&points));
    assert!(leaves_perpendicularly(&points, &node_bounds(&result.report, "source")));
    assert!(enters_perpendicularly(&points, &node_bounds(&result.report, "target")));
    assert!(result.report.issues.is_empty());

    assert_fix_snapshots!(result);
}

/// Given a tall source facing a target whose left side midpoint sits at a
///   different height from the source's right side midpoint
/// When the diagram is fixed
/// Then the connector to the facing target is a single straight segment
#[test]
fn aligns_ports_on_facing_sides_so_the_connector_stays_straight() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="-100 -100 600 400">
        <g data-node="source">
          <rect x="20" y="20" width="100" height="90" />
          <text x="70" y="70" text-anchor="middle" font-size="14">Source</text>
        </g>
        <g data-node="upper">
          <rect x="260" y="20" width="100" height="56" />
          <text x="310" y="53" text-anchor="middle" font-size="14">Upper</text>
        </g>
        <g data-node="lower">
          <rect x="260" y="140" width="100" height="56" />
          <text x="310" y="173" text-anchor="middle" font-size="14">Lower</text>
        </g>
        <line id="to-upper" data-from="source" data-to="upper"
          x1="100" y1="50" x2="280" y2="48" />
        <line id="to-lower" data-from="source" data-to="lower"
          x1="100" y1="80" x2="280" y2="168" />
      </svg>
    "#;

    let result = fix(svg);
    let points = connector_points(&result.report, "to-upper");

    assert_eq!(points.len(), 2);
    assert!(is_orthogonal(&points));
    assert_eq!(
        side_of(points[0], &node_bounds(&result.report, "source")),
        Some(Side::Right)
    );
    assert_eq!(
        side_of(points[1], &node_bounds(&result.report, "upper")),
        Some(Side::Left)
    );
    assert!(result.report.issues.is_empty());

    assert_fix_snapshots!(result);
}

/// Given a target whose bottom is blocked by a node right below it, so
///   its left side takes two connectors, and a facing source whose port
///   would land 9px from the target's other port
/// When the diagram is fixed
/// Then the source port moves instead, and the connector stays straight
#[test]
fn keeps_aligned_ports_at_least_10px_from_other_ports_on_the_side() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="-100 -100 600 400">
        <g data-node="source">
          <rect x="20" y="20" width="100" height="56" />
          <text x="70" y="53" text-anchor="middle" font-size="14">Source</text>
        </g>
        <g data-node="other">
          <rect x="20" y="140" width="100" height="56" />
          <text x="70" y="173" text-anchor="middle" font-size="14">Other</text>
        </g>
        <g data-node="target">
          <rect x="260" y="20" width="100" height="56" />
          <text x="310" y="53" text-anchor="middle" font-size="14">Target</text>
        </g>
        <g data-node="floor">
          <rect x="260" y="91" width="100" height="56" />
          <text x="310" y="124" text-anchor="middle" font-size="14">Floor</text>
        </g>
        <line id="facing" data-from="source" data-to="target"
          x1="100" y1="48" x2="280" y2="48" />
        <line id="climbing" data-from="other" data-to="target"
          x1="100" y1="168" x2="280" y2="60" />
      </svg>
    "#;

    let result = fix(svg);
    let target = node_bounds(&result.report, "target");
    let facing = connector_points(&result.report, "facing");
    let facing_end = *facing.last().expect("facing end");
    let climbing_end = *connector_points(&result.report, "climbing")
        .last()
        .expect("climbing end");

    assert_eq!(facing.len(), 2);
    assert!(is_orthogonal(&facing));
    assert_eq!(side_of(facing_end, &target), Some(Side::Left));
    assert_eq!(side_of(climbing_end, &target), Some(Side::Left));
    assert!((facing_end.y - climbing_end.y).abs() >= 10.0);
    assert!(result.report.issues.is_empty());

    assert_fix_snapshots!(result);
}

/// Given a target below and to the right, where right-to-left ports need
///   a two-bend jog
/// When the diagram is fixed
/// Then the connector reaches the target with a single bend
#[test]
fn switches_to_another_side_when_it_saves_bends() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="-100 -100 600 400">
        <g data-node="source">
          <rect x="20" y="20" width="100" height="56" />
          <text x="70" y="53" text-anchor="middle" font-size="14">Source</text>
        </g>
        <g data-node="target">
          <rect x="260" y="160" width="100" height="56" />
          <text x="310" y="193" text-anchor="middle" font-size="14">Target</text>
        </g>
        <line id="flow" data-from="source" data-to="target"
          x1="100" y1="48" x2="300" y2="180" />
      </svg>
    "#;

    let result = fix(svg);
    let points = connector_points(&result.report, "flow");

    assert!(is_orthogonal(&points));
    assert_eq!(bend_count(&points), 1);
    assert!(leaves_perpendicularly(&points, &node_bounds(&result.report, "source")));
    assert!(enters_perpendicularly(&points, &node_bounds(&result.report, "target")));
    assert!(result.report.issues.is_empty());

    assert_fix_snapshots!(result);
}

/// Given eight nodes from a data flow diagram where the fewest-bend routes
///   would bunch several connectors onto the same node sides, leaving one
///   label no slot clear of them
/// When the diagram is fixed
/// Then the connectors spread over less crowded sides and every label sits
///   clear of every connector
#[test]
fn keeps_connectors_off_crowded_sides_so_every_label_finds_a_slot() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 2306 1060">
        <g data-node="aml">
          <rect x="1258" y="40" width="196" height="60" rx="14"/>
          <text x="1356" y="66" font-size="13" text-anchor="middle">9 AML Screening</text>
          <text x="1356" y="85" font-size="11" text-anchor="middle">fuzzy name match</text>
        </g>
        <g data-node="tfront">
          <rect x="40" y="270" width="196" height="60" rx="14"/>
          <text x="138" y="296" font-size="13" text-anchor="middle">3 Teller Front</text>
          <text x="138" y="315" font-size="11" text-anchor="middle">VB6 · 2006</text>
        </g>
        <g data-node="domadp">
          <rect x="2070" y="270" width="196" height="60" rx="14"/>
          <text x="2168" y="296" font-size="13" text-anchor="middle">17 Zengin Adapter</text>
          <text x="2168" y="315" font-size="11" text-anchor="middle">C · 2006</text>
        </g>
        <g data-node="partneradp">
          <rect x="1664" y="730" width="196" height="60" rx="14"/>
          <text x="1762" y="756" font-size="13" text-anchor="middle">24 Partner Adapter</text>
          <text x="1762" y="775" font-size="11" text-anchor="middle">REST + SFTP</text>
        </g>
        <g data-node="regrep">
          <rect x="2070" y="730" width="196" height="60" rx="14"/>
          <text x="2168" y="756" font-size="13" text-anchor="middle">25 Reg Reporting</text>
          <text x="2168" y="775" font-size="11" text-anchor="middle">STR / CTR export</text>
        </g>
        <g data-node="ledger">
          <rect x="852" y="960" width="196" height="60"/>
          <text x="964" y="986" font-size="13" text-anchor="middle">D4 Transfer Ledger</text>
          <text x="964" y="1005" font-size="11" text-anchor="middle">Oracle · 2006</text>
        </g>
        <g data-node="mq">
          <rect x="1664" y="960" width="196" height="60"/>
          <text x="1776" y="986" font-size="13" text-anchor="middle">D6 Message Queue</text>
          <text x="1776" y="1005" font-size="11" text-anchor="middle">IBM MQ + Kafka</text>
        </g>
        <g data-node="audit">
          <rect x="2070" y="960" width="196" height="60"/>
          <text x="2182" y="986" font-size="13" text-anchor="middle">D7 Audit Log</text>
          <text x="2182" y="1005" font-size="11" text-anchor="middle">WORM storage</text>
        </g>
        <path id="f57" data-from="partneradp" data-to="mq" d="M 1762 790 L 1762 960"/>
        <path id="f58" data-from="domadp" data-to="mq" d="M 2168 330 L 1762 960"/>
        <path id="f70" data-from="ledger" data-to="regrep" d="M 1048 990 L 2070 760"/>
        <path id="f71" data-from="aml" data-to="regrep" d="M 1454 70 L 2070 760"/>
        <path id="f75" data-from="aml" data-to="audit" d="M 1356 100 L 2168 960"/>
        <path id="f76" data-from="tfront" data-to="audit" d="M 236 300 L 2070 990"/>
        <text data-label="l57" data-label-for="f57" x="1762" y="869" font-size="12" text-anchor="middle">status event</text>
        <text data-label="l58" data-label-for="f58" x="1965" y="639" font-size="12" text-anchor="middle">status event</text>
        <text data-label="l70" data-label-for="f70" x="1559" y="869" font-size="12" text-anchor="middle">transactions</text>
        <text data-label="l71" data-label-for="f71" x="1762" y="409" font-size="12" text-anchor="middle">STR candidates</text>
        <text data-label="l75" data-label-for="f75" x="1762" y="524" font-size="12" text-anchor="middle">screening log</text>
        <text data-label="l76" data-label-for="f76" x="1153" y="639" font-size="12" text-anchor="middle">teller log</text>
      </svg>
    "#;

    let result = fix(svg);

    for connector in &result.report.diagram.connectors {
        assert!(is_orthogonal(&connector.points));
    }
    assert!(!has_issue(&result.report, "connector-label-clearance"));
    assert!(result.report.issues.is_empty());

    assert_fix_snapshots!(result);
}

/// Given nine nodes from a data flow diagram where one pass routes some
///   connectors before the label they later pass too close to is placed
/// When the diagram is fixed
/// Then a further pass routes them clear of that label, so fewer issues
///   remain than after a single pass, here none
#[test]
fn fixes_again_while_another_pass_leaves_fewer_issues() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1494 1060">
        <g data-node="aml">
          <rect x="40" y="40" width="196" height="60" rx="14"/>
          <text x="138" y="66" font-size="13" text-anchor="middle">9 AML Screening</text>
          <text x="138" y="85" font-size="11" text-anchor="middle">fuzzy name match</text>
        </g>
        <g data-node="fx">
          <rect x="446" y="40" width="196" height="60" rx="14"/>
          <text x="544" y="66" font-size="13" text-anchor="middle">10 FX Quote</text>
          <text x="544" y="85" font-size="11" text-anchor="middle">spread by segment</text>
        </g>
        <g data-node="fee">
          <rect x="40" y="270" width="196" height="60" rx="14"/>
          <text x="138" y="296" font-size="13" text-anchor="middle">11 Fee Calc</text>
          <text x="138" y="315" font-size="11" text-anchor="middle">legacy + new, diffed</text>
        </g>
        <g data-node="route">
          <rect x="446" y="270" width="196" height="60" rx="14"/>
          <text x="544" y="296" font-size="13" text-anchor="middle">16 Routing Engine</text>
          <text x="544" y="315" font-size="11" text-anchor="middle">rules in DB + code</text>
        </g>
        <g data-node="domadp">
          <rect x="852" y="270" width="196" height="60" rx="14"/>
          <text x="950" y="296" font-size="13" text-anchor="middle">17 Zengin Adapter</text>
          <text x="950" y="315" font-size="11" text-anchor="middle">C · 2006</text>
        </g>
        <g data-node="mxconv">
          <rect x="446" y="500" width="196" height="60" rx="14"/>
          <text x="544" y="526" font-size="13" text-anchor="middle">20 MT103 to MX</text>
          <text x="544" y="545" font-size="11" text-anchor="middle">ISO 20022 · 2023</text>
        </g>
        <g data-node="partneradp">
          <rect x="446" y="730" width="196" height="60" rx="14"/>
          <text x="544" y="756" font-size="13" text-anchor="middle">24 Partner Adapter</text>
          <text x="544" y="775" font-size="11" text-anchor="middle">REST + SFTP</text>
        </g>
        <g data-node="partner">
          <rect x="1258" y="730" width="196" height="60"/>
          <text x="1356" y="756" font-size="13" text-anchor="middle">Partner Remitter</text>
          <text x="1356" y="775" font-size="11" text-anchor="middle">12 corridors</text>
        </g>
        <g data-node="ratefee">
          <rect x="40" y="960" width="196" height="60"/>
          <text x="152" y="986" font-size="13" text-anchor="middle">D5 Rate / Fee Tables</text>
          <text x="152" y="1005" font-size="11" text-anchor="middle">Redis + Oracle</text>
        </g>
        <path id="f35" data-from="aml" data-to="route" d="M 236 70 L 446 300"/>
        <path id="f37" data-from="fx" data-to="ratefee" d="M 544 100 L 138 960"/>
        <path id="f42" data-from="fx" data-to="fee" d="M 446 70 L 236 300"/>
        <path id="f44" data-from="fee" data-to="route" d="M 236 300 L 446 300"/>
        <path id="f46" data-from="route" data-to="domadp" d="M 642 300 L 852 300"/>
        <path id="f47" data-from="route" data-to="mxconv" d="M 544 330 L 544 500"/>
        <path id="f48" data-from="route" data-to="partneradp" d="M 544 330 L 544 730"/>
        <path id="f56" data-from="partner" data-to="partneradp" d="M 1258 760 L 642 760"/>
        <text data-label="l35" data-label-for="f35" x="341" y="179" font-size="12" text-anchor="middle">cleared order</text>
        <text data-label="l37" data-label-for="f37" x="341" y="524" font-size="12" text-anchor="middle">cached rate</text>
        <text data-label="l42" data-label-for="f42" x="341" y="179" font-size="12" text-anchor="middle">quote</text>
        <text data-label="l44" data-label-for="f44" x="341" y="294" font-size="12" text-anchor="middle">priced order</text>
        <text data-label="l46" data-label-for="f46" x="747" y="294" font-size="12" text-anchor="middle">domestic</text>
        <text data-label="l47" data-label-for="f47" x="544" y="409" font-size="12" text-anchor="middle">MT103</text>
        <text data-label="l48" data-label-for="f48" x="544" y="524" font-size="12" text-anchor="middle">corridor</text>
        <text data-label="l56" data-label-for="f56" x="950" y="754" font-size="12" text-anchor="middle">payout confirm</text>
      </svg>
    "#;

    let single = fix_with_passes(svg, 1);
    let result = fix(svg);

    assert!(!single.report.issues.is_empty());
    assert!(result.report.issues.len() < single.report.issues.len());
    assert!(result.report.issues.is_empty());

    assert_fix_snapshots!(result);
}

/// Given a label tied to a connector by data-label-for, left at the old
///   midpoint after the connector has to detour
/// When the diagram is fixed
/// Then the label sits centred above the detour's longest segment
#[test]
fn moves_a_connector_s_label_beside_its_rerouted_path() {
    let svg = &blocked_row(RETRY_LABEL);

    let result = fix(svg);
    let label = label_bounds(&result.report, "retry");
    let (from, to) = longest_segment(&connector_points(&result.report, "flow"));

    assert_eq!(from.y, to.y);
    assert!((label.x + label.width / 2.0 - (from.x + to.x) / 2.0).abs() < 0.5);
    assert!(label.y + label.height <= from.y - 8.0);
    assert!(change_codes(&result).contains(&"move-label".to_owned()));
    assert!(result.report.issues.is_empty());

    assert_fix_snapshots!(result);
}

/// Given a straight connector whose tied label floats 45px above it
/// When the diagram is fixed
/// Then the label sits beside the connector and nothing is reported
#[test]
fn brings_a_detached_label_back_beside_its_connector() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="-100 -100 600 300">
        <g data-node="source">
          <rect x="20" y="20" width="100" height="56" />
          <text x="70" y="53" text-anchor="middle" font-size="14">Source</text>
        </g>
        <g data-node="target">
          <rect x="260" y="20" width="100" height="56" />
          <text x="310" y="53" text-anchor="middle" font-size="14">Target</text>
        </g>
        <line id="flow" data-from="source" data-to="target"
          x1="120" y1="48" x2="260" y2="48" />
        <text data-label="retry" data-label-for="flow"
          x="190" y="0" text-anchor="middle" font-size="14">Retry</text>
      </svg>
    "#;

    let result = fix(svg);

    let distance = distance_to_route(
        &label_bounds(&result.report, "retry"),
        &connector_points(&result.report, "flow"),
    );
    assert!(distance <= 16.0);
    assert!(result.report.issues.is_empty());

    assert_fix_snapshots!(result);
}

/// Given a connector drawn as a sound two-bend path whose tied label
///   floats far from it, with nothing else wrong
/// When the diagram is fixed
/// Then the connector keeps its path and only the label moves beside it
#[test]
fn moves_only_a_detached_label_leaving_its_sound_connector_as_drawn() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="-100 -100 600 400">
        <g data-node="source">
          <rect x="20" y="20" width="100" height="56" />
          <text x="70" y="53" text-anchor="middle" font-size="14">Source</text>
        </g>
        <g data-node="target">
          <rect x="260" y="160" width="100" height="56" />
          <text x="310" y="193" text-anchor="middle" font-size="14">Target</text>
        </g>
        <path id="flow" data-from="source" data-to="target"
          d="M 120 48 L 190 48 L 190 188 L 260 188" />
        <text data-label="retry" data-label-for="flow"
          x="310" y="0" text-anchor="middle" font-size="14">Retry</text>
      </svg>
    "#;

    let result = fix(svg);
    let points = connector_points(&result.report, "flow");

    assert_eq!(points, connector_points(&analyze(svg), "flow"));
    assert!(distance_to_route(&label_bounds(&result.report, "retry"), &points) <= 16.0);
    assert_eq!(change_codes(&result), ["move-label"]);
    assert!(result.report.issues.is_empty());

    assert_fix_snapshots!(result);
}

/// Given a straight connector whose tied label sits close to it but on top
///   of a node beside the line
/// When the diagram is fixed
/// Then the connector keeps its path and only the label moves to a free
///   spot beside it, off the node
#[test]
fn moves_a_label_off_a_node_to_a_free_spot_beside_its_connector() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 -40 440 180">
        <g data-node="source">
          <rect x="20" y="20" width="100" height="56" />
          <text x="70" y="53" text-anchor="middle" font-size="14">Source</text>
        </g>
        <g data-node="target">
          <rect x="300" y="20" width="100" height="56" />
          <text x="350" y="53" text-anchor="middle" font-size="14">Target</text>
        </g>
        <g data-node="note">
          <rect x="150" y="62" width="120" height="48" />
          <text x="210" y="91" text-anchor="middle" font-size="14">Note</text>
        </g>
        <line id="flow" data-from="source" data-to="target" x1="120" y1="48" x2="300" y2="48" />
        <text data-label="retry" data-label-for="flow"
          x="210" y="74" text-anchor="middle" font-size="14">Retry</text>
      </svg>
    "#;

    let result = fix(svg);
    let points = connector_points(&result.report, "flow");

    assert_eq!(issue_codes(&analyze(svg)), ["label-node-overlap"]);
    assert_eq!(points, connector_points(&analyze(svg), "flow"));
    assert!(distance_to_route(&label_bounds(&result.report, "retry"), &points) <= 16.0);
    assert_eq!(change_codes(&result), ["move-label"]);
    assert!(result.report.issues.is_empty());

    assert_fix_snapshots!(result);
}

/// Given a sound connector whose detached label sets its anchor in the
///   style attribute
/// When the diagram is fixed
/// Then the label sits beside its connector with the anchor it was placed
///   with, so nothing is reported
#[test]
fn moves_a_label_whose_anchor_is_set_in_its_style() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="-100 -100 600 400">
        <g data-node="source">
          <rect x="20" y="20" width="100" height="56" />
          <text x="70" y="53" text-anchor="middle" font-size="14">Source</text>
        </g>
        <g data-node="target">
          <rect x="260" y="160" width="100" height="56" />
          <text x="310" y="193" text-anchor="middle" font-size="14">Target</text>
        </g>
        <path id="flow" data-from="source" data-to="target"
          d="M 120 48 L 190 48 L 190 188 L 260 188" />
        <text data-label="retry" data-label-for="flow"
          x="310" y="0" style="text-anchor: middle; font-size: 14px">Retry after</text>
      </svg>
    "#;

    let result = fix(svg);
    let points = connector_points(&result.report, "flow");

    assert_eq!(points, connector_points(&analyze(svg), "flow"));
    assert!(distance_to_route(&label_bounds(&result.report, "retry"), &points) <= 16.0);
    assert!(result.report.issues.is_empty(), "{:?}", result.report.issues);

    assert_fix_snapshots!(result);
}

/// Given a straight connector whose detached label has a central baseline
/// When the diagram is fixed
/// Then the label moves beside the connector, keeping its clearance, and
///   nothing is reported
#[test]
fn moves_a_label_with_a_central_baseline_beside_its_connector() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="-100 -100 600 300">
        <g data-node="source">
          <rect x="20" y="20" width="100" height="56" />
          <text x="70" y="53" text-anchor="middle" font-size="14">Source</text>
        </g>
        <g data-node="target">
          <rect x="300" y="20" width="100" height="56" />
          <text x="350" y="53" text-anchor="middle" font-size="14">Target</text>
        </g>
        <line id="flow" data-from="source" data-to="target" x1="120" y1="48" x2="300" y2="48" />
        <text data-label="retry" data-label-for="flow"
          x="210" y="150" text-anchor="middle" font-size="14" dominant-baseline="central">Retry</text>
      </svg>
    "#;

    let result = fix(svg);
    let points = connector_points(&result.report, "flow");

    assert_eq!(change_codes(&result), ["move-label"]);
    assert!(distance_to_route(&label_bounds(&result.report, "retry"), &points) <= 16.0);
    assert!(result.report.issues.is_empty(), "{:?}", result.report.issues);

    assert_fix_snapshots!(result);
}

/// A sound two-bend connector with a three-line label written by `label`.
fn connector_with_label(label: &str) -> String {
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="-100 -100 600 400">
          <g data-node="source">
            <rect x="20" y="20" width="100" height="56" />
            <text x="70" y="53" text-anchor="middle" font-size="14">Source</text>
          </g>
          <g data-node="target">
            <rect x="260" y="160" width="100" height="56" />
            <text x="310" y="193" text-anchor="middle" font-size="14">Target</text>
          </g>
          <path id="flow" data-from="source" data-to="target"
            d="M 120 48 L 190 48 L 190 188 L 260 188" />
          {label}
        </svg>"#
    )
}

/// Moving a label as a whole keeps the size of the box around its lines.
fn assert_moved_as_one_block(svg: &str, result: &FixResult) {
    let before = label_bounds(&analyze(svg), "retry");
    let after = label_bounds(&result.report, "retry");
    let points = connector_points(&result.report, "flow");

    assert_eq!(change_codes(result), ["move-label"]);
    assert_eq!((after.width, after.height), (before.width, before.height));
    assert!(distance_to_route(&after, &points) <= 16.0);
    assert!(result.report.issues.is_empty(), "{:?}", result.report.issues);
}

/// Given a detached label written as three texts sharing one data-label
/// When the diagram is fixed
/// Then all three lines move together beside the connector
#[test]
fn moves_every_text_of_a_label_split_across_texts() {
    let svg = connector_with_label(
        r#"<text data-label="retry" data-label-for="flow" x="310" y="-40" font-size="14">Retry</text>
          <text data-label="retry" data-label-for="flow" x="310" y="-23" font-size="14">with backoff</text>
          <text data-label="retry" data-label-for="flow" x="310" y="-6" font-size="14">3 times</text>"#,
    );

    let result = fix(&svg);

    assert_moved_as_one_block(&svg, &result);
    assert_fix_snapshots!(result);
}

/// Given a label written as one text with a tspan per line, whose lower
///   lines lie on the target node
/// When the diagram is fixed
/// Then all three lines move together beside the connector, off the node
#[test]
fn moves_every_tspan_line_of_a_label() {
    let svg = connector_with_label(
        r#"<text data-label="retry" data-label-for="flow" x="200" y="140" font-size="14">
            <tspan x="200">Retry</tspan>
            <tspan x="200" dy="17">with backoff</tspan>
            <tspan x="200" dy="17">3 times</tspan>
          </text>"#,
    );

    let result = fix(&svg);

    assert_eq!(issue_codes(&analyze(&svg)), ["label-node-overlap"]);
    assert_moved_as_one_block(&svg, &result);
    assert_fix_snapshots!(result);
}

/// Given a detached label written as a group carrying data-label around
///   three plain texts
/// When the diagram is fixed
/// Then all three lines move together beside the connector
#[test]
fn moves_every_text_of_a_labelled_group() {
    let svg = connector_with_label(
        r#"<g data-label="retry" data-label-for="flow">
            <text x="310" y="-40" font-size="14">Retry</text>
            <text x="310" y="-23" font-size="14">with backoff</text>
            <text x="310" y="-6" font-size="14">3 times</text>
          </g>"#,
    );

    let result = fix(&svg);

    assert_moved_as_one_block(&svg, &result);
    assert_fix_snapshots!(result);
}

/// Given a straight connector whose detached label would naturally go just
///   above it, where another connector runs 14px above that spot
/// When the diagram is fixed
/// Then the label goes below its connector instead, at least 20px from the
///   other one, so it is clear which connector it belongs to
#[test]
fn keeps_a_moved_label_clear_of_connectors_it_does_not_belong_to() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 -100 640 440">
        <g data-node="a1"><rect x="20" y="76" width="100" height="48"/><text x="70" y="105" text-anchor="middle" font-size="14">A1</text></g>
        <g data-node="a2"><rect x="500" y="76" width="100" height="48"/><text x="550" y="105" text-anchor="middle" font-size="14">A2</text></g>
        <g data-node="b1"><rect x="150" y="-60" width="100" height="48"/><text x="200" y="-31" text-anchor="middle" font-size="14">B1</text></g>
        <g data-node="b2"><rect x="400" y="-60" width="100" height="48"/><text x="450" y="-31" text-anchor="middle" font-size="14">B2</text></g>
        <line id="flow" data-from="a1" data-to="a2" x1="120" y1="100" x2="500" y2="100"/>
        <path id="other" data-from="b1" data-to="b2" d="M 200 -12 L 200 60 L 450 60 L 450 -12"/>
        <text data-label="retry" data-label-for="flow" x="310" y="300" text-anchor="middle" font-size="14">Retry</text>
      </svg>
    "#;

    let result = fix(svg);
    let label = label_bounds(&result.report, "retry");

    assert_eq!(change_codes(&result), ["move-label"]);
    assert!(distance_to_route(&label, &connector_points(&result.report, "flow")) <= 16.0);
    assert!(
        distance_to_route(&label, &connector_points(&result.report, "other")) >= 20.0,
        "{label:?}"
    );
    assert!(result.report.issues.is_empty(), "{:?}", result.report.issues);

    assert_fix_snapshots!(result);
}

/// Given two parallel vertical connectors 44px apart, where the left one's
///   label sits between them, 9px from its own line but only 14px from the
///   other, with free space on the far side of its own line
/// When the diagram is fixed
/// Then the label moves to where it is at least 20px from the other
///   connector, still beside its own
#[test]
fn moves_a_label_away_from_a_connector_it_does_not_belong_to() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 440 440">
        <g data-node="order"><rect x="160" y="20" width="160" height="48"/><text x="240" y="49" text-anchor="middle" font-size="14">Order</text></g>
        <g data-node="postgres"><rect x="100" y="360" width="120" height="48"/><text x="160" y="389" text-anchor="middle" font-size="14">PostgreSQL</text></g>
        <g data-node="kafka"><rect x="240" y="360" width="100" height="48"/><text x="290" y="389" text-anchor="middle" font-size="14">Kafka</text></g>
        <line id="sql" data-from="order" data-to="postgres" x1="200" y1="68" x2="200" y2="360"/>
        <line id="events" data-from="order" data-to="kafka" x1="244" y1="68" x2="244" y2="360"/>
        <text data-label="sql-label" data-label-for="sql" x="209" y="220" font-size="12">SQL</text>
      </svg>
    "#;

    let before = analyze(svg);
    let result = fix(svg);
    let label = label_bounds(&result.report, "sql-label");

    assert!(before.issues.is_empty(), "{:?}", before.issues);
    assert!(
        distance_to_route(
            &label_bounds(&before, "sql-label"),
            &connector_points(&before, "events")
        ) < 20.0
    );
    assert_eq!(change_codes(&result), ["move-label"]);
    assert!(distance_to_route(&label, &connector_points(&result.report, "sql")) <= 16.0);
    assert!(
        distance_to_route(&label, &connector_points(&result.report, "events")) >= 20.0,
        "{label:?}"
    );
    assert!(result.report.issues.is_empty(), "{:?}", result.report.issues);

    assert_fix_snapshots!(result);
}

/// Given a straight connector with another connector running parallel 10px
///   above most of it, and its label detached
/// When the diagram is fixed
/// Then the label sits below the middle of its connector: its own line lies
///   between it and the other connector, so it is clear which one it
///   belongs to, and the other connector does not push it towards an end
#[test]
fn places_a_label_beside_the_middle_when_a_parallel_connector_is_across_its_own() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 -100 640 440">
        <g data-node="a1"><rect x="20" y="76" width="100" height="48"/><text x="70" y="105" text-anchor="middle" font-size="14">A1</text></g>
        <g data-node="a2"><rect x="500" y="76" width="100" height="48"/><text x="550" y="105" text-anchor="middle" font-size="14">A2</text></g>
        <g data-node="b1"><rect x="150" y="-60" width="100" height="48"/><text x="200" y="-31" text-anchor="middle" font-size="14">B1</text></g>
        <g data-node="b2"><rect x="400" y="-60" width="100" height="48"/><text x="450" y="-31" text-anchor="middle" font-size="14">B2</text></g>
        <line id="flow" data-from="a1" data-to="a2" x1="120" y1="100" x2="500" y2="100"/>
        <path id="other" data-from="b1" data-to="b2" d="M 200 -12 L 200 90 L 450 90 L 450 -12"/>
        <text data-label="retry" data-label-for="flow" x="310" y="300" text-anchor="middle" font-size="14">Retry</text>
      </svg>
    "#;

    let result = fix(svg);
    let label = label_bounds(&result.report, "retry");
    let middle = 310.0;

    assert_eq!(change_codes(&result), ["move-label"]);
    assert!(distance_to_route(&label, &connector_points(&result.report, "flow")) <= 16.0);
    assert!(label.y > 100.0, "{label:?} is not below its connector");
    assert!(
        (label.x + label.width / 2.0 - middle).abs() <= 40.0,
        "{label:?} is pushed away from the middle"
    );
    assert!(result.report.issues.is_empty(), "{:?}", result.report.issues);

    assert_fix_snapshots!(result);
}

/// Given a long straight connector crossed through its middle by another
///   connector, with its label detached
/// When the diagram is fixed
/// Then the label moves beside its connector as far as it can from the
///   crossing and from the connector's ends: midway between the crossing
///   and one end, not beside the crossing nor at an arrowhead
#[test]
fn places_a_label_away_from_where_another_connector_crosses() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 -60 640 340">
        <g data-node="a1"><rect x="20" y="76" width="100" height="48"/><text x="70" y="105" text-anchor="middle" font-size="14">A1</text></g>
        <g data-node="a2"><rect x="520" y="76" width="100" height="48"/><text x="570" y="105" text-anchor="middle" font-size="14">A2</text></g>
        <g data-node="b1"><rect x="280" y="-40" width="100" height="48"/><text x="330" y="-11" text-anchor="middle" font-size="14">B1</text></g>
        <g data-node="b2"><rect x="280" y="190" width="100" height="48"/><text x="330" y="219" text-anchor="middle" font-size="14">B2</text></g>
        <line id="flow" data-from="a1" data-to="a2" x1="120" y1="100" x2="520" y2="100"/>
        <line id="cross" data-from="b1" data-to="b2" x1="330" y1="8" x2="330" y2="190"/>
        <text data-label="retry" data-label-for="flow" x="330" y="260" text-anchor="middle" font-size="14">Retry</text>
      </svg>
    "#;

    let result = fix(svg);
    let label = label_bounds(&result.report, "retry");
    let gap = |p: Point| {
        ((label.x - p.x).max(p.x - (label.x + label.width)).max(0.0))
            .hypot((label.y - p.y).max(p.y - (label.y + label.height)).max(0.0))
    };
    let nearest = [point(330.0, 100.0), point(120.0, 100.0), point(520.0, 100.0)]
        .into_iter()
        .map(gap)
        .fold(f64::INFINITY, f64::min);

    assert_eq!(change_codes(&result), ["move-label"]);
    assert!(distance_to_route(&label, &connector_points(&result.report, "flow")) <= 16.0);
    assert!(
        nearest >= 75.0,
        "{label:?} only {nearest}px from the crossing or an end"
    );
    assert!(result.report.issues.is_empty(), "{:?}", result.report.issues);

    assert_fix_snapshots!(result);
}

/// Given a straight connector with its label beside its middle, and another
///   connector cutting through a node that, once rerouted, crosses the
///   first one 65px from the label
/// When the diagram is fixed
/// Then the label moves to where it is furthest from the new crossing and
///   the connector's ends: the crossing it was placed against has changed
#[test]
fn places_a_label_again_when_the_crossings_on_its_connector_change() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 -60 660 320">
        <g data-node="a1"><rect x="20" y="76" width="100" height="48"/><text x="70" y="105" text-anchor="middle" font-size="14">A1</text></g>
        <g data-node="a2"><rect x="520" y="76" width="100" height="48"/><text x="570" y="105" text-anchor="middle" font-size="14">A2</text></g>
        <g data-node="b1"><rect x="205" y="-40" width="100" height="48"/><text x="255" y="-11" text-anchor="middle" font-size="14">B1</text></g>
        <g data-node="b2"><rect x="205" y="190" width="100" height="48"/><text x="255" y="219" text-anchor="middle" font-size="14">B2</text></g>
        <line id="flow" data-from="a1" data-to="a2" x1="120" y1="100" x2="520" y2="100"/>
        <path id="cross" data-from="b1" data-to="b2" d="M 255 8 L 255 40 L 570 40 L 570 214 L 305 214"/>
        <text data-label="retry" data-label-for="flow" x="320" y="125" text-anchor="middle" font-size="14">Retry</text>
      </svg>
    "#;

    let before = analyze(svg);
    let result = fix(svg);
    let flow = connector_points(&result.report, "flow");
    let label = label_bounds(&result.report, "retry");
    let gap = |p: Point| {
        ((label.x - p.x).max(p.x - (label.x + label.width)).max(0.0))
            .hypot((label.y - p.y).max(p.y - (label.y + label.height)).max(0.0))
    };
    let crossing = point(connector_points(&result.report, "cross")[0].x, 100.0);
    let nearest = [crossing, flow[0], flow[flow.len() - 1]]
        .into_iter()
        .map(gap)
        .fold(f64::INFINITY, f64::min);

    assert!(!crosses_route(
        &connector_points(&before, "flow"),
        &connector_points(&before, "cross")
    ));
    assert!(crosses_route(&flow, &connector_points(&result.report, "cross")));
    assert!(distance_to_route(&label, &flow) <= 16.0);
    assert!(
        nearest >= 75.0,
        "{label:?} only {nearest}px from the crossing or an end"
    );
    assert!(result.report.issues.is_empty(), "{:?}", result.report.issues);

    assert_fix_snapshots!(result);
}

/// Given two connectors entering the left of a node, one turning 1px beside
///   where the other turns, so the two read as one line turning
/// When the diagram is fixed
/// Then they are rerouted to keep apart, and nothing is reported
#[test]
fn reroutes_connectors_that_turn_just_beside_each_other() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="200 560 440 500">
        <g data-node="credit"><rect x="330" y="600" width="100" height="48"/><text x="380" y="629" text-anchor="middle" font-size="14">Credit</text></g>
        <g data-node="models"><rect x="230" y="940" width="130" height="40"/><text x="295" y="965" text-anchor="middle" font-size="14">Models</text></g>
        <g data-node="assess"><rect x="481" y="930" width="120" height="100"/><text x="541" y="985" text-anchor="middle" font-size="14">Assess</text></g>
        <path id="score" data-from="credit" data-to="assess" d="M 380 648 L 380 959 L 481 959"/>
        <path id="model" data-from="models" data-to="assess" d="M 360 960 L 379 960 L 379 1001 L 481 1001"/>
      </svg>
    "#;

    let before = analyze(svg);
    let result = fix(svg);

    assert!(has_issue(&before, "connector-near-miss"));
    assert!(is_orthogonal(&connector_points(&result.report, "score")));
    assert!(is_orthogonal(&connector_points(&result.report, "model")));
    assert_eq!(near_misses(&result.report), Vec::<(String, String)>::new());
    assert!(result.report.issues.is_empty(), "{:?}", result.report.issues);

    assert_fix_snapshots!(result);
}

/// Given two boxes side by side with two vertical connectors standing
///   between them, and a connector drawn between the boxes that needs
///   rerouting
/// When the diagram is fixed
/// Then the connector goes around the vertical ones rather than crossing
///   them: a detour of a few hundred pixels beats two crossings
#[test]
fn detours_around_connectors_rather_than_crossing_them() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 640 420">
        <g data-node="a"><rect x="20" y="176" width="100" height="48"/><text x="70" y="205" text-anchor="middle" font-size="14">A</text></g>
        <g data-node="b"><rect x="520" y="176" width="100" height="48"/><text x="570" y="205" text-anchor="middle" font-size="14">B</text></g>
        <g data-node="c1"><rect x="210" y="52" width="100" height="48"/><text x="260" y="81" text-anchor="middle" font-size="14">C1</text></g>
        <g data-node="d1"><rect x="210" y="300" width="100" height="48"/><text x="260" y="329" text-anchor="middle" font-size="14">D1</text></g>
        <g data-node="c2"><rect x="350" y="52" width="100" height="48"/><text x="400" y="81" text-anchor="middle" font-size="14">C2</text></g>
        <g data-node="d2"><rect x="350" y="300" width="100" height="48"/><text x="400" y="329" text-anchor="middle" font-size="14">D2</text></g>
        <path id="first" data-from="c1" data-to="d1" d="M 260 100 L 260 300"/>
        <path id="second" data-from="c2" data-to="d2" d="M 400 100 L 400 300"/>
        <line id="flow" data-from="a" data-to="b" x1="70" y1="200" x2="570" y2="200"/>
      </svg>
    "#;

    let result = fix(svg);
    let flow = connector_points(&result.report, "flow");

    assert!(is_orthogonal(&flow));
    assert!(
        !crosses_route(&flow, &connector_points(&result.report, "first")),
        "{flow:?}"
    );
    assert!(
        !crosses_route(&flow, &connector_points(&result.report, "second")),
        "{flow:?}"
    );
    assert!(result.report.issues.is_empty(), "{:?}", result.report.issues);

    assert_fix_snapshots!(result);
}

/// Given a box with two connectors leaving its left side for two nodes far
///   below it, past a column of nodes in between, so both run down the left
/// When the diagram is fixed
/// Then the two connectors do not cross: the one leaving lower turns down
///   inside the other
#[test]
fn trades_ports_on_a_side_so_two_connectors_leaving_it_do_not_cross() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 640 900">
        <g data-node="intake"><rect x="260" y="20" width="120" height="48"/><text x="320" y="49" text-anchor="middle" font-size="13">Intake</text></g>
        <g data-node="verify"><rect x="260" y="160" width="120" height="48"/><text x="320" y="189" text-anchor="middle" font-size="13">Verify</text></g>
        <g data-node="assess"><rect x="260" y="320" width="120" height="48"/><text x="320" y="349" text-anchor="middle" font-size="13">Assess</text></g>
        <g data-node="offer"><rect x="260" y="480" width="120" height="48"/><text x="320" y="509" text-anchor="middle" font-size="13">Offer</text></g>
        <g data-node="notify"><rect x="260" y="760" width="120" height="48"/><text x="320" y="789" text-anchor="middle" font-size="13">Notify</text></g>
        <g data-node="side"><rect x="440" y="20" width="120" height="48"/><text x="500" y="49" text-anchor="middle" font-size="13">Side</text></g>
        <path id="checks" data-from="intake" data-to="verify" d="M 320 68 L 320 160"/>
        <path id="scores" data-from="verify" data-to="assess" d="M 320 208 L 320 320"/>
        <path id="decision" data-from="assess" data-to="offer" d="M 320 368 L 320 480"/>
        <path id="details" data-from="offer" data-to="notify" d="M 320 528 L 320 760"/>
        <path id="aside" data-from="intake" data-to="side" d="M 380 44 L 440 44"/>
        <line id="terms" data-from="intake" data-to="offer" x1="320" y1="44" x2="320" y2="504"/>
        <line id="receipt" data-from="intake" data-to="notify" x1="320" y1="44" x2="320" y2="784"/>
      </svg>
    "#;

    let result = fix(svg);
    let terms = connector_points(&result.report, "terms");
    let receipt = connector_points(&result.report, "receipt");

    assert!(is_orthogonal(&terms) && is_orthogonal(&receipt));
    assert!(!crosses_route(&terms, &receipt), "{terms:?} crosses {receipt:?}");
    assert!(result.report.issues.is_empty(), "{:?}", result.report.issues);

    assert_fix_snapshots!(result);
}

/// Given a connector drawn as a staircase of segments all shorter than its
///   tied label, which floats far away
/// When the diagram is fixed
/// Then the connector is rerouted so the label can sit beside it, and
///   nothing is reported
#[test]
fn reroutes_a_connector_when_its_detached_label_has_no_room_beside_it() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="-100 -100 600 300">
        <g data-node="source">
          <rect x="20" y="20" width="100" height="56" />
          <text x="70" y="53" text-anchor="middle" font-size="14">Source</text>
        </g>
        <g data-node="target">
          <rect x="260" y="20" width="100" height="56" />
          <text x="310" y="53" text-anchor="middle" font-size="14">Target</text>
        </g>
        <path id="flow" data-from="source" data-to="target"
          d="M 120 48 L 150 48 L 150 60 L 180 60 L 180 36 L 210 36 L 210 60 L 240 60 L 240 48 L 260 48" />
        <text data-label="retry" data-label-for="flow"
          x="190" y="-40" text-anchor="middle" font-size="14">Retry</text>
      </svg>
    "#;

    let result = fix(svg);
    let points = connector_points(&result.report, "flow");

    assert!(is_orthogonal(&points));
    assert!(distance_to_route(&label_bounds(&result.report, "retry"), &points) <= 16.0);
    assert!(result.report.issues.is_empty());

    assert_fix_snapshots!(result);
}

/// Given a free note sitting where the moved label would go first
/// When the diagram is fixed
/// Then the label moves along the detour, at least 4px from the note,
///   and no issue remains
#[test]
fn keeps_a_moved_label_clear_of_other_labels() {
    let svg = &blocked_row(NOTE_AND_RETRY_LABEL);

    let result = fix(svg);

    assert_eq!(label_bounds(&result.report, "note"), bounds(173.5, -14.0, 33.0, 17.0));
    assert!(
        gap_between(
            &label_bounds(&result.report, "retry"),
            &label_bounds(&result.report, "note")
        ) >= 4.0
    );
    assert!(result.report.issues.is_empty());

    assert_fix_snapshots!(result);
}

/// Given a crossing connector whose shortest detour is taken by a bus line
/// When the diagram is fixed
/// Then the connector detours on the free side instead of overlapping
#[test]
fn does_not_reroute_a_connector_onto_another_connector() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="-100 -100 600 300">
        <g data-node="west">
          <rect x="-60" y="-10" width="60" height="40" />
          <text x="-30" y="15" text-anchor="middle" font-size="12">W</text>
        </g>
        <g data-node="source">
          <rect x="20" y="20" width="100" height="56" />
          <text x="70" y="53" text-anchor="middle" font-size="14">Source</text>
        </g>
        <g data-node="obstacle">
          <rect x="150" y="20" width="80" height="56" />
          <text x="190" y="53" text-anchor="middle" font-size="14">Block</text>
        </g>
        <g data-node="target">
          <rect x="260" y="20" width="100" height="56" />
          <text x="310" y="53" text-anchor="middle" font-size="14">Target</text>
        </g>
        <g data-node="east">
          <rect x="400" y="-10" width="60" height="40" />
          <text x="430" y="15" text-anchor="middle" font-size="12">E</text>
        </g>
        <line id="bus" data-from="west" data-to="east"
          x1="0" y1="12" x2="400" y2="12" />
        <line id="flow" data-from="source" data-to="target"
          x1="120" y1="48" x2="260" y2="48" />
      </svg>
    "#;

    let result = fix(svg);
    let bus = connector_points(&result.report, "bus");
    let flow = connector_points(&result.report, "flow");

    assert_eq!(bus, [point(0.0, 12.0), point(400.0, 12.0)]);
    assert!(is_orthogonal(&flow));
    assert!(!overlaps_route(&flow, &bus));
    assert!(result.report.issues.is_empty());

    assert_fix_snapshots!(result);
}

/// Given a short connector segment on the detour's shortest leg, lying
///   between two neighbouring grid lines
/// When the diagram is fixed
/// Then the detour takes the other side instead of overlapping it
#[test]
fn does_not_reroute_onto_a_short_segment_between_grid_lines() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="-100 -100 600 300">
        <g data-node="west">
          <rect x="-60" y="-60" width="60" height="40" />
          <text x="-30" y="-35" text-anchor="middle" font-size="12">W</text>
        </g>
        <g data-node="source">
          <rect x="20" y="20" width="100" height="56" />
          <text x="70" y="53" text-anchor="middle" font-size="14">Source</text>
        </g>
        <g data-node="obstacle">
          <rect x="150" y="20" width="80" height="56" />
          <text x="190" y="53" text-anchor="middle" font-size="14">Block</text>
        </g>
        <g data-node="target">
          <rect x="260" y="20" width="100" height="56" />
          <text x="310" y="53" text-anchor="middle" font-size="14">Target</text>
        </g>
        <path id="tick" data-from="west" data-to="target"
          d="M 0 -40 L 195 -40 L 195 12 L 200 12" />
        <line id="flow" data-from="source" data-to="target"
          x1="120" y1="48" x2="260" y2="48" />
      </svg>
    "#;

    let result = fix(svg);
    let flow = connector_points(&result.report, "flow");

    assert!(is_orthogonal(&flow));
    assert!(!overlaps_route(&flow, &connector_points(&result.report, "tick")));
    assert!(!has_issue(&result.report, "connector-overlap"));

    assert_fix_snapshots!(result);
}

/// Given a row of equally tall nodes, so the only detour lines run 8px
///   above and below them, and two buses already occupying both lines
/// When the diagram is fixed
/// Then the crossing connector detours in a lane beside a bus instead of
///   staying a straight line through the obstacle
#[test]
fn detours_in_a_parallel_lane_when_both_detour_lanes_are_taken() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="-100 -100 600 300">
        
      <g data-node="west">
        <rect x="-60" y="20" width="60" height="56" />
        <text x="-30" y="53" text-anchor="middle"
          font-size="12">W</text>
      </g>
        
      <g data-node="source">
        <rect x="20" y="20" width="100" height="56" />
        <text x="70" y="53" text-anchor="middle"
          font-size="12">S</text>
      </g>
        
      <g data-node="obstacle">
        <rect x="150" y="20" width="80" height="56" />
        <text x="190" y="53" text-anchor="middle"
          font-size="12">O</text>
      </g>
        
      <g data-node="target">
        <rect x="260" y="20" width="100" height="56" />
        <text x="310" y="53" text-anchor="middle"
          font-size="12">T</text>
      </g>
        
      <g data-node="east">
        <rect x="400" y="20" width="60" height="56" />
        <text x="430" y="53" text-anchor="middle"
          font-size="12">E</text>
      </g>
        <line id="upper" data-from="west" data-to="east"
          x1="0" y1="12" x2="400" y2="12" />
        <line id="lower" data-from="west" data-to="east"
          x1="0" y1="84" x2="400" y2="84" />
        <line id="flow" data-from="source" data-to="target"
          x1="120" y1="48" x2="260" y2="48" />
      </svg>
    "#;

    let result = fix(svg);
    let flow = connector_points(&result.report, "flow");

    assert!(is_orthogonal(&flow));
    assert!(!enters_box(&flow, &node_bounds(&result.report, "obstacle")));
    assert!(result.report.issues.is_empty());

    assert_fix_snapshots!(result);
}

/// Given a request and a reply drawn on the same diagonal between a circle
///   and a box below and to its right, so both must turn a corner
/// When the diagram is fixed
/// Then the two routes nest around the corner instead of crossing, and
///   nothing is reported
#[test]
fn nests_a_request_and_reply_that_turn_the_same_corner() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 400 300">
        <g data-node="ship">
          <circle cx="100" cy="80" r="50" />
          <text x="100" y="85" text-anchor="middle" font-size="14">Ship</text>
        </g>
        <g data-node="carrier">
          <rect x="240" y="200" width="120" height="52" />
          <text x="300" y="231" text-anchor="middle" font-size="14">Carrier</text>
        </g>
        <line id="shipment" data-from="ship" data-to="carrier" x1="100" y1="80" x2="300" y2="226" />
        <line id="tracking" data-from="carrier" data-to="ship" x1="300" y1="226" x2="100" y2="80" />
      </svg>
    "#;

    let result = fix(svg);
    let shipment = connector_points(&result.report, "shipment");
    let tracking = connector_points(&result.report, "tracking");

    assert!(bend_count(&shipment) > 0 && bend_count(&tracking) > 0);
    assert!(is_orthogonal(&shipment) && is_orthogonal(&tracking));
    assert!(!crosses_route(&shipment, &tracking), "{shipment:?} {tracking:?}");
    assert!(result.report.issues.is_empty());

    assert_fix_snapshots!(result);
}

/// Given a request and a reply between two nodes stacked with a third node
///   between them, so both must leave and enter on the same-facing sides
///   in a U shape
/// When the diagram is fixed
/// Then the two U-shaped routes nest instead of crossing, and nothing is
///   reported
#[test]
fn nests_a_request_and_reply_that_both_detour_in_a_u_shape() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 400 420">
        <g data-node="browse">
          <circle cx="200" cy="80" r="50" />
          <text x="200" y="85" text-anchor="middle" font-size="14">Browse</text>
        </g>
        <g data-node="cart">
          <circle cx="200" cy="220" r="50" />
          <text x="200" y="225" text-anchor="middle" font-size="14">Cart</text>
        </g>
        <g data-node="products">
          <rect x="130" y="340" width="130" height="40" />
          <text x="195" y="365" text-anchor="middle" font-size="14">Products</text>
        </g>
        <line id="query" data-from="browse" data-to="products" x1="200" y1="80" x2="195" y2="360" />
        <line id="details" data-from="products" data-to="browse" x1="195" y1="360" x2="200" y2="80" />
      </svg>
    "#;

    let result = fix(svg);
    let query = connector_points(&result.report, "query");
    let details = connector_points(&result.report, "details");

    assert!(bend_count(&query) > 1 && bend_count(&details) > 1);
    assert!(is_orthogonal(&query) && is_orthogonal(&details));
    assert!(!crosses_route(&query, &details), "{query:?} {details:?}");
    assert!(result.report.issues.is_empty());

    assert_fix_snapshots!(result);
}

/// Given a request and a reply that overlap between the same two nodes
/// When the diagram is fixed
/// Then their endpoints are spread along the node edges and no overlap remains
#[test]
fn separates_a_request_and_reply_drawn_on_the_same_line() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="-100 -100 600 300">
        <g data-node="client">
          <rect x="20" y="20" width="100" height="60" />
          <text x="70" y="55" text-anchor="middle" font-size="14">Client</text>
        </g>
        <g data-node="server">
          <rect x="260" y="20" width="100" height="60" />
          <text x="310" y="55" text-anchor="middle" font-size="14">Server</text>
        </g>
        <line id="request" data-from="client" data-to="server"
          x1="120" y1="50" x2="260" y2="50" />
        <line id="reply" data-from="server" data-to="client"
          x1="260" y1="50" x2="120" y2="50" />
      </svg>
    "#;

    let result = fix(svg);
    let request = connector_points(&result.report, "request");
    let reply = connector_points(&result.report, "reply");
    let client = node_bounds(&result.report, "client");
    let server = node_bounds(&result.report, "server");

    assert!(!overlaps_route(&request, &reply));
    for (route, from, to) in [(&request, &client, &server), (&reply, &server, &client)] {
        assert!(is_orthogonal(route));
        assert!(leaves_perpendicularly(route, from));
        assert!(enters_perpendicularly(route, to));
    }
    assert!(result.report.issues.is_empty());

    assert_fix_snapshots!(result);
}

/// Given a 3x3 grid of nodes and a connector between opposite corners
/// When the diagram is fixed
/// Then the connector becomes an orthogonal path that crosses no node
#[test]
fn reroutes_with_several_bends_when_one_detour_cannot_clear_the_obstacles() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="-100 -100 600 500">
        
      <g data-node="n00">
        <rect x="0" y="0" width="100" height="50" />
        <text x="50" y="30" text-anchor="middle" font-size="14">n00</text>
      </g>
      <g data-node="n01">
        <rect x="150" y="0" width="100" height="50" />
        <text x="200" y="30" text-anchor="middle" font-size="14">n01</text>
      </g>
      <g data-node="n02">
        <rect x="300" y="0" width="100" height="50" />
        <text x="350" y="30" text-anchor="middle" font-size="14">n02</text>
      </g>
      <g data-node="n10">
        <rect x="0" y="100" width="100" height="50" />
        <text x="50" y="130" text-anchor="middle" font-size="14">n10</text>
      </g>
      <g data-node="n11">
        <rect x="150" y="100" width="100" height="50" />
        <text x="200" y="130" text-anchor="middle" font-size="14">n11</text>
      </g>
      <g data-node="n12">
        <rect x="300" y="100" width="100" height="50" />
        <text x="350" y="130" text-anchor="middle" font-size="14">n12</text>
      </g>
      <g data-node="n20">
        <rect x="0" y="200" width="100" height="50" />
        <text x="50" y="230" text-anchor="middle" font-size="14">n20</text>
      </g>
      <g data-node="n21">
        <rect x="150" y="200" width="100" height="50" />
        <text x="200" y="230" text-anchor="middle" font-size="14">n21</text>
      </g>
      <g data-node="n22">
        <rect x="300" y="200" width="100" height="50" />
        <text x="350" y="230" text-anchor="middle" font-size="14">n22</text>
      </g>
        <line id="flow" data-from="n20" data-to="n02"
          x1="100" y1="225" x2="300" y2="25" />
      </svg>
    "#;

    let result = fix(svg);
    let points = connector_points(&result.report, "flow");

    assert!(is_orthogonal(&points));
    assert!(leaves_perpendicularly(&points, &node_bounds(&result.report, "n20")));
    assert!(enters_perpendicularly(&points, &node_bounds(&result.report, "n02")));
    assert!(!has_issue(&result.report, "connector-node-crossing"));

    assert_fix_snapshots!(result);
}
