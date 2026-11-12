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
/// Then the connector ends up with fewer bends, the nodes and routes where they are with the same
///   diagram in SVG
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
    assert_eq!(arranged.report.diagram, from_svg.report.diagram);
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

/// Given two nodes sharing the id "order"
/// When the sketch is read
/// Then it is an error naming the second of them and the id, rather than
///   one of the nodes left out
#[test]
fn rejects_two_nodes_with_the_same_id() {
    let input = sketch(json!({
        "nodes": [
            {"id": "order", "x": 0, "y": 0, "width": 100, "height": 50, "label": "First"},
            {"id": "order", "x": 300, "y": 200, "width": 100, "height": 50, "label": "Second"}
        ]
    }));

    assert_eq!(
        nicevg::analyze_sketch(&input).unwrap_err(),
        SketchError::DuplicateNode {
            referrer: "nodes[1].id".to_owned(),
            node: "order".to_owned()
        }
    );
}

/// A data store open on its right: its id, a dividing line, then its name.
const STORE: &str = r#"<path d="M 120 0 H 0 V 40 H 120" fill="none" stroke="black"/><line x1="30" y1="0" x2="30" y2="40" stroke="black"/><text x="15" y="20" font-size="14" text-anchor="middle" dominant-baseline="central">D1</text><text x="42" y="20" font-size="14" dominant-baseline="central">Orders</text>"#;

/// Given a node drawn by its own SVG, with no width or height
/// When the sketch is analyzed
/// Then the node lies around what it draws, from its x and y, and its texts are labels to keep clear of,
///   not checked to fit, though the id sits nearer its side than 12px
#[test]
fn reads_a_node_drawn_by_its_svg_as_the_box_around_the_drawing() {
    let input = sketch(json!({"nodes": [{"id": "orders", "x": 200, "y": 100, "svg": STORE}]}));

    let report = nicevg::analyze_sketch(&input).expect("valid sketch");

    assert_eq!(node_bounds(&report, "orders"), bounds(200.0, 100.0, 120.0, 40.0));
    let node = report.diagram.node("orders").expect("node");
    assert_eq!(node.label_bounds.len(), 2);
    assert!(node.custom);
    assert!(report.issues.is_empty(), "{:?}", issue_codes(&report));
}

