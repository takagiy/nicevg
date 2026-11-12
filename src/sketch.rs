//! Diagrams given as shapes rather than SVG: nodes and connectors as plain
//! data. A sketch is written out as the SVG it stands for, following the
//! SVG contract, and read and repaired like any other diagram.

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, LazyLock};

use regex::Regex;
use std::fmt::Write;

use serde::{Deserialize, Serialize};

use crate::arrow::shaft_end;
use crate::diagram::{custom_extent, node_group, non_empty, number_attribute, translation};
use crate::geometry::{Bounds, Point, enclosing, format_number, point};
use crate::inspect::DiagramIssue;
use crate::inspect::PARENT_PADDING;
use crate::text::text_bounds;
use crate::xml::{self, Document, Element, Node};
use crate::{AnalysisReport, FixResult};
use crate::{FixChange, analyze};

/// A diagram as nodes and the connectors between them.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Sketch {
    pub nodes: Vec<SketchNode>,
    #[serde(default)]
    pub connectors: Vec<SketchConnector>,
    /// Attributes for the root `svg`, such as `font-family`, which its
    /// texts inherit. nicevg writes `xmlns`, `viewBox`, `width` and
    /// `height` itself.
    #[serde(default)]
    pub root_attrs: BTreeMap<String, String>,
}

/// A box or a circle; a circle's bounds are the square around it. A node
/// with `svg` is that picture instead, drawn with its origin at `x`, `y`:
/// its bounds are the box around what it draws, and fixing moves it but
/// never resizes it.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SketchNode {
    pub id: String,
    /// Its top left. A box with children may leave it out, to lie round
    /// them.
    #[serde(default)]
    pub x: Option<f64>,
    #[serde(default)]
    pub y: Option<f64>,
    /// Required unless the node has `svg`, or is a box with children: for
    /// those, with `x` and `y`, the least it takes, grown round them.
    #[serde(default)]
    pub width: Option<f64>,
    #[serde(default)]
    pub height: Option<f64>,
    /// SVG elements drawing the node, such as `<rect width="80" height="40"/>`.
    #[serde(default)]
    pub svg: Option<String>,
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
    /// How far its children, and a box's title, keep inside its border:
    /// 12px unless given.
    #[serde(default)]
    pub padding: Option<f64>,
    /// A parent a style draws: [`arrange_sketch_in_steps`] asks for its
    /// picture at the sizes it needs instead of taking `svg`.
    #[serde(default)]
    pub redrawn: bool,
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
    /// The port of `from` it leaves, one its drawing marks `data-port`.
    #[serde(default)]
    pub from_port: Option<String>,
    /// The port of `to` it reaches.
    #[serde(default)]
    pub to_port: Option<String>,
    #[serde(default)]
    pub arrow: SketchArrow,
    /// The paint of its line and heads, such as `#c00`: black unless given.
    #[serde(default)]
    pub color: Option<String>,
    /// Its line's `stroke-width` and `stroke-dasharray`, as SVG reads them.
    #[serde(default)]
    pub stroke_width: Option<f64>,
    #[serde(default)]
    pub stroke_dasharray: Option<String>,
}

/// The ends of a connector drawn with an arrowhead.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SketchArrow {
    /// The `to` end.
    #[default]
    End,
    Both,
    None,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SketchError {
    /// The field at `referrer` (such as `connectors[0].to` or
    /// `nodes[2].parent`) names a node the sketch does not have.
    UnknownNode { referrer: String, node: String },
    /// The id at `referrer` (such as `nodes[3].id`) is one an earlier node
    /// already has.
    DuplicateNode { referrer: String, node: String },
    /// The node at `referrer` has neither `svg` nor both `width` and `height`.
    MissingSize { referrer: String },
    /// The field at `referrer` (such as `nodes[0].parent`) names a parent,
    /// `node`, that lies inside the node it is the field of.
    ParentCycle { referrer: String, node: String },
    /// The node at `referrer` has no children to lie round, and no `x` and `y`.
    MissingPosition { referrer: String },
    /// The parent at `referrer` has no picture for a step that asked its
    /// style for one.
    MissingPicture { referrer: String },
    /// The `svg` at `referrer` is not well-formed SVG or draws nothing.
    InvalidSvg { referrer: String, message: String },
    /// The root attribute `name` is not an XML name, or one nicevg writes.
    InvalidRootAttribute { name: String, message: String },
    /// The field at `referrer` (such as `connectors[0].toPort`) names a
    /// port that the drawing of `node` does not mark.
    UnknownPort {
        referrer: String,
        node: String,
        port: String,
    },
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
            SketchError::DuplicateNode { referrer, node } => {
                write!(formatter, "{referrer} is \"{node}\", the id of an earlier node too")
            }
            SketchError::MissingSize { referrer } => {
                write!(formatter, "{referrer} needs a width and a height, or svg")
            }
            SketchError::ParentCycle { referrer, node } => {
                write!(
                    formatter,
                    "{referrer} is \"{node}\", which lies inside it: parents cannot loop"
                )
            }
            SketchError::MissingPicture { referrer } => {
                write!(
                    formatter,
                    "{referrer} has no picture from its style for the step that asked for one"
                )
            }
            SketchError::MissingPosition { referrer } => write!(
                formatter,
                "{referrer} needs x and y: only a box with children takes them from its children"
            ),
            SketchError::InvalidSvg { referrer, message } => write!(formatter, "{referrer}: {message}"),
            SketchError::InvalidRootAttribute { name, message } => write!(formatter, "rootAttrs.{name}: {message}"),
            SketchError::UnknownPort { referrer, node, port } => {
                write!(
                    formatter,
                    "{referrer} refers to port \"{port}\", which node \"{node}\" does not mark"
                )
            }
        }
    }
}

