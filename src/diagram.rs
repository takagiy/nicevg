//! What a document means as a diagram: nodes, edges, labels and the
//! shapes left unsupported, read without changing anything.

use std::collections::HashMap;
use std::sync::LazyLock;

use regex::Regex;
use serde::Serialize;

use crate::arrow::{drawn_edges, names_an_end};
use crate::geometry::{Bounds, Point, enclosing, finite_number, parse_number, point, point_list, points_bounds};
use crate::inspect::PARENT_PADDING;
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
    /// The named points its picture marks for edge ends to meet.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub ports: Vec<Port>,
    /// How far its children keep inside its outline, as its `data-padding`
    /// says, when it says.
    #[serde(skip)]
    pub padding: Option<f64>,
    /// A custom parent whose picture is drawn again once laid out
    /// (`data-redrawn`), so its frame may grow meanwhile.
    #[serde(skip)]
    pub redrawn: bool,
}

/// A point a custom node's picture marks `data-port` for edge ends.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Port {
    pub name: String,
    pub point: Point,
    /// The side of the node's bounds it lies on, which edges leave it out of.
    pub side: Side,
}

impl DiagramNode {
    pub fn is_circle(&self) -> bool {
        self.shape == Some(Shape::Circle)
    }

    /// Whether fixing may grow it round its children: a rect of its own,
    /// or the frame of a picture drawn again later, which nicevg rewrites,
    /// rather than a picture kept as drawn or a circle.
    pub fn grows(&self) -> bool {
        (!self.custom || self.redrawn) && !self.is_circle()
    }

    /// How far its children keep inside its outline: its `data-padding`,
    /// else 12px.
    pub fn child_padding(&self) -> f64 {
        self.padding.unwrap_or(PARENT_PADDING)
    }

