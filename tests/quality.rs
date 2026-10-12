//! The quality measures the snapshots record: each counts or sums something
//! that reads worse, so a regression always shows as a larger number, and
//! one more of a bad thing never makes a measure look better.

mod support;

use support::*;

/// An SVG with boxes `(id, x, y, width, height)` and connectors
/// `(id, from, to, path data)`.
fn diagram(nodes: &[(&str, f64, f64, f64, f64)], connectors: &[(&str, &str, &str, &str)]) -> String {
    let nodes: String = nodes
        .iter()
        .map(|(id, x, y, width, height)| {
            format!(r#"<g data-node="{id}"><rect x="{x}" y="{y}" width="{width}" height="{height}"/></g>"#)
        })
        .collect();
    let connectors: String = connectors
        .iter()
        .map(|(id, from, to, d)| format!(r#"<path id="{id}" data-from="{from}" data-to="{to}" d="{d}"/>"#))
        .collect();
    format!(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 600 400">{nodes}{connectors}</svg>"#)
}

fn quality(svg: &str) -> Quality {
    quality_of(&analyze(svg))
}

const TWO_BOXES: [(&str, f64, f64, f64, f64); 2] = [("a", 20.0, 20.0, 80.0, 40.0), ("b", 400.0, 20.0, 80.0, 40.0)];

/// Given a connector that steps sideways by 5px between two runs heading the same way
/// When its quality is measured
/// Then it has one jog, falling 15px short of a clear 20px step
#[test]
fn measures_how_far_a_jog_falls_short_of_a_clear_step() {
    let svg = diagram(&TWO_BOXES, &[("f", "a", "b", "M 100 40 L 250 40 L 250 45 L 400 45")]);

    let quality = quality(&svg);

    assert_eq!((quality.jogs, quality.jog_shortfall), (1, 15.0));
}

/// Given one connector with a 10px jog, and the same connector with a second 10px jog
/// When their qualities are measured
/// Then the second jog adds to the shortfall instead of lowering any average
#[test]
fn counts_each_further_jog_as_worse() {
    let one = diagram(&TWO_BOXES, &[("f", "a", "b", "M 100 40 L 250 40 L 250 50 L 400 50")]);
    let two = diagram(
        &TWO_BOXES,
        &[("f", "a", "b", "M 100 40 L 200 40 L 200 50 L 300 50 L 300 40 L 400 40")],
    );

    let (one, two) = (quality(&one), quality(&two));

    assert_eq!((one.jogs, two.jogs), (1, 2));
    assert!(two.jog_shortfall > one.jog_shortfall);
}

/// Given a connector whose short middle segment turns back, a U and not a step
/// When its quality is measured
/// Then it has no jog
#[test]
fn does_not_count_a_short_turn_back_as_a_jog() {
    let svg = diagram(
        &[("a", 20.0, 20.0, 80.0, 40.0), ("b", 20.0, 80.0, 80.0, 40.0)],
        &[("f", "a", "b", "M 100 40 L 140 40 L 140 50 L 100 50 L 100 100")],
    );

    let quality = quality(&svg);

    assert_eq!((quality.jogs, quality.jog_shortfall), (0, 0.0));
}

/// Given two connectors that cross once in the open, far from their ends
/// When their quality is measured
/// Then there is one crossing and none near an end
#[test]
fn counts_a_crossing_away_from_the_ends() {
    let svg = diagram(
        &[
            ("a", 20.0, 140.0, 80.0, 40.0),
            ("b", 400.0, 140.0, 80.0, 40.0),
            ("c", 210.0, 20.0, 80.0, 40.0),
            ("d", 210.0, 300.0, 80.0, 40.0),
        ],
        &[
            ("h", "a", "b", "M 100 160 L 400 160"),
            ("v", "c", "d", "M 250 60 L 250 300"),
        ],
    );

    let quality = quality(&svg);

    assert_eq!((quality.crossings, quality.crossings_near_ends), (1, 0));
}

/// Given two connectors that cross 10px from where one leaves its node
/// When their quality is measured
/// Then the crossing also counts as one near an end
#[test]
fn counts_a_crossing_beside_a_node_as_near_an_end() {
    let svg = diagram(
        &[
            ("a", 20.0, 140.0, 80.0, 40.0),
            ("b", 400.0, 140.0, 80.0, 40.0),
            ("c", 70.0, 20.0, 80.0, 40.0),
            ("d", 70.0, 300.0, 80.0, 40.0),
        ],
        &[
            ("h", "a", "b", "M 100 160 L 400 160"),
            ("v", "c", "d", "M 110 60 L 110 300"),
        ],
    );

    let quality = quality(&svg);

    assert_eq!((quality.crossings, quality.crossings_near_ends), (1, 1));
}

/// Given a connector that ends on a node lying on another connector's line, touching it without passing over
/// When their quality is measured
/// Then they do not cross
#[test]
fn does_not_count_routes_that_only_touch_as_crossing() {
    let svg = diagram(
        &[
            ("a", 20.0, 20.0, 80.0, 40.0),
            ("b", 400.0, 20.0, 80.0, 40.0),
            ("c", 210.0, 300.0, 80.0, 40.0),
            ("e", 210.0, 0.0, 80.0, 40.0),
        ],
        &[
            ("f", "a", "b", "M 100 40 L 400 40"),
            ("g", "c", "e", "M 250 300 L 250 40"),
        ],
    );

    assert_eq!(quality(&svg).crossings, 0);
}

/// Given a connector that turns 12px after leaving its node
/// When its quality is measured
/// Then that end is short, 8px short of a clear 20px
#[test]
fn measures_how_far_a_short_end_falls_short() {
    let svg = diagram(
        &[("a", 20.0, 20.0, 80.0, 40.0), ("b", 400.0, 120.0, 80.0, 40.0)],
        &[("f", "a", "b", "M 100 40 L 112 40 L 112 140 L 400 140")],
    );

    let quality = quality(&svg);

    assert_eq!((quality.short_ends, quality.short_end_shortfall), (1, 8.0));
}

/// Given a straight connector between two nodes 12px apart
/// When its quality is measured
/// Then its ends are not short, as nothing turns near them
#[test]
fn does_not_count_a_short_straight_connector_as_short_ends() {
    let svg = diagram(
        &[("a", 20.0, 20.0, 80.0, 40.0), ("b", 112.0, 20.0, 80.0, 40.0)],
        &[("f", "a", "b", "M 100 40 L 112 40")],
    );

    assert_eq!(quality(&svg).short_ends, 0);
}

/// Given the only connector on its 40px box sides, leaving and arriving 2px from a corner
/// When its quality is measured
/// Then each end falls 8px short of half the even gap (40 / 2 / 2 = 10px), and a centred one does not
#[test]
fn measures_how_far_an_end_crowds_its_side_s_corner() {
    let crowded = diagram(&TWO_BOXES, &[("f", "a", "b", "M 100 22 L 400 22")]);
    let centred = diagram(&TWO_BOXES, &[("f", "a", "b", "M 100 40 L 400 40")]);

    assert_eq!(quality(&crowded).corner_shortfall, 16.0);
    assert_eq!(quality(&centred).corner_shortfall, 0.0);
}

/// Given a connector that goes 60px out of its way and back
/// When its quality is measured
/// Then its detour is the 120px it runs beyond the shortest orthogonal route
#[test]
fn measures_a_detour_beyond_the_shortest_orthogonal_route() {
    let svg = diagram(
        &TWO_BOXES,
        &[("f", "a", "b", "M 100 40 L 200 40 L 200 100 L 300 100 L 300 40 L 400 40")],
    );

    assert_eq!(quality(&svg).detour, 120.0);
}

/// Given one straight connector and one that bends three times
/// When their quality is measured
/// Then one connector is bent, however many times it bends
#[test]
fn counts_each_bent_connector_once() {
    let svg = diagram(
        &[
            ("a", 20.0, 20.0, 80.0, 40.0),
            ("b", 400.0, 20.0, 80.0, 40.0),
            ("c", 400.0, 120.0, 80.0, 40.0),
        ],
        &[
            ("f", "a", "b", "M 100 40 L 400 40"),
            ("g", "a", "c", "M 60 60 L 60 90 L 380 90 L 380 140 L 400 140"),
        ],
    );

    assert_eq!(quality(&svg).bent_connectors, 1);
}

/// Given two connectors that run 10px apart for a stretch and then cross, and two that only cross
/// When their quality is measured
/// Then only the first crossing is tangled with a run alongside
#[test]
fn counts_a_crossing_between_connectors_running_alongside_as_tangled() {
    let nodes = [
        ("a", 20.0, 140.0, 80.0, 40.0),
        ("b", 400.0, 140.0, 80.0, 40.0),
        ("c", 20.0, 20.0, 80.0, 40.0),
        ("d", 210.0, 300.0, 80.0, 40.0),
    ];
    let tangled = diagram(
        &nodes,
        &[
            ("h", "a", "b", "M 100 160 L 400 160"),
            ("v", "c", "d", "M 100 40 L 120 40 L 120 150 L 250 150 L 250 300"),
        ],
    );
    let plain = diagram(
        &nodes,
        &[
            ("h", "a", "b", "M 100 160 L 400 160"),
            ("v", "c", "d", "M 100 40 L 250 40 L 250 300"),
        ],
    );

    let (tangled, plain) = (quality(&tangled), quality(&plain));

    assert_eq!((tangled.crossings, tangled.tangled_crossings), (1, 1));
    assert_eq!((plain.crossings, plain.tangled_crossings), (1, 0));
}

/// Given a straight connector that gains a 10px jog, and the same connector detouring 60px around nothing
/// When their quality is measured
/// Then the jog reads worse, as people judged the paired comparisons
#[test]
fn rates_a_small_jog_worse_than_a_detour_around_nothing() {
    let jogged = diagram(&TWO_BOXES, &[("f", "a", "b", "M 100 40 L 250 40 L 250 50 L 400 50")]);
    let detoured = diagram(
        &TWO_BOXES,
        &[("f", "a", "b", "M 100 40 L 200 40 L 200 100 L 300 100 L 300 40 L 400 40")],
    );

    assert!(quality(&jogged).badness > quality(&detoured).badness);
}

/// Given a straight connector crossing another, and the same straight connector clear of it
/// When their quality is measured
/// Then the crossing raises the badness
#[test]
fn counts_a_crossing_in_the_badness() {
    let line = ("h", "a", "b", "M 100 160 L 400 160");
    let nodes = |x: f64| {
        [
            ("a", 20.0, 140.0, 80.0, 40.0),
            ("b", 400.0, 140.0, 80.0, 40.0),
            ("c", x, 20.0, 80.0, 40.0),
            ("d", x, 300.0, 80.0, 40.0),
        ]
    };
    let crossing = diagram(&nodes(210.0), &[line, ("v", "c", "d", "M 250 60 L 250 300")]);
    let clear = diagram(&nodes(500.0), &[line, ("v", "c", "d", "M 540 60 L 540 300")]);

    let (crossing, clear) = (quality(&crossing), quality(&clear));
    assert_eq!((crossing.crossings, clear.crossings), (1, 0));
    assert!(crossing.badness > clear.badness);
}

/// Given a connector turning twice, and one reaching the same ends with four turns at the same length
/// When their quality is measured
/// Then the turns past the second raise the badness
#[test]
fn rates_turns_past_the_second_worse() {
    let nodes = [("a", 20.0, 20.0, 80.0, 40.0), ("b", 400.0, 120.0, 80.0, 40.0)];
    let twice = diagram(&nodes, &[("f", "a", "b", "M 100 40 L 250 40 L 250 140 L 400 140")]);
    let four = diagram(
        &nodes,
        &[("f", "a", "b", "M 100 40 L 200 40 L 200 90 L 300 90 L 300 140 L 400 140")],
    );

    assert!(quality(&four).badness > quality(&twice).badness);
}

/// Given one connector with a 10px jog, and the same connector with a second 10px jog
/// When their quality is measured
/// Then the second jog raises the badness
#[test]
fn counts_each_further_jog_in_the_badness() {
    let one = diagram(&TWO_BOXES, &[("f", "a", "b", "M 100 40 L 250 40 L 250 50 L 400 50")]);
    let two = diagram(
        &TWO_BOXES,
        &[("f", "a", "b", "M 100 40 L 200 40 L 200 50 L 300 50 L 300 40 L 400 40")],
    );

    assert!(quality(&two).badness > quality(&one).badness);
}