impl std::error::Error for SketchError {}

const NODE_FONT_SIZE: f64 = 14.0;
const LABEL_FONT_SIZE: f64 = 12.0;
const LINE_HEIGHT: f64 = 1.2;
const MARGIN: f64 = 20.0;

pub fn analyze_sketch(sketch: &Sketch) -> Result<AnalysisReport, SketchError> {
    Ok(crate::analyze(&sketch_svg(sketch)?).expect("a sketch's SVG parses"))
}

pub fn fix_sketch(sketch: &Sketch) -> Result<FixResult, SketchError> {
    Ok(with_heads(
        crate::fix(&sketch_svg(sketch)?).expect("a sketch's SVG parses"),
        sketch,
    ))
}

pub fn arrange_sketch(sketch: &Sketch) -> Result<FixResult, SketchError> {
    Ok(with_heads(
        crate::arrange(&sketch_svg(sketch)?).expect("a sketch's SVG parses"),
        sketch,
    ))
}

/// The fixed diagram with each connector's heads drawn at the ends of the
/// route fixing gave it, its line and heads in a group. Drawn after fixing,
/// the heads take their direction from a route that has one even for a
/// connector from a node to itself, whose route in the sketch's SVG is a point.
fn with_heads(mut result: FixResult, sketch: &Sketch) -> FixResult {
    let mut document = xml::parse(&result.svg).expect("the fixed SVG parses");
    for (index, connector) in sketch.connectors.iter().enumerate() {
        let id = connector_id(index);
        let route = &result
            .report
            .diagram
            .connectors
            .iter()
            .find(|drawn| drawn.id == id)
            .expect("every connector read back")
            .points;
        let (shaft, heads) = shaft_and_heads(route, connector.arrow);
        if heads.is_empty() {
            continue;
        }
        let (path, line) = document
            .elements()
            .into_iter()
            .find(|(_, element)| element.attr("id") == id)
            .map(|(path, element)| (path, element.clone()))
            .expect("the connector's line");
        let color = line.attr("stroke").to_owned();
        let heads = heads.iter().map(|points| {
            Element::new("polygon")
                .with_attr("fill", &color)
                .with_attr("points", points)
        });
        let mut group = Element::new("g");
        group.children = Arc::new(
            std::iter::once(line.with_attr("d", &path_data(&shaft)))
                .chain(heads)
                .map(Node::Element)
                .collect(),
        );
        document = document.replace(&path, group);
    }
    result.svg = xml::serialize(&document);
    result
}

/// A picture a step asks a parent's style to draw.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PictureRequest {
    pub id: String,
    pub width: f64,
    pub height: f64,
    /// Whether it is the picture the diagram keeps, at the size laying out
    /// gave the parent, rather than one to lay the parent out by.
    pub last: bool,
}

/// Where the steps left a sketch's layout, for the next step to go on from.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StepState {
    svg: String,
    changes: Vec<FixChange>,
    round: usize,
}

/// What arranging a sketch in steps does next.
#[derive(Clone, Debug, PartialEq)]
pub enum SketchStep {
    /// Ask the styles for these pictures, then step again with them and the state.
    Draw {
        pictures: Vec<PictureRequest>,
        state: Option<StepState>,
    },
    /// The diagram, arranged and drawn.
    Done(FixResult),
}

