//! How the space a text takes is estimated: character widths, presentation
//! properties, baselines and labels spanning several lines.

mod support;

use nicevg::{AnalysisReport, Bounds};
use support::*;

/// Given a free label of six full-width Japanese characters at 14px
/// When the diagram is analyzed
/// Then the label is estimated at about one font size per character
#[test]
fn estimates_full_width_characters_at_one_font_size_each() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 300 100">
        <text data-label="pay" x="20" y="50" font-size="14">決済サービス</text>
      </svg>
    "#;

    let report = analyze(svg);

    assert!((label_bounds(&report, "pay").width - 6.0 * 14.0).abs() < 1.0);
}

/// Given a free label whose anchor and size are set only in its style
///   attribute, end-anchored at x=160 and 14px
/// When the diagram is analyzed
/// Then the label ends at x=160 and is as tall as 14px text
#[test]
fn reads_text_anchor_and_font_size_from_the_style_attribute() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 300 100">
        <text data-label="note" x="160" y="50" style="text-anchor: end; font-size: 14px">Note</text>
        <text data-label="reference" x="160" y="80" text-anchor="end" font-size="14">Note</text>
      </svg>
    "#;

    let report = analyze(svg);
    let note = label_bounds(&report, "note");

    assert_eq!(note.x + note.width, 160.0);
    assert_eq!(note.width, label_bounds(&report, "reference").width);
    assert_eq!(note.height, label_bounds(&report, "reference").height);
}

/// Given a free label inside a group that sets the anchor and size for the
///   texts in it
/// When the diagram is analyzed
/// Then the label is estimated with the group's anchor and size
#[test]
fn inherits_text_anchor_and_font_size_from_ancestor_groups() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 300 100">
        <g text-anchor="middle" style="font-size: 20px">
          <text data-label="note" x="150" y="50">Note</text>
        </g>
        <text data-label="reference" x="150" y="80" text-anchor="middle" font-size="20">Note</text>
      </svg>
    "#;

    let report = analyze(svg);
    let note = label_bounds(&report, "note");
    let reference = label_bounds(&report, "reference");

    assert_eq!(
        (note.x, note.width, note.height),
        (reference.x, reference.width, reference.height)
    );
}

/// Given free labels at y=50 with a central baseline and with a hanging one
/// When the diagram is analyzed
/// Then the central label is centred on y=50 and the hanging one hangs
///   from it
#[test]
fn places_text_vertically_by_its_dominant_baseline() {
    let svg = r#"
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 300 100">
        <text data-label="central" x="20" y="50" font-size="14" dominant-baseline="central">Central</text>
        <text data-label="hanging" x="160" y="50" font-size="14" style="dominant-baseline: hanging">Hanging</text>
      </svg>
    "#;

    let report = analyze(svg);
    let central = label_bounds(&report, "central");
    let hanging = label_bounds(&report, "hanging");

    assert!((central.y + central.height / 2.0 - 50.0).abs() <= 0.5, "{central:?}");
    assert!((hanging.y - 50.0).abs() <= 1.0, "{hanging:?}");
}

/// Separate single-line reference labels, one per line, at 14px.
fn reference_lines(lines: &[(f64, f64, &str)]) -> String {
    lines
        .iter()
        .enumerate()
        .map(|(index, (x, y, text))| {
            format!(r#"<text data-label="line-{index}" x="{x}" y="{y}" font-size="14">{text}</text>"#)
        })
        .collect()
}

fn union_of_lines(report: &AnalysisReport, count: usize) -> Bounds {
    (0..count)
        .map(|index| label_bounds(report, &format!("line-{index}")))
        .reduce(|first, second| {
            let x = first.x.min(second.x);
            let y = first.y.min(second.y);
            let right = (first.x + first.width).max(second.x + second.width);
            let bottom = (first.y + first.height).max(second.y + second.height);
            bounds(x, y, right - x, bottom - y)
        })
        .expect("at least one line")
}

const LINES: [(f64, f64, &str); 3] = [
    (40.0, 40.0, "Retry with"),
    (40.0, 57.0, "backoff, three times"),
    (40.0, 78.0, "and give up"),
];

/// Given a label written as one text whose tspans each start a new line
///   with their own x and a dy
/// When the diagram is analyzed
/// Then the label takes the space of its three lines, as separate texts
///   at the same places would
#[test]
fn estimates_a_text_with_tspan_lines_line_by_line() {
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 600 200">
          <text data-label="note" x="40" y="40" font-size="14">
            <tspan x="40">Retry with</tspan>
            <tspan x="40" dy="17">backoff, three times</tspan>
            <tspan x="40" dy="1.5em">and give up</tspan>
          </text>
          <g transform="translate(300 0)">{}</g>
        </svg>"#,
        reference_lines(&LINES)
    );

    let report = analyze(&svg);
    let lines = union_of_lines(&report, 3);

    assert_eq!(
        label_bounds(&report, "note"),
        bounds(lines.x - 300.0, lines.y, lines.width, lines.height)
    );
}

/// Given a label written as three texts that share one data-label
/// When the diagram is analyzed
/// Then they form one label taking the space of all three lines
#[test]
fn treats_texts_sharing_a_data_label_as_one_label() {
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 600 200">
          <text data-label="note" x="40" y="40" font-size="14">Retry with</text>
          <text data-label="note" x="40" y="57" font-size="14">backoff, three times</text>
          <text data-label="note" x="40" y="78" font-size="14">and give up</text>
          <g transform="translate(300 0)">{}</g>
        </svg>"#,
        reference_lines(&LINES)
    );

    let report = analyze(&svg);
    let lines = union_of_lines(&report, 3);

    assert_eq!(
        report.diagram.labels.iter().filter(|label| label.id == "note").count(),
        1
    );
    assert_eq!(
        label_bounds(&report, "note"),
        bounds(lines.x - 300.0, lines.y, lines.width, lines.height)
    );
}

/// Given a group carrying data-label around three plain texts
/// When the diagram is analyzed
/// Then the group is one label taking the space of all three lines
#[test]
fn treats_a_labelled_group_of_texts_as_one_label() {
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 600 200">
          <g data-label="note">
            <text x="40" y="40" font-size="14">Retry with</text>
            <text x="40" y="57" font-size="14">backoff, three times</text>
            <text x="40" y="78" font-size="14">and give up</text>
          </g>
          <g transform="translate(300 0)">{}</g>
        </svg>"#,
        reference_lines(&LINES)
    );

    let report = analyze(&svg);
    let lines = union_of_lines(&report, 3);

    assert_eq!(
        label_bounds(&report, "note"),
        bounds(lines.x - 300.0, lines.y, lines.width, lines.height)
    );
}
