//! Diagrams given as shapes rather than SVG: nodes and connectors as plain
//! data, read and repaired like the SVG they stand for.

mod support;

use nicevg::{Shape, Sketch, SketchError};
use serde_json::json;
use support::*;

fn sketch(value: serde_json::Value) -> Sketch {
    serde_json::from_value(value).expect("a well-formed sketch")
}

/// The route of the connector from `from` to `to`: a sketch's connectors
/// have no id, and their ends tell them apart.
fn route(report: &nicevg::AnalysisReport, from: &str, to: &str) -> Vec<nicevg::Point> {
    report
        .diagram
        .connectors
        .iter()
        .find(|connector| connector.from == from && connector.to == to)
        .map(|connector| connector.points.clone())
        .expect("a connector between the two")
}

fn two_boxes(connector: serde_json::Value) -> Sketch {
    sketch(json!({
        "nodes": [
            {"id": "order", "x": 20, "y": 20, "width": 120, "height": 56, "label": "Order"},
            {"id": "bill", "x": 300, "y": 160, "width": 120, "height": 56, "label": "Bill"}
        ],
        "connectors": [connector]
    }))
}

/// Given a sketch with a box node and a round node
/// When it is analyzed
/// Then both are nodes at the given place and size, the round one marked as a circle
#[test]
fn reads_box_and_round_nodes_where_they_are_given() {
    let input = sketch(json!({
        "nodes": [
            {"id": "store", "x": 20, "y": 30, "width": 120, "height": 56, "label": "Store"},
            {"id": "user", "x": 200, "y": 30, "width": 80, "height": 80, "shape": "circle", "label": "User"}
        ]
    }));

    let report = nicevg::analyze_sketch(&input).expect("valid sketch");

    assert_eq!(node_bounds(&report, "store"), bounds(20.0, 30.0, 120.0, 56.0));
    assert_eq!(node_bounds(&report, "user"), bounds(200.0, 30.0, 80.0, 80.0));
    let user = report.diagram.node("user").expect("user node");
    assert_eq!(user.shape, Some(Shape::Circle));
}

/// Given a sketch with a node whose parent is another node
/// When it is analyzed
/// Then the node is a child inside its parent
#[test]
fn reads_a_node_with_a_parent_as_inside_it() {
    let input = sketch(json!({
        "nodes": [
            {"id": "cluster", "x": 0, "y": 0, "width": 300, "height": 200, "label": "Cluster"},
            {"id": "service", "x": 40, "y": 60, "width": 120, "height": 56, "label": "Service", "parent": "cluster"}
        ]
    }));

    let report = nicevg::analyze_sketch(&input).expect("valid sketch");

    let service = report.diagram.node("service").expect("service node");
    assert_eq!(service.parent_id.as_deref(), Some("cluster"));
    assert!(report.issues.is_empty(), "{:?}", issue_codes(&report));
}

/// Given a connector between two boxes with no route given
/// When the sketch is fixed
/// Then it runs orthogonally from a side of one box to a side of the other
#[test]
fn routes_a_connector_without_points_orthogonally_between_its_nodes() {
    let input = two_boxes(json!({"from": "order", "to": "bill"}));

    let result = nicevg::fix_sketch(&input).expect("valid sketch");

    let points = route(&result.report, "order", "bill");
    assert!(is_orthogonal(&points), "{points:?}");
    assert!(leaves_perpendicularly(&points, &node_bounds(&result.report, "order")));
    assert!(enters_perpendicularly(&points, &node_bounds(&result.report, "bill")));
    assert!(result.report.issues.is_empty(), "{:?}", issue_codes(&result.report));
}

/// Given a connector with its route given
/// When the sketch is analyzed
/// Then the connector runs along that route
#[test]
fn reads_a_connector_along_the_route_given() {
    let input = two_boxes(json!({
        "from": "order", "to": "bill",
        "points": [{"x": 140, "y": 48}, {"x": 360, "y": 48}, {"x": 360, "y": 160}]
    }));

    let report = nicevg::analyze_sketch(&input).expect("valid sketch");

    assert_eq!(
        route(&report, "order", "bill"),
        [point(140.0, 48.0), point(360.0, 48.0), point(360.0, 160.0)]
    );
}

/// Given a connector with a label
/// When the sketch is fixed
/// Then the label sits beside that connector
#[test]
fn places_a_connector_label_beside_its_connector() {
    let input = two_boxes(json!({"from": "order", "to": "bill", "label": "invoice"}));

    let result = nicevg::fix_sketch(&input).expect("valid sketch");

    let label = result
        .report
        .diagram
        .labels
        .iter()
        .find(|label| label.connector.is_some())
        .expect("the connector's label");
    assert!(distance_to_route(&label.bounds, &route(&result.report, "order", "bill")) <= 16.0);
    assert!(result.report.issues.is_empty(), "{:?}", issue_codes(&result.report));
}