/// Arranges a sketch whose `redrawn` parents styles draw, a step at a time.
/// `pictures` holds the pictures drawn so far, by parent, in the order the
/// steps asked for them.
///
/// The first steps ask, innermost parents first, for each parent's picture
/// at the size of its children, and lay the sketch out by those pictures;
/// the next asks for each parent's picture at the size laying out gave it.
/// A picture that differs from the one laid out by, in its size or where its
/// texts or ports lie, lays the sketch out once more by it; one that still
/// differs is fixed round and reported as `parent-drawn-differently`.
pub fn arrange_sketch_in_steps(
    sketch: &Sketch,
    pictures: &HashMap<String, Vec<String>>,
    state: Option<StepState>,
) -> Result<SketchStep, SketchError> {
    let Some(StepState { svg, changes, round }) = state else {
        return Ok(match layout_svg(sketch, Some((pictures, 1)))? {
            Layout::Needs(needed) => SketchStep::Draw {
                pictures: needed,
                state: None,
            },
            Layout::Svg(svg) => ask_for_last(sketch, crate::arrange(&svg).expect("a sketch's SVG parses"), 1),
        });
    };
    let root_attrs = root_attributes(&sketch.root_attrs)?;
    let laid_out = analyze(&svg).expect("a step's SVG parses");
    let mut lasts = Vec::new();
    let mut differences = Vec::new();
    for (index, node) in sketch.nodes.iter().enumerate().filter(|(_, node)| node.redrawn) {
        let Some(frame) = laid_out.diagram.node(&node.id).map(|node| node.bounds) else {
            continue;
        };
        let referrer = format!("nodes[{index}]");
        let picture = |round: usize| {
            let missing = || SketchError::MissingPicture {
                referrer: referrer.clone(),
            };
            pictures
                .get(&node.id)
                .and_then(|drawn| drawn.get(round))
                .ok_or_else(missing)
        };
        let before = measure_picture(picture(round - 1)?, &root_attrs, &referrer)?;
        let last = measure_picture(picture(round)?, &root_attrs, &referrer)?;
        let what = differences_between(&before, &last, frame);
        if !what.is_empty() {
            differences.push((node.id.clone(), what));
        }
        lasts.push((node.id.as_str(), last));
    }
    if !differences.is_empty() && round == 1 {
        // From the sketch again, laid out by the last pictures, so the frames
        // fit them as they fitted the first.
        let Layout::Svg(svg) = layout_svg(sketch, Some((pictures, round + 1)))? else {
            unreachable!("every parent laid out has a last picture");
        };
        return Ok(ask_for_last(
            sketch,
            crate::arrange(&svg).expect("a sketch's SVG parses"),
            round + 1,
        ));
    }
    let document = xml::parse(&svg).expect("a step's SVG parses");
    let drawn = xml::serialize(&with_pictures(&document, &lasts));
    let mut result = if differences.is_empty() {
        FixResult {
            report: analyze(&drawn).expect("a step's SVG parses"),
            svg: drawn,
            changes,
        }
    } else {
        let fixed = crate::fix(&drawn).expect("a step's SVG parses");
        FixResult {
            changes: [changes, fixed.changes].concat(),
            ..fixed
        }
    };
    result
        .report
        .issues
        .extend(differences.into_iter().map(|(id, what)| DiagramIssue {
            code: "parent-drawn-differently".to_owned(),
            message: format!(
                "The style of parent \"{id}\" drew it {} than it was laid out by.",
                what.join(" and ")
            ),
            elements: vec![id],
            details: None,
        }));
    Ok(SketchStep::Done(with_heads(result, sketch)))
}

/// The step after laying out: done when no style draws a parent, else
/// asking for each parent's picture at the size laying out gave it.
fn ask_for_last(sketch: &Sketch, result: FixResult, round: usize) -> SketchStep {
    let needed: Vec<PictureRequest> = sketch
        .nodes
        .iter()
        .filter(|node| node.redrawn)
        .filter_map(|node| {
            let bounds = result.report.diagram.node(&node.id)?.bounds;
            Some(PictureRequest {
                id: node.id.clone(),
                width: bounds.width,
                height: bounds.height,
                last: true,
            })
        })
        .collect();
    if needed.is_empty() {
        return SketchStep::Done(with_heads(result, sketch));
    }
    SketchStep::Draw {
        pictures: needed,
        state: Some(StepState {
            svg: result.svg,
            changes: result.changes,
            round,
        }),
    }
}

