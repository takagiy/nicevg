//! Diagrams given as shapes rather than SVG: nodes and connectors as plain
//! data. A sketch is written out as the SVG it stands for, following the
//! SVG contract, and read and repaired like any other diagram.

use std::collections::{HashMap, HashSet};
use std::fmt::Write;

use serde::Deserialize;

use crate::geometry::{Point, format_number, point};
use crate::{AnalysisReport, FixResult};

/// A diagram as nodes and the connectors between them.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Sketch {
    pub nodes: Vec<SketchNode>,
    #[serde(default)]
    pub connectors: Vec<SketchConnector>,
}

/// A box or a circle; a circle's bounds are the square around it.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SketchNode {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    #[serde(default)]
    pub shape: SketchShape,
    /// Its text, lines split at `\n`; a node with children shows it at its
    /// top left, any other node at its centre.
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub font_size: Option<f64>,
    /// The node this one sits inside.
    #[serde(default)]
    pub parent: Option<String>,
    /// Whether it may overlap other nodes on purpose.
    #[serde(default)]
    pub allow_overlap: bool,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SketchShape {
    #[default]
    Rect,
    Circle,
}

/// A connector from one node to another, along `points` when given. Without
/// them it runs from centre to centre, its ends inside the nodes, which
/// analysis reports and fixing reroutes from side to side.
///
/// Connectors have no id of their own: in the diagram read back, their
/// `from` and `to` tell them apart. They are named `connector-0`,
/// `connector-1` and so on there, for their labels to refer to.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SketchConnector {
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub points: Vec<Point>,
    /// Its text, lines split at `\n`, placed beside it.
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub font_size: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SketchError {
    /// The field at `referrer` (such as `connectors[0].to` or
    /// `nodes[2].parent`) names a node the sketch does not have.
    UnknownNode { referrer: String, node: String },
}

impl std::fmt::Display for SketchError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SketchError::UnknownNode { referrer, node } => {
                write!(
                    formatter,
                    "{referrer} refers to node \"{node}\", which the sketch does not have"
                )
            }
        }
    }
}

impl std::error::Error for SketchError {}

const NODE_FONT_SIZE: f64 = 14.0;
const LABEL_FONT_SIZE: f64 = 12.0;
const LINE_HEIGHT: f64 = 1.2;
/// How far a container's title sits in from its top left corner.
const TITLE_INSET: f64 = 12.0;
const MARGIN: f64 = 20.0;

pub fn analyze_sketch(sketch: &Sketch) -> Result<AnalysisReport, SketchError> {
    Ok(crate::analyze(&sketch_svg(sketch)?).expect("a sketch's SVG parses"))
}

pub fn fix_sketch(sketch: &Sketch) -> Result<FixResult, SketchError> {
    Ok(crate::fix(&sketch_svg(sketch)?).expect("a sketch's SVG parses"))
}

pub fn arrange_sketch(sketch: &Sketch) -> Result<FixResult, SketchError> {
    Ok(crate::arrange(&sketch_svg(sketch)?).expect("a sketch's SVG parses"))
}

/// The SVG a sketch stands for: a `data-node` group for each node, with
/// its children nested in it, and a `data-from`/`data-to` path for each
/// connector, its label a `data-label-for` text.
pub fn sketch_svg(sketch: &Sketch) -> Result<String, SketchError> {
    let nodes: HashMap<&str, &SketchNode> = sketch.nodes.iter().map(|node| (node.id.as_str(), node)).collect();
    for (index, node) in sketch.nodes.iter().enumerate() {
        if let Some(parent) = node.parent.as_deref().filter(|parent| !nodes.contains_key(parent)) {
            return Err(unknown(format!("nodes[{index}].parent"), parent));
        }
    }
    for (index, connector) in sketch.connectors.iter().enumerate() {
        for (field, end) in [("from", &connector.from), ("to", &connector.to)] {
            if !nodes.contains_key(end.as_str()) {
                return Err(unknown(format!("connectors[{index}].{field}"), end));
            }
        }
    }

    let mut children: HashMap<&str, Vec<&SketchNode>> = HashMap::new();
    for node in &sketch.nodes {
        if let Some(parent) = &node.parent {
            children.entry(parent.as_str()).or_default().push(node);
        }
    }
    let routes: Vec<Vec<Point>> = sketch
        .connectors
        .iter()
        .map(|connector| {
            if connector.points.len() >= 2 {
                connector.points.clone()
            } else {
                let (from, to) = (nodes[connector.from.as_str()], nodes[connector.to.as_str()]);
                vec![centre(from), centre(to)]
            }
        })
        .collect();

    let mut body = String::new();
    let mut written: HashSet<&str> = HashSet::new();
    // Nodes outside any other first, each with its children inside; nodes
    // in a cycle of parents last, outside one another.
    for node in sketch.nodes.iter().filter(|node| node.parent.is_none()) {
        write_node(&mut body, node, &children, &mut written, 1);
    }
    for node in &sketch.nodes {
        if !written.contains(node.id.as_str()) {
            write_node(&mut body, node, &children, &mut written, 1);
        }
    }
    for (index, (connector, route)) in sketch.connectors.iter().zip(&routes).enumerate() {
        let id = connector_id(index);
        let d = route
            .iter()
            .enumerate()
            .map(|(index, p)| format!("{} {} {}", if index == 0 { "M" } else { "L" }, number(p.x), number(p.y)))
            .collect::<Vec<_>>()
            .join(" ");
        let _ = writeln!(
            body,
            r#"  <path id="{id}" data-from="{}" data-to="{}" fill="none" stroke="black" d="{d}"/>"#,
            escape(&connector.from),
            escape(&connector.to)
        );
        if let Some(label) = &connector.label {
            let (first, last) = (route[0], route[route.len() - 1]);
            let at = point((first.x + last.x) / 2.0, (first.y + last.y) / 2.0);
            let font_size = connector.font_size.unwrap_or(LABEL_FONT_SIZE);
            let _ = writeln!(
                body,
                r#"  <text data-label="{id}" data-label-for="{id}" text-anchor="middle" font-size="{}">{}</text>"#,
                number(font_size),
                lines(label, at.x, at.y, font_size),
            );
        }
    }

    let points = sketch
        .nodes
        .iter()
        .flat_map(|node| [point(node.x, node.y), point(node.x + node.width, node.y + node.height)])
        .chain(routes.iter().flatten().copied());
    let (mut left, mut top, mut right, mut bottom) =
        (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
    for p in points {
        (left, top, right, bottom) = (left.min(p.x), top.min(p.y), right.max(p.x), bottom.max(p.y));
    }
    if !left.is_finite() {
        (left, top, right, bottom) = (0.0, 0.0, 0.0, 0.0);
    }
    let (x, y) = (left - MARGIN, top - MARGIN);
    let (width, height) = (right - left + 2.0 * MARGIN, bottom - top + 2.0 * MARGIN);
    Ok(format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{} {} {} {}\" width=\"{}\" height=\"{}\">\n{body}</svg>\n",
        number(x),
        number(y),
        number(width),
        number(height),
        number(width),
        number(height)
    ))
}

