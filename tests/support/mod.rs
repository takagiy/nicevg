//! Shared helpers for the integration tests: geometry predicates for
//! asserting what a fixed diagram means rather than its exact coordinates,
//! readers for the written SVG, and recording for the fix gallery.
#![allow(dead_code)]

use std::collections::HashMap;

use nicevg::{AnalysisReport, Bounds, FixResult, Point};
use serde::Serialize;

pub const fn point(x: f64, y: f64) -> Point {
    Point { x, y }
}

pub const fn bounds(x: f64, y: f64, width: f64, height: f64) -> Bounds {
    Bounds { x, y, width, height }
}

pub fn analyze(svg: &str) -> AnalysisReport {
    nicevg::analyze(svg).expect("valid SVG")
}

/// Fixes the SVG and, when NICEVG_RECORD_DIR is set, records the call for
/// the fix gallery under the running test's name.
pub fn fix(svg: &str) -> FixResult {
    record(svg, nicevg::fix(svg).expect("valid SVG"))
}

pub fn fix_with_passes(svg: &str, passes: usize) -> FixResult {
    record(svg, nicevg::fix_with_passes(svg, passes).expect("valid SVG"))
}

pub fn arrange(svg: &str) -> FixResult {
    record(svg, nicevg::arrange(svg).expect("valid SVG"))
}

fn record(input: &str, result: FixResult) -> FixResult {
    if let Ok(directory) = std::env::var("NICEVG_RECORD_DIR") {
        let test = std::thread::current()
            .name()
            .unwrap_or("unknown")
            .rsplit("::")
            .next()
            .unwrap_or("unknown")
            .to_owned();
        let file = std::path::Path::new(&directory).join(format!("{test}.jsonl"));
        let call = serde_json::json!({
            "test": test,
            "input": input,
            "output": result.svg,
            "changes": result.changes,
            "before": nicevg::analyze(input).ok(),
            "after": result.report,
        });
        std::fs::create_dir_all(&directory).expect("record directory");
        let mut line = serde_json::to_string(&call).expect("serializable record");
        line.push('\n');
        use std::io::Write;
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(file)
            .and_then(|mut handle| handle.write_all(line.as_bytes()))
            .expect("record written");
    }
    result
}

/// Snapshots a fixed diagram's quality and its written SVG, named after the
/// test, so any change in shape shows up as a snapshot diff.
#[macro_export]
macro_rules! assert_fix_snapshots {
    ($result:expr) => {{
        let result = &$result;
        insta::assert_json_snapshot!(support::quality_of(&result.report));
        insta::assert_snapshot!(result.svg.as_str());
    }};
}

pub fn connector_points(report: &AnalysisReport, id: &str) -> Vec<Point> {
    report
        .diagram
        .connectors
        .iter()
        .find(|connector| connector.id == id)
        .map(|connector| connector.points.clone())
        .unwrap_or_default()
}

pub fn node_bounds(report: &AnalysisReport, id: &str) -> Bounds {
    report
        .diagram
        .nodes
        .iter()
        .find(|node| node.id == id)
        .map(|node| node.bounds)
        .expect("node in report")
}

pub fn label_bounds(report: &AnalysisReport, id: &str) -> Bounds {
    report
        .diagram
        .labels
        .iter()
        .find(|label| label.id == id)
        .map(|label| label.bounds)
        .expect("label in report")
}

pub fn issue_codes(report: &AnalysisReport) -> Vec<String> {
    report.issues.iter().map(|issue| issue.code.clone()).collect()
}

pub fn has_issue(report: &AnalysisReport, code: &str) -> bool {
    report.issues.iter().any(|issue| issue.code == code)
}

pub fn change_codes(result: &FixResult) -> Vec<String> {
    result.changes.iter().map(|change| change.code.clone()).collect()
}

/// Issues as JSON, to compare with expected values written as `json!`.
pub fn issues_json(report: &AnalysisReport) -> Vec<serde_json::Value> {
    report
        .issues
        .iter()
        .map(|issue| serde_json::to_value(issue).expect("serializable issue"))
        .collect()
}