/// How a parent's last picture, asked for at its frame's size, differs from
/// the picture it was laid out by.
fn differences_between(before: &Picture, last: &Picture, frame: Bounds) -> Vec<&'static str> {
    let near = |a: f64, b: f64| (a - b).abs() <= 0.5;
    let same =
        |a: &Bounds, b: &Bounds| near(a.x, b.x) && near(a.y, b.y) && near(a.width, b.width) && near(a.height, b.height);
    let mut what = Vec::new();
    if !near(last.bounds.width, frame.width) || !near(last.bounds.height, frame.height) {
        what.push("at another size");
    }
    if before.texts.len() != last.texts.len() || !before.texts.iter().zip(&last.texts).all(|(a, b)| same(a, b)) {
        what.push("with its texts elsewhere");
    }
    if before.ports.len() != last.ports.len()
        || !before
            .ports
            .iter()
            .zip(&last.ports)
            .all(|((name, at), (other, there))| name == other && near(at.x, there.x) && near(at.y, there.y))
    {
        what.push("with its ports elsewhere");
    }
    what
}

/// The document with each parent's last picture drawn at its frame's top
/// left, in place of the frame and the picture it was laid out by.
fn with_pictures(document: &Document, lasts: &[(&str, Picture)]) -> Document {
    lasts.iter().fold(document.clone(), |document, (id, picture)| {
        let Some(path) = node_group(&document, id) else {
            return document;
        };
        let group = document.element(&path).clone();
        let marked =
            |node: &Node, attribute: &str| matches!(node, Node::Element(element) if element.has_attr(attribute));
        let Some(Node::Element(frame)) = group.children.iter().find(|node| marked(node, "data-frame")).cloned() else {
            return document;
        };
        let origin = point(number_attribute(&frame, "x"), number_attribute(&frame, "y"));
        let parsed = xml::parse(&format!("<svg xmlns=\"{}\">{}</svg>", xml::SVG_NAMESPACE, picture.svg))
            .expect("a measured picture parses");
        let drawn = Element {
            children: Arc::clone(&parsed.root().children),
            ..Element::new("g").with_attr(
                "transform",
                &format!("translate({} {})", number(origin.x), number(origin.y)),
            )
        };
        let rest = group
            .children
            .iter()
            .filter(|node| !marked(node, "data-frame") && !marked(node, "data-picture"))
            .cloned();
        let mut group = Element {
            children: Arc::new(std::iter::once(Node::Element(drawn)).chain(rest).collect()),
            ..group
        };
        if let Some(padding) = picture.padding {
            group = group.with_attr("data-padding", &number(padding));
        }
        group.attributes.retain(|(name, _)| name != "data-redrawn");
        document.replace(&path, group)
    })
}

/// An SVG path's `d` running through `points`.
fn path_data(points: &[Point]) -> String {
    points
        .iter()
        .enumerate()
        .map(|(index, p)| format!("{} {} {}", if index == 0 { "M" } else { "L" }, number(p.x), number(p.y)))
        .collect::<Vec<_>>()
        .join(" ")
}

/// The SVG a sketch stands for: a `data-node` group for each node, with
/// its children nested in it, and a `data-from`/`data-to` path for each
/// connector, its label a `data-label-for` text.
pub fn sketch_svg(sketch: &Sketch) -> Result<String, SketchError> {
    match layout_svg(sketch, None)? {
        Layout::Svg(svg) => Ok(svg),
        Layout::Needs(_) => unreachable!("pictures are asked for only in steps"),
    }
}

/// What laying out a sketch starts from: its SVG, or the pictures it needs
/// first.
enum Layout {
    Svg(String),
    Needs(Vec<PictureRequest>),
}