/// The id of the connector at `index` in a sketch, as the diagram read back
/// names it, and the label describing it.
fn connector_id(index: usize) -> String {
    format!("connector-{index}")
}

fn unknown(referrer: String, node: &str) -> SketchError {
    SketchError::UnknownNode {
        referrer,
        node: node.to_owned(),
    }
}

fn write_node<'a>(
    body: &mut String,
    node: &'a SketchNode,
    children: &HashMap<&str, Vec<&'a SketchNode>>,
    written: &mut HashSet<&'a str>,
    depth: usize,
) {
    if !written.insert(node.id.as_str()) {
        return;
    }
    let indent = "  ".repeat(depth);
    let _ = writeln!(
        body,
        "{indent}<g data-node=\"{}\"{}>",
        escape(&node.id),
        if node.allow_overlap {
            r#" data-allow-overlap="true""#
        } else {
            ""
        }
    );
    match node.shape {
        SketchShape::Rect => {
            let _ = writeln!(
                body,
                r#"{indent}  <rect x="{}" y="{}" width="{}" height="{}" fill="white" stroke="black"/>"#,
                number(node.x),
                number(node.y),
                number(node.width),
                number(node.height)
            );
        }
        SketchShape::Circle => {
            let centre = centre(node);
            let _ = writeln!(
                body,
                r#"{indent}  <circle cx="{}" cy="{}" r="{}" fill="white" stroke="black"/>"#,
                number(centre.x),
                number(centre.y),
                number(node.width.min(node.height) / 2.0)
            );
        }
    }
    let inside = children.get(node.id.as_str());
    if let Some(label) = &node.label {
        let font_size = node.font_size.unwrap_or(NODE_FONT_SIZE);
        if inside.is_some() {
            let _ = writeln!(
                body,
                r#"{indent}  <text font-size="{}">{}</text>"#,
                number(font_size),
                lines(
                    label,
                    node.x + TITLE_INSET,
                    node.y + TITLE_INSET + font_size * 0.35 + font_size / 2.0,
                    font_size
                )
            );
        } else {
            let centre = centre(node);
            let _ = writeln!(
                body,
                r#"{indent}  <text text-anchor="middle" font-size="{}">{}</text>"#,
                number(font_size),
                lines(label, centre.x, centre.y, font_size)
            );
        }
    }
    for child in inside.into_iter().flatten() {
        write_node(body, child, children, written, depth + 1);
    }
    let _ = writeln!(body, "{indent}</g>");
}

/// A label's lines as `tspan`s, centred on `y` as a block.
fn lines(text: &str, x: f64, y: f64, font_size: f64) -> String {
    let lines: Vec<&str> = text.split('\n').collect();
    let first = y - (lines.len() - 1) as f64 * font_size * LINE_HEIGHT / 2.0 + font_size * 0.35;
    lines
        .iter()
        .enumerate()
        .map(|(index, line)| {
            format!(
                r#"<tspan x="{}" y="{}">{}</tspan>"#,
                number(x),
                number(first + index as f64 * font_size * LINE_HEIGHT),
                escape(line)
            )
        })
        .collect()
}

fn centre(node: &SketchNode) -> Point {
    point(node.x + node.width / 2.0, node.y + node.height / 2.0)
}

fn number(value: f64) -> String {
    format_number(value)
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
