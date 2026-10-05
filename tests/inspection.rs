//! Mechanical checks: each issue a diagram can have, and the near misses
//! that must not be reported.

mod support;

use serde_json::json;
use support::*;

/// Given a diagram whose marks fit inside the viewBox with 20px padding
/// When the diagram is analyzed
/// Then the report is valid and contains the measured drawing bounds
#[test]
fn accepts_a_diagram_whose_drawing_fits_inside_the_safe_viewport() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 240 120">
        <g data-node="checkout">
          <rect x="20" y="20" width="120" height="56" />
          <text x="80" y="53" text-anchor="middle" font-size="14">Checkout</text>
        </g>
      </svg>
    "#;

    let report = analyze(svg);

    assert!(report.valid);
    assert!(report.issues.is_empty());
    assert_eq!(report.drawing_bounds, Some(bounds(20.0, 20.0, 120.0, 56.0)));
}

/// Given a node whose right edge plus safety padding exceeds the viewBox
/// When the diagram is analyzed
/// Then a viewport clipping issue identifies the right side and required viewBox
#[test]
fn reports_the_side_and_required_bounds_when_the_viewbox_clips_content() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 120">
        <g data-node="checkout">
          <rect x="20" y="20" width="100" height="56" />
          <text x="70" y="53" text-anchor="middle" font-size="14">Checkout</text>
        </g>
      </svg>
    "#;

    let report = analyze(svg);

    assert!(!report.valid);
    assert_eq!(
        issues_json(&report),
        [json!({
            "code": "viewport-clipping",
            "message": "Drawing exceeds the safe viewBox on the right side.",
            "elements": ["svg"],
            "details": {
                "sides": ["right"],
                "requiredViewBox": { "x": 0, "y": 0, "width": 140, "height": 96 },
            },
        })]
    );
}

/// Given a label wider than its node's inner area
/// When the diagram is analyzed
/// Then a text overflow issue reports the minimum box dimensions
#[test]
fn reports_the_required_box_size_when_a_label_violates_node_padding() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="-100 -100 400 300">
        <g data-node="confirm">
          <rect x="20" y="20" width="70" height="40" />
          <text x="55" y="52" text-anchor="middle" font-size="14">Confirm payment</text>
        </g>
      </svg>
    "#;

    let report = analyze(svg);

    assert_eq!(
        issues_json(&report),
        [json!({
            "code": "text-overflow",
            "message": "Label does not fit inside node \"confirm\" with 12px padding.",
            "elements": ["confirm"],
            "details": { "requiredWidth": 148, "requiredHeight": 41 },
        })]
    );
}

/// Given two nodes with intersecting rectangles and no containment relation
/// When the diagram is analyzed
/// Then the overlapping pair and intersection bounds are reported
#[test]
fn reports_overlap_between_unrelated_nodes() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="-100 -100 500 300">
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

    let report = analyze(svg);

    assert!(issues_json(&report).contains(&json!({
        "code": "node-overlap",
        "message": "Nodes \"first\" and \"second\" overlap.",
        "elements": ["first", "second"],
        "details": { "intersection": { "x": 90, "y": 20, "width": 30, "height": 56 } },
    })));
}

/// Given two horizontally adjacent nodes with only a 10px gap
/// When the diagram is analyzed
/// Then the report identifies the missing distance to the required 20px gap
#[test]
fn reports_insufficient_spacing_between_nodes_in_the_same_row() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="-100 -100 500 300">
        <g data-node="first">
          <rect x="20" y="20" width="100" height="56" />
          <text x="70" y="53" text-anchor="middle" font-size="14">First</text>
        </g>
        <g data-node="second">
          <rect x="130" y="20" width="100" height="56" />
          <text x="180" y="53" text-anchor="middle" font-size="14">Second</text>
        </g>
      </svg>
    "#;

    let report = analyze(svg);

    assert!(issues_json(&report).contains(&json!({
        "code": "node-gap",
        "message": "Nodes \"first\" and \"second\" have a 10px gap; 20px is required.",
        "elements": ["first", "second"],
        "details": { "actualGap": 10, "requiredGap": 20, "shortage": 10 },
    })));
}