/// The SVG a sketch stands for. In steps, its `redrawn` parents are drawn
/// with the pictures their styles drew so far, the `round`th of each,
/// counting from 1, laid out by; a parent without one asks for it at the
/// size of its children.
fn layout_svg(sketch: &Sketch, steps: Option<(&HashMap<String, Vec<String>>, usize)>) -> Result<Layout, SketchError> {
    let mut nodes: HashMap<&str, &SketchNode> = HashMap::new();
    for (index, node) in sketch.nodes.iter().enumerate() {
        if nodes.insert(node.id.as_str(), node).is_some() {
            return Err(SketchError::DuplicateNode {
                referrer: format!("nodes[{index}].id"),
                node: node.id.clone(),
            });
        }
    }
    for (index, node) in sketch.nodes.iter().enumerate() {
        if let Some(parent) = node.parent.as_deref().filter(|parent| !nodes.contains_key(parent)) {
            return Err(unknown(format!("nodes[{index}].parent"), parent));
        }
    }
    // Followed up, a node's parents reach one without a parent.
    for (index, node) in sketch.nodes.iter().enumerate() {
        let mut above = node.parent.as_deref();
        for _ in 0..sketch.nodes.len() {
            match above {
                Some(id) if id == node.id => {
                    return Err(SketchError::ParentCycle {
                        referrer: format!("nodes[{index}].parent"),
                        node: node.parent.clone().expect("a node with parents"),
                    });
                }
                Some(id) => above = nodes[id].parent.as_deref(),
                None => break,
            }
        }
    }
    for (index, connector) in sketch.connectors.iter().enumerate() {
        for (field, end) in [("from", &connector.from), ("to", &connector.to)] {
            if !nodes.contains_key(end.as_str()) {
                return Err(unknown(format!("connectors[{index}].{field}"), end));
            }
        }
    }

    let root_attrs = root_attributes(&sketch.root_attrs)?;
    let mut children: HashMap<&str, Vec<&SketchNode>> = HashMap::new();
    for node in &sketch.nodes {
        if let Some(parent) = &node.parent {
            children.entry(parent.as_str()).or_default().push(node);
        }
    }
    // A box with children lies round them, so its box waits for theirs.
    let fits_children = |node: &SketchNode| {
        node.svg.is_none() && node.shape == SketchShape::Rect && children.contains_key(node.id.as_str())
    };
    let mut boxes: HashMap<&str, Bounds> = HashMap::new();
    let mut ports: HashMap<&str, Vec<String>> = HashMap::new();
    let mut waiting = Vec::new();
    for (index, node) in sketch.nodes.iter().enumerate() {
        if fits_children(node) {
            waiting.push((index, node));
            ports.insert(node.id.as_str(), Vec::new());
            continue;
        }
        let (bounds, marked) = node_bounds(node, index, &root_attrs)?;
        boxes.insert(node.id.as_str(), bounds);
        ports.insert(node.id.as_str(), marked);
    }
    let mut drawn: HashMap<&str, (&str, Option<f64>)> = HashMap::new();
    let mut needed = Vec::new();
    while let Some(next) = waiting.iter().position(|(_, node)| {
        children[node.id.as_str()]
            .iter()
            .all(|child| boxes.contains_key(child.id.as_str()))
    }) {
        let (index, node) = waiting.remove(next);
        let inner: Vec<Bounds> = children[node.id.as_str()]
            .iter()
            .map(|child| boxes[child.id.as_str()])
            .collect();
        let picture = match steps.filter(|_| node.redrawn) {
            Some((pictures, round)) => match pictures.get(&node.id).and_then(|drawn| drawn.get(round - 1)) {
                Some(svg) => Some(measure_picture(svg, &root_attrs, &format!("nodes[{index}]"))?),
                None => {
                    let fitted = parent_frame(node, &inner, None);
                    needed.push(PictureRequest {
                        id: node.id.clone(),
                        width: fitted.width,
                        height: fitted.height,
                        last: false,
                    });
                    continue;
                }
            },
            None => None,
        };
        boxes.insert(node.id.as_str(), parent_frame(node, &inner, picture.as_ref()));
        if let Some(picture) = picture {
            ports.insert(
                node.id.as_str(),
                picture.ports.iter().map(|(name, _)| name.clone()).collect(),
            );
            drawn.insert(node.id.as_str(), (picture.svg, picture.padding.or(node.padding)));
        }
    }
    if !needed.is_empty() {
        return Ok(Layout::Needs(needed));
    }
    for (index, connector) in sketch.connectors.iter().enumerate() {
        for (field, end, port) in [
            ("fromPort", &connector.from, &connector.from_port),
            ("toPort", &connector.to, &connector.to_port),
        ] {
            if let Some(port) = port.as_ref().filter(|port| !ports[end.as_str()].contains(port)) {
                return Err(SketchError::UnknownPort {
                    referrer: format!("connectors[{index}].{field}"),
                    node: end.clone(),
                    port: port.clone(),
                });
            }
        }
    }
    let routes: Vec<Vec<Point>> = sketch
        .connectors
        .iter()
        .map(|connector| {
            if connector.points.len() >= 2 {
                connector.points.clone()
            } else {
                vec![
                    boxes[connector.from.as_str()].centre(),
                    boxes[connector.to.as_str()].centre(),
                ]
            }
        })
        .collect();

    let mut body = String::new();
    let place = Places {
        boxes: &boxes,
        children: &children,
        drawn: &drawn,
    };
    // Nodes outside any other, each with its children inside.
    for node in sketch.nodes.iter().filter(|node| node.parent.is_none()) {
        write_node(&mut body, node, &place, 1);
    }
    for (index, (connector, route)) in sketch.connectors.iter().zip(&routes).enumerate() {
        let id = connector_id(index);
        let d = path_data(route);
        let port = |name: &str, port: &Option<String>| {
            port.as_deref()
                .map_or_else(String::new, |port| format!(r#" data-{name}-port="{}""#, escape(port)))
        };
        let color = escape(connector.color.as_deref().unwrap_or("black"));
        let width = connector
            .stroke_width
            .map_or_else(String::new, |width| format!(r#" stroke-width="{}""#, number(width)));
        let dashes = connector
            .stroke_dasharray
            .as_deref()
            .map_or_else(String::new, |dashes| {
                format!(r#" stroke-dasharray="{}""#, escape(dashes))
            });
        let _ = writeln!(
            body,
            r#"  <path id="{id}" data-from="{}"{} data-to="{}"{} fill="none" stroke="{color}"{width}{dashes} d="{d}"/>"#,
            escape(&connector.from),
            port("from", &connector.from_port),
            escape(&connector.to),
            port("to", &connector.to_port)
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
        .flat_map(|node| {
            let bounds = boxes[node.id.as_str()];
            [point(bounds.x, bounds.y), point(bounds.right(), bounds.bottom())]
        })
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
    Ok(Layout::Svg(format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{} {} {} {}\" width=\"{}\" height=\"{}\"{root_attrs}>\n{body}</svg>\n",
        number(x),
        number(y),
        number(width),
        number(height),
        number(width),
        number(height)
    )))
}

const HEAD_LENGTH: f64 = 10.0;
const HEAD_HALF_WIDTH: f64 = 5.0;

/// A connector's line along `route`, stopped short of each end with a head
/// as `shaft_end` says, and the `points` of the heads: a corner of the base,
/// the tip at the route's end, the other corner.
fn shaft_and_heads(route: &[Point], arrow: SketchArrow) -> (Vec<Point>, Vec<String>) {
    let last = route.len() - 1;
    let ends: &[(usize, usize)] = match arrow {
        SketchArrow::End => &[(last, last - 1)],
        SketchArrow::Both => &[(0, 1), (last, last - 1)],
        SketchArrow::None => &[],
    };
    let mut shaft = route.to_vec();
    let heads = ends
        .iter()
        .filter_map(|&(end, inner)| {
            let tip = route[end];
            let (stop, Point { x: dx, y: dy }) = shaft_end(route[inner], tip, HEAD_LENGTH)?;
            shaft[end] = stop;
            let base = point(tip.x - dx * HEAD_LENGTH, tip.y - dy * HEAD_LENGTH);
            let (nx, ny) = (-dy * HEAD_HALF_WIDTH, dx * HEAD_HALF_WIDTH);
            Some(format!(
                "{},{} {},{} {},{}",
                number(base.x + nx),
                number(base.y + ny),
                number(tip.x),
                number(tip.y),
                number(base.x - nx),
                number(base.y - ny)
            ))
        })
        .collect();
    (shaft, heads)
}
/// The id of the connector at `index` in a sketch, as the diagram read back
/// names it, and the label describing it.
fn connector_id(index: usize) -> String {
    format!("connector-{index}")
}

/// Where a node lies: the box around its picture, or its given size.
/// A node's box, and the ports its drawing marks.
fn node_bounds(node: &SketchNode, index: usize, root_attrs: &str) -> Result<(Bounds, Vec<String>), SketchError> {
    let referrer = format!("nodes[{index}]");
    let (Some(x), Some(y)) = (node.x, node.y) else {
        return Err(SketchError::MissingPosition { referrer });
    };
    let Some(svg) = &node.svg else {
        let (Some(width), Some(height)) = (node.width, node.height) else {
            return Err(SketchError::MissingSize { referrer });
        };
        return Ok((Bounds { x, y, width, height }, Vec::new()));
    };
    let picture = measure_picture(svg, root_attrs, &format!("{referrer}.svg"))?;
    Ok((
        Bounds {
            x: picture.bounds.x + x,
            y: picture.bounds.y + y,
            ..picture.bounds
        },
        picture.ports.into_iter().map(|(name, _)| name).collect(),
    ))
}

/// A picture a node's `svg` or a parent's style draws, measured from its
/// own origin: its extent, its texts, its ports and its frame's padding.
struct Picture<'a> {
    svg: &'a str,
    bounds: Bounds,
    texts: Vec<Bounds>,
    ports: Vec<(String, Point)>,
    padding: Option<f64>,
}

fn measure_picture<'a>(svg: &'a str, root_attrs: &str, referrer: &str) -> Result<Picture<'a>, SketchError> {
    let invalid = |message: String| SketchError::InvalidSvg {
        referrer: referrer.to_owned(),
        message,
    };
    // Measured under the same root attributes, which its texts inherit.
    let document = xml::parse(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\"{root_attrs}>{svg}</svg>"
    ))
    .map_err(|error| invalid(error.0))?;
    let root = [document.root_index()];
    let (_, bounds) = custom_extent(&document, &root).ok_or_else(|| invalid("draws nothing".to_owned()))?;
    let elements = document.descendants(&root);
    Ok(Picture {
        svg,
        bounds,
        texts: elements
            .iter()
            .filter(|(_, element)| element.name == "text")
            .map(|(path, _)| text_bounds(&document, path))
            .collect(),
        ports: elements
            .iter()
            .filter(|(_, element)| element.name == "circle" && element.has_attr("data-port"))
            .map(|(path, element)| {
                let offset = translation(&document, path);
                let at = point(
                    number_attribute(element, "cx") + offset.x,
                    number_attribute(element, "cy") + offset.y,
                );
                (element.attr("data-port").to_owned(), at)
            })
            .collect(),
        padding: elements
            .iter()
            .find(|(_, element)| element.has_attr("data-bounds"))
            .and_then(|(_, element)| non_empty(element.attr("data-padding"))?.parse().ok()),
    })
}

/// A parent's frame: its children `padding` in from its sides, grown round
/// whatever of its place and size it was given, then round its picture, if
/// a style drew one, with that frame's top left as the picture's origin. Its
/// texts take no room: fixing moves the children that lie on them.
fn parent_frame(node: &SketchNode, children: &[Bounds], picture: Option<&Picture>) -> Bounds {
    let padding = picture
        .and_then(|picture| picture.padding)
        .or(node.padding)
        .unwrap_or(PARENT_PADDING);
    let inner = enclosing(children).expect("a parent has children");
    // Out to whole pixels, which only widen the gaps, so the SVG reads cleanly.
    let (left, top) = ((inner.x - padding).floor(), (inner.y - padding).floor());
    let (right, bottom) = ((inner.right() + padding).ceil(), (inner.bottom() + padding).ceil());
    let fitted = Bounds {
        x: left,
        y: top,
        width: right - left,
        height: bottom - top,
    };
    let given = Bounds {
        x: node.x.unwrap_or(fitted.x),
        y: node.y.unwrap_or(fitted.y),
        width: node.width.unwrap_or(0.0),
        height: node.height.unwrap_or(0.0),
    };
    let frame = enclosing(&[fitted, given]).expect("two boxes");
    let Some(picture) = picture else {
        return frame;
    };
    let drawn = Bounds {
        x: frame.x + picture.bounds.x,
        y: frame.y + picture.bounds.y,
        ..picture.bounds
    };
    enclosing(&[frame, drawn]).expect("two boxes")
}

/// A box parent's title, its lines starting `padding` in from `corner`, the
/// frame's top left.
fn title_markup(node: &SketchNode, corner: Point) -> String {
    let (Some(label), padding) = (&node.label, node.padding.unwrap_or(PARENT_PADDING)) else {
        return String::new();
    };
    let font_size = node.font_size.unwrap_or(NODE_FONT_SIZE);
    format!(
        r#"<text font-size="{}">{}</text>"#,
        number(font_size),
        lines(
            label,
            corner.x + padding,
            corner.y + padding + font_size * 0.35 + font_size / 2.0,
            font_size
        )
    )
}

static XML_NAME: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[A-Za-z_][A-Za-z0-9_.:-]*$").expect("valid pattern"));

/// The root attributes written out, each with a leading space.
fn root_attributes(attrs: &BTreeMap<String, String>) -> Result<String, SketchError> {
    attrs.iter().try_fold(String::new(), |mut written, (name, value)| {
        let invalid = |message: &str| SketchError::InvalidRootAttribute {
            name: name.clone(),
            message: message.to_owned(),
        };
        if !XML_NAME.is_match(name) {
            return Err(invalid("is not an attribute name"));
        }
        if ["xmlns", "viewBox", "width", "height"].contains(&name.as_str()) {
            return Err(invalid("is written by nicevg"));
        }
        let _ = write!(written, " {name}=\"{}\"", escape(value));
        Ok(written)
    })
}

fn unknown(referrer: String, node: &str) -> SketchError {
    SketchError::UnknownNode {
        referrer,
        node: node.to_owned(),
    }
}

/// Where the nodes are written: each one's box and children, and the
/// picture a style drew for a parent laid out in steps, with its padding.
struct Places<'a> {
    boxes: &'a HashMap<&'a str, Bounds>,
    children: &'a HashMap<&'a str, Vec<&'a SketchNode>>,
    drawn: &'a HashMap<&'a str, (&'a str, Option<f64>)>,
}

fn write_node(body: &mut String, node: &SketchNode, place: &Places, depth: usize) {
    let indent = "  ".repeat(depth);
    let overlap = if node.allow_overlap {
        r#" data-allow-overlap="true""#
    } else {
        ""
    };
    let padding_attribute = |padding: Option<f64>| {
        padding.map_or_else(String::new, |padding| format!(r#" data-padding="{}""#, number(padding)))
    };
    let padding = padding_attribute(node.padding);
    let inside = place.children.get(node.id.as_str());
    if let Some((svg, padding)) = place.drawn.get(node.id.as_str()) {
        // The frame, which laying out may grow, and the picture to lay out
        // by, drawn again at the frame's size once it is laid out.
        let frame = place.boxes[node.id.as_str()];
        let _ = writeln!(
            body,
            "{indent}<g data-node=\"{}\" data-custom=\"true\" data-redrawn=\"true\"{overlap}{}>\n{indent}  <rect data-bounds=\"true\" data-frame=\"true\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"none\"/>\n{indent}  <g data-picture=\"true\" transform=\"translate({} {})\">{svg}</g>",
            escape(&node.id),
            padding_attribute(*padding),
            number(frame.x),
            number(frame.y),
            number(frame.width),
            number(frame.height),
            number(frame.x),
            number(frame.y),
        );
    } else if let Some(svg) = &node.svg {
        let _ = writeln!(
            body,
            // The picture alone is translated, so children keep the places given.
            "{indent}<g data-node=\"{}\" data-custom=\"true\"{overlap}{padding}>\n{indent}  <g transform=\"translate({} {})\">{svg}</g>",
            escape(&node.id),
            // A node with svg has a place: `node_bounds` refuses one without.
            number(node.x.unwrap_or_default()),
            number(node.y.unwrap_or_default()),
        );
    } else {
        let bounds = place.boxes[node.id.as_str()];
        let _ = writeln!(body, "{indent}<g data-node=\"{}\"{overlap}{padding}>", escape(&node.id));
        write_shape(body, node, bounds, inside.is_some(), &indent);
    }
    for child in inside.into_iter().flatten() {
        write_node(body, child, place, depth + 1);
    }
    let _ = writeln!(body, "{indent}</g>");
}

/// A box or circle node's shape and label.
fn write_shape(body: &mut String, node: &SketchNode, bounds: Bounds, has_children: bool, indent: &str) {
    match node.shape {
        SketchShape::Rect => {
            let _ = writeln!(
                body,
                r#"{indent}  <rect x="{}" y="{}" width="{}" height="{}" fill="white" stroke="black"/>"#,
                number(bounds.x),
                number(bounds.y),
                number(bounds.width),
                number(bounds.height)
            );
        }
        SketchShape::Circle => {
            let centre = bounds.centre();
            let _ = writeln!(
                body,
                r#"{indent}  <circle cx="{}" cy="{}" r="{}" fill="white" stroke="black"/>"#,
                number(centre.x),
                number(centre.y),
                number(bounds.width.min(bounds.height) / 2.0)
            );
        }
    }
    if let Some(label) = &node.label {
        let font_size = node.font_size.unwrap_or(NODE_FONT_SIZE);
        if has_children {
            let _ = writeln!(body, "{indent}  {}", title_markup(node, point(bounds.x, bounds.y)));
        } else {
            let centre = bounds.centre();
            let _ = writeln!(
                body,
                r#"{indent}  <text text-anchor="middle" font-size="{}">{}</text>"#,
                number(font_size),
                lines(label, centre.x, centre.y, font_size)
            );
        }
    }
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

fn number(value: f64) -> String {
    format_number(value)
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
