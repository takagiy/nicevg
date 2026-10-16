//! What a document means as a diagram: nodes, connectors, labels and the
//! shapes left unsupported, read without changing anything.

use std::collections::HashMap;
use std::sync::LazyLock;

use regex::Regex;
use serde::Serialize;

use crate::arrow::{drawn_connectors, names_an_end};
use crate::geometry::{Bounds, Point, enclosing, number_list, parse_number, point, points_bounds};
use crate::pathdata;
use crate::route::Side;
use crate::text::{label_bounds, label_text_index, text_bounds};
use crate::xml::{Document, Element, Path};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Shape {
    Circle,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagramNode {
    pub id: String,
    /// Circular nodes report their circle's bounding box; rectangles omit it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shape: Option<Shape>,
    pub bounds: Bounds,
    pub label_bounds: Vec<Bounds>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub allow_overlap: bool,
    /// Drawn as a picture of its own: its texts are kept clear of, but not
    /// checked to fit, since its drawing decides its size.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub custom: bool,
    /// The named points its picture marks for connector ends to meet.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub ports: Vec<Port>,
}

/// A point a custom node's picture marks `data-port` for connector ends.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Port {
    pub name: String,
    pub point: Point,
    /// The side of the node's bounds it lies on, which connectors leave it out of.
    pub side: Side,
}

