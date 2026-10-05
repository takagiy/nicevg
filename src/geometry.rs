//! Geometry on diagram coordinates, with the number semantics of the
//! ECMAScript reference implementation the snapshots were recorded with.

use serde::{Serialize, Serializer};

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct Point {
    #[serde(serialize_with = "number")]
    pub x: f64,
    #[serde(serialize_with = "number")]
    pub y: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct Bounds {
    #[serde(serialize_with = "number")]
    pub x: f64,
    #[serde(serialize_with = "number")]
    pub y: f64,
    #[serde(serialize_with = "number")]
    pub width: f64,
    #[serde(serialize_with = "number")]
    pub height: f64,
}

pub const fn point(x: f64, y: f64) -> Point {
    Point { x, y }
}

impl Bounds {
    pub fn right(&self) -> f64 {
        self.x + self.width
    }

    pub fn bottom(&self) -> f64 {
        self.y + self.height
    }

    pub fn centre(&self) -> Point {
        point(self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    pub fn inflate(&self, amount: f64) -> Bounds {
        Bounds {
            x: self.x - amount,
            y: self.y - amount,
            width: self.width + amount * 2.0,
            height: self.height + amount * 2.0,
        }
    }

    /// Space-separated `x y width height`, as viewBox attributes are written.
    pub fn format(&self) -> String {
        [self.x, self.y, self.width, self.height].map(format_number).join(" ")
    }
}

pub type Segment = (Point, Point);

/// `Math.round`: halves round towards positive infinity.
pub fn round(value: f64) -> f64 {
    let floor = value.floor();
    if value - floor >= 0.5 { floor + 1.0 } else { floor }
}

/// `Math.hypot`, which the reference runtime computes with the platform's
/// `hypot` like the standard library does.
pub fn hypot(a: f64, b: f64) -> f64 {
    a.hypot(b)
}

/// `Number.prototype.toString()` for the values diagrams produce.
pub fn format_number(value: f64) -> String {
    if value.is_nan() {
        "NaN".to_owned()
    } else if value.is_infinite() {
        if value > 0.0 { "Infinity" } else { "-Infinity" }.to_owned()
    } else if value == 0.0 {
        "0".to_owned()
    } else if value.fract() == 0.0 && value.abs() < 1e21 {
        format!("{value:.0}")
    } else {
        format!("{value}")
    }
}

/// `Number(string)`: NaN for anything that is not a number.
pub fn js_number(text: &str) -> f64 {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return 0.0;
    }
    match trimmed {
        "Infinity" | "+Infinity" => return f64::INFINITY,
        "-Infinity" => return f64::NEG_INFINITY,
        _ => {}
    }
    let radixes = [("0x", 16), ("0X", 16), ("0o", 8), ("0O", 8), ("0b", 2), ("0B", 2)];
    if let Some((digits, radix)) = radixes
        .iter()
        .find_map(|(prefix, radix)| trimmed.strip_prefix(prefix).map(|digits| (digits, *radix)))
    {
        return u64::from_str_radix(digits, radix).map_or(f64::NAN, |value| value as f64);
    }
    let decimal = trimmed
        .chars()
        .all(|c| c.is_ascii_digit() || matches!(c, '+' | '-' | '.' | 'e' | 'E'));
    if decimal {
        trimmed.parse::<f64>().unwrap_or(f64::NAN)
    } else {
        f64::NAN
    }
}

/// `Number(string)` for attribute values, collapsing anything that is not
/// a finite number to zero as the attribute readers do.
pub fn parse_number(text: &str) -> f64 {
    let value = js_number(text);
    if value.is_finite() { value } else { 0.0 }
}

/// `string.trim().split(/[\s,]+/).map(Number)`.
pub fn number_list(text: &str) -> Vec<f64> {
    static SEPARATORS: std::sync::LazyLock<regex::Regex> =
        std::sync::LazyLock::new(|| regex::Regex::new(r"[\s,]+").expect("valid pattern"));
    SEPARATORS.split(text.trim()).map(js_number).collect()
}

pub fn number<S: Serializer>(value: &f64, serializer: S) -> Result<S::Ok, S::Error> {
    if value.fract() == 0.0 && value.abs() < 9.007_199_254_740_992e15 {
        serializer.serialize_i64(*value as i64)
    } else {
        serializer.serialize_f64(*value)
    }
}

pub fn json_number(value: f64) -> serde_json::Value {
    if value.fract() == 0.0 && value.abs() < 9.007_199_254_740_992e15 {
        serde_json::Value::from(value as i64)
    } else {
        serde_json::Number::from_f64(value)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null)
    }
}

pub fn json_bounds(bounds: &Bounds) -> serde_json::Value {
    serde_json::json!({
        "x": json_number(bounds.x),
        "y": json_number(bounds.y),
        "width": json_number(bounds.width),
        "height": json_number(bounds.height),
    })
}

pub fn enclosing(bounds: &[Bounds]) -> Option<Bounds> {
    if bounds.is_empty() {
        return None;
    }
    let left = bounds.iter().map(|b| b.x).fold(f64::INFINITY, f64::min);
    let top = bounds.iter().map(|b| b.y).fold(f64::INFINITY, f64::min);
    let right = bounds.iter().map(Bounds::right).fold(f64::NEG_INFINITY, f64::max);
    let bottom = bounds.iter().map(Bounds::bottom).fold(f64::NEG_INFINITY, f64::max);
    Some(Bounds {
        x: left,
        y: top,
        width: right - left,
        height: bottom - top,
    })
}

pub fn points_bounds(points: &[Point]) -> Option<Bounds> {
    if points.is_empty() {
        return None;
    }
    let left = points.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
    let top = points.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
    let right = points.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max);
    let bottom = points.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);
    Some(Bounds {
        x: left,
        y: top,
        width: right - left,
        height: bottom - top,
    })
}