/// Given labels with two lines, and with `&` and `<` in them
/// When the sketch is turned into SVG and analyzed
/// Then the characters come through as written, and two lines read taller than one
#[test]
fn keeps_multi_line_labels_and_markup_characters_as_written() {
    let input = sketch(json!({
        "nodes": [
            {"id": "one", "x": 20, "y": 20, "width": 160, "height": 80, "label": "Orders & bills"},
            {"id": "two", "x": 220, "y": 20, "width": 160, "height": 80, "label": "a < b\nc > d"}
        ]
    }));

    let svg = nicevg::sketch_svg(&input).expect("valid sketch");
    let report = analyze(&svg);

    assert!(svg.contains("Orders &amp; bills") && svg.contains("a &lt; b") && svg.contains("c &gt; d"));
    let height = |id: &str| -> f64 {
        let node = report.diagram.node(id).expect("node");
        node.label_bounds
            .iter()
            .map(|b| b.y + b.height)
            .fold(f64::MIN, f64::max)
            - node.label_bounds.iter().map(|b| b.y).fold(f64::MAX, f64::min)
    };
    assert!(height("two") > height("one") * 1.5);
}

/// Given a sketch whose connector bends where moving a node would straighten it
/// When the sketch is arranged
/// Then the connector ends up with fewer bends, as with the same diagram in SVG
#[test]
fn arranges_a_sketch_like_the_svg_it_stands_for() {
    let input = sketch(json!({
        "nodes": [
            {"id": "notify", "x": 30, "y": 100, "width": 120, "height": 56, "label": "Notify"},
            {"id": "mail", "x": 280, "y": 52, "width": 120, "height": 56, "label": "Mail"}
        ],
        "connectors": [{"from": "notify", "to": "mail",
            "points": [{"x": 150, "y": 128}, {"x": 220, "y": 128}, {"x": 220, "y": 80}, {"x": 280, "y": 80}]}]
    }));

    let arranged = nicevg::arrange_sketch(&input).expect("valid sketch");
    let from_svg = nicevg::arrange(&nicevg::sketch_svg(&input).expect("valid sketch")).expect("valid SVG");

    assert_eq!(bend_count(&route(&arranged.report, "notify", "mail")), 0);
    assert_eq!(arranged.svg, from_svg.svg);
}

/// Given a connector to a node that is not in the sketch, and a node whose parent is not
/// When the sketch is read
/// Then each is an error naming where in the sketch the reference is and what it points to
#[test]
fn rejects_references_to_nodes_not_in_the_sketch() {
    let dangling = two_boxes(json!({"from": "order", "to": "ghost"}));
    let orphan = sketch(json!({
        "nodes": [{"id": "service", "x": 0, "y": 0, "width": 100, "height": 50, "parent": "nowhere"}]
    }));

    assert_eq!(
        nicevg::analyze_sketch(&dangling).unwrap_err(),
        SketchError::UnknownNode {
            referrer: "connectors[0].to".to_owned(),
            node: "ghost".to_owned()
        }
    );
    assert_eq!(
        nicevg::analyze_sketch(&orphan).unwrap_err(),
        SketchError::UnknownNode {
            referrer: "nodes[0].parent".to_owned(),
            node: "nowhere".to_owned()
        }
    );
}

/// A data store open on its right: its id, a dividing line, then its name.
const STORE: &str = r#"<path d="M 120 0 H 0 V 40 H 120" fill="none" stroke="black"/><line x1="30" y1="0" x2="30" y2="40" stroke="black"/><text x="15" y="20" font-size="14" text-anchor="middle" dominant-baseline="central">D1</text><text x="42" y="20" font-size="14" dominant-baseline="central">Orders</text>"#;

/// Given a node drawn by its own SVG, with no width or height
/// When the sketch is analyzed
/// Then the node lies around what it draws, from its x and y, and its texts are not labels to fit
#[test]
fn reads_a_node_drawn_by_its_svg_as_the_box_around_the_drawing() {
    let input = sketch(json!({"nodes": [{"id": "orders", "x": 200, "y": 100, "svg": STORE}]}));

    let report = nicevg::analyze_sketch(&input).expect("valid sketch");

    assert_eq!(node_bounds(&report, "orders"), bounds(200.0, 100.0, 120.0, 40.0));
    assert!(report.diagram.node("orders").expect("node").label_bounds.is_empty());
    assert!(report.issues.is_empty(), "{:?}", issue_codes(&report));
}