/// Given a node drawn by its SVG with a circle marked data-bounds, and a shadow sticking out beside it
/// When the sketch is analyzed
/// Then the node is that circle, round, and the shadow is not part of it
#[test]
fn takes_the_shape_marked_data_bounds_as_the_node() {
    let svg = r##"<rect x="6" y="6" width="80" height="80" fill="#ccc"/><circle data-bounds="true" cx="40" cy="40" r="40" fill="white" stroke="black"/><text x="40" y="40" font-size="14" text-anchor="middle" dominant-baseline="central">User</text>"##;
    let input = sketch(json!({"nodes": [{"id": "user", "x": 100, "y": 60, "svg": svg}]}));

    let report = nicevg::analyze_sketch(&input).expect("valid sketch");

    let node = report.diagram.node("user").expect("node");
    assert_eq!(node.bounds, bounds(100.0, 60.0, 80.0, 80.0));
    assert_eq!(node.shape, Some(Shape::Circle));
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

/// Given two nodes whose data-bounds cover only their icons, their names drawn far below them, joined
///   by a connector with no route
/// When the sketch is fixed
/// Then the connector runs orthogonally between sides of the icons, the nodes' extents
#[test]
fn routes_from_the_data_bounds_even_with_a_picture_reaching_past_them() {
    let svg = r##"<rect data-bounds="true" width="40" height="40" fill="white" stroke="black"/><text x="20" y="220" font-size="14" text-anchor="middle">Far below</text>"##;
    let input = sketch(json!({
        "nodes": [
            {"id": "left", "x": 20, "y": 20, "svg": svg},
            {"id": "right", "x": 300, "y": 60, "svg": svg}
        ],
        "connectors": [{"from": "left", "to": "right"}]
    }));

    let result = nicevg::fix_sketch(&input).expect("valid sketch");

    let points = route(&result.report, "left", "right");
    assert_eq!(node_bounds(&result.report, "left"), bounds(20.0, 20.0, 40.0, 40.0));
    assert!(is_orthogonal(&points), "{points:?}");
    assert!(
        leaves_perpendicularly(&points, &node_bounds(&result.report, "left")),
        "{points:?}"
    );
    assert!(
        enters_perpendicularly(&points, &node_bounds(&result.report, "right")),
        "{points:?}"
    );
}

/// A table 120 wide and 96 high drawn by its SVG, with the port `port` on
/// both sides 62px down.
fn table_with_port(port: &str) -> String {
    format!(
        r#"<rect data-bounds="true" width="120" height="96" fill="white" stroke="black"/><circle data-port="{port}" cx="0" cy="62" r="0"/><circle data-port="{port}" cx="120" cy="62" r="0"/>"#
    )
}

/// Given two drawn tables and a connector naming a port of each, with no route
/// When the sketch is fixed
/// Then the connector leaves and reaches those ports, which the diagram read back names
#[test]
fn routes_a_sketch_connector_between_the_ports_it_names() {
    let input = sketch(json!({
        "nodes": [
            {"id": "article", "x": 20, "y": 20, "svg": table_with_port("author")},
            {"id": "user", "x": 300, "y": 20, "svg": table_with_port("id")}
        ],
        "connectors": [{"from": "article", "fromPort": "author", "to": "user", "toPort": "id"}]
    }));

    let result = nicevg::fix_sketch(&input).expect("valid sketch");

    let connector = &result.report.diagram.connectors[0];
    assert_eq!(
        (connector.from_port.as_deref(), connector.to_port.as_deref()),
        (Some("author"), Some("id"))
    );
    assert_eq!(connector.points, [point(140.0, 82.0), point(300.0, 82.0)]);
}

/// Given a connector naming a port "email" that the drawn table `user` does not mark
/// When the sketch is read
/// Then it is an error naming the connector's end and the port
#[test]
fn rejects_a_connector_naming_a_port_its_node_does_not_mark() {
    let input = sketch(json!({
        "nodes": [
            {"id": "article", "x": 20, "y": 20, "svg": table_with_port("author")},
            {"id": "user", "x": 300, "y": 20, "svg": table_with_port("id")}
        ],
        "connectors": [{"from": "article", "fromPort": "author", "to": "user", "toPort": "email"}]
    }));

    assert_eq!(
        nicevg::analyze_sketch(&input).unwrap_err(),
        SketchError::UnknownPort {
            referrer: "connectors[0].toPort".to_owned(),
            node: "user".to_owned(),
            port: "email".to_owned()
        }
    );
}

/// The arrowheads written, in order, each as its tip and the middle of its base.
fn heads(svg: &str) -> Vec<(nicevg::Point, nicevg::Point)> {
    let written = Written::parse(svg);
    (0..svg.matches("<polygon").count())
        .map(|index| {
            let v: Vec<f64> = attr(written.nth("polygon", index), "points")
                .split([' ', ','])
                .map(|value| value.parse().expect("a number"))
                .collect();
            (point(v[2], v[3]), point((v[0] + v[4]) / 2.0, (v[1] + v[5]) / 2.0))
        })
        .collect()
}

/// The way from `from` to `to`, `length` long.
fn towards(from: nicevg::Point, to: nicevg::Point, length: f64) -> (f64, f64) {
    let whole = (to.x - from.x).hypot(to.y - from.y);
    ((to.x - from.x) / whole * length, (to.y - from.y) / whole * length)
}

/// Given a connector between two boxes with no arrow and no route given
/// When the sketch is fixed
/// Then one head sits at its `to` end, its tip on a side of `bill` and pointing along the last
///   segment, and the line stops at the head's base, 10px before the tip
#[test]
fn draws_a_head_at_the_to_end_unless_told_otherwise() {
    let input = two_boxes(json!({"from": "order", "to": "bill"}));

    let result = nicevg::fix_sketch(&input).expect("valid sketch");

    let points = route(&result.report, "order", "bill");
    let (end, before) = (points[points.len() - 1], points[points.len() - 2]);
    let [(tip, base)] = heads(&result.svg)[..] else {
        panic!("one head in {}", result.svg);
    };
    assert_eq!(tip, end);
    assert!(side_of(end, &node_bounds(&result.report, "bill")).is_some());
    assert_eq!((tip.x - base.x, tip.y - base.y), towards(before, end, 10.0));
    let shaft = attr(Written::parse(&result.svg).nth("path", 0), "d").to_owned();
    assert!(shaft.ends_with(&format!("{} {}", base.x, base.y)), "{shaft}");
    assert!(result.report.issues.is_empty(), "{:?}", issue_codes(&result.report));
}

/// Given a connector between two boxes with `arrow: "both"` and no route given
/// When the sketch is fixed
/// Then a head sits at each end, its tip on a side of the node there, pointing into it
#[test]
fn draws_a_head_at_both_ends_when_told() {
    let input = two_boxes(json!({"from": "order", "to": "bill", "arrow": "both"}));

    let result = nicevg::fix_sketch(&input).expect("valid sketch");

    let points = route(&result.report, "order", "bill");
    let [(start, start_base), (end, end_base)] = heads(&result.svg)[..] else {
        panic!("two heads in {}", result.svg);
    };
    assert_eq!((start, end), (points[0], points[points.len() - 1]));
    assert!(side_of(start, &node_bounds(&result.report, "order")).is_some());
    assert!(side_of(end, &node_bounds(&result.report, "bill")).is_some());
    assert_eq!(
        (start.x - start_base.x, start.y - start_base.y),
        towards(points[1], points[0], 10.0)
    );
    assert_eq!(
        (end.x - end_base.x, end.y - end_base.y),
        towards(points[points.len() - 2], end, 10.0)
    );
    assert!(result.report.issues.is_empty(), "{:?}", issue_codes(&result.report));
}

/// Given a connector from `order` to itself with no route given
/// When the sketch is fixed
/// Then it loops out of `order` and back, with a head at its end whose tip is on a side of
///   `order`, pointing along the last segment
#[test]
fn draws_a_head_on_a_connector_to_its_own_node() {
    let input = two_boxes(json!({"from": "order", "to": "order"}));

    let result = nicevg::fix_sketch(&input).expect("valid sketch");

    let points = route(&result.report, "order", "order");
    let (end, before) = (points[points.len() - 1], points[points.len() - 2]);
    let [(tip, base)] = heads(&result.svg)[..] else {
        panic!("one head in {}", result.svg);
    };
    assert_eq!(tip, end);
    assert!(side_of(end, &node_bounds(&result.report, "order")).is_some());
    assert_eq!((tip.x - base.x, tip.y - base.y), towards(before, end, 10.0));
    assert!(result.report.issues.is_empty(), "{:?}", issue_codes(&result.report));
}
/// Given a connector between two boxes with `arrow: "none"`
/// When the sketch is fixed
/// Then it is a line with no head, running to a side of `bill`
#[test]
fn draws_no_head_when_told() {
    let input = two_boxes(json!({"from": "order", "to": "bill", "arrow": "none"}));

    let result = nicevg::fix_sketch(&input).expect("valid sketch");

    assert_eq!(heads(&result.svg), []);
    let points = route(&result.report, "order", "bill");
    assert!(side_of(points[points.len() - 1], &node_bounds(&result.report, "bill")).is_some());
}

/// Given a connector with `color: "#c00"`, one with `&` and `"` in its colour, and one with no colour
/// When each sketch is fixed
/// Then the line and the head are written in that colour, read back as given, or in black
#[test]
fn draws_a_connector_and_its_head_in_its_colour() {
    let painted = |connector: serde_json::Value| {
        let result = nicevg::fix_sketch(&two_boxes(connector)).expect("valid sketch");
        let written = Written::parse(&result.svg);
        (
            attr(written.nth("path", 0), "stroke").to_owned(),
            attr(written.nth("polygon", 0), "fill").to_owned(),
        )
    };

    let colour = |value: &str| (value.to_owned(), value.to_owned());
    assert_eq!(
        painted(json!({"from": "order", "to": "bill", "color": "#c00"})),
        colour("#c00")
    );
    assert_eq!(
        painted(json!({"from": "order", "to": "bill", "color": "a&\"b"})),
        colour("a&\"b")
    );
    assert_eq!(painted(json!({"from": "order", "to": "bill"})), colour("black"));
}

/// Given a connector with `strokeWidth: 2` and `strokeDasharray: "6 4"`, and one with neither
/// When each sketch is fixed
/// Then the first's line is written 2 wide and dashed 6 4, and the other's carries neither
///   attribute
#[test]
fn draws_a_connector_at_its_stroke_width_and_dashes() {
    let stroked = |connector: serde_json::Value| {
        let result = nicevg::fix_sketch(&two_boxes(connector)).expect("valid sketch");
        let written = Written::parse(&result.svg);
        let line = written.nth("path", 0);
        (line.get("stroke-width").cloned(), line.get("stroke-dasharray").cloned())
    };

    assert_eq!(
        stroked(json!({"from": "order", "to": "bill", "strokeWidth": 2, "strokeDasharray": "6 4"})),
        (Some("2".to_owned()), Some("6 4".to_owned()))
    );
    assert_eq!(stroked(json!({"from": "order", "to": "bill"})), (None, None));
}

/// Given a connector with its route given, the last segment 6px long, and the default head
/// When the sketch is fixed and the fixed SVG is read again
/// Then the route read back is the one given, its end at the head's tip, and the head is read as
///   the connector's, leaving no stray shape
#[test]
fn reads_a_headed_connector_along_the_route_given_even_with_a_short_last_segment() {
    let given = [
        point(140.0, 48.0),
        point(360.0, 48.0),
        point(360.0, 154.0),
        point(366.0, 154.0),
    ];
    let input = sketch(json!({
        "nodes": [
            {"id": "order", "x": 20, "y": 20, "width": 120, "height": 56},
            {"id": "bill", "x": 366, "y": 130, "width": 120, "height": 56}
        ],
        "connectors": [{"from": "order", "to": "bill", "points": given}]
    }));

    let result = nicevg::fix_sketch(&input).expect("valid sketch");

    let report = analyze(&result.svg);
    assert_eq!(heads(&result.svg).len(), 1);
    assert_eq!(route(&report, "order", "bill"), given);
    assert!(report.issues.is_empty(), "{:?}", issue_codes(&report));
}

/// Two boxes for a parent `group` to hold: `a` at x 100 to 180, y 100 to
/// 140, and `b` at x 220 to 280, y 140 to 180.
fn two_children() -> [serde_json::Value; 2] {
    [
        json!({"id": "a", "x": 100, "y": 100, "width": 80, "height": 40, "parent": "group"}),
        json!({"id": "b", "x": 220, "y": 140, "width": 60, "height": 40, "parent": "group"}),
    ]
}

/// Given a box parent titled "Orders" with neither position nor size, round
///   two children, the first just under where its title goes
/// When the sketch is fixed
/// Then the parent lies 12px out from its children as given, its title 12px
///   in from its top left, and the first child moves down to 12px below the
///   title, the parent's top staying where it was
#[test]
fn sizes_and_places_a_parent_round_its_children_and_moves_them_off_its_title() {
    let [a, b] = two_children();
    let input = sketch(json!({"nodes": [{"id": "group", "label": "Orders"}, a, b]}));

    let result = nicevg::fix_sketch(&input).expect("valid sketch");

    let group = node_bounds(&result.report, "group");
    let title = result.report.diagram.node("group").expect("the parent").label_bounds[0];
    assert_eq!(group, bounds(88.0, 88.0, 204.0, 104.0));
    assert_eq!(title.x - group.x, 12.0);
    assert_eq!(node_bounds(&result.report, "a").y, title.bottom() + 12.0);
    assert!(result.report.issues.is_empty(), "{:?}", issue_codes(&result.report));
}

/// Given the same parent placed and sized as x 40 to 340, y 20 to 120, its
///   children reaching below its bottom
/// When the sketch is analyzed
/// Then the parent keeps the left, top and right it was given, and its
///   bottom moves down to 12px below the children
#[test]
fn grows_a_placed_and_sized_parent_only_where_its_children_need() {
    let [a, b] = two_children();
    let input = sketch(json!({"nodes": [
        {"id": "group", "label": "Orders", "x": 40, "y": 20, "width": 300, "height": 100}, a, b
    ]}));

    let report = nicevg::analyze_sketch(&input).expect("valid sketch");

    assert_eq!(node_bounds(&report, "group"), bounds(40.0, 20.0, 300.0, 172.0));
}

/// Given an untitled parent with padding 20 and no position or size, round
///   one child at x 100 to 180, y 100 to 140
/// When the sketch is analyzed
/// Then the parent lies 20px out from the child on every side, and the
///   child is not reported as outside it
#[test]
fn keeps_a_parent_s_padding_round_its_children() {
    let [a, _] = two_children();
    let input = sketch(json!({"nodes": [{"id": "group", "padding": 20}, a]}));

    let report = nicevg::analyze_sketch(&input).expect("valid sketch");

    assert_eq!(node_bounds(&report, "group"), bounds(80.0, 80.0, 120.0, 80.0));
    assert!(report.issues.is_empty(), "{:?}", issue_codes(&report));
}

/// Given a node with no children and no x or y
/// When the sketch is read
/// Then it is an error naming the node: only a parent takes its position
///   from its children
#[test]
fn rejects_a_node_without_children_or_position() {
    let input = sketch(json!({"nodes": [{"id": "box", "width": 80, "height": 40}]}));

    assert_eq!(
        nicevg::analyze_sketch(&input).unwrap_err(),
        SketchError::MissingPosition {
            referrer: "nodes[0]".to_owned()
        }
    );
}

/// Given two nodes each naming the other as its parent
/// When the sketch is read
/// Then it is an error naming the first node's parent field and the parent
///   it names, which lies inside it
#[test]
fn rejects_parents_that_loop() {
    let input = sketch(json!({"nodes": [
        {"id": "a", "x": 0, "y": 0, "width": 80, "height": 40, "parent": "b"},
        {"id": "b", "x": 100, "y": 0, "width": 80, "height": 40, "parent": "a"}
    ]}));

    assert_eq!(
        nicevg::analyze_sketch(&input).unwrap_err(),
        SketchError::ParentCycle {
            referrer: "nodes[0].parent".to_owned(),
            node: "b".to_owned()
        }
    );
}