/// Given a connector whose segment passes through a third node
/// When the diagram is analyzed
/// Then the connector and obstructing node are reported
#[test]
fn reports_a_connector_crossing_an_unrelated_node() {
    let svg = r#"
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
        <line id="flow" data-from="source" data-to="target"
          x1="120" y1="48" x2="260" y2="48" />
      </svg>
    "#;

    let report = analyze(svg);

    assert!(issues_json(&report).contains(&json!({
        "code": "connector-node-crossing",
        "message": "Connector \"flow\" crosses unrelated node \"obstacle\".",
        "elements": ["flow", "obstacle"],
    })));
}

/// Given two nodes inside a cluster container, a connector between them and
///   another from inside the cluster to a node outside it
/// When the diagram is analyzed
/// Then neither connector is reported as crossing the cluster that contains
///   its ends
#[test]
fn does_not_report_crossing_a_container_that_holds_the_connector_ends() {
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
        <path id="ab" data-from="a" data-to="b" d="M 150 108 L 225 108 L 225 168 L 300 168" />
        <path id="bx" data-from="b" data-to="ext" d="M 400 168 L 460 168 L 460 108 L 480 108" />
      </svg>
    "#;

    assert!(!has_issue(&analyze(svg), "connector-node-crossing"));
}

/// Given a connector that passes 5px below an edge label
/// When the diagram is analyzed
/// Then the connector-label clearance violation is reported
#[test]
fn reports_a_connector_passing_within_8px_of_a_free_label() {
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
        <text id="retry-label" data-label="retry"
          x="150" y="38" font-size="14">Retry</text>
        <line id="flow" data-from="source" data-to="target"
          x1="120" y1="46" x2="260" y2="46" />
      </svg>
    "#;

    let report = analyze(svg);

    assert!(issues_json(&report).contains(&json!({
        "code": "connector-label-clearance",
        "message": "Connector \"flow\" passes within 8px of label \"retry\".",
        "elements": ["flow", "retry"],
        "details": { "requiredClearance": 8 },
    })));
}

/// Given a connector whose endpoints stop inside the source and target
/// When the diagram is analyzed
/// Then each endpoint is reported instead of being accepted as edge-to-edge
#[test]
fn reports_connector_endpoints_placed_inside_their_nodes() {
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
          x1="110" y1="48" x2="270" y2="48" />
      </svg>
    "#;

    let report = analyze(svg);
    let endpoints: Vec<serde_json::Value> = issues_json(&report)
        .into_iter()
        .filter(|issue| issue["code"] == "connector-endpoint-inside")
        .collect();

    assert_eq!(
        endpoints,
        [
            json!({
                "code": "connector-endpoint-inside",
                "message": "Connector \"flow\" starts inside node \"source\".",
                "elements": ["flow", "source"],
                "details": { "endpoint": "start" },
            }),
            json!({
                "code": "connector-endpoint-inside",
                "message": "Connector \"flow\" ends inside node \"target\".",
                "elements": ["flow", "target"],
                "details": { "endpoint": "end" },
            }),
        ]
    );
}

/// Given overlapping peer nodes where one declares intentional overlap
/// When the diagram is analyzed
/// Then no overlap issue is reported for that pair
#[test]
fn allows_overlap_explicitly_marked_as_intentional() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="-100 -100 500 300">
        <g data-node="back">
          <rect x="20" y="20" width="100" height="56" />
          <text x="70" y="53" text-anchor="middle" font-size="14">Back</text>
        </g>
        <g data-node="front" data-allow-overlap="true">
          <rect x="90" y="20" width="100" height="56" />
          <text x="140" y="53" text-anchor="middle" font-size="14">Front</text>
        </g>
      </svg>
    "#;

    assert!(!has_issue(&analyze(svg), "node-overlap"));
}

