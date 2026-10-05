//! What a document means as a diagram: nodes, connectors, labels and the
//! shapes left unsupported, read without changing anything.

use std::sync::LazyLock;

use regex::Regex;
use serde::Serialize;

use crate::geometry::{Bounds, Point, enclosing, measure_text, number_list, parse_number, point, points_bounds};
use crate::pathdata;
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
}

impl DiagramNode {
    pub fn is_circle(&self) -> bool {
        self.shape == Some(Shape::Circle)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct DiagramConnector {
    pub id: String,
    pub from: String,
    pub to: String,
    pub points: Vec<Point>,
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

static TRANSLATE: LazyLock<Regex> = LazyLock::new(|| {
    let number = r"[-+]?(?:\d*\.?\d+)(?:[eE][-+]?\d+)?";
    Regex::new(&format!(r"translate\(\s*({number})(?:[\s,]+({number}))?\s*\)")).expect("valid pattern")
});

pub fn number_attribute(element: &Element, name: &str) -> f64 {
    parse_number(element.attr(name))
}

pub fn font_size(element: &Element) -> f64 {
    match number_attribute(element, "font-size") {
        0.0 => 16.0,
        size => size,
    }
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

pub fn text_bounds(document: &Document, path: &[usize]) -> Bounds {
    let element = document.element(path);
    let offset = translation(document, path);
    measure_text(
        element.text_length(),
        font_size(element),
        point(
            number_attribute(element, "x") + offset.x,
            number_attribute(element, "y") + offset.y,
        ),
        element.attr("text-anchor"),
    )
}

fn connector_points(document: &Document, path: &[usize]) -> Vec<Point> {
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

/// The `text` element a label id refers to, by `data-label` or `id`.
pub fn label_text(document: &Document, id: &str) -> Option<Path> {
    document
        .elements_named("text")
        .into_iter()
        .find(|(_, text)| non_empty(text.attr("data-label")).or_else(|| non_empty(text.attr("id"))) == Some(id))
        .map(|(path, _)| path)
}

pub fn connector_elements(document: &Document) -> Vec<(Path, &Element)> {
    ["line", "polyline", "path"]
        .iter()
        .flat_map(|name| document.elements_named(name))
        .collect()
}

fn non_empty(value: &str) -> Option<&str> {
    (!value.is_empty()).then_some(value)
}

struct Recognized {
    path: Path,
    id: String,
    shape_path: Path,
    labels: Vec<Path>,
    allow_overlap: bool,
}

fn recognize_nodes(document: &Document) -> Vec<Recognized> {
    document
        .elements_named("g")
        .into_iter()
        .enumerate()
        .filter_map(|(index, (path, group))| {
            let children = child_element_paths(document, &path);
            let annotation = group.attr("data-node");
            let direct_shape = children
                .iter()
                .find(|child| is_node_shape(document.element(child)))
                .cloned();
            let labels: Vec<Path> = children
                .into_iter()
                .filter(|child| document.element(child).name == "text")
                .collect();
            let annotated = !annotation.is_empty();
            if !annotated && (direct_shape.is_none() || labels.is_empty()) {
                return None;
            }
            let shape_path = if annotated {
                document
                    .elements()
                    .into_iter()
                    .find(|(candidate, element)| {
                        candidate.len() > path.len() && candidate.starts_with(&path) && is_node_shape(element)
                    })
                    .map(|(candidate, _)| candidate)
            } else {
                direct_shape
            }?;
            let id = non_empty(annotation)
                .or_else(|| non_empty(group.attr("id")))
                .map_or_else(|| format!("node-{}", index + 1), str::to_owned);
            Some(Recognized {
                allow_overlap: group.attr("data-allow-overlap") == "true",
                path,
                id,
                shape_path,
                labels,
            })
        })
        .collect()
}

pub fn read(document: &Document) -> Diagram {
    let recognized = recognize_nodes(document);
    let nodes = recognized
        .iter()
        .map(|node| {
            let parent_id = (1..node.path.len())
                .rev()
                .find_map(|depth| recognized.iter().find(|other| other.path == node.path[..depth]))
                .map(|parent| parent.id.clone());
            DiagramNode {
                id: node.id.clone(),
                shape: (document.element(&node.shape_path).name == "circle").then_some(Shape::Circle),
                bounds: shape_bounds(document, &node.shape_path),
                label_bounds: node.labels.iter().map(|label| text_bounds(document, label)).collect(),
                parent_id,
                allow_overlap: node.allow_overlap,
            }
        })
        .collect();

    let connectors = connector_elements(document)
        .into_iter()
        .enumerate()
        .filter_map(|(index, (path, element))| {
            let (from, to) = (element.attr("data-from"), element.attr("data-to"));
            (!from.is_empty() && !to.is_empty()).then(|| DiagramConnector {
                id: non_empty(element.attr("id")).map_or_else(|| format!("connector-{}", index + 1), str::to_owned),
                from: from.to_owned(),
                to: to.to_owned(),
                points: connector_points(document, &path),
            })
        })
        .collect();

    let labels = document
        .elements_named("text")
        .into_iter()
        .filter_map(|(path, text)| {
            let annotation = text.attr("data-label");
            (!annotation.is_empty()).then(|| DiagramLabel {
                id: annotation.to_owned(),
                bounds: text_bounds(document, &path),
                connector: non_empty(text.attr("data-label-for")).map(str::to_owned),
            })
        })
        .collect();

    let unsupported_elements = document
        .root()
        .children
        .iter()
        .enumerate()
        .filter_map(|(index, child)| match child {
            crate::xml::Node::Element(element)
                if ["rect", "circle", "ellipse", "polygon", "path"].contains(&element.name.as_str())
                    && element.attr("data-from").is_empty() =>
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