impl DiagramNode {
    pub fn is_circle(&self) -> bool {
        self.shape == Some(Shape::Circle)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagramConnector {
    pub id: String,
    pub from: String,
    pub to: String,
    pub points: Vec<Point>,
    /// The port of `from` it leaves, when it names one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_port: Option<String>,
    /// The port of `to` it reaches, when it names one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_port: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct DiagramLabel {
    pub id: String,
    pub bounds: Bounds,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connector: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnsupportedElement {
    pub id: String,
    pub tag_name: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagram {
    pub nodes: Vec<DiagramNode>,
    pub connectors: Vec<DiagramConnector>,
    pub labels: Vec<DiagramLabel>,
    pub unsupported_elements: Vec<UnsupportedElement>,
}

impl Diagram {
    pub fn node(&self, id: &str) -> Option<&DiagramNode> {
        self.nodes.iter().find(|node| node.id == id)
    }

    pub fn connector(&self, id: &str) -> Option<&DiagramConnector> {
        self.connectors.iter().find(|connector| connector.id == id)
    }

    /// The box around every node, label and connector.
    pub fn drawing_bounds(&self) -> Option<Bounds> {
        let bounds: Vec<Bounds> = self
            .nodes
            .iter()
            .map(|node| node.bounds)
            .chain(self.labels.iter().map(|label| label.bounds))
            .chain(
                self.connectors
                    .iter()
                    .filter_map(|connector| points_bounds(&connector.points)),
            )
            .collect();
        enclosing(&bounds)
    }
}

/// Ids of the containers around a node, nearest first.
pub fn ancestor_ids<'a>(nodes: &'a [DiagramNode], id: &str) -> Vec<&'a str> {
    let parent_of = |id: &str| {
        nodes
            .iter()
            .find(|node| node.id == id)
            .and_then(|node| node.parent_id.as_deref())
    };
    std::iter::successors(parent_of(id), |parent| parent_of(parent))
        .take(nodes.len())
        .collect()
}

/// Whether a node is one of the connector's ends or a container around one,
/// which the connector legitimately reaches into.
pub fn holds_end(nodes: &[DiagramNode], connector: &DiagramConnector, node: &DiagramNode) -> bool {
    node.id == connector.from
        || node.id == connector.to
        || ancestor_ids(nodes, &connector.from).contains(&node.id.as_str())
        || ancestor_ids(nodes, &connector.to).contains(&node.id.as_str())
}

static TRANSLATE: LazyLock<Regex> = LazyLock::new(|| {
    let number = r"[-+]?(?:\d*\.?\d+)(?:[eE][-+]?\d+)?";
    Regex::new(&format!(r"translate\(\s*({number})(?:[\s,]+({number}))?\s*\)")).expect("valid pattern")
});

pub fn number_attribute(element: &Element, name: &str) -> f64 {
    parse_number(element.attr(name))
}

/// Sum of the `translate(...)` transforms on an element and its ancestors.
pub fn translation(document: &Document, path: &[usize]) -> Point {
    document
        .lineage(path)
        .iter()
        .flat_map(|element| {
            TRANSLATE
                .captures_iter(element.attr("transform"))
                .map(|captures| {
                    let value = |index: usize| captures.get(index).map_or(0.0, |m| parse_number(m.as_str()));
                    point(value(1), value(2))
                })
                .collect::<Vec<_>>()
        })
        .fold(point(0.0, 0.0), |sum, offset| point(sum.x + offset.x, sum.y + offset.y))
}

pub fn rect_bounds(document: &Document, path: &[usize]) -> Bounds {
    let element = document.element(path);
    let offset = translation(document, path);
    Bounds {
        x: number_attribute(element, "x") + offset.x,
        y: number_attribute(element, "y") + offset.y,
        width: number_attribute(element, "width"),
        height: number_attribute(element, "height"),
    }
}

pub fn circle_bounds(document: &Document, path: &[usize]) -> Bounds {
    let element = document.element(path);
    let offset = translation(document, path);
    let radius = number_attribute(element, "r");
    Bounds {
        x: number_attribute(element, "cx") + offset.x - radius,
        y: number_attribute(element, "cy") + offset.y - radius,
        width: radius * 2.0,
        height: radius * 2.0,
    }
}

pub fn is_node_shape(element: &Element) -> bool {
    element.name == "rect" || element.name == "circle"
}

fn shape_bounds(document: &Document, path: &[usize]) -> Bounds {
    if document.element(path).name == "circle" {
        circle_bounds(document, path)
    } else {
        rect_bounds(document, path)
    }
}

pub(crate) fn connector_points(document: &Document, path: &[usize]) -> Vec<Point> {
    let element = document.element(path);
    let offset = translation(document, path);
    let shifted = |p: Point| point(p.x + offset.x, p.y + offset.y);
    match element.name.as_str() {
        "line" => vec![
            shifted(point(number_attribute(element, "x1"), number_attribute(element, "y1"))),
            shifted(point(number_attribute(element, "x2"), number_attribute(element, "y2"))),
        ],
        "polyline" => number_list(element.attr("points"))
            .as_chunks::<2>()
            .0
            .iter()
            .map(|[x, y]| shifted(point(*x, *y)))
            .collect(),
        "path" => pathdata::end_points(element.attr("d"))
            .into_iter()
            .map(shifted)
            .collect(),
        _ => Vec::new(),
    }
}

/// Paths of the child elements of the element at `path`.
pub fn child_element_paths(document: &Document, path: &[usize]) -> Vec<Path> {
    document
        .element(path)
        .children
        .iter()
        .enumerate()
        .filter(|(_, child)| matches!(child, crate::xml::Node::Element(_)))
        .map(|(index, _)| [path, &[index]].concat())
        .collect()
}

/// The `g` element a node id refers to, by `data-node` or `id`.
pub fn node_group(document: &Document, id: &str) -> Option<Path> {
    document
        .elements_named("g")
        .into_iter()
        .find(|(_, group)| non_empty(group.attr("data-node")).or_else(|| non_empty(group.attr("id"))) == Some(id))
        .map(|(path, _)| path)
}

/// The lines, then the polylines, then the paths of a document, each in
/// document order, from one walk.
pub fn connector_elements(document: &Document) -> Vec<(Path, &Element)> {
    let names = ["line", "polyline", "path"];
    let mut elements: Vec<(Path, &Element)> = document
        .elements()
        .into_iter()
        .filter(|(_, element)| names.contains(&element.name.as_str()))
        .collect();
    elements.sort_by_key(|(_, element)| names.iter().position(|name| *name == element.name));
    elements
}

pub(crate) fn non_empty(value: &str) -> Option<&str> {
    (!value.is_empty()).then_some(value)
}

/// The elements inside the element at `path`, leaving out the nodes nested
/// in it and what they hold.
fn picture_elements<'a>(document: &'a Document, path: &[usize]) -> Vec<(Path, &'a Element)> {
    let descendants = document.descendants(path);
    let nested: Vec<Path> = descendants
        .iter()
        .filter(|(_, element)| element.name == "g" && !element.attr("data-node").is_empty())
        .map(|(path, _)| path.clone())
        .collect();
    descendants
        .into_iter()
        .filter(|(path, _)| !nested.iter().any(|node| path.starts_with(node)))
        .collect()
}

/// A custom node's shape and extent: the rect or circle marked data-bounds
/// in its picture, or else the box around the picture.
pub fn custom_extent(document: &Document, path: &[usize]) -> Option<(Option<Shape>, Bounds)> {
    let picture = picture_elements(document, path);
    let marked = picture
        .iter()
        .find(|(_, element)| is_node_shape(element) && element.has_attr("data-bounds"));
    match marked {
        Some((shape_path, element)) => Some((
            (element.name == "circle").then_some(Shape::Circle),
            shape_bounds(document, shape_path),
        )),
        None => Some((None, content_bounds(document, path)?)),
    }
}

struct Recognized {
    path: Path,
    id: String,
    shape: Option<Shape>,
    bounds: Bounds,
    labels: Vec<Path>,
    allow_overlap: bool,
    custom: bool,
    ports: Vec<Port>,
}

/// Whether a node group is drawn as it is: its contents are a picture, not
/// a shape with labels, and fixing never resizes it.
fn is_custom(group: &Element) -> bool {
    group.attr("data-custom") == "true"
}

/// The box around what the element at `path` draws: its rects, circles,
/// ellipses, lines, polylines, polygons, paths and texts, leaving out the
/// nodes nested in it.
pub fn content_bounds(document: &Document, path: &[usize]) -> Option<Bounds> {
    let bounds: Vec<Bounds> = picture_elements(document, path)
        .iter()
        .filter_map(|(path, element)| match element.name.as_str() {
            "rect" => Some(rect_bounds(document, path)),
            "circle" => Some(circle_bounds(document, path)),
            "ellipse" => {
                let offset = translation(document, path);
                let (rx, ry) = (number_attribute(element, "rx"), number_attribute(element, "ry"));
                Some(Bounds {
                    x: number_attribute(element, "cx") + offset.x - rx,
                    y: number_attribute(element, "cy") + offset.y - ry,
                    width: rx * 2.0,
                    height: ry * 2.0,
                })
            }
            // shortcut: a curve or arc counts by its end points only, so a bulge past them sits outside the bounds; measure curves if custom shapes need them.
            "line" | "polyline" | "path" => points_bounds(&connector_points(document, path)),
            "polygon" => {
                let offset = translation(document, path);
                let points: Vec<Point> = number_list(element.attr("points"))
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|[x, y]| point(x + offset.x, y + offset.y))
                    .collect();
                points_bounds(&points)
            }
            "text" => Some(text_bounds(document, path)),
            _ => None,
        })
        .collect();
    enclosing(&bounds)
}