/// Given two free labels whose estimated bounding boxes intersect
/// When the diagram is analyzed
/// Then the overlapping label pair is reported
#[test]
fn reports_overlapping_free_labels() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="-100 -100 500 300">
        <text data-label="approved" x="40" y="60" font-size="14">Approved</text>
        <text data-label="pending" x="80" y="60" font-size="14">Pending</text>
      </svg>
    "#;

    let report = analyze(svg);

    assert!(issues_json(&report).contains(&json!({
        "code": "label-overlap",
        "message": "Labels \"approved\" and \"pending\" overlap.",
        "elements": ["approved", "pending"],
    })));
}

/// Given a free label that extends beyond the viewBox safety area
/// When the diagram is analyzed
/// Then viewport clipping is reported even when no nodes exist
#[test]
fn includes_free_labels_when_checking_the_safe_viewbox() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 120 80">
        <text data-label="note" x="90" y="40" font-size="14">Note</text>
      </svg>
    "#;

    let report = analyze(svg);

    assert!(issues_json(&report).contains(&json!({
        "code": "viewport-clipping",
        "message": "Drawing exceeds the safe viewBox on the right side.",
        "elements": ["svg"],
        "details": {
            "sides": ["right"],
            "requiredViewBox": { "x": 70, "y": 6, "width": 73, "height": 57 },
        },
    })));
}

/// Given a request and a reply drawn on the same horizontal line
/// When the diagram is analyzed
/// Then the pair of overlapping connectors is reported
#[test]
fn reports_connectors_that_run_along_the_same_segment() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="-100 -100 600 300">
        <g data-node="client">
          <rect x="20" y="20" width="100" height="56" />
          <text x="70" y="53" text-anchor="middle" font-size="14">Client</text>
        </g>
        <g data-node="server">
          <rect x="260" y="20" width="100" height="56" />
          <text x="310" y="53" text-anchor="middle" font-size="14">Server</text>
        </g>
        <line id="request" data-from="client" data-to="server"
          x1="120" y1="48" x2="260" y2="48" />
        <line id="reply" data-from="server" data-to="client"
          x1="260" y1="48" x2="120" y2="48" />
      </svg>
    "#;

    let report = analyze(svg);

    assert!(issues_json(&report).contains(&json!({
        "code": "connector-overlap",
        "message": "Connectors \"request\" and \"reply\" overlap along a segment.",
        "elements": ["request", "reply"],
    })));
}

/// Given a horizontal and a vertical connector that cross at one point
/// When the diagram is analyzed
/// Then no connector overlap is reported
#[test]
fn does_not_report_connectors_that_only_cross_each_other() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="-100 -100 600 400">
        <g data-node="west">
          <rect x="0" y="100" width="80" height="40" />
          <text x="40" y="125" text-anchor="middle" font-size="14">West</text>
        </g>
        <g data-node="east">
          <rect x="300" y="100" width="80" height="40" />
          <text x="340" y="125" text-anchor="middle" font-size="14">East</text>
        </g>
        <g data-node="north">
          <rect x="150" y="0" width="80" height="40" />
          <text x="190" y="25" text-anchor="middle" font-size="14">North</text>
        </g>
        <g data-node="south">
          <rect x="150" y="200" width="80" height="40" />
          <text x="190" y="225" text-anchor="middle" font-size="14">South</text>
        </g>
        <line id="across" data-from="west" data-to="east"
          x1="80" y1="120" x2="300" y2="120" />
        <line id="down" data-from="north" data-to="south"
          x1="190" y1="40" x2="190" y2="200" />
      </svg>
    "#;

    assert!(!has_issue(&analyze(svg), "connector-overlap"));
}

fn tied_label(label_y: u32) -> String {
    format!(
        r#"
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
        x="190" y="{label_y}" text-anchor="middle" font-size="14">Retry</text>
    </svg>
  "#
    )
}

/// Given a label tied to a connector by data-label-for, 45px above it
/// When the diagram is analyzed
/// Then the label is reported as detached from that connector
#[test]
fn reports_a_tied_label_that_sits_away_from_its_connector() {
    let report = analyze(&tied_label(0));

    assert_eq!(issues_with_code(&report, "label-detached"), [["retry", "flow"]]);
}

