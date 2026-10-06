//! Reading a diagram: which SVG elements become nodes, connectors and
//! unsupported shapes.

mod support;

use nicevg::{DiagramConnector, DiagramNode, Shape, UnsupportedElement};
use support::*;

/// Given an SVG group explicitly marked as a diagram node
/// When the SVG is analyzed
/// Then the public report exposes the node, its box, and its label bounds
#[test]
fn recognizes_an_annotated_node_with_its_box_and_label() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 240 120">
        <g data-node="checkout">
          <rect x="20" y="20" width="120" height="56" />
          <text x="80" y="53" text-anchor="middle" font-size="14">Checkout</text>
        </g>
      </svg>
    "#;

    let report = analyze(svg);

    assert_eq!(
        report.diagram.nodes,
        [DiagramNode {
            id: "checkout".to_owned(),
            shape: None,
            bounds: bounds(20.0, 20.0, 120.0, 56.0),
            label_bounds: vec![bounds(47.0, 39.0, 66.0, 17.0)],
            parent_id: None,
            allow_overlap: false,
        }]
    );
}

/// Given an unannotated SVG group with a direct rectangle and text child
/// When the SVG is analyzed
/// Then the group is exposed as a node using its id
#[test]
fn infers_a_node_from_a_group_containing_a_rectangle_and_text() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 240 120">
        <g id="review">
          <rect x="30" y="24" width="100" height="48" />
          <text x="80" y="53" text-anchor="middle" font-size="14">Review</text>
        </g>
      </svg>
    "#;

    let ids: Vec<String> = analyze(svg).diagram.nodes.into_iter().map(|node| node.id).collect();

    assert_eq!(ids, ["review"]);
}

/// Given an annotated group whose shape is a circle
/// When the SVG is analyzed
/// Then it is a circular node whose bounds are the circle's bounding box
#[test]
fn recognizes_an_annotated_circle_as_a_circular_node() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 240 160">
        <g data-node="start">
          <circle cx="80" cy="70" r="40" />
          <text x="80" y="75" text-anchor="middle" font-size="14">Start</text>
        </g>
      </svg>
    "#;

    let report = analyze(svg);
    let node = &report.diagram.nodes[0];

    assert_eq!(node.id, "start");
    assert_eq!(node.shape, Some(Shape::Circle));
    assert_eq!(node.bounds, bounds(40.0, 30.0, 80.0, 80.0));
}

/// Given an unannotated group with a direct circle and text child
/// When the SVG is analyzed
/// Then the group is exposed as a circular node using its id
#[test]
fn infers_a_circular_node_from_a_group_containing_a_circle_and_text() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 240 160">
        <g id="done">
          <circle cx="80" cy="70" r="40" />
          <text x="80" y="75" text-anchor="middle" font-size="14">Done</text>
        </g>
      </svg>
    "#;

    let nodes: Vec<(String, Option<Shape>)> = analyze(svg)
        .diagram
        .nodes
        .into_iter()
        .map(|node| (node.id, node.shape))
        .collect();

    assert_eq!(nodes, [("done".to_owned(), Some(Shape::Circle))]);
}

/// Given a line with explicit source and target node ids
/// When the SVG is analyzed
/// Then the public report exposes the connector and its endpoints
#[test]
fn recognizes_an_annotated_connector_between_nodes() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 320 120">
        <g data-node="submit">
          <rect x="20" y="20" width="100" height="56" />
          <text x="70" y="53" text-anchor="middle">Submit</text>
        </g>
        <g data-node="review">
          <rect x="200" y="20" width="100" height="56" />
          <text x="250" y="53" text-anchor="middle">Review</text>
        </g>
        <line id="submit-review" data-from="submit" data-to="review"
          x1="120" y1="48" x2="200" y2="48" />
      </svg>
    "#;

    let report = analyze(svg);

    assert_eq!(
        report.diagram.connectors,
        [DiagramConnector {
            id: "submit-review".to_owned(),
            from: "submit".to_owned(),
            to: "review".to_owned(),
            points: vec![point(120.0, 48.0), point(200.0, 48.0)],
        }]
    );
}

/// Two nodes in a row joined by `arrow`, an arrow drawn with a polygon head.
fn row_with_arrow(arrow: &str) -> String {
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 400 120">
          <g data-node="submit">
            <rect x="20" y="20" width="100" height="56" />
            <text x="70" y="53" text-anchor="middle">Submit</text>
          </g>
          <g data-node="review">
            <rect x="260" y="20" width="100" height="56" />
            <text x="310" y="53" text-anchor="middle">Review</text>
          </g>
          {arrow}
        </svg>"#
    )
}