fn recognize_nodes(document: &Document) -> Vec<Recognized> {
    let groups = document.elements_named("g");
    let custom: Vec<&Path> = groups
        .iter()
        .filter(|(_, group)| is_custom(group) && !group.attr("data-node").is_empty())
        .map(|(path, _)| path)
        .collect();
    groups
        .iter()
        .enumerate()
        .filter_map(|(index, (path, group))| {
            let annotation = group.attr("data-node");
            let annotated = !annotation.is_empty();
            let id = non_empty(annotation)
                .or_else(|| non_empty(group.attr("id")))
                .map_or_else(|| format!("node-{}", index + 1), str::to_owned);
            let allow_overlap = group.attr("data-allow-overlap") == "true";
            if annotated && is_custom(group) {
                let (shape, bounds) = custom_extent(document, path)?;
                let picture = picture_elements(document, path);
                let ports = picture
                    .iter()
                    .filter(|(_, element)| element.name == "circle" && element.has_attr("data-port"))
                    .map(|(port, element)| {
                        let offset = translation(document, port);
                        let at = point(
                            number_attribute(element, "cx") + offset.x,
                            number_attribute(element, "cy") + offset.y,
                        );
                        Port {
                            name: element.attr("data-port").to_owned(),
                            point: at,
                            side: Side::nearest(&bounds, at),
                        }
                    })
                    .collect();
                return Some(Recognized {
                    path: path.clone(),
                    id,
                    shape,
                    bounds,
                    labels: picture
                        .into_iter()
                        .filter(|(_, element)| element.name == "text")
                        .map(|(text, _)| text)
                        .collect(),
                    allow_overlap,
                    custom: true,
                    ports,
                });
            }
            // A group inside a custom node's picture is part of the picture.
            if !annotated
                && custom
                    .iter()
                    .any(|outer| path.len() > outer.len() && path.starts_with(outer))
            {
                return None;
            }
            let children = child_element_paths(document, path);
            let direct_shape = children
                .iter()
                .find(|child| is_node_shape(document.element(child)))
                .cloned();
            let labels: Vec<Path> = children
                .into_iter()
                .filter(|child| document.element(child).name == "text")
                .collect();
            if !annotated && (direct_shape.is_none() || labels.is_empty()) {
                return None;
            }
            let shape_path = if annotated {
                document
                    .descendants(path)
                    .into_iter()
                    .find(|(_, element)| is_node_shape(element))
                    .map(|(candidate, _)| candidate)
            } else {
                direct_shape
            }?;
            Some(Recognized {
                path: path.clone(),
                id,
                shape: (document.element(&shape_path).name == "circle").then_some(Shape::Circle),
                bounds: shape_bounds(document, &shape_path),
                labels,
                allow_overlap,
                custom: false,
                ports: Vec::new(),
            })
        })
        .collect()
}