pub fn issues_with_code(report: &AnalysisReport, code: &str) -> Vec<Vec<String>> {
    report
        .issues
        .iter()
        .filter(|issue| issue.code == code)
        .map(|issue| issue.elements.clone())
        .collect()
}

pub fn segments(points: &[Point]) -> Vec<(Point, Point)> {
    points.windows(2).map(|pair| (pair[0], pair[1])).collect()
}

pub fn is_orthogonal(points: &[Point]) -> bool {
    points.len() >= 2
        && segments(points)
            .iter()
            .all(|(from, to)| from.x == to.x || from.y == to.y)
}

pub fn bend_count(points: &[Point]) -> usize {
    points.len().saturating_sub(2)
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Side {
    Top,
    Right,
    Bottom,
    Left,
}

pub fn side_of(target: Point, box_: &Bounds) -> Option<Side> {
    let within_x = target.x >= box_.x && target.x <= box_.x + box_.width;
    let within_y = target.y >= box_.y && target.y <= box_.y + box_.height;
    if within_x && target.y == box_.y {
        Some(Side::Top)
    } else if within_x && target.y == box_.y + box_.height {
        Some(Side::Bottom)
    } else if within_y && target.x == box_.x {
        Some(Side::Left)
    } else if within_y && target.x == box_.x + box_.width {
        Some(Side::Right)
    } else {
        None
    }
}

/// Whether the segment from `port` towards `next` heads straight out of the
/// node side the port sits on.
fn heads_outward(port: Option<&Point>, next: Option<&Point>, box_: &Bounds) -> bool {
    let (Some(port), Some(next)) = (port, next) else {
        return false;
    };
    match side_of(*port, box_) {
        Some(Side::Top) => next.x == port.x && next.y < port.y,
        Some(Side::Bottom) => next.x == port.x && next.y > port.y,
        Some(Side::Left) => next.y == port.y && next.x < port.x,
        Some(Side::Right) => next.y == port.y && next.x > port.x,
        None => false,
    }
}

pub fn leaves_perpendicularly(points: &[Point], box_: &Bounds) -> bool {
    heads_outward(points.first(), points.get(1), box_)
}

pub fn enters_perpendicularly(points: &[Point], box_: &Bounds) -> bool {
    heads_outward(
        points.last(),
        points.len().checked_sub(2).and_then(|index| points.get(index)),
        box_,
    )
}

fn shared_length(a1: f64, a2: f64, b1: f64, b2: f64) -> f64 {
    a1.max(a2).min(b1.max(b2)) - a1.min(a2).max(b1.min(b2))
}

fn collinear_overlap((a, b): &(Point, Point), (c, d): &(Point, Point)) -> bool {
    (a.y == b.y && c.y == d.y && a.y == c.y && shared_length(a.x, b.x, c.x, d.x) > 0.0)
        || (a.x == b.x && c.x == d.x && a.x == c.x && shared_length(a.y, b.y, c.y, d.y) > 0.0)
}

fn outline(box_: &Bounds) -> Vec<(Point, Point)> {
    let right = box_.x + box_.width;
    let bottom = box_.y + box_.height;
    vec![
        (point(box_.x, box_.y), point(right, box_.y)),
        (point(right, box_.y), point(right, bottom)),
        (point(box_.x, bottom), point(right, bottom)),
        (point(box_.x, box_.y), point(box_.x, bottom)),
    ]
}

pub fn runs_along(points: &[Point], box_: &Bounds) -> bool {
    segments(points)
        .iter()
        .any(|segment| outline(box_).iter().any(|side| collinear_overlap(segment, side)))
}

pub fn overlaps_route(first: &[Point], second: &[Point]) -> bool {
    let others = segments(second);
    segments(first)
        .iter()
        .any(|segment| others.iter().any(|other| collinear_overlap(segment, other)))
}

/// Whether a horizontal and a vertical segment of the two routes pass
/// through each other.
pub fn crosses_route(first: &[Point], second: &[Point]) -> bool {
    let others = segments(second);
    let crosses = |(a, b): &(Point, Point), (c, d): &(Point, Point)| {
        a.y == b.y && c.x == d.x && a.x.min(b.x) < c.x && c.x < a.x.max(b.x) && c.y.min(d.y) < a.y && a.y < c.y.max(d.y)
    };
    segments(first).iter().any(|segment| {
        others
            .iter()
            .any(|other| crosses(segment, other) || crosses(other, segment))
    })
}

/// Pairs of parallel segments from different connectors running closer
/// than `distance` (but not on the same line) for at least 20px.
pub fn crowded_pairs(report: &AnalysisReport, distance: f64) -> usize {
    let routes: Vec<Vec<(Point, Point)>> = report
        .diagram
        .connectors
        .iter()
        .map(|connector| segments(&connector.points))
        .collect();
    let close = |(a, b): &(Point, Point), (c, d): &(Point, Point)| {
        let span = |p: f64, q: f64| (p.min(q), p.max(q));
        let (gap, (low, high), (other_low, other_high)) = if a.y == b.y && c.y == d.y {
            ((a.y - c.y).abs(), span(a.x, b.x), span(c.x, d.x))
        } else if a.x == b.x && c.x == d.x {
            ((a.x - c.x).abs(), span(a.y, b.y), span(c.y, d.y))
        } else {
            return false;
        };
        gap > 0.0 && gap < distance && high.min(other_high) - low.max(other_low) >= 20.0
    };
    routes
        .iter()
        .enumerate()
        .flat_map(|(index, first)| {
            routes[index + 1..].iter().map(move |second| {
                first
                    .iter()
                    .map(|a| second.iter().filter(|b| close(a, b)).count())
                    .sum::<usize>()
            })
        })
        .sum()
}

/// The shortest first or last segment among the connectors ending at a node.
pub fn shortest_end_segment(report: &AnalysisReport, node: &str) -> f64 {
    report
        .diagram
        .connectors
        .iter()
        .flat_map(|c| {
            let p = &c.points;
            let length = |a: Point, b: Point| (a.x - b.x).abs() + (a.y - b.y).abs();
            [
                (c.from == node).then(|| length(p[0], p[1])),
                (c.to == node).then(|| length(p[p.len() - 1], p[p.len() - 2])),
            ]
        })
        .flatten()
        .fold(f64::INFINITY, f64::min)
}

/// The nearest point to a node, along a connector's end segment there, where
/// another connector crosses that segment.
pub fn nearest_crossing_at_end(report: &AnalysisReport, node: &str) -> f64 {
    let connectors = &report.diagram.connectors;
    connectors
        .iter()
        .flat_map(|c| {
            let p = &c.points;
            [
                (c.from == node).then(|| (p[0], p[1])),
                (c.to == node).then(|| (p[p.len() - 1], p[p.len() - 2])),
            ]
            .into_iter()
            .flatten()
            .map(move |segment| (c.id.clone(), segment))
        })
        .flat_map(|(id, (end, inner))| {
            connectors
                .iter()
                .filter(move |other| other.id != id)
                .flat_map(move |other| {
                    segments(&other.points).into_iter().filter_map(move |(a, b)| {
                        // Only perpendicular crossings of axis-aligned segments.
                        if end.y == inner.y && a.x == b.x {
                            let (low, high) = (end.x.min(inner.x), end.x.max(inner.x));
                            (low < a.x && a.x < high && a.y.min(b.y) < end.y && end.y < a.y.max(b.y))
                                .then(|| (a.x - end.x).abs())
                        } else if end.x == inner.x && a.y == b.y {
                            let (low, high) = (end.y.min(inner.y), end.y.max(inner.y));
                            (low < a.y && a.y < high && a.x.min(b.x) < end.x && end.x < a.x.max(b.x))
                                .then(|| (a.y - end.y).abs())
                        } else {
                            None
                        }
                    })
                })
        })
        .fold(f64::INFINITY, f64::min)
}

/// Pairs of connectors with segments that come within 5px of each other
/// without meeting: they read as touching or as one line.
pub fn near_misses(report: &AnalysisReport) -> Vec<(String, String)> {
    let connectors = &report.diagram.connectors;
    let span = |a: f64, b: f64| (a.min(b), a.max(b));
    connectors
        .iter()
        .enumerate()
        .flat_map(|(index, first)| connectors[index + 1..].iter().map(move |second| (first, second)))
        .filter(|(first, second)| {
            segments(&first.points).iter().any(|(a, b)| {
                segments(&second.points).iter().any(|(c, d)| {
                    let ((left, right), (top, bottom)) = (span(a.x, b.x), span(a.y, b.y));
                    let ((other_left, other_right), (other_top, other_bottom)) = (span(c.x, d.x), span(c.y, d.y));
                    let dx = (other_left - right).max(left - other_right).max(0.0);
                    let dy = (other_top - bottom).max(top - other_bottom).max(0.0);
                    let gap = dx.hypot(dy);
                    gap > 0.0 && gap < 5.0
                })
            })
        })
        .map(|(first, second)| (first.id.clone(), second.id.clone()))
        .collect()
}

pub fn inflate(box_: &Bounds, amount: f64) -> Bounds {
    bounds(
        box_.x - amount,
        box_.y - amount,
        box_.width + amount * 2.0,
        box_.height + amount * 2.0,
    )
}

/// Whether any (orthogonal) segment passes through the open interior of the box.
pub fn enters_box(points: &[Point], box_: &Bounds) -> bool {
    segments(points).iter().any(|(from, to)| {
        from.x.max(to.x) > box_.x
            && from.x.min(to.x) < box_.x + box_.width
            && from.y.max(to.y) > box_.y
            && from.y.min(to.y) < box_.y + box_.height
    })
}

pub fn contains(outer: &Bounds, inner: &Bounds) -> bool {
    inner.x >= outer.x
        && inner.y >= outer.y
        && inner.x + inner.width <= outer.x + outer.width
        && inner.y + inner.height <= outer.y + outer.height
}

/// Empty space between two boxes along the axis that separates them.
pub fn gap_between(first: &Bounds, second: &Bounds) -> f64 {
    [
        second.x - (first.x + first.width),
        first.x - (second.x + second.width),
        second.y - (first.y + first.height),
        first.y - (second.y + second.height),
    ]
    .into_iter()
    .fold(f64::NEG_INFINITY, f64::max)
}

pub fn segment_length((from, to): &(Point, Point)) -> f64 {
    (to.x - from.x).abs() + (to.y - from.y).abs()
}

pub fn longest_segment(points: &[Point]) -> (Point, Point) {
    segments(points)
        .into_iter()
        .reduce(|longest, segment| {
            if segment_length(&segment) > segment_length(&longest) {
                segment
            } else {
                longest
            }
        })
        .expect("a route needs two points")
}

/// Shortest distance between a box and any segment of an orthogonal route.
pub fn distance_to_route(box_: &Bounds, points: &[Point]) -> f64 {
    segments(points)
        .iter()
        .map(|(from, to)| {
            let dx = 0.0_f64
                .max(box_.x - from.x.max(to.x))
                .max(from.x.min(to.x) - (box_.x + box_.width));
            let dy = 0.0_f64
                .max(box_.y - from.y.max(to.y))
                .max(from.y.min(to.y) - (box_.y + box_.height));
            dx.hypot(dy)
        })
        .fold(f64::INFINITY, f64::min)
}

/// Distance of a point from the circle inscribed in a circular node's bounds.
pub fn distance_from_circle(target: Point, box_: &Bounds) -> f64 {
    ((target.x - (box_.x + box_.width / 2.0)).hypot(target.y - (box_.y + box_.height / 2.0)) - box_.width / 2.0).abs()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Quality {
    pub bends: usize,
    pub detached_labels: usize,
    pub diagonal_segments: usize,
    /// How crowded connectors running side by side are: 1000 / d² summed
    /// over each pair of parallel segments, from different connectors, that
    /// run alongside each other for a stretch d apart. Rounded to hundredths.
    pub parallel_density: f64,
    pub remaining_issues: usize,
}

/// Quality of a fixed diagram, for snapshot assertions that catch
/// regressions the per-test expectations do not pin down.
pub fn quality_of(report: &AnalysisReport) -> Quality {
    let connectors = &report.diagram.connectors;
    Quality {
        bends: connectors.iter().map(|connector| bend_count(&connector.points)).sum(),
        detached_labels: report
            .diagram
            .labels
            .iter()
            .filter(|label| {
                connectors
                    .iter()
                    .find(|connector| Some(&connector.id) == label.connector.as_ref())
                    .is_some_and(|connector| distance_to_route(&label.bounds, &connector.points) > 16.0)
            })
            .count(),
        diagonal_segments: connectors
            .iter()
            .map(|connector| {
                segments(&connector.points)
                    .iter()
                    .filter(|(a, b)| a.x != b.x && a.y != b.y)
                    .count()
            })
            .sum(),
        parallel_density: (parallel_density(report) * 100.0).round() / 100.0,
        remaining_issues: report.issues.len(),
    }
}

/// 1000 / d² summed over each pair of parallel segments from different
/// connectors whose spans along their direction overlap for a stretch,
/// d apart. Segments on one line (d = 0) are overlaps, reported as issues.
pub fn parallel_density(report: &AnalysisReport) -> f64 {
    let connectors = &report.diagram.connectors;
    connectors
        .iter()
        .enumerate()
        .flat_map(|(index, first)| connectors[index + 1..].iter().map(move |second| (first, second)))
        .flat_map(|(first, second)| {
            let others = segments(&second.points);
            segments(&first.points)
                .into_iter()
                .flat_map(move |(a, b)| others.clone().into_iter().map(move |(c, d)| (a, b, c, d)))
        })
        .filter_map(|(a, b, c, d)| {
            let span = |p: f64, q: f64| (p.min(q), p.max(q));
            let (distance, (low, high), (other_low, other_high)) =
                if a.y == b.y && c.y == d.y && a.x != b.x && c.x != d.x {
                    ((a.y - c.y).abs(), span(a.x, b.x), span(c.x, d.x))
                } else if a.x == b.x && c.x == d.x && a.y != b.y && c.y != d.y {
                    ((a.x - c.x).abs(), span(a.y, b.y), span(c.y, d.y))
                } else {
                    return None;
                };
            (distance > 0.0 && high.min(other_high) > low.max(other_low)).then(|| 1000.0 / (distance * distance))
        })
        .sum()
}

/// Attributes of the elements of the written SVG, by element id or by tag
/// name and position.
pub struct Written {
    elements: Vec<(String, HashMap<String, String>)>,
}

impl Written {
    pub fn parse(svg: &str) -> Written {
        let document = roxmltree::Document::parse(svg).expect("written SVG parses");
        Written {
            elements: document
                .descendants()
                .filter(|node| node.is_element())
                .map(|node| {
                    let attributes = node
                        .attributes()
                        .map(|attribute| (attribute.name().to_owned(), attribute.value().to_owned()));
                    (node.tag_name().name().to_owned(), attributes.collect())
                })
                .collect(),
        }
    }

    pub fn by_id(&self, id: &str) -> (&str, &HashMap<String, String>) {
        self.elements
            .iter()
            .find(|(_, attributes)| attributes.get("id").map(String::as_str) == Some(id))
            .map(|(name, attributes)| (name.as_str(), attributes))
            .expect("element with id")
    }

    pub fn nth(&self, tag: &str, index: usize) -> &HashMap<String, String> {
        self.elements
            .iter()
            .filter(|(name, _)| name == tag)
            .nth(index)
            .map(|(_, attributes)| attributes)
            .expect("element")
    }

    pub fn root(&self) -> &HashMap<String, String> {
        &self.elements[0].1
    }
}

pub fn attr<'a>(attributes: &'a HashMap<String, String>, name: &str) -> &'a str {
    attributes.get(name).map_or("", String::as_str)
}

pub fn view_box(attributes: &HashMap<String, String>) -> Bounds {
    let values: Vec<f64> = attr(attributes, "viewBox")
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter_map(|v| v.parse().ok())
        .collect();
    bounds(values[0], values[1], values[2], values[3])
}

/// A length attribute split into its number and unit, as in `200px`.
pub fn split_length(text: &str) -> (f64, String) {
    let unit_at = text
        .find(|c: char| c.is_ascii_alphabetic() || c == '%')
        .unwrap_or(text.len());
    (
        text[..unit_at].trim().parse().expect("numeric length"),
        text[unit_at..].trim().to_owned(),
    )
}