    /// Its bounds grown just enough to hold `child` its padding inside them.
    pub fn grown_round(&self, child: &Bounds) -> Bounds {
        enclosing(&[self.bounds, child.inflate(self.child_padding())]).expect("two boxes")
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagramEdge {
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
    pub edge: Option<String>,
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
    pub edges: Vec<DiagramEdge>,
    pub labels: Vec<DiagramLabel>,
    pub unsupported_elements: Vec<UnsupportedElement>,
}

impl Diagram {
    pub fn node(&self, id: &str) -> Option<&DiagramNode> {
        self.nodes.iter().find(|node| node.id == id)
    }

    pub fn edge(&self, id: &str) -> Option<&DiagramEdge> {
        self.edges.iter().find(|edge| edge.id == id)
    }

    /// The box around every node, label and edge.
    pub fn diagram_bounds(&self) -> Option<Bounds> {
        let bounds: Vec<Bounds> = self
            .nodes
            .iter()
            .map(|node| node.bounds)
            .chain(self.labels.iter().map(|label| label.bounds))
            .chain(self.edges.iter().filter_map(|edge| points_bounds(&edge.points)))
            .collect();
        enclosing(&bounds)
    }
}

/// Ids of the parents around a node, nearest first.
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

/// Whether a node is one of the edge's ends, a parent around one,
/// or a parent one lies across the outline of: the edge
/// legitimately reaches into each.
pub fn holds_end(nodes: &[DiagramNode], edge: &DiagramEdge, node: &DiagramNode) -> bool {
    let is_parent = || {
        nodes
            .iter()
            .any(|child| child.parent_id.as_deref() == Some(node.id.as_str()))
    };
    let lies_across = || {
        nodes
            .iter()
            .filter(|end| end.id == edge.from || end.id == edge.to)
            .any(|end| crate::geometry::intersection(&end.bounds, &node.bounds).is_some())
    };
    node.id == edge.from
        || node.id == edge.to
        || ancestor_ids(nodes, &edge.from).contains(&node.id.as_str())
        || ancestor_ids(nodes, &edge.to).contains(&node.id.as_str())
        || (is_parent() && lies_across())
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

/// The shape a node's group draws it as: the first in the group, which holds
/// it directly unless it is annotated `data-node`.
pub fn node_shape(document: &Document, group: &[usize]) -> Option<Path> {
    if document.element(group).attr("data-node").is_empty() {
        child_element_paths(document, group)
            .into_iter()
            .find(|child| is_node_shape(document.element(child)))
    } else {
        document
            .descendants(group)
            .into_iter()
            .find(|(_, element)| is_node_shape(element))
            .map(|(shape, _)| shape)
    }
}

fn shape_bounds(document: &Document, path: &[usize]) -> Bounds {
    if document.element(path).name == "circle" {
        circle_bounds(document, path)
    } else {
        rect_bounds(document, path)
    }
}

pub(crate) fn edge_points(document: &Document, path: &[usize]) -> Vec<Point> {
    let element = document.element(path);
    let offset = translation(document, path);
    let shifted = |p: Point| point(p.x + offset.x, p.y + offset.y);
    match element.name.as_str() {
        "line" => vec![
            shifted(point(number_attribute(element, "x1"), number_attribute(element, "y1"))),
            shifted(point(number_attribute(element, "x2"), number_attribute(element, "y2"))),
        ],
        "polyline" => point_list(element.attr("points")).into_iter().map(shifted).collect(),
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
pub fn edge_elements(document: &Document) -> Vec<(Path, &Element)> {
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

#[derive(Clone)]
struct Recognized {
    path: Path,
    id: String,
    shape: Option<Shape>,
    bounds: Bounds,
    labels: Vec<Path>,
    allow_overlap: bool,
    custom: bool,
    ports: Vec<Port>,
    padding: Option<f64>,
    redrawn: bool,
}

/// A node's `data-padding`: its group's, else its frame's.
fn padding(group: &Element, frame: Option<&Element>) -> Option<f64> {
    std::iter::once(group)
        .chain(frame)
        .find_map(|element| finite_number(non_empty(element.attr("data-padding"))?))
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
            "line" | "polyline" | "path" => points_bounds(&edge_points(document, path)),
            "polygon" => {
                let offset = translation(document, path);
                let points: Vec<Point> = point_list(element.attr("points"))
                    .into_iter()
                    .map(|p| point(p.x + offset.x, p.y + offset.y))
                    .collect();
                points_bounds(&points)
            }
            "text" => Some(text_bounds(document, path)),
            _ => None,
        })
        .collect();
    enclosing(&bounds)
}

/// The node a group is, if it is one. `index` is the group's place among
/// the document's groups and `custom` the groups of its custom nodes.
fn recognize(document: &Document, custom: &[&Path], index: usize, path: &Path, group: &Element) -> Option<Recognized> {
    let annotation = group.attr("data-node");
    let annotated = !annotation.is_empty();
    let id = non_empty(annotation)
        .or_else(|| non_empty(group.attr("id")))
        .map_or_else(|| format!("node-{}", index + 1), str::to_owned);
    let allow_overlap = group.attr("data-allow-overlap") == "true";
    if annotated && is_custom(group) {
        let (shape, bounds) = custom_extent(document, path)?;
        let picture = picture_elements(document, path);
        let frame = picture
            .iter()
            .find(|(_, element)| is_node_shape(element) && element.has_attr("data-bounds"))
            .map(|(_, element)| *element);
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
            padding: padding(group, frame),
            redrawn: group.attr("data-redrawn") == "true",
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
    let labels: Vec<Path> = child_element_paths(document, path)
        .into_iter()
        .filter(|child| document.element(child).name == "text")
        .collect();
    if !annotated && labels.is_empty() {
        return None;
    }
    let shape_path = node_shape(document, path)?;
    Some(Recognized {
        path: path.clone(),
        id,
        shape: (document.element(&shape_path).name == "circle").then_some(Shape::Circle),
        bounds: shape_bounds(document, &shape_path),
        labels,
        allow_overlap,
        custom: false,
        ports: Vec::new(),
        padding: padding(group, Some(document.element(&shape_path))),
        redrawn: false,
    })
}

/// A document's nodes as read. Kept for the documents changed from it away
/// from its nodes, which are the same there (proofs/NicevgProofs/Beside.lean).
pub(crate) struct ReadNodes {
    groups: Vec<ReadGroup>,
    pub(crate) nodes: Vec<DiagramNode>,
}

/// A group of the document with the node it is, if any, as recognized, and
/// the bounds of that node's texts.
type ReadGroup = (Path, Option<(Recognized, Vec<Bounds>)>);

/// Whether what stands at `path` and what stands at each of `edited` lie
/// beside each other: neither holds the other.
fn stands_beside(path: &[usize], edited: &[impl AsRef<[usize]>]) -> bool {
    edited.iter().all(|edit| {
        let edit = edit.as_ref();
        !edit.starts_with(path) && !path.starts_with(edit)
    })
}

impl ReadNodes {
    /// Whether replacing the elements at `edited`, none of them a group or
    /// by one, leaves every node as read: none is a node's group, lies
    /// inside one or holds one.
    pub(crate) fn stand_beside(&self, edited: &[&[usize]]) -> bool {
        self.groups
            .iter()
            .filter(|(_, node)| node.is_some())
            .all(|(path, _)| stands_beside(path, edited))
    }
}

/// Reads a document's nodes. `before` gives the nodes of the document this
/// one was changed from and the groups whose own attributes changed: a
/// group beside those is the node it was.
#[tracing::instrument(skip_all)]
pub(crate) fn read_nodes(document: &Document, before: Option<(&ReadNodes, &[Path])>) -> ReadNodes {
    let groups = document.elements_named("g");
    let custom: Vec<&Path> = groups
        .iter()
        .filter(|(_, group)| is_custom(group) && !group.attr("data-node").is_empty())
        .map(|(path, _)| path)
        .collect();
    let before = before.filter(|(read, _)| {
        read.groups.len() == groups.len()
            && read
                .groups
                .iter()
                .zip(&groups)
                .all(|((kept, _), (path, _))| kept == path)
    });
    let groups: Vec<ReadGroup> = groups
        .iter()
        .enumerate()
        .map(|(index, (path, group))| {
            if let Some((read, edited)) = before
                && stands_beside(path, edited)
            {
                return read.groups[index].clone();
            }
            let node = recognize(document, &custom, index, path, group).map(|node| {
                let texts = node.labels.iter().map(|label| text_bounds(document, label)).collect();
                (node, texts)
            });
            (path.clone(), node)
        })
        .collect();
    let recognized: Vec<&(Recognized, Vec<Bounds>)> = groups.iter().filter_map(|(_, node)| node.as_ref()).collect();
    let by_path: HashMap<&[usize], &Recognized> = recognized
        .iter()
        .map(|(node, _)| (node.path.as_slice(), node))
        .collect();
    let nodes = recognized
        .iter()
        .map(|(node, texts)| {
            let parent_id = (1..node.path.len())
                .rev()
                .find_map(|depth| by_path.get(&node.path[..depth]))
                .map(|parent| parent.id.clone());
            DiagramNode {
                id: node.id.clone(),
                shape: node.shape,
                bounds: node.bounds,
                label_bounds: texts.clone(),
                parent_id,
                allow_overlap: node.allow_overlap,
                custom: node.custom,
                ports: node.ports.clone(),
                padding: node.padding,
                redrawn: node.redrawn,
            }
        })
        .collect();
    ReadNodes { groups, nodes }
}

/// The diagram of a document whose nodes are already read.
pub(crate) fn read_around(document: &Document, nodes: Vec<DiagramNode>) -> Diagram {
    let phase = tracing::info_span!("read_edges").entered();
    let edges = drawn_edges(document)
        .into_iter()
        .map(|drawn| {
            let shaft = document.element(&drawn.shaft);
            DiagramEdge {
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
                    label.edge = label
                        .edge
                        .take()
                        .or_else(|| non_empty(element.attr("data-label-for")).map(str::to_owned));
                }
                None => {
                    if let Some(bounds) = label_bounds(document, texts.get(id).map_or(&[][..], Vec::as_slice)) {
                        labels.push(DiagramLabel {
                            id: id.to_owned(),
                            bounds,
                            edge: non_empty(element.attr("data-label-for")).map(str::to_owned),
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
        edges,
        labels,
        unsupported_elements,
    }
}