pub fn read(document: &Document) -> Diagram {
    let phase = tracing::info_span!("read_recognize_nodes").entered();
    let recognized = recognize_nodes(document);
    drop(phase);
    let phase = tracing::info_span!("read_nodes").entered();
    let by_path: HashMap<&[usize], &Recognized> = recognized.iter().map(|node| (node.path.as_slice(), node)).collect();
    let nodes = recognized
        .iter()
        .map(|node| {
            let parent_id = (1..node.path.len())
                .rev()
                .find_map(|depth| by_path.get(&node.path[..depth]))
                .map(|parent| parent.id.clone());
            DiagramNode {
                id: node.id.clone(),
                shape: node.shape,
                bounds: node.bounds,
                label_bounds: node.labels.iter().map(|label| text_bounds(document, label)).collect(),
                parent_id,
                allow_overlap: node.allow_overlap,
                custom: node.custom,
                ports: node.ports.clone(),
            }
        })
        .collect();

    drop(phase);
    let phase = tracing::info_span!("read_connectors").entered();
    let connectors = drawn_connectors(document)
        .into_iter()
        .map(|drawn| {
            let shaft = document.element(&drawn.shaft);
            DiagramConnector {
                from_port: non_empty(shaft.attr("data-from-port")).map(str::to_owned),
                to_port: non_empty(shaft.attr("data-to-port")).map(str::to_owned),
                id: drawn.id,
                from: drawn.from,
                to: drawn.to,
                points: drawn.points,
            }
        })
        .collect();

    drop(phase);
    let phase = tracing::info_span!("read_labels").entered();
    // Labels in document order: a group carrying `data-label` is one label
    // with the texts inside it; texts sharing a `data-label` are one label.
    let labelled: Vec<(Path, &Element)> = document
        .elements()
        .into_iter()
        .filter(|(_, element)| {
            (element.name == "g" || element.name == "text") && !element.attr("data-label").is_empty()
        })
        .collect();
    let texts = label_text_index(document);
    let labels = labelled
        .iter()
        .filter(|(path, _)| {
            !labelled
                .iter()
                .any(|(group, element)| element.name == "g" && path.len() > group.len() && path.starts_with(group))
        })
        .fold(Vec::<DiagramLabel>::new(), |mut labels, (_, element)| {
            let id = element.attr("data-label");
            match labels.iter_mut().find(|label| label.id == id) {
                Some(label) => {
                    label.connector = label
                        .connector
                        .take()
                        .or_else(|| non_empty(element.attr("data-label-for")).map(str::to_owned));
                }
                None => {
                    if let Some(bounds) = label_bounds(document, texts.get(id).map_or(&[][..], Vec::as_slice)) {
                        labels.push(DiagramLabel {
                            id: id.to_owned(),
                            bounds,
                            connector: non_empty(element.attr("data-label-for")).map(str::to_owned),
                        });
                    }
                }
            }
            labels
        });

    drop(phase);
    let unsupported_elements = document
        .root()
        .children
        .iter()
        .enumerate()
        .filter_map(|(index, child)| match child {
            crate::xml::Node::Element(element)
                if ["rect", "circle", "ellipse", "polygon", "path"].contains(&element.name.as_str())
                    && !names_an_end(element) =>
            {
                Some(UnsupportedElement {
                    id: non_empty(element.attr("id"))
                        .map_or_else(|| format!("unsupported-{}", index + 1), str::to_owned),
                    tag_name: element.name.clone(),
                })
            }
            _ => None,
        })
        .collect();

    Diagram {
        nodes,
        connectors,
        labels,
        unsupported_elements,
    }
}
