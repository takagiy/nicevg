//! Connectors as drawn: a shaft (`line`, `polyline` or `path`) and any
//! arrowheads drawn as separate polygons touching its ends.
//!
//! A connector is annotated either on its shaft or on a group around the
//! shaft and its heads. A head lies at the end it touches, and the
//! connector runs on to the head's tip, where it meets its node.

use crate::diagram::{connector_elements, connector_points, non_empty, translation};
use crate::geometry::{Point, format_number, hypot, number_list, point, round};
use crate::xml::{Document, Element, Path};

/// An arrowhead polygon at one end of a shaft: where its tip points along
/// the shaft's direction there, and how far that tip lies beyond the shaft.
#[derive(Clone, Debug, PartialEq)]
pub struct Head {
    pub path: Path,
    pub at_start: bool,
    pub tip: Point,
    pub direction: Point,
    pub inset: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Drawn {
    pub id: String,
    pub from: String,
    pub to: String,
    pub shaft: Path,
    pub heads: Vec<Head>,
    /// The shaft's points, run on to the tips of its heads.
    pub points: Vec<Point>,
}

/// Every annotated connector: annotated shafts first, in the order they
/// were always numbered in, then annotated groups.
pub fn drawn_connectors(document: &Document) -> Vec<Drawn> {
    let shafts = connector_elements(document);
    let groups: Vec<(Path, &Element)> = document
        .elements_named("g")
        .into_iter()
        .filter(|(_, group)| annotation(group).is_some())
        .collect();
    let from_shafts = shafts.iter().enumerate().filter_map(|(index, (path, element))| {
        let (from, to) = annotation(element)?;
        let id = non_empty(element.attr("id")).map_or_else(|| format!("connector-{}", index + 1), str::to_owned);
        let parent = &path[..path.len() - 1];
        let beside: Vec<Path> = if document.element(parent).name == "g" {
            polygons_within(document, parent)
                .into_iter()
                .filter(|polygon| polygon.len() == path.len())
                .collect()
        } else {
            Vec::new()
        };
        Some(drawn(document, id, from, to, path.clone(), &beside))
    });
    let from_groups = groups.iter().enumerate().filter_map(|(index, (group, element))| {
        let (from, to) = annotation(element)?;
        let shaft = shafts
            .iter()
            .filter(|(path, _)| path.len() > group.len() && path.starts_with(group))
            .min_by_key(|(path, _)| path.clone())?;
        let id = non_empty(element.attr("id"))
            .map_or_else(|| format!("connector-{}", shafts.len() + index + 1), str::to_owned);
        Some(drawn(
            document,
            id,
            from,
            to,
            shaft.0.clone(),
            &polygons_within(document, group),
        ))
    });
    from_shafts.chain(from_groups).collect()
}

/// The node ids a connector joins: `data-from` and `data-to`, or the same
/// as `data-a` and `data-b`.
fn annotation(element: &Element) -> Option<(&str, &str)> {
    [("data-from", "data-to"), ("data-a", "data-b")]
        .into_iter()
        .find_map(|(from, to)| Some((non_empty(element.attr(from))?, non_empty(element.attr(to))?)))
}

/// Whether an element names at least one end of a connector.
pub fn names_an_end(element: &Element) -> bool {
    ["data-from", "data-to", "data-a", "data-b"]
        .iter()
        .any(|name| !element.attr(name).is_empty())
}

fn polygons_within(document: &Document, ancestor: &[usize]) -> Vec<Path> {
    document
        .elements_named("polygon")
        .into_iter()
        .filter(|(path, _)| path.len() > ancestor.len() && path.starts_with(ancestor))
        .map(|(path, _)| path)
        .collect()
}

fn polygon_points(document: &Document, path: &[usize]) -> Vec<Point> {
    let offset = translation(document, path);
    number_list(document.element(path).attr("points"))
        .as_chunks::<2>()
        .0
        .iter()
        .map(|[x, y]| point(x + offset.x, y + offset.y))
        .collect()
}

fn unit(from: Point, to: Point) -> Option<Point> {
    let length = hypot(to.x - from.x, to.y - from.y);
    (length > 0.0).then(|| point((to.x - from.x) / length, (to.y - from.y) / length))
}

fn drawn(document: &Document, id: String, from: &str, to: &str, shaft: Path, polygons: &[Path]) -> Drawn {
    let points = connector_points(document, &shaft);
    let heads: Vec<Head> = match points.as_slice() {
        [first, second, ..] => {
            let (last, before) = (points[points.len() - 1], points[points.len() - 2]);
            [(true, *first, *second), (false, last, before)].into_iter().fold(
                Vec::new(),
                |mut heads, (at_start, end, inner)| {
                    let Some(direction) = unit(inner, end) else {
                        return heads;
                    };
                    let head = polygons
                        .iter()
                        .filter(|polygon| heads.iter().all(|head: &Head| head.path != **polygon))
                        .find_map(|polygon| head_at(document, polygon, end, direction, at_start));
                    heads.extend(head);
                    heads
                },
            )
        }
        _ => Vec::new(),
    };
    let extended = heads.iter().fold(points, |mut points, head| {
        let index = if head.at_start { 0 } else { points.len() - 1 };
        let end = points[index];
        points[index] = point(
            end.x + head.direction.x * head.inset,
            end.y + head.direction.y * head.inset,
        );
        points
    });
    Drawn {
        id,
        from: from.to_owned(),
        to: to.to_owned(),
        shaft,
        heads,
        points: extended,
    }
}

/// The polygon as a head at `end` when its box touches that end; its tip is
/// the vertex furthest along the shaft's direction.
fn head_at(document: &Document, polygon: &[usize], end: Point, direction: Point, at_start: bool) -> Option<Head> {
    let vertices = polygon_points(document, polygon);
    let touching = |pick: fn(&Point) -> f64, value: f64| {
        let values = vertices.iter().map(pick);
        let (low, high) = values.fold((f64::INFINITY, f64::NEG_INFINITY), |(low, high), v| {
            (low.min(v), high.max(v))
        });
        low - 1.0 <= value && value <= high + 1.0
    };
    if vertices.len() < 3 || !touching(|p| p.x, end.x) || !touching(|p| p.y, end.y) {
        return None;
    }
    let along = |vertex: &Point| (vertex.x - end.x) * direction.x + (vertex.y - end.y) * direction.y;
    let tip = vertices
        .iter()
        .copied()
        .reduce(|best, vertex| if along(&vertex) > along(&best) { vertex } else { best })?;
    Some(Head {
        path: polygon.to_vec(),
        at_start,
        tip,
        direction,
        inset: along(&tip).max(0.0),
    })
}

/// The route the shaft takes when its connector follows `route`: each end
/// with a head stops short of it by the head's inset, never past the
/// nearest bend.
pub fn shaft_route(drawn: &Drawn, route: &[Point]) -> Vec<Point> {
    drawn.heads.iter().fold(route.to_vec(), |mut shaft, head| {
        let (end, inner) = if head.at_start {
            (0, 1)
        } else {
            (shaft.len() - 1, shaft.len() - 2)
        };
        let (tip, before) = (route[end], route[inner]);
        let length = hypot(tip.x - before.x, tip.y - before.y);
        let Some(direction) = unit(before, tip) else {
            return shaft;
        };
        let back = head.inset.min(length);
        shaft[end] = point(tip.x - direction.x * back, tip.y - direction.y * back);
        shaft
    })
}

/// The head's polygon turned and moved so its tip sits at the route's end,
/// pointing along the route's last segment there.
pub fn moved_head(document: &Document, head: &Head, route: &[Point]) -> Option<Element> {
    let (tip, inner) = if head.at_start {
        (route[0], route[1])
    } else {
        (route[route.len() - 1], route[route.len() - 2])
    };
    let direction = unit(inner, tip)?;
    let (cos, sin) = (
        head.direction.x * direction.x + head.direction.y * direction.y,
        head.direction.x * direction.y - head.direction.y * direction.x,
    );
    let offset = translation(document, &head.path);
    let tidy = |value: f64| format_number(round(value * 1000.0) / 1000.0);
    let points = polygon_points(document, &head.path)
        .into_iter()
        .map(|vertex| {
            let (dx, dy) = (vertex.x - head.tip.x, vertex.y - head.tip.y);
            let x = tip.x + dx * cos - dy * sin - offset.x;
            let y = tip.y + dx * sin + dy * cos - offset.y;
            format!("{},{}", tidy(x), tidy(y))
        })
        .collect::<Vec<_>>()
        .join(" ");
    Some(document.element(&head.path).clone().with_attr("points", &points))
}