/// Given an arrow drawn as a group carrying data-from and data-to around a
///   path that stops at the tail of a polygon arrowhead
/// When the SVG is analyzed
/// Then it is one connector running to the arrowhead's tip on the target
#[test]
fn recognizes_an_annotated_group_of_a_line_and_a_polygon_head_as_a_connector() {
    let svg = row_with_arrow(
        r#"<g id="submit-review" data-from="submit" data-to="review">
            <path d="M 120 48 L 250 48" />
            <polygon points="250,43 260,48 250,53" />
          </g>"#,
    );

    let report = analyze(&svg);

    assert_eq!(
        report.diagram.connectors,
        [DiagramConnector {
            id: "submit-review".to_owned(),
            from: "submit".to_owned(),
            to: "review".to_owned(),
            points: vec![point(120.0, 48.0), point(260.0, 48.0)],
        }]
    );
    assert!(report.issues.is_empty(), "{:?}", report.issues);
}

/// Given an annotated line grouped with a polygon arrowhead at its end, and
///   another polygon in the group away from either end
/// When the SVG is analyzed
/// Then the connector runs to the arrowhead's tip, ignoring the other polygon
#[test]
fn extends_an_annotated_line_to_the_tip_of_a_polygon_head_beside_it() {
    let svg = row_with_arrow(
        r#"<g>
            <line id="submit-review" data-from="submit" data-to="review" x1="120" y1="48" x2="250" y2="48" />
            <polygon points="250,43 260,48 250,53" />
            <polygon points="180,90 190,95 180,100" />
          </g>"#,
    );

    let report = analyze(&svg);

    assert_eq!(
        connector_points(&report, "submit-review"),
        [point(120.0, 48.0), point(260.0, 48.0)]
    );
}

/// Given a path annotated with data-a and data-b at the top level, and an
///   arrow group annotated the same way
/// When the SVG is analyzed
/// Then both are connectors between the named nodes, as data-from and
///   data-to would make them, and neither is reported as unsupported
#[test]
fn recognizes_data_a_and_data_b_like_data_from_and_data_to() {
    let svg = row_with_arrow(
        r#"<path id="plain" data-a="submit" data-b="review" d="M 120 40 L 260 40" />
          <g id="grouped" data-a="submit" data-b="review">
            <path d="M 120 56 L 250 56" />
            <polygon points="250,51 260,56 250,61" />
          </g>"#,
    );

    let report = analyze(&svg);
    let ends: Vec<(&str, &str, &str)> = report
        .diagram
        .connectors
        .iter()
        .map(|connector| (connector.id.as_str(), connector.from.as_str(), connector.to.as_str()))
        .collect();

    assert_eq!(ends, [("plain", "submit", "review"), ("grouped", "submit", "review")]);
    assert_eq!(
        connector_points(&report, "grouped"),
        [point(120.0, 56.0), point(260.0, 56.0)]
    );
    assert!(report.diagram.unsupported_elements.is_empty());
}

/// Given a child node nested inside a parent node group
/// When the SVG is analyzed
/// Then the child references its parent and their geometric overlap is allowed
#[test]
fn treats_a_nested_node_as_intentional_containment() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="-100 -100 600 400">
        <g data-node="system">
          <rect x="10" y="10" width="300" height="180" />
          <text x="30" y="35" font-size="14">System</text>
          <g data-node="service">
            <rect x="50" y="60" width="100" height="56" />
            <text x="100" y="93" text-anchor="middle" font-size="14">Service</text>
          </g>
        </g>
      </svg>
    "#;

    let report = analyze(svg);
    let child = report
        .diagram
        .nodes
        .iter()
        .find(|node| node.id == "service")
        .expect("service node");

    assert_eq!(child.parent_id.as_deref(), Some("system"));
    assert!(!has_issue(&report, "node-overlap"));
}

/// Given malformed SVG XML
/// When the public API analyzes it
/// Then callers receive a typed input error instead of a partial report
#[test]
fn rejects_malformed_xml_as_an_svg_input_error() {
    assert!(nicevg::analyze("<svg><g></svg>").is_err());
}

/// Given a standalone shape that cannot be mapped to a diagram node
/// When the SVG is analyzed
/// Then it is exposed for diagnostics rather than assigned guessed semantics
#[test]
fn reports_standalone_freeform_shapes_as_unsupported() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 120">
        <circle id="mechanism-wheel" cx="70" cy="60" r="30" />
      </svg>
    "#;

    let report = analyze(svg);

    assert_eq!(
        report.diagram.unsupported_elements,
        [UnsupportedElement {
            id: "mechanism-wheel".to_owned(),
            tag_name: "circle".to_owned()
        }]
    );
}