/// Given a node drawn by its SVG overlapping a box, joined to it by a connector with no route
/// When the sketch is fixed
/// Then the drawn node moves clear as it was drawn, keeping its size, and the connector meets its side
#[test]
fn moves_a_drawn_node_without_resizing_it_and_routes_to_its_sides() {
    let input = sketch(json!({
        "nodes": [
            {"id": "order", "x": 20, "y": 20, "width": 120, "height": 56, "label": "Order"},
            {"id": "orders", "x": 100, "y": 30, "svg": STORE}
        ],
        "connectors": [{"from": "order", "to": "orders"}]
    }));

    let result = nicevg::fix_sketch(&input).expect("valid sketch");

    let moved = node_bounds(&result.report, "orders");
    assert_eq!((moved.width, moved.height), (120.0, 40.0));
    assert!(
        result
            .svg
            .contains(STORE.split("<text").next().expect("the drawing's lines"))
    );
    let points = route(&result.report, "order", "orders");
    assert!(is_orthogonal(&points), "{points:?}");
    assert!(enters_perpendicularly(&points, &moved));
    assert!(result.report.issues.is_empty(), "{:?}", issue_codes(&result.report));
}

/// Given a box node with no size, and a node whose SVG is not well-formed
/// When the sketch is read
/// Then each is an error naming the node
#[test]
fn rejects_a_node_with_neither_size_nor_well_formed_svg() {
    let sizeless = sketch(json!({"nodes": [{"id": "box", "x": 0, "y": 0}]}));
    let broken = sketch(json!({"nodes": [{"id": "store", "x": 0, "y": 0, "svg": "<path d=\"M 0 0 H 10\">"}]}));

    assert_eq!(
        nicevg::analyze_sketch(&sizeless).unwrap_err(),
        SketchError::MissingSize {
            referrer: "nodes[0]".to_owned()
        }
    );
    assert!(matches!(
        nicevg::analyze_sketch(&broken).unwrap_err(),
        SketchError::InvalidSvg { referrer, .. } if referrer == "nodes[0].svg"
    ));
}

/// Given a box whose parent is a node drawn by its SVG
/// When the sketch is analyzed
/// Then the box is inside that parent at the place given, and the parent lies around its own drawing
#[test]
fn reads_a_child_of_a_drawn_node_where_it_is_given() {
    let input = sketch(json!({
        "nodes": [
            {"id": "zone", "x": 100, "y": 100, "svg": "<rect width=\"300\" height=\"200\" fill=\"none\" stroke=\"black\"/>"},
            {"id": "service", "x": 120, "y": 130, "width": 120, "height": 56, "label": "Service", "parent": "zone"}
        ]
    }));

    let report = nicevg::analyze_sketch(&input).expect("valid sketch");

    assert_eq!(node_bounds(&report, "service"), bounds(120.0, 130.0, 120.0, 56.0));
    assert_eq!(node_bounds(&report, "zone"), bounds(100.0, 100.0, 300.0, 200.0));
    let service = report.diagram.node("service").expect("service node");
    assert_eq!(service.parent_id.as_deref(), Some("zone"));
}

/// Given root attributes setting the font, with `&` in a value, and a node drawing a text of no size of its own
/// When the sketch is fixed
/// Then the fixed SVG's root keeps them escaped, and the drawn text measures at the root's font size
#[test]
fn writes_root_attributes_that_drawn_texts_inherit() {
    let input = sketch(json!({
        "nodes": [{"id": "note", "x": 20, "y": 20, "svg": "<text x=\"0\" y=\"0\" dominant-baseline=\"hanging\">Note</text>"}],
        "rootAttrs": {"font-family": "'Yu Gothic' & sans-serif", "font-size": "20"}
    }));

    let result = nicevg::fix_sketch(&input).expect("valid sketch");

    let root = result.svg.lines().next().expect("the root");
    assert!(root.contains(r#"font-family="'Yu Gothic' &amp; sans-serif""#), "{root}");
    assert!(root.contains(r#"font-size="20""#), "{root}");
    assert_eq!(node_bounds(&result.report, "note").height, 24.0);
}

/// Given a root attribute nicevg writes itself, and one that is not an attribute name
/// When the sketch is read
/// Then each is an error naming the attribute
#[test]
fn rejects_root_attributes_nicevg_writes_or_that_are_not_names() {
    for name in ["viewBox", "bad name"] {
        let mut input = two_boxes(json!({"from": "order", "to": "bill"}));
        input.root_attrs.insert(name.to_owned(), "0".to_owned());

        assert!(matches!(
            nicevg::analyze_sketch(&input).unwrap_err(),
            SketchError::InvalidRootAttribute { name: rejected, .. } if rejected == name
        ));
    }
}