/// Given a label tied to a connector, keeping 9px above it
/// When the diagram is analyzed
/// Then no detached label is reported
#[test]
fn accepts_a_tied_label_right_beside_its_connector() {
    assert!(!has_issue(&analyze(&tied_label(36)), "label-detached"));
}

/// Given two circular nodes: one whose two-line label fits its bounding
///   box with padding but reaches within 12px of the circle, and one with
///   a short label near its centre
/// When the diagram is analyzed
/// Then only the first is reported as overflowing
#[test]
fn reports_a_label_that_cuts_into_a_circular_node_s_padding() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 360 200">
        <g data-node="crowded">
          <circle cx="80" cy="80" r="50" />
          <text x="80" y="77" text-anchor="middle" font-size="14">Received</text>
          <text x="80" y="97" text-anchor="middle" font-size="14">Received</text>
        </g>
        <g data-node="roomy">
          <circle cx="240" cy="80" r="50" />
          <text x="240" y="85" text-anchor="middle" font-size="14">Start</text>
        </g>
      </svg>
    "#;

    assert_eq!(issues_with_code(&analyze(svg), "text-overflow"), [["crowded"]]);
}

/// Given a circular node, an L-shaped connector that turns inside a corner
///   of the circle's bounding box without touching the circle, and a
///   straight connector through the circle's centre
/// When the diagram is analyzed
/// Then only the straight connector is reported as crossing the node
#[test]
fn reports_connectors_through_a_circle_but_not_past_its_bounding_corners() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="-20 -100 400 280">
        <g data-node="stop">
          <circle cx="200" cy="80" r="40" />
          <text x="200" y="85" text-anchor="middle" font-size="14">Stop</text>
        </g>
        <g data-node="north">
          <rect x="140" y="-76" width="50" height="56" />
          <text x="165" y="-43" text-anchor="middle" font-size="14">N</text>
        </g>
        <g data-node="west">
          <rect x="40" y="22" width="60" height="56" />
          <text x="70" y="55" text-anchor="middle" font-size="14">W</text>
        </g>
        <g data-node="left">
          <rect x="0" y="100" width="40" height="56" />
          <text x="20" y="133" text-anchor="middle" font-size="14">L</text>
        </g>
        <g data-node="right">
          <rect x="320" y="52" width="40" height="56" />
          <text x="340" y="85" text-anchor="middle" font-size="14">R</text>
        </g>
        <path id="corner" data-from="north" data-to="west"
          d="M 165 -20 L 165 50 L 100 50" />
        <path id="through" data-from="left" data-to="right"
          d="M 40 128 L 120 128 L 120 80 L 320 80" />
      </svg>
    "#;

    assert_eq!(
        issues_with_code(&analyze(svg), "connector-node-crossing"),
        [["through", "stop"]]
    );
}

/// Given a circular node with one connector starting on its circle, at a
///   point inside the bounding box, and another starting inside the circle
/// When the diagram is analyzed
/// Then only the connector starting inside the circle is reported
#[test]
fn accepts_endpoints_on_a_circle_and_reports_ones_inside_it() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 -60 420 220">
        <g data-node="hub">
          <circle cx="200" cy="80" r="50" />
          <text x="200" y="85" text-anchor="middle" font-size="14">Hub</text>
        </g>
        <g data-node="east">
          <rect x="320" y="-40" width="60" height="56" />
          <text x="350" y="-7" text-anchor="middle" font-size="14">E</text>
        </g>
        <path id="on-circle" data-from="hub" data-to="east"
          d="M 230 40 L 230 -12 L 320 -12" />
        <path id="inside" data-from="hub" data-to="east"
          d="M 215 60 L 290 60 L 290 0 L 320 0" />
      </svg>
    "#;

    assert_eq!(
        issues_with_code(&analyze(svg), "connector-endpoint-inside"),
        [["inside", "hub"]]
    );
}