pub fn intersection(first: &Bounds, second: &Bounds) -> Option<Bounds> {
    let left = first.x.max(second.x);
    let top = first.y.max(second.y);
    let right = first.right().min(second.right());
    let bottom = first.bottom().min(second.bottom());
    (right > left && bottom > top).then_some(Bounds {
        x: left,
        y: top,
        width: right - left,
        height: bottom - top,
    })
}

pub fn segments(points: &[Point]) -> Vec<Segment> {
    points.windows(2).map(|pair| (pair[0], pair[1])).collect()
}

pub fn segment_length((from, to): &Segment) -> f64 {
    (to.x - from.x).abs() + (to.y - from.y).abs()
}

/// Whether the segment passes through the open interior of the box.
pub fn segment_intersects_interior(start: Point, end: Point, bounds: &Bounds) -> bool {
    let epsilon = 0.001;
    let left = bounds.x + epsilon;
    let right = bounds.x + bounds.width - epsilon;
    let top = bounds.y + epsilon;
    let bottom = bounds.y + bounds.height - epsilon;
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let checks = [
        (-dx, start.x - left),
        (dx, right - start.x),
        (-dy, start.y - top),
        (dy, bottom - start.y),
    ];
    let mut entering = 0.0_f64;
    let mut leaving = 1.0_f64;
    for (direction, distance) in checks {
        if direction == 0.0 {
            if distance < 0.0 {
                return false;
            }
            continue;
        }
        let ratio = distance / direction;
        if direction < 0.0 {
            entering = entering.max(ratio);
        } else {
            leaving = leaving.min(ratio);
        }
        if entering > leaving {
            return false;
        }
    }
    entering <= 1.0 && leaving >= 0.0
}

pub fn route_is_clear(points: &[Point], obstacles: &[Bounds]) -> bool {
    segments(points).iter().all(|(from, to)| {
        obstacles
            .iter()
            .all(|obstacle| !segment_intersects_interior(*from, *to, obstacle))
    })
}

pub fn distance_to_segment(target: Point, start: Point, end: Point) -> f64 {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let length_squared = dx * dx + dy * dy;
    let t = if length_squared == 0.0 {
        0.0
    } else {
        (((target.x - start.x) * dx + (target.y - start.y) * dy) / length_squared).clamp(0.0, 1.0)
    };
    hypot(start.x + t * dx - target.x, start.y + t * dy - target.y)
}

/// Shortest distance between a box and any segment of a route.
pub fn distance_to_route(bounds: &Bounds, points: &[Point]) -> f64 {
    segments(points)
        .iter()
        .map(|(from, to)| {
            let dx = 0.0_f64
                .max(bounds.x - from.x.max(to.x))
                .max(from.x.min(to.x) - bounds.right());
            let dy = 0.0_f64
                .max(bounds.y - from.y.max(to.y))
                .max(from.y.min(to.y) - bounds.bottom());
            hypot(dx, dy)
        })
        .fold(f64::INFINITY, f64::min)
}

fn shared_length(a1: f64, a2: f64, b1: f64, b2: f64) -> f64 {
    a1.max(a2).min(b1.max(b2)) - a1.min(a2).max(b1.min(b2))
}

pub fn segments_overlap((first_start, first_end): &Segment, (second_start, second_end): &Segment) -> bool {
    let horizontal = |start: &Point, end: &Point| start.y == end.y;
    let vertical = |start: &Point, end: &Point| start.x == end.x;
    if horizontal(first_start, first_end) && horizontal(second_start, second_end) && first_start.y == second_start.y {
        return shared_length(first_start.x, first_end.x, second_start.x, second_end.x) > 0.0;
    }
    if vertical(first_start, first_end) && vertical(second_start, second_end) && first_start.x == second_start.x {
        return shared_length(first_start.y, first_end.y, second_start.y, second_end.y) > 0.0;
    }
    false
}

pub fn routes_overlap(first: &[Point], second: &[Point]) -> bool {
    let others = segments(second);
    segments(first)
        .iter()
        .any(|segment| others.iter().any(|other| segments_overlap(segment, other)))
}

pub fn point_is_inside(target: Point, bounds: &Bounds) -> bool {
    target.x > bounds.x && target.x < bounds.right() && target.y > bounds.y && target.y < bounds.bottom()
}

/// Drops points in the middle of straight runs.
pub fn without_redundant_points(points: &[Point]) -> Vec<Point> {
    points
        .iter()
        .enumerate()
        .filter(|(index, current)| {
            let (Some(before), Some(after)) = (
                index.checked_sub(1).and_then(|previous| points.get(previous)),
                points.get(index + 1),
            ) else {
                return true;
            };
            !((before.x == current.x && current.x == after.x) || (before.y == current.y && current.y == after.y))
        })
        .map(|(_, current)| *current)
        .collect()
}

/// Estimates the box of a text run from its anchor point; glyph metrics are
/// approximated by an average advance of 0.59em.
pub fn measure_text(length: usize, font_size: f64, anchor_point: Point, anchor: &str) -> Bounds {
    let width = round(length as f64 * font_size * 0.59);
    let x = match anchor {
        "middle" => anchor_point.x - width / 2.0,
        "end" => anchor_point.x - width,
        _ => anchor_point.x,
    };
    Bounds {
        x,
        y: anchor_point.y - font_size,
        width,
        height: round(font_size * 1.2),
    }
}
