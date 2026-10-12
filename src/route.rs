//! Orthogonal routing: choosing node sides and ports, searching a sparse
//! grid for the cheapest route, and finding label slots beside routes.

use std::cmp::{Ordering, Reverse};
use std::collections::{BinaryHeap, HashMap};

use crate::diagram::DiagramNode;
use crate::geometry::{
    Bounds, Point, distance_to_route, distance_to_segment, enclosing, hypot, intersection, point, round,
    route_is_clear, routes_overlap, segment_intersects_interior, segment_length, segments, without_redundant_points,
};

pub const BEND_PENALTY: f64 = 40.0;
/// The most end terminals a search is given: three stubs on each of the
/// four sides.
const MAX_ENDS: usize = 12;
/// A route's first two turns cost the bend penalty; each later one costs
/// this many times as much, so a route turns twice rather than four times
/// when it can.
const LATE_TURN_FACTOR: f64 = 3.0;
const EASY_TURNS: usize = 2;
/// Choosing sides weighs bends more than routing does: a side that saves a
/// bend wins even when the route to it runs a little longer.
const SIDE_BEND_PENALTY: f64 = 80.0;
/// Below one unit of length: changing a side only wins when it saves length
/// or bends, never on a tie.
pub const SIDE_CHANGE_COST: f64 = 0.5;
/// Cost of each connector already attached to a side; below two bends, so
/// one neighbour never forces an extra bend but a bundle can.
pub const CROWDING_COST: f64 = 30.0;
pub const LANE_SPACING: f64 = 10.0;
/// Grid lines this close or closer add vertices without giving a route any
/// real choice, so they are merged or left out.
const LINE_MERGE: f64 = 4.0;
/// Running alongside another connector closer than a lane costs this
/// much, once however long the two run together: closely packed lines read
/// as one band. Half as much 12px apart and nothing from 16px, where the
/// lines already read as two.
pub(crate) const CLOSE_RUN: Fade = Fade {
    start: 8.0,
    reach: 16.0,
    full: 120.0,
};
/// How much more crossing a route costs by how near it runs alongside the
/// other somewhere: in full within 20px, fading out by 40px.
const TANGLED: Fade = Fade {
    start: 20.0,
    reach: 40.0,
    full: 1.6,
};
/// How much of running alongside two segments pay by how far they overlap:
/// overlapping less than 20px they only brush past each other.
const BRUSHING: Fade = Fade {
    start: 0.0,
    reach: 20.0,
    full: 1.0,
};
/// Crossing another connector costs as much as a detour of 300px: a route
/// goes the long way round rather than cross when it can.
pub(crate) const CROSSING_PENALTY: f64 = 300.0;
/// Turning on or just beside another connector, or running past its turn,
/// makes the two read as one line turning: worse than a crossing.
const NEAR_MISS_PENALTY: f64 = 2.0 * CROSSING_PENALTY;
pub const CLEARANCE: f64 = 8.0;
/// A box a route may push aside, such as another connector's label, and
/// what pushing it costs, worked out only once a route reaches it: `None`
/// when it cannot move.
pub type Pushable<'a> = (Bounds, &'a dyn Fn() -> Option<f64>);
/// No pushable box on a step.
const NO_BOX: u32 = u32::MAX;
/// How far a route runs straight into its target before the last bend,
/// longest first. An arrowhead is about 10px long, so the clearance alone
/// leaves the last bend under it; shorter stubs are the fallbacks when
/// another node is in the way.
pub const END_STUBS: [f64; 3] = [24.0, 16.0, CLEARANCE];
/// An end segment shorter than this, or a crossing nearer the node than
/// this, makes it hard to see the connector arrive.
pub(crate) const SHORT_END: f64 = 20.0;
/// What a route searched or a diagram arranged pays for an end segment
/// that short: in full up to the shorter stub, half at a short end, nothing
/// from the full stub on.
pub(crate) const SHORT_END_FADE: Fade = Fade {
    start: 16.0,
    reach: 24.0,
    full: 300.0,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Side {
    Top,
    Right,
    Bottom,
    Left,
}

impl Side {
    pub const ALL: [Side; 4] = [Side::Top, Side::Right, Side::Bottom, Side::Left];

    pub fn name(self) -> &'static str {
        match self {
            Side::Top => "top",
            Side::Right => "right",
            Side::Bottom => "bottom",
            Side::Left => "left",
        }
    }

    fn is_horizontal_edge(self) -> bool {
        matches!(self, Side::Top | Side::Bottom)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ports {
    pub start: Point,
    pub end: Point,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sides {
    pub start: Side,
    pub end: Side,
}

/// Ports facing along the dominant axis between two boxes.
pub fn connector_ports(source: &Bounds, target: &Bounds) -> Ports {
    let (from, to) = (source.centre(), target.centre());
    if (to.x - from.x).abs() >= (to.y - from.y).abs() {
        let right = to.x >= from.x;
        Ports {
            start: point(if right { source.right() } else { source.x }, from.y),
            end: point(if right { target.x } else { target.right() }, to.y),
        }
    } else {
        let down = to.y >= from.y;
        Ports {
            start: point(from.x, if down { source.bottom() } else { source.y }),
            end: point(to.x, if down { target.y } else { target.bottom() }),
        }
    }
}

pub fn side_of(port: Point, bounds: &Bounds) -> Side {
    if port.x == bounds.x {
        Side::Left
    } else if port.x == bounds.right() {
        Side::Right
    } else if port.y == bounds.y {
        Side::Top
    } else {
        Side::Bottom
    }
}

pub fn is_on_outline(target: Point, bounds: &Bounds) -> bool {
    let within_x = target.x >= bounds.x && target.x <= bounds.right();
    let within_y = target.y >= bounds.y && target.y <= bounds.bottom();
    (within_x && (target.y == bounds.y || target.y == bounds.bottom()))
        || (within_y && (target.x == bounds.x || target.x == bounds.right()))
}

pub fn port_on_side(bounds: &Bounds, side: Side) -> Point {
    point(
        match side {
            Side::Left => bounds.x,
            Side::Right => bounds.right(),
            _ => round(bounds.x + bounds.width / 2.0),
        },
        match side {
            Side::Top => bounds.y,
            Side::Bottom => bounds.bottom(),
            _ => round(bounds.y + bounds.height / 2.0),
        },
    )
}

fn outward(port: Point, side: Side, distance: f64) -> Point {
    point(
        port.x
            + match side {
                Side::Left => -distance,
                Side::Right => distance,
                _ => 0.0,
            },
        port.y
            + match side {
                Side::Top => -distance,
                Side::Bottom => distance,
                _ => 0.0,
            },
    )
}

/// A place a route may start or end: the point outside a port, the axis the
/// route must leave or arrive along (0 horizontal, 1 vertical), the port it
/// leads to and an extra cost for choosing it.
#[derive(Clone, Copy, Debug)]
pub struct Terminal {
    pub point: Point,
    pub axis: usize,
    pub port: Point,
    pub cost: f64,
}

fn terminal_at(port: Point, bounds: &Bounds, cost: f64) -> Terminal {
    stub_terminal(port, bounds, CLEARANCE, cost)
}

fn stub_terminal(port: Point, bounds: &Bounds, length: f64, cost: f64) -> Terminal {
    let side = side_of(port, bounds);
    Terminal {
        point: outward(port, side, length),
        axis: usize::from(side.is_horizontal_edge()),
        port,
        cost,
    }
}

/// Ends at each stub length whose straight run to the port stays clear of
/// the obstacles; every pixel of stub given up costs a pixel of route, and
/// a stub too short to see the arrival costs a short end on top.
fn end_terminals(port: Point, target: &Bounds, obstacles: &[Bounds]) -> Vec<Terminal> {
    END_STUBS
        .iter()
        .filter(|length| {
            **length == CLEARANCE || route_is_clear(&[port, outward(port, side_of(port, target), **length)], obstacles)
        })
        .map(|length| {
            let short = SHORT_END_FADE.cost(*length);
            stub_terminal(port, target, *length, END_STUBS[0] - length + short)
        })
        .collect()
}

/// Whether a move from `here` to `next` arrives at an end terminal from the
/// port's side, which would double back along the stub.
fn arrives_backwards(here: Point, next: Point, terminal: &Terminal) -> bool {
    (next.x - here.x) * (terminal.port.x - next.x) + (next.y - here.y) * (terminal.port.y - next.y) < 0.0
}

pub struct Found {
    pub path: Vec<Point>,
    pub start: Option<usize>,
    pub end: Option<usize>,
}

/// Sorted distinct values, like `[...new Set(values)].sort((a, b) => a - b)`.
fn sorted_distinct(values: impl IntoIterator<Item = f64>) -> Vec<f64> {
    let mut sorted: Vec<f64> = values.into_iter().collect();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    sorted.dedup_by(|a, b| a == b);
    sorted
}

fn index_of(lines: &[f64], value: f64) -> Option<usize> {
    let index = lines.partition_point(|line| *line < value);
    (lines.get(index) == Some(&value)).then_some(index)
}

/// Indices of the grid steps (between lines[i] and lines[i + 1]) that share
/// some length with the span from a to b; lines are sorted ascending.
fn steps_covering(lines: &[f64], a: f64, b: f64) -> std::ops::Range<usize> {
    let (low, high) = (a.min(b), a.max(b));
    lines.partition_point(|line| *line <= low).saturating_sub(1)
        ..lines
            .partition_point(|line| *line < high)
            .min(lines.len().saturating_sub(1))
}

/// Indices of the lines strictly between low and high; lines are sorted
/// ascending.
fn lines_inside(lines: &[f64], low: f64, high: f64) -> std::ops::Range<usize> {
    lines.partition_point(|line| *line <= low)..lines.partition_point(|line| *line < high)
}

/// A queued state: its priority (cost so far plus estimate), the state and
/// its cost so far when queued.
#[derive(PartialEq)]
struct Entry(f64, usize, f64);

impl Eq for Entry {}

impl Ord for Entry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.total_cmp(&other.0).then(self.1.cmp(&other.1))
    }
}

impl PartialOrd for Entry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// The grid a route is searched on and what each of its points and steps
/// costs. A point (xi, yi) is at `yi * xs.len() + xi`, and a step at the
/// index of its left or upper point.
struct Grid {
    xs: Vec<f64>,
    ys: Vec<f64>,
    /// Points inside an obstacle.
    blocked_cell: Vec<bool>,
    /// Steps entering an obstacle or running on another connector.
    blocked_horizontal: Vec<bool>,
    blocked_vertical: Vec<bool>,
    /// Steps on an obstacle's clearance outline.
    outline_horizontal: Vec<bool>,
    outline_vertical: Vec<bool>,
    /// Points inside a box the route may push aside, such as another
    /// connector's label, and the box each step enters, if any.
    pushable_cell: Vec<bool>,
    pushable_horizontal: Vec<u32>,
    pushable_vertical: Vec<u32>,
    /// What running alongside another connector costs on each step.
    near_horizontal: Vec<f64>,
    near_vertical: Vec<f64>,
    /// Steps crossing another connector.
    crossing_horizontal: Vec<bool>,
    crossing_vertical: Vec<bool>,
    /// Points within the near-miss distance of another connector.
    turn_beside: Vec<bool>,
    /// Steps passing within the near-miss distance of another's turn.
    past_turn_horizontal: Vec<bool>,
    past_turn_vertical: Vec<bool>,
    /// Running counts of blocked and crossing steps along each row and
    /// each column.
    row_blocked: Vec<u32>,
    row_crossing: Vec<u32>,
    column_blocked: Vec<u32>,
    column_crossing: Vec<u32>,
}

/// Which lines a grid draws.
#[derive(Clone, Copy)]
enum Lines {
    /// Every line.
    All,
    /// No lines along the connectors and rails or through the middle of
    /// channels.
    Coarse,
    /// Only the lines inside an area.
    Inside(Bounds),
}

/// Builds the grid for a search from the obstacles, the connectors already
/// drawn, the rails and the terminals, drawing the `lines` asked for.
fn grid(
    starts: &[Terminal],
    ends: &[Terminal],
    obstacles: &[Bounds],
    pushable: &[Pushable<'_>],
    occupied: &[Vec<Point>],
    rails: &[(Point, Point)],
    lines: Lines,
) -> Grid {
    let phase = tracing::info_span!("search_grid_lines").entered();
    let coarse = matches!(lines, Lines::Coarse);
    let area = match lines {
        Lines::Inside(area) => Some(area),
        Lines::All | Lines::Coarse => None,
    };
    // Inside an area, only the obstacles' parts within it count, kept one
    // unit past its edges so the edge lines stay inside what they cut.
    let area = area.map(|area| area.inflate(1.0));
    let clip = |b: &Bounds| -> Option<Bounds> {
        let Some(area) = area else { return Some(*b) };
        if !(b.x < area.right() && area.x < b.right() && b.y < area.bottom() && area.y < b.bottom()) {
            return None;
        }
        let (left, top) = (b.x.max(area.x), b.y.max(area.y));
        Some(Bounds {
            x: left,
            y: top,
            width: b.right().min(area.right()) - left,
            height: b.bottom().min(area.bottom()) - top,
        })
    };
    let clipped: Vec<Bounds> = obstacles.iter().filter_map(clip).collect();
    let (pushed_boxes, box_indices): (Vec<Bounds>, Vec<u32>) = pushable
        .iter()
        .enumerate()
        .filter_map(|(index, (b, _))| Some((clip(b)?, index as u32)))
        .unzip();
    // Pushable boxes get their own lines like obstacles, after them.
    let all = [clipped.as_slice(), pushed_boxes.as_slice()].concat();
    let obstacles = all.as_slice();
    let crossed_segments: Vec<(Point, Point)> = occupied.iter().flat_map(|route| segments(route)).collect();
    let occupied_segments: Vec<(Point, Point)> = crossed_segments.iter().chain(rails).copied().collect();
    let axis = |pick: fn(&Point) -> f64, low: fn(&Bounds) -> f64, high: fn(&Bounds) -> f64| {
        let spans: Vec<(f64, f64)> = obstacles.iter().map(|bounds| (low(bounds), high(bounds))).collect();
        let bounds_of_area = area.map(|area| (low(&area), high(&area)));
        let within = |value: f64| bounds_of_area.is_none_or(|(from, to)| from <= value && value <= to);
        let terminals: Vec<f64> = starts
            .iter()
            .chain(ends)
            .map(|terminal| pick(&terminal.point))
            .chain(bounds_of_area.into_iter().flat_map(|(from, to)| [from, to]))
            .collect();
        // Connectors (and rails) running along this axis' lines; a coarse
        // grid leaves them out.
        let along: Vec<f64> = occupied_segments
            .iter()
            .filter(|(from, to)| !coarse && pick(from) == pick(to) && within(pick(from)))
            .map(|(from, _)| pick(from))
            .collect();
        let (lines, spans) = axis_lines(&spans, &terminals, &along, &along, !coarse);
        // Obstacles pushed out past the area stop at its edges, which are lines.
        let spans = match bounds_of_area {
            Some((from, to)) => spans
                .into_iter()
                .map(|(low, high)| (low.max(from), high.min(to)))
                .collect(),
            None => spans,
        };
        (
            lines.into_iter().filter(|line| within(*line)).collect::<Vec<_>>(),
            spans,
        )
    };
    let (xs, x_spans) = axis(|p| p.x, |b| b.x, Bounds::right);
    let (ys, y_spans) = axis(|p| p.y, |b| b.y, Bounds::bottom);
    // Obstacles pushed outwards onto the shared lines.
    let mut obstacles: Vec<Bounds> = x_spans
        .iter()
        .zip(&y_spans)
        .map(|((left, right), (top, bottom))| Bounds {
            x: *left,
            y: *top,
            width: right - left,
            height: bottom - top,
        })
        .collect();
    let pushable = obstacles.split_off(clipped.len());
    drop(phase);
    let width = xs.len();
    let cells = width * ys.len();
    let key = |xi: usize, yi: usize| yi * width + xi;

    let phase = tracing::info_span!("search_obstacles").entered();
    // Grid lines include every obstacle edge, so an edge between neighbouring
    // grid points enters an obstacle exactly when its midpoint is inside it.
    let mut blocked_cell = vec![false; cells];
    let mut blocked_horizontal = vec![false; cells];
    let mut blocked_vertical = vec![false; cells];
    // Edges on an obstacle's clearance outline carry a tiny extra cost that
    // only breaks ties, so equal detours run mid-channel instead of skirting
    // a node.
    let mut outline_horizontal = vec![false; cells];
    let mut outline_vertical = vec![false; cells];
    // Points and steps inside a box the route may push aside.
    let mut pushable_cell = vec![false; cells];
    let mut pushable_horizontal = vec![NO_BOX; cells];
    let mut pushable_vertical = vec![NO_BOX; cells];
    for (bounds, box_index) in pushable.iter().zip(&box_indices) {
        let (Some(left), Some(right), Some(top), Some(bottom)) = (
            index_of(&xs, bounds.x),
            index_of(&xs, bounds.right()),
            index_of(&ys, bounds.y),
            index_of(&ys, bounds.bottom()),
        ) else {
            continue;
        };
        for yi in top..=bottom {
            for xi in left..=right {
                let index = key(xi, yi);
                let inside_x = xi > left && xi < right;
                let inside_y = yi > top && yi < bottom;
                pushable_cell[index] |= inside_x && inside_y;
                if inside_y && xi < right {
                    pushable_horizontal[index] = *box_index;
                }
                if inside_x && yi < bottom {
                    pushable_vertical[index] = *box_index;
                }
            }
        }
    }
    for bounds in &obstacles {
        let (Some(left), Some(right), Some(top), Some(bottom)) = (
            index_of(&xs, bounds.x),
            index_of(&xs, bounds.right()),
            index_of(&ys, bounds.y),
            index_of(&ys, bounds.bottom()),
        ) else {
            continue;
        };
        for yi in top..=bottom {
            for xi in left..=right {
                let index = key(xi, yi);
                let inside_x = xi > left && xi < right;
                let inside_y = yi > top && yi < bottom;
                blocked_cell[index] |= inside_x && inside_y;
                blocked_horizontal[index] |= inside_y && xi < right;
                blocked_vertical[index] |= inside_x && yi < bottom;
                outline_horizontal[index] |= (yi == top || yi == bottom) && xi < right;
                outline_vertical[index] |= (xi == left || xi == right) && yi < bottom;
            }
        }
    }
    drop(phase);
    let phase = tracing::info_span!("search_taken").entered();
    // Grid edges that share any length with another connector's segment are
    // taken, including segments shorter than one grid step.
    for (from, to) in &occupied_segments {
        if from.y == to.y {
            let Some(yi) = index_of(&ys, from.y) else { continue };
            for xi in steps_covering(&xs, from.x, to.x) {
                blocked_horizontal[key(xi, yi)] = true;
            }
        } else if from.x == to.x {
            let Some(xi) = index_of(&xs, from.x) else { continue };
            for yi in steps_covering(&ys, from.y, to.y) {
                blocked_vertical[key(xi, yi)] = true;
            }
        }
    }

    drop(phase);
    let phase = tracing::info_span!("search_near_runs").entered();
    // Grid edges running parallel to another connector's segment, closer
    // than two lanes, run alongside it: a route pays for that once, as it
    // starts running alongside.
    let mut near_horizontal = vec![0.0f64; cells];
    let mut near_vertical = vec![0.0f64; cells];
    // Only lines closer than the reach of running alongside cost anything.
    let reach = CLOSE_RUN.reach;
    for (from, to) in &occupied_segments {
        if from.y == to.y {
            let steps = steps_covering(&xs, from.x, to.x);
            for yi in lines_inside(&ys, from.y - reach, from.y + reach) {
                let extra = close_run_cost((ys[yi] - from.y).abs());
                for xi in steps.clone() {
                    let index = key(xi, yi);
                    near_horizontal[index] = near_horizontal[index].max(extra);
                }
            }
        } else if from.x == to.x {
            let steps = steps_covering(&ys, from.y, to.y);
            for xi in lines_inside(&xs, from.x - reach, from.x + reach) {
                let extra = close_run_cost((xs[xi] - from.x).abs());
                for yi in steps.clone() {
                    let index = key(xi, yi);
                    near_vertical[index] = near_vertical[index].max(extra);
                }
            }
        }
    }

    drop(phase);
    let phase = tracing::info_span!("search_crossings").entered();
    // Grid edges that cross another connector's segment, or reach a point
    // inside it, cost a crossing. An edge ending on the segment counts, the
    // one leaving it does not, so passing straight through counts once.
    let mut crossing_horizontal = vec![false; cells];
    let mut crossing_vertical = vec![false; cells];
    // The step that reaches a value: the one from the last line below it.
    let step_reaching = |lines: &[f64], value: f64| {
        let next = lines.partition_point(|line| *line < value);
        (next > 0 && next < lines.len()).then(|| next - 1)
    };
    for (from, to) in &crossed_segments {
        if from.x == to.x {
            let Some(xi) = step_reaching(&xs, from.x) else { continue };
            for yi in lines_inside(&ys, from.y.min(to.y), from.y.max(to.y)) {
                crossing_horizontal[key(xi, yi)] = true;
            }
        } else if from.y == to.y {
            let Some(yi) = step_reaching(&ys, from.y) else { continue };
            for xi in lines_inside(&xs, from.x.min(to.x), from.x.max(to.x)) {
                crossing_vertical[key(xi, yi)] = true;
            }
        }
    }

    drop(phase);
    let phase = tracing::info_span!("search_near_misses").entered();
    let rows = ys.len();
    // Grid points within the near-miss distance of another connector cost
    // extra to turn at, and grid edges passing that close to another
    // connector's turn cost extra to take.
    let reach = crate::inspect::NEAR_MISS_DISTANCE;
    let lines_near = |lines: &[f64], low: f64, high: f64| {
        lines.partition_point(|line| *line <= low - reach)..lines.partition_point(|line| *line < high + reach)
    };
    let mut turn_beside = vec![false; cells];
    for (from, to) in &crossed_segments {
        for yi in lines_near(&ys, from.y.min(to.y), from.y.max(to.y)) {
            for xi in lines_near(&xs, from.x.min(to.x), from.x.max(to.x)) {
                if distance_to_segment(point(xs[xi], ys[yi]), *from, *to) < reach {
                    turn_beside[key(xi, yi)] = true;
                }
            }
        }
    }
    let mut past_turn_horizontal = vec![false; cells];
    let mut past_turn_vertical = vec![false; cells];
    for corner in occupied
        .iter()
        .flat_map(|route| route.iter().skip(1).take(route.len().saturating_sub(2)))
    {
        let (columns, rows_near) = (lines_near(&xs, corner.x, corner.x), lines_near(&ys, corner.y, corner.y));
        // Edges along the rows near the turn, from the line before it.
        for yi in rows_near.clone() {
            for xi in columns.start.saturating_sub(1)..columns.end.min(width.saturating_sub(1)) {
                if distance_to_segment(*corner, point(xs[xi], ys[yi]), point(xs[xi + 1], ys[yi])) < reach {
                    past_turn_horizontal[key(xi, yi)] = true;
                }
            }
        }
        for xi in columns {
            for yi in rows_near.start.saturating_sub(1)..rows_near.end.min(rows.saturating_sub(1)) {
                if distance_to_segment(*corner, point(xs[xi], ys[yi]), point(xs[xi], ys[yi + 1])) < reach {
                    past_turn_vertical[key(xi, yi)] = true;
                }
            }
        }
    }
    // Running counts along each row of the horizontal edges (and along each
    // column of the vertical ones) that cannot be taken or that cross a
    // connector, so a straight run's count is a difference of two.
    let running = |edges: &[bool], along_rows: bool| {
        let (lines, steps) = if along_rows { (rows, width) } else { (width, rows) };
        let mut counts = vec![0u32; lines * (steps + 1)];
        for line in 0..lines {
            for step in 0..steps.saturating_sub(1) {
                let edge = if along_rows { key(step, line) } else { key(line, step) };
                counts[line * (steps + 1) + step + 1] = counts[line * (steps + 1) + step] + u32::from(edges[edge]);
            }
        }
        counts
    };
    let row_blocked = running(&blocked_horizontal, true);
    let row_crossing = running(&crossing_horizontal, true);
    let column_blocked = running(&blocked_vertical, false);
    let column_crossing = running(&crossing_vertical, false);
    drop(phase);
    Grid {
        xs,
        ys,
        blocked_cell,
        blocked_horizontal,
        blocked_vertical,
        outline_horizontal,
        outline_vertical,
        pushable_cell,
        pushable_horizontal,
        pushable_vertical,
        near_horizontal,
        near_vertical,
        crossing_horizontal,
        crossing_vertical,
        turn_beside,
        past_turn_horizontal,
        past_turn_vertical,
        row_blocked,
        row_crossing,
        column_blocked,
        column_crossing,
    }
}

/// A* over a sparse orthogonal grid built from obstacle edges, the channels
/// between them, lanes beside drawn connectors and the terminals. Starts
/// from whichever start terminal and stops at whichever end terminal is
/// cheapest overall.
/// `rails` are lines a route may cross but should not run along, such as
/// the borders of containers around its ends: running on one is ruled
/// out and running close beside one costs like running beside a connector.
#[tracing::instrument(skip_all, fields(columns = tracing::field::Empty, rows = tracing::field::Empty, occupied = occupied.len(), obstacles = obstacles.len()))]
pub fn search(
    starts: &[Terminal],
    ends: &[Terminal],
    obstacles: &[Bounds],
    pushable: &[Pushable<'_>],
    occupied: &[Vec<Point>],
    rails: &[(Point, Point)],
    bend_penalty: f64,
) -> Option<Found> {
    let grid = grid(starts, ends, obstacles, pushable, occupied, rails, Lines::All);
    tracing::Span::current()
        .record("columns", grid.xs.len())
        .record("rows", grid.ys.len());
    astar(&grid, starts, ends, bend_penalty, None, pushable)
}

/// Like `search`, but quicker and not always the cheapest: a coarse grid,
/// without the lines along other connectors or through the middle of
/// channels, finds the way round, and the fine grid is built and searched
/// only along the channels that way passes. Picking node sides needs only
/// where the route ends.
#[tracing::instrument(skip_all)]
fn search_along_rough_route(
    starts: &[Terminal],
    ends: &[Terminal],
    obstacles: &[Bounds],
    pushable: &[Pushable<'_>],
    occupied: &[Vec<Point>],
    rails: &[(Point, Point)],
    bend_penalty: f64,
) -> Option<Found> {
    let coarse = grid(starts, ends, obstacles, pushable, occupied, rails, Lines::Coarse);
    let corridor = astar(&coarse, starts, ends, bend_penalty, None, pushable)
        .map(|rough| corridor(&rough.path, &coarse.xs, &coarse.ys));
    corridor
        .and_then(|corridor| {
            let fine = grid(
                starts,
                ends,
                obstacles,
                pushable,
                occupied,
                rails,
                enclosing(&corridor).map_or(Lines::All, Lines::Inside),
            );
            astar(&fine, starts, ends, bend_penalty, Some(&corridor), pushable)
        })
        .or_else(|| search(starts, ends, obstacles, pushable, occupied, rails, bend_penalty))
}

/// The channels a route on a grid runs along: each segment widened to the
/// grid lines on either side of it.
fn corridor(path: &[Point], xs: &[f64], ys: &[f64]) -> Vec<Bounds> {
    let span = |lines: &[f64], a: f64, b: f64| {
        let low = lines[lines.partition_point(|line| *line < a.min(b)).saturating_sub(1)];
        let high = lines[lines.partition_point(|line| *line <= a.max(b)).min(lines.len() - 1)];
        (low, high)
    };
    path.windows(2)
        .map(|pair| {
            let ((left, right), (top, bottom)) = (span(xs, pair[0].x, pair[1].x), span(ys, pair[0].y, pair[1].y));
            Bounds {
                x: left,
                y: top,
                width: right - left,
                height: bottom - top,
            }
        })
        .collect()
}

/// A* over a grid, from whichever start to whichever end is cheapest,
/// through the points inside `corridor` when one is given.
fn astar(
    grid: &Grid,
    starts: &[Terminal],
    ends: &[Terminal],
    bend_penalty: f64,
    corridor: Option<&[Bounds]>,
    pushable: &[Pushable<'_>],
) -> Option<Found> {
    let Grid {
        xs,
        ys,
        blocked_cell,
        blocked_horizontal,
        blocked_vertical,
        outline_horizontal,
        outline_vertical,
        pushable_cell,
        pushable_horizontal,
        pushable_vertical,
        near_horizontal,
        near_vertical,
        crossing_horizontal,
        crossing_vertical,
        turn_beside,
        past_turn_horizontal,
        past_turn_vertical,
        row_blocked,
        row_crossing,
        column_blocked,
        column_crossing,
    } = grid;
    let (width, rows) = (xs.len(), ys.len());
    let cells = width * rows;
    let key = |xi: usize, yi: usize| yi * width + xi;
    let phase = tracing::info_span!("search_heuristic").entered();
    // Edges taken and crossings on the straight run between two grid points
    // on one row or one column.
    let run_counts = |from: (usize, usize), to: (usize, usize)| -> (u32, u32) {
        if from.1 == to.1 {
            let (low, high) = (from.0.min(to.0), from.0.max(to.0));
            let base = from.1 * (width + 1);
            (
                row_blocked[base + high] - row_blocked[base + low],
                row_crossing[base + high] - row_crossing[base + low],
            )
        } else {
            let (low, high) = (from.1.min(to.1), from.1.max(to.1));
            let base = from.0 * (rows + 1);
            (
                column_blocked[base + high] - column_blocked[base + low],
                column_crossing[base + high] - column_crossing[base + low],
            )
        }
    };
    let end_indices: Vec<Option<(usize, usize)>> = ends
        .iter()
        .map(|terminal| Some((index_of(xs, terminal.point.x)?, index_of(ys, terminal.point.y)?)))
        .collect();
    drop(phase);
    let phase = tracing::info_span!("search_astar", visited = tracing::field::Empty).entered();

    let cell_of = |target: &Point| Some(key(index_of(xs, target.x)?, index_of(ys, target.y)?));
    let end_cells: HashMap<usize, (usize, Terminal)> = ends
        .iter()
        .enumerate()
        .filter_map(|(index, terminal)| Some((cell_of(&terminal.point)?, (index, *terminal))))
        .collect();
    let point_of = |cell: usize| point(xs[cell % width], ys[cell / width]);
    // A state is a cell, the axis the route arrived along (0: horizontally,
    // 1: vertically) and how many times it has turned, counting up to the
    // turns that cost the same.
    let levels = EASY_TURNS + 1;
    let state_of = |cell: usize, axis: usize, turned: usize| (cell * 2 + axis) * levels + turned.min(EASY_TURNS);
    let cell_of_state = |state: usize| state / levels / 2;
    let axis_of = |state: usize| state / levels % 2;
    let turned_of = |state: usize| state % levels;
    // A*: a lower bound on what is left to pay, so the cheapest route is
    // still found. Every edge costs at least its length and the turns the
    // route cannot avoid cost at least their price, so the end stub's own
    // cost, the Manhattan distance and those turns are one. When the fewest
    // turns leave a single way in, the route also pays either for what
    // blocks or crosses that way or for two more turns to leave it.
    let estimate = |state: usize| {
        // An end reached is paid for in full, its stub and any turn onto it.
        if end_cells.contains_key(&cell_of_state(state)) {
            return 0.0;
        }
        let cell = cell_of_state(state);
        let here = point_of(cell);
        let (xi, yi) = (cell % width, cell / width);
        let (axis, turned) = (axis_of(state), turned_of(state));
        if ends.len() > MAX_ENDS {
            return ends
                .iter()
                .map(|terminal| (terminal.point.x - here.x).abs() + (terminal.point.y - here.y).abs())
                .fold(f64::INFINITY, f64::min);
        }
        // The cheap part first; an end whose cheap part already reaches the
        // best total cannot win, so its way in is not looked at.
        let mut cheap = [(f64::INFINITY, 0usize, 0usize); MAX_ENDS];
        for (index, terminal) in ends.iter().enumerate().take(MAX_ENDS) {
            let fewest = fewest_turns(here, axis, terminal);
            let turns: f64 = (0..fewest).map(|turn| turn_cost(turned + turn, bend_penalty)).sum();
            cheap[index] = (
                terminal.cost + (terminal.point.x - here.x).abs() + (terminal.point.y - here.y).abs() + turns,
                index,
                fewest,
            );
        }
        let cheap = &mut cheap[..ends.len().min(MAX_ENDS)];
        cheap.sort_unstable_by(|a, b| a.0.total_cmp(&b.0));
        let mut best = f64::INFINITY;
        for &(cost, index, fewest) in cheap.iter() {
            if cost >= best {
                break;
            }
            let terminal = &ends[index];
            // With no turn or one to spare, the route has one way in: straight
            // on, or straight to the corner and then straight in. Leaving it,
            // the route turns twice more; keeping to it, it crosses what lies
            // across it.
            let detour = match (fewest, end_indices[index]) {
                (0 | 1, Some((end_x, end_y))) => {
                    let (blocked, crossed) = if fewest == 0 {
                        run_counts((xi, yi), (end_x, end_y))
                    } else {
                        let corner = if axis == 1 { (xi, end_y) } else { (end_x, yi) };
                        let corner_point = point(xs[corner.0], ys[corner.1]);
                        // The last leg has to run towards the port.
                        let backwards = (terminal.point.x - corner_point.x) * (terminal.port.x - terminal.point.x)
                            + (terminal.point.y - corner_point.y) * (terminal.port.y - terminal.point.y)
                            < 0.0;
                        let (first_blocked, first_crossed) = run_counts((xi, yi), corner);
                        let (last_blocked, last_crossed) = run_counts(corner, (end_x, end_y));
                        (
                            first_blocked + last_blocked + u32::from(backwards),
                            first_crossed + last_crossed,
                        )
                    };
                    let two_more: f64 = (fewest..fewest + 2)
                        .map(|turn| turn_cost(turned + turn, bend_penalty))
                        .sum();
                    if blocked > 0 {
                        two_more
                    } else {
                        two_more.min(f64::from(crossed) * CROSSING_PENALTY)
                    }
                }
                _ => 0.0,
            };
            best = best.min(cost + detour);
        }
        best
    };

    let mut distance = vec![f64::INFINITY; cells * 2 * levels];
    let mut previous: Vec<Option<usize>> = vec![None; cells * 2 * levels];
    let mut queue = BinaryHeap::new();
    for terminal in starts {
        let Some(cell) = cell_of(&terminal.point) else { continue };
        let state = state_of(cell, terminal.axis, 0);
        if blocked_cell[cell] || terminal.cost >= distance[state] {
            continue;
        }
        distance[state] = terminal.cost;
        queue.push(Reverse(Entry(terminal.cost + estimate(state), state, terminal.cost)));
    }
    let mut reached = None;
    let mut visited = 0usize;
    while let Some(Reverse(Entry(_, state, reached_at))) = queue.pop() {
        visited += 1;
        let cell = cell_of_state(state);
        let cost = distance[state];
        // A cheaper way here was found after this entry was queued.
        if reached_at > cost {
            continue;
        }
        if end_cells.contains_key(&cell) {
            reached = Some(state);
            break;
        }
        let here = point_of(cell);
        let (xi, yi) = (cell % width, cell / width);
        let neighbours = [
            (xi.checked_sub(1), Some(yi)),
            (Some(xi + 1), Some(yi)),
            (Some(xi), yi.checked_sub(1)),
            (Some(xi), Some(yi + 1)),
        ];
        for (nxi, nyi) in neighbours {
            let (Some(nxi), Some(nyi)) = (nxi, nyi) else { continue };
            if nxi >= width || nyi >= ys.len() {
                continue;
            }
            let next_cell = key(nxi, nyi);
            let end = end_cells.get(&next_cell);
            if end.is_none() && blocked_cell[next_cell] {
                continue;
            }
            let next = point_of(next_cell);
            let inside = |area: &Bounds| {
                area.x <= next.x && next.x <= area.right() && area.y <= next.y && next.y <= area.bottom()
            };
            if end.is_none() && corridor.is_some_and(|corridor| !corridor.iter().any(inside)) {
                continue;
            }
            if end.is_some_and(|(_, terminal)| arrives_backwards(here, next, terminal)) {
                continue;
            }
            let vertical = usize::from(nxi == xi);
            let edge = key(xi.min(nxi), yi.min(nyi));
            let blocked = if vertical == 1 {
                &blocked_vertical
            } else {
                &blocked_horizontal
            };
            if blocked[edge] {
                continue;
            }
            // The path continues straight out of the start stub and straight
            // into the end stub, so turning onto or off them counts as a bend.
            let turned = turned_of(state);
            let turns = usize::from(axis_of(state) != vertical);
            let bend = if turns == 1 {
                turn_cost(turned, bend_penalty)
            } else {
                0.0
            } + end.map_or(0.0, |(_, terminal)| {
                terminal.cost
                    + if vertical != terminal.axis {
                        turn_cost(turned + turns, bend_penalty)
                    } else {
                        0.0
                    }
            });
            let next_state = state_of(next_cell, vertical, turned + turns);
            let length = (next.x - here.x).abs() + (next.y - here.y).abs();
            let outline = if vertical == 1 {
                &outline_vertical
            } else {
                &outline_horizontal
            };
            let near = if vertical == 1 {
                near_vertical[edge]
            } else {
                near_horizontal[edge]
            };
            let crossing = if vertical == 1 {
                crossing_vertical[edge]
            } else {
                crossing_horizontal[edge]
            };
            // Turning here, or onto the end stub there, beside another
            // connector, or running past another's turn.
            let beside = (turns == 1 && turn_beside[cell])
                || end.is_some_and(|(_, terminal)| vertical != terminal.axis && turn_beside[next_cell]);
            let past = if vertical == 1 {
                past_turn_vertical[edge]
            } else {
                past_turn_horizontal[edge]
            };
            // Running alongside another connector is paid once, on the step
            // that starts it (or brings the route closer), not again on each
            // step that keeps it going straight on.
            let near_before = if turns == 0 {
                previous[state].map_or(0.0, |before| {
                    let before = cell_of_state(before);
                    let edge = key(xi.min(before % width), yi.min(before / width));
                    if vertical == 1 {
                        near_vertical[edge]
                    } else {
                        near_horizontal[edge]
                    }
                })
            } else {
                0.0
            };
            let entered = if vertical == 1 {
                pushable_vertical[edge]
            } else {
                pushable_horizontal[edge]
            };
            let pushes = if pushable_cell[cell] || entered == NO_BOX {
                0.0
            } else {
                let Some(cost) = (pushable[entered as usize].1)() else {
                    continue;
                };
                cost
            };
            let next_cost = cost
                + pushes
                + length * if outline[edge] { 1.01 } else { 1.0 }
                + (near - near_before).max(0.0)
                + bend
                + if crossing { CROSSING_PENALTY } else { 0.0 }
                + if beside || past { NEAR_MISS_PENALTY } else { 0.0 };
            if next_cost >= distance[next_state] {
                continue;
            }
            distance[next_state] = next_cost;
            previous[next_state] = Some(state);
            queue.push(Reverse(Entry(next_cost + estimate(next_state), next_state, next_cost)));
        }
    }
    phase.record("visited", visited);
    drop(phase);
    let reached = reached?;
    let path: Vec<Point> = std::iter::successors(Some(reached), |state| previous[*state])
        .map(|state| point_of(cell_of_state(state)))
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    let first = path.first().copied();
    Some(Found {
        start: starts.iter().position(|terminal| Some(terminal.point) == first),
        end: end_cells.get(&cell_of_state(reached)).map(|(index, _)| *index),
        path: without_redundant_points(&path),
    })
}

/// The grid lines along one axis, and the obstacles' `spans` along it with
/// edges lying within the merge distance of each other pushed outwards onto
/// one shared line. Lines run on every obstacle edge, on `terminals` and on
/// `occupied` connectors (both kept where they are, never crossed by a push),
/// through the middle of each gap wide enough between edges and terminals,
/// and one and two lanes beside the connectors at `lanes_beside`, where
/// clear of the other lines.
fn axis_lines(
    spans: &[(f64, f64)],
    terminals: &[f64],
    occupied: &[f64],
    lanes_beside: &[f64],
    midlines: bool,
) -> (Vec<f64>, Vec<(f64, f64)>) {
    let fixed = sorted_distinct(terminals.iter().chain(occupied).copied());
    // A push from `from` to `to` that would carry a fixed line inside the
    // obstacle.
    let carries_fixed = |from: f64, to: f64| {
        fixed.iter().any(|line| {
            if to < from {
                to < *line && *line <= from
            } else {
                from <= *line && *line < to
            }
        })
    };
    // Edges as (position, whether it is the high edge, obstacle), in order.
    let mut edges: Vec<(f64, bool, usize)> = spans
        .iter()
        .enumerate()
        .flat_map(|(index, (low, high))| [(*low, false, index), (*high, true, index)])
        .collect();
    edges.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut snapped: Vec<(f64, f64)> = spans.to_vec();
    let mut start = 0;
    while start < edges.len() {
        let mut end = start + 1;
        while end < edges.len() && edges[end].0 - edges[start].0 <= LINE_MERGE {
            end += 1;
        }
        let cluster = &edges[start..end];
        let lowest_low = cluster
            .iter()
            .filter(|edge| !edge.1)
            .map(|edge| edge.0)
            .fold(f64::INFINITY, f64::min);
        let highest_high = cluster
            .iter()
            .filter(|edge| edge.1)
            .map(|edge| edge.0)
            .fold(f64::NEG_INFINITY, f64::max);
        // One line serves all when every high edge lies below every low one:
        // a fixed line between them if there is one, else the middle.
        let shared = (highest_high <= lowest_low).then(|| {
            fixed
                .iter()
                .copied()
                .find(|line| (highest_high..=lowest_low).contains(line))
                .unwrap_or_else(|| round((highest_high + lowest_low) / 2.0).clamp(highest_high, lowest_low))
        });
        for &(position, high, index) in cluster {
            let target = match (shared, high) {
                (Some(line), _) if lowest_low.is_finite() && highest_high.is_finite() => line,
                (_, false) => lowest_low,
                (_, true) => highest_high,
            };
            if target != position && !carries_fixed(position, target) {
                if high {
                    snapped[index].1 = target;
                } else {
                    snapped[index].0 = target;
                }
            }
        }
        start = end;
    }
    let base = sorted_distinct(
        snapped
            .iter()
            .flat_map(|(low, high)| [*low, *high])
            .chain(terminals.iter().copied()),
    );
    // Midlines of the gaps between edges wide enough to hold one clear of
    // both, rounded to whole units.
    let channels = base
        .windows(2)
        .filter(|pair| midlines && pair[1] - pair[0] > 2.0 * LINE_MERGE)
        .map(|pair| round((pair[0] + pair[1]) / 2.0));
    let mut lines = sorted_distinct(base.iter().copied().chain(channels).chain(occupied.iter().copied()));
    // Lanes one and two lanes beside connectors already drawn let new routes
    // run alongside them when every other line through a channel is taken.
    let lanes = sorted_distinct(lanes_beside.iter().flat_map(|at| {
        [
            at - 2.0 * LANE_SPACING,
            at - LANE_SPACING,
            at + LANE_SPACING,
            at + 2.0 * LANE_SPACING,
        ]
    }));
    for lane in lanes {
        let at = lines.partition_point(|line| *line < lane);
        let clear = [at.checked_sub(1), Some(at)]
            .into_iter()
            .flatten()
            .filter_map(|index| lines.get(index))
            .all(|line| (line - lane).abs() > LINE_MERGE);
        if clear {
            lines.insert(at, lane);
        }
    }
    (lines, snapped)
}

/// The fewest turns a route arriving at `here` along `axis` can still make
/// to reach `terminal`. Each turn swaps the axis, so their number is even
/// when the axes match and odd when they differ; none only when the
/// terminal lies straight ahead, reached from the side away from its port.
fn fewest_turns(here: Point, axis: usize, terminal: &Terminal) -> usize {
    if axis != terminal.axis {
        return 1;
    }
    let on_line = if axis == 1 {
        here.x == terminal.point.x
    } else {
        here.y == terminal.point.y
    };
    // Moving towards the port; coming from its side would double back.
    let towards_port = (terminal.point.x - here.x) * (terminal.port.x - terminal.point.x)
        + (terminal.point.y - here.y) * (terminal.port.y - terminal.point.y)
        >= 0.0;
    if on_line && towards_port { 0 } else { 2 }
}

/// A cost for something too short or too close, so a pixel either side of
/// a threshold never swings it: `full` up to `start`, nothing from `reach`
/// on, and in between a reversed smootherstep, flat where it leaves the
/// full cost and where it reaches nothing.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Fade {
    pub start: f64,
    pub reach: f64,
    pub full: f64,
}

impl Fade {
    pub(crate) fn cost(self, length: f64) -> f64 {
        let t = ((length - self.start) / (self.reach - self.start)).clamp(0.0, 1.0);
        self.full * (1.0 - t * t * t * (t * (6.0 * t - 15.0) + 10.0))
    }
}

/// What running alongside another connector `distance` away costs. Running
/// on it is an overlap, ruled out elsewhere.
fn close_run_cost(distance: f64) -> f64 {
    if distance <= 0.0 { 0.0 } else { CLOSE_RUN.cost(distance) }
}

/// What a drawn route costs on its own, priced as the search prices it:
/// its length and its bends.
pub(crate) fn route_cost(route: &[Point]) -> f64 {
    let length: f64 = segments(route)
        .iter()
        .map(|(a, b)| (a.x - b.x).abs() + (a.y - b.y).abs())
        .sum();
    length
        + (0..turns(route))
            .map(|turned| turn_cost(turned, BEND_PENALTY))
            .sum::<f64>()
}

/// What two drawn routes cost each other, priced as the search prices a
/// route against those already drawn: each crossing, each stretch running
/// alongside within two lanes, and each turn just beside the other route.
/// Crossing a route that also runs alongside costs more, up to 2.6 times as
/// much the nearer they run, as the two are then hard to tell apart.
pub(crate) fn routes_cost(first: &[Point], second: &[Point]) -> f64 {
    let (first_segments, second_segments) = (segments(first), segments(second));
    let mut cost = 0.0;
    let (mut crossed, mut closest) = (0usize, 0.0f64);
    for (a, b) in &first_segments {
        for (c, d) in &second_segments {
            if chords_cross(*a, *b, *c, *d) {
                crossed += 1;
            }
            let distance = if a.y == b.y && c.y == d.y {
                (a.y - c.y).abs()
            } else if a.x == b.x && c.x == d.x {
                (a.x - c.x).abs()
            } else {
                continue;
            };
            let overlap = overlap((*a, *b), (*c, *d));
            if overlap > 0.0 {
                let shared = 1.0 - BRUSHING.cost(overlap);
                cost += close_run_cost(distance) * shared;
                if distance > 0.0 {
                    closest = closest.max(TANGLED.cost(distance) * shared);
                }
            }
        }
    }
    cost + crossed as f64 * CROSSING_PENALTY * (1.0 + closest)
        + (crate::inspect::turns_beside(first, &second_segments)
            + crate::inspect::turns_beside(second, &first_segments)) as f64
            * NEAR_MISS_PENALTY
}

/// How far two parallel segments run alongside each other: the length of
/// the stretch they share along their direction, zero or less when they
/// share none.
pub(crate) fn overlap((a, b): (Point, Point), (c, d): (Point, Point)) -> f64 {
    let span = |p: f64, q: f64| (p.min(q), p.max(q));
    let ((low, high), (other_low, other_high)) = if a.x == b.x {
        (span(a.y, b.y), span(c.y, d.y))
    } else {
        (span(a.x, b.x), span(c.x, d.x))
    };
    high.min(other_high) - low.max(other_low)
}

/// The cost of a route's next turn after it has turned `turned` times.
fn turn_cost(turned: usize, penalty: f64) -> f64 {
    if turned >= EASY_TURNS {
        penalty * LATE_TURN_FACTOR
    } else {
        penalty
    }
}

/// A route between two ports: straight when the ports line up and nothing
/// is in the way, otherwise the cheapest detour leaving and entering the
/// node sides at right angles.
#[tracing::instrument(skip_all)]
pub fn route_connector(
    source: &Bounds,
    target: &Bounds,
    raw_obstacles: &[Bounds],
    raw_pushable: &[Pushable<'_>],
    ports: Ports,
    occupied: &[Vec<Point>],
    rails: &[(Point, Point)],
) -> Vec<Point> {
    let obstacles: Vec<Bounds> = raw_obstacles.iter().map(|bounds| bounds.inflate(CLEARANCE)).collect();
    let pushable: Vec<Pushable<'_>> = raw_pushable
        .iter()
        .map(|(bounds, cost)| (bounds.inflate(CLEARANCE), *cost))
        .collect();
    let direct = vec![ports.start, ports.end];
    // A straight line has to meet both sides at a right angle: a vertical
    // one leaves and enters through tops and bottoms, a horizontal one
    // through left and right sides.
    let meets =
        |port: Point, bounds: &Bounds| side_of(port, bounds).is_horizontal_edge() == (ports.start.x == ports.end.x);
    if (ports.start.x == ports.end.x || ports.start.y == ports.end.y)
        && meets(ports.start, source)
        && meets(ports.end, target)
        && route_is_clear(&direct, &obstacles)
        && route_is_clear(&direct, &pushable.iter().map(|(bounds, _)| *bounds).collect::<Vec<_>>())
        && occupied
            .iter()
            .all(|other| !routes_overlap(&direct, other) && crossings(&direct, other) == 0)
    {
        return direct;
    }
    let all_obstacles: Vec<Bounds> = obstacles
        .iter()
        .copied()
        .chain([source.inflate(CLEARANCE), target.inflate(CLEARANCE)])
        .collect();
    match search(
        &[terminal_at(ports.start, source, 0.0)],
        &end_terminals(
            ports.end,
            target,
            &[obstacles.as_slice(), &[source.inflate(CLEARANCE)]].concat(),
        ),
        &all_obstacles,
        &pushable,
        occupied,
        rails,
        BEND_PENALTY,
    ) {
        Some(found) => without_redundant_points(
            &std::iter::once(ports.start)
                .chain(found.path)
                .chain(std::iter::once(ports.end))
                .collect::<Vec<_>>(),
        ),
        None => direct,
    }
}

/// Picks the pair of node sides whose route needs the fewest bends, trying
/// every side of both ends from its midpoint. Each connector already on a
/// side makes that side cost more, so bends are not saved by bunching
/// connectors together. The sides facing along the dominant axis win ties.
#[tracing::instrument(skip_all)]
pub fn choose_sides(
    source: &Bounds,
    target: &Bounds,
    raw_obstacles: &[Bounds],
    raw_pushable: &[Pushable<'_>],
    occupied: &[Vec<Point>],
    rails: &[(Point, Point)],
    load: &dyn Fn(&Bounds, Side) -> f64,
) -> Sides {
    let preferred = connector_ports(source, target);
    let fallback = Sides {
        start: side_of(preferred.start, source),
        end: side_of(preferred.end, target),
    };
    let terminals = |bounds: &Bounds, keep: Side| -> Vec<Terminal> {
        Side::ALL
            .iter()
            .map(|side| {
                let cost = if *side == keep { 0.0 } else { SIDE_CHANGE_COST } + CROWDING_COST * load(bounds, *side);
                terminal_at(port_on_side(bounds, *side), bounds, cost)
            })
            .collect()
    };
    let obstacles: Vec<Bounds> = raw_obstacles
        .iter()
        .map(|bounds| bounds.inflate(CLEARANCE))
        .chain([source.inflate(CLEARANCE), target.inflate(CLEARANCE)])
        .collect();
    let pushable: Vec<Pushable<'_>> = raw_pushable
        .iter()
        .map(|(bounds, cost)| (bounds.inflate(CLEARANCE), *cost))
        .collect();
    let found = search_along_rough_route(
        &terminals(source, fallback.start),
        &terminals(target, fallback.end),
        &obstacles,
        &pushable,
        occupied,
        rails,
        SIDE_BEND_PENALTY,
    );
    match found.and_then(|found| Some((Side::ALL.get(found.start?)?, Side::ALL.get(found.end?)?))) {
        Some((start, end)) => Sides {
            start: *start,
            end: *end,
        },
        None => fallback,
    }
}

/// A rerouted connector's ends and the node sides they attach to.
#[derive(Clone, Debug)]
pub struct PortRequest {
    pub id: String,
    pub source_id: String,
    pub target_id: String,
    pub source: Bounds,
    pub target: Bounds,
    /// Whether each end's node is round: its ports end on the circle, so
    /// one near a corner of its box is no nearer the edge of the node.
    pub source_round: bool,
    pub target_round: bool,
    pub sides: Sides,
}

#[derive(Clone, Copy)]
enum End {
    Start,
    End,
}

fn get(ports: &Ports, end: End) -> Point {
    match end {
        End::Start => ports.start,
        End::End => ports.end,
    }
}

fn set(ports: Ports, end: End, value: Point) -> Ports {
    match end {
        End::Start => Ports { start: value, ..ports },
        End::End => Ports { end: value, ..ports },
    }
}

/// Spreads the endpoints that share a node side evenly along that side,
/// ordered by the position of the node at the other end, so parallel
/// connectors do not collapse onto one line; then straightens connectors
/// between facing sides.
/// Spreads the ports of the requests evenly along each side they share,
/// counting the ports already on that side (`fixed`, keyed like the sides
/// and given along it) as taking the even slots nearest them.
#[tracing::instrument(skip_all, fields(requests = requests.len()))]
pub fn spread_ports(requests: &[PortRequest], fixed: &HashMap<String, Vec<f64>>) -> HashMap<String, Ports> {
    let midpoints: HashMap<String, Ports> = requests
        .iter()
        .map(|request| {
            let start = port_on_side(&request.source, request.sides.start);
            let end = port_on_side(&request.target, request.sides.end);
            (request.id.clone(), Ports { start, end })
        })
        .collect();
    let spread = sides_in_use(requests, &midpoints)
        .iter()
        .fold(midpoints.clone(), |ports, (key, users)| {
            spread_side(ports, key, users, fixed.get(key).map_or(&[], Vec::as_slice))
        });
    align_facing_ports(requests, untangle_pairs(requests, spread))
}

/// Connectors joining the same two node sides tie when ordered by the node
/// at their other end, so both sides get the same order: right for facing
/// sides, but around a corner or in a U shape the routes cross. Wherever
/// two such connectors would cross, their ports on one node trade places so
/// the routes nest instead.
fn untangle_pairs(requests: &[PortRequest], ports: HashMap<String, Ports>) -> HashMap<String, Ports> {
    requests
        .iter()
        .enumerate()
        .flat_map(|(index, first)| requests[index + 1..].iter().map(move |second| (first, second)))
        .fold(ports, |mut ports, (first, second)| {
            let reversed = if first.source_id == second.source_id
                && first.target_id == second.target_id
                && first.sides.start == second.sides.start
                && first.sides.end == second.sides.end
            {
                false
            } else if first.source_id == second.target_id
                && first.target_id == second.source_id
                && first.sides.start == second.sides.end
                && first.sides.end == second.sides.start
            {
                true
            } else {
                return ports;
            };
            let (Some(one), Some(other)) = (ports.get(&first.id).copied(), ports.get(&second.id).copied()) else {
                return ports;
            };
            // The other connector's ports on the first one's source and target.
            let (other_start, other_end) = if reversed {
                (other.end, other.start)
            } else {
                (other.start, other.end)
            };
            if !would_cross(first.sides, (one.start, one.end), (other_start, other_end)) {
                return ports;
            }
            let traded = if reversed {
                Ports {
                    start: one.end,
                    ..other
                }
            } else {
                Ports { end: one.end, ..other }
            };
            ports.insert(first.id.clone(), Ports { end: other_end, ..one });
            ports.insert(second.id.clone(), traded);
            ports
        })
}

/// Whether routes between two pairs of ports on the same node sides would
/// cross. Between sides facing the same way the routes run out and back in
/// a U, crossing when their spans along the sides interleave; otherwise
/// they cross when the straight chords between their ports do.
fn would_cross(sides: Sides, (a, b): (Point, Point), (c, d): (Point, Point)) -> bool {
    if sides.start != sides.end {
        return chords_cross(a, b, c, d);
    }
    let along = |p: Point| if sides.start.is_horizontal_edge() { p.x } else { p.y };
    let span = |p: Point, q: Point| (along(p).min(along(q)), along(p).max(along(q)));
    let ((low, high), (other_low, other_high)) = (span(a, b), span(c, d));
    (low < other_low && other_low < high && high < other_high)
        || (other_low < low && low < other_high && other_high < high)
}

/// How many times two routes pass through each other.
pub(crate) fn crossings(first: &[Point], second: &[Point]) -> usize {
    let others = segments(second);
    segments(first)
        .iter()
        .map(|(a, b)| others.iter().filter(|(c, d)| chords_cross(*a, *b, *c, *d)).count())
        .sum()
}

/// How many times a route changes direction.
pub(crate) fn turns(route: &[Point]) -> usize {
    route
        .windows(3)
        .filter(|w| {
            let (a, b, c) = (w[0], w[1], w[2]);
            (b.x - a.x) * (c.y - b.y) != (b.y - a.y) * (c.x - b.x)
        })
        .count()
}

/// Whether two straight segments pass through each other.
pub(crate) fn chords_cross(a: Point, b: Point, c: Point, d: Point) -> bool {
    let turn = |p: Point, q: Point, r: Point| ((q.x - p.x) * (r.y - p.y) - (q.y - p.y) * (r.x - p.x)).signum();
    turn(a, b, c) * turn(a, b, d) < 0.0 && turn(c, d, a) * turn(c, d, b) < 0.0
}

/// An endpoint attached to a node side, with the node at the other end.
struct SideUser {
    id: String,
    end: End,
    bounds: Bounds,
    other: Bounds,
}

/// Endpoints grouped by the node side they attach to, in first-use order.
fn sides_in_use(requests: &[PortRequest], ports: &HashMap<String, Ports>) -> Vec<(String, Vec<SideUser>)> {
    requests
        .iter()
        .filter_map(|request| Some((request, *ports.get(&request.id)?)))
        .flat_map(|(request, port)| {
            [
                (End::Start, request.source, request.target),
                (End::End, request.target, request.source),
            ]
            .map(|(end, bounds, other)| {
                let key = format!("{}:{}", bounds.format(), side_of(get(&port, end), &bounds).name());
                (
                    key,
                    SideUser {
                        id: request.id.clone(),
                        end,
                        bounds,
                        other,
                    },
                )
            })
        })
        .fold(Vec::new(), |mut sides: Vec<(String, Vec<SideUser>)>, (key, user)| {
            match sides.iter_mut().find(|(existing, _)| *existing == key) {
                Some((_, users)) => users.push(user),
                None => sides.push((key, vec![user])),
            }
            sides
        })
}

/// Places the endpoints on one side at even fractions along it, ordered by
/// the position of the node at their other end. Ports already there take
/// the fractions nearest them; the endpoints get the rest.
fn spread_side(ports: HashMap<String, Ports>, key: &str, users: &[SideUser], fixed: &[f64]) -> HashMap<String, Ports> {
    let along_x = key.ends_with(":top") || key.ends_with(":bottom");
    let centre = |bounds: &Bounds| {
        if along_x {
            bounds.x + bounds.width / 2.0
        } else {
            bounds.y + bounds.height / 2.0
        }
    };
    let mut ordered: Vec<(usize, &SideUser)> = users.iter().enumerate().collect();
    ordered.sort_by(|(first_index, first), (second_index, second)| {
        let difference = centre(&first.other) - centre(&second.other);
        if difference != 0.0 && !difference.is_nan() {
            difference.partial_cmp(&0.0).unwrap_or(Ordering::Equal)
        } else {
            first_index.cmp(second_index)
        }
    });
    let Some(first) = ordered.first().map(|(_, user)| user.bounds) else {
        return ports;
    };
    let (low, length) = if along_x {
        (first.x, first.width)
    } else {
        (first.y, first.height)
    };
    let count = (ordered.len() + fixed.len()) as f64;
    let mut slots: Vec<f64> = (1..=ordered.len() + fixed.len())
        .map(|index| index as f64 / (count + 1.0))
        .collect();
    for taken in fixed {
        let nearest = slots
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                ((low + length * **a) - taken)
                    .abs()
                    .total_cmp(&((low + length * **b) - taken).abs())
            })
            .map(|(index, _)| index);
        if let Some(index) = nearest {
            slots.remove(index);
        }
    }
    ordered
        .iter()
        .zip(slots)
        .fold(ports, |mut ports, ((_, user), fraction)| {
            let Some(port) = ports.get(&user.id).copied() else {
                return ports;
            };
            let current = get(&port, user.end);
            let moved = if along_x {
                point(round(user.bounds.x + user.bounds.width * fraction), current.y)
            } else {
                point(current.x, round(user.bounds.y + user.bounds.height * fraction))
            };
            ports.insert(user.id.clone(), set(port, user.end, moved));
            ports
        })
}

/// Straightens connectors between facing sides: when one end's coordinate
/// also fits on the other end's side, in the middle half of a box's side or
/// away from a circle's corners, and at least a lane away from other ports
/// there, both ends share it.
fn align_facing_ports(requests: &[PortRequest], ports: HashMap<String, Ports>) -> HashMap<String, Ports> {
    requests.iter().fold(ports, |mut ports, request| {
        let Some(port) = ports.get(&request.id).copied() else {
            return ports;
        };
        let start_side = side_of(port.start, &request.source);
        let end_side = side_of(port.end, &request.target);
        let facing = matches!(
            (start_side, end_side),
            (Side::Right, Side::Left)
                | (Side::Left, Side::Right)
                | (Side::Bottom, Side::Top)
                | (Side::Top, Side::Bottom)
        );
        if !facing {
            return ports;
        }
        let along_y = matches!(start_side, Side::Left | Side::Right);
        let coordinate = |p: Point| if along_y { p.y } else { p.x };
        if coordinate(port.start) == coordinate(port.end) {
            return ports;
        }
        // Within the middle half of a box's side, as a port moved near a
        // corner looks pushed aside.
        let fits = |bounds: &Bounds, round: bool, value: f64| {
            let (low, length) = if along_y {
                (bounds.y, bounds.height)
            } else {
                (bounds.x, bounds.width)
            };
            let margin = if round { 8.0 } else { (length / 4.0).max(8.0) };
            value >= low + margin && value <= low + length - margin
        };
        let taken = |ports: &HashMap<String, Ports>, owner_id: &str, side: Side, value: f64| {
            requests.iter().filter(|other| other.id != request.id).any(|other| {
                let Some(other_port) = ports.get(&other.id) else {
                    return false;
                };
                [
                    (End::Start, &other.source_id, &other.source),
                    (End::End, &other.target_id, &other.target),
                ]
                .iter()
                .any(|(end, id, owner)| {
                    let p = get(other_port, *end);
                    *id == owner_id
                        && side_of(p, owner) == side
                        && (if matches!(side, Side::Left | Side::Right) {
                            p.y
                        } else {
                            p.x
                        } - value)
                            .abs()
                            < LANE_SPACING
                })
            })
        };
        let moved = |p: Point, value: f64| if along_y { point(p.x, value) } else { point(value, p.y) };
        let (start_value, end_value) = (coordinate(port.start), coordinate(port.end));
        if fits(&request.target, request.target_round, start_value)
            && !taken(&ports, &request.target_id, end_side, start_value)
        {
            ports.insert(
                request.id.clone(),
                Ports {
                    end: moved(port.end, start_value),
                    ..port
                },
            );
        } else if fits(&request.source, request.source_round, end_value)
            && !taken(&ports, &request.source_id, start_side, end_value)
        {
            ports.insert(
                request.id.clone(),
                Ports {
                    start: moved(port.start, end_value),
                    ..port
                },
            );
        }
        ports
    })
}

/// Routes meet the bounding box of a node; on a circular node, slide each end
/// along its first or last segment until it reaches the circle, keeping the
/// route orthogonal. Coordinates are rounded to whole units.
fn on_circle(port: Point, next: Point, node: &DiagramNode) -> Point {
    if !node.is_circle() {
        return port;
    }
    let centre = node.bounds.centre();
    let radius = node.bounds.width / 2.0;
    if port.y == next.y && port.x != next.x {
        let offset = port.y - centre.y;
        if offset.abs() >= radius {
            return port;
        }
        let reach = (radius * radius - offset * offset).sqrt();
        return point(round(centre.x + if port.x < centre.x { -reach } else { reach }), port.y);
    }
    if port.x == next.x && port.y != next.y {
        let offset = port.x - centre.x;
        if offset.abs() >= radius {
            return port;
        }
        let reach = (radius * radius - offset * offset).sqrt();
        return point(port.x, round(centre.y + if port.y < centre.y { -reach } else { reach }));
    }
    port
}

pub fn end_on_shapes(route: Vec<Point>, source: &DiagramNode, target: &DiagramNode) -> Vec<Point> {
    if route.len() < 2 {
        return route;
    }
    let last = route.len() - 1;
    route
        .iter()
        .enumerate()
        .map(|(index, current)| match index {
            0 => on_circle(*current, route[1], source),
            _ if index == last => on_circle(*current, route[last - 1], target),
            _ => *current,
        })
        .collect()
}

pub struct Placement {
    /// The `text-anchor` that keeps the label's lines against the side
    /// facing its connector.
    pub anchor: &'static str,
    pub bounds: Bounds,
}

fn bounds_at(x: f64, y: f64, width: f64, height: f64) -> Bounds {
    Bounds { x, y, width, height }
}

/// Finds a spot for a connector label beside one of the connector's
/// segments, trying the longest segment first and fanning out from its
/// midpoint. The label keeps the connector clearance from every route and
/// stays at least 4px away from other labels and nodes.
/// Where two crossing segments meet.
fn crossing_point(a: Point, b: Point, c: Point, d: Point) -> Point {
    let denominator = (a.x - b.x) * (c.y - d.y) - (a.y - b.y) * (c.x - d.x);
    if denominator == 0.0 {
        return a;
    }
    let t = ((a.x - c.x) * (c.y - d.y) - (a.y - c.y) * (c.x - d.x)) / denominator;
    point(a.x + t * (b.x - a.x), a.y + t * (b.y - a.y))
}

/// Where a connector is crossed by others, turns and ends: a label reads
/// best well away from these, sorted so two sets compare equal when the
/// same.
pub fn label_landmarks(route: &[Point], routes: &[Vec<Point>]) -> Vec<Point> {
    let own = segments(route);
    let others: Vec<(Point, Point)> = routes
        .iter()
        .flat_map(|other| segments(other))
        .filter(|segment| !own.contains(segment))
        .collect();
    let mut landmarks: Vec<Point> = own
        .iter()
        .flat_map(|(a, b)| {
            others
                .iter()
                .filter(move |(c, d)| chords_cross(*a, *b, *c, *d))
                .map(move |(c, d)| crossing_point(*a, *b, *c, *d))
        })
        .chain(route.iter().copied())
        .collect();
    landmarks.sort_by(|p, q| p.x.total_cmp(&q.x).then(p.y.total_cmp(&q.y)));
    landmarks
}

/// How far a label keeps from other connectors when it can: twice its gap
/// to its own line, so it reads as belonging to its own.
pub const LABEL_CLEAR_OF_OTHERS: f64 = 2.0 * (CLEARANCE + 1.0);
/// Other connectors nearer a label than this still make it a little less
/// clear: each pixel nearer weighs like this many pixels nearer one of its
/// connector's crossings, bends or ends.
const LABEL_COMFORT: f64 = 30.0;
const CROWDED_LABEL_WEIGHT: f64 = 3.0;

/// Whether `other` runs parallel to `beside`, the segment a label sits
/// beside, on its far side: the label's own line then lies between them,
/// so the other connector leaves no doubt which one the label belongs to.
pub fn runs_across((from, to): (Point, Point), label: &Bounds, (c, d): (Point, Point)) -> bool {
    let centre = point(label.x + label.width / 2.0, label.y + label.height / 2.0);
    if from.y == to.y && c.y == d.y {
        (centre.y - from.y) * (c.y - from.y) < 0.0
    } else if from.x == to.x && c.x == d.x {
        (centre.x - from.x) * (c.x - from.x) < 0.0
    } else {
        false
    }
}

/// `containers` are the boxes around the connector's ends: the label may
/// sit inside or outside each, but not across its border.
pub fn place_label(
    width: f64,
    height: f64,
    route: &[Point],
    routes: &[Vec<Point>],
    blocked: &[Bounds],
    containers: &[Bounds],
) -> Option<Placement> {
    find_label_spot(width, height, route, routes, blocked, containers, false)
}

/// A spot for a label at least a label clearance from every connector but
/// its own, if there is one.
pub fn place_label_clear(
    width: f64,
    height: f64,
    route: &[Point],
    routes: &[Vec<Point>],
    blocked: &[Bounds],
    containers: &[Bounds],
) -> Option<Placement> {
    find_label_spot(width, height, route, routes, blocked, containers, true)
}

#[tracing::instrument(skip_all, fields(routes = routes.len(), blocked = blocked.len()))]
fn find_label_spot(
    width: f64,
    height: f64,
    route: &[Point],
    routes: &[Vec<Point>],
    blocked: &[Bounds],
    containers: &[Bounds],
    clear_only: bool,
) -> Option<Placement> {
    let gap = CLEARANCE + 1.0;
    let mut ordered: Vec<(usize, (Point, Point))> = segments(route).into_iter().enumerate().collect();
    ordered.sort_by(|(first_index, first), (second_index, second)| {
        let difference = segment_length(second) - segment_length(first);
        if difference != 0.0 {
            difference.partial_cmp(&0.0).unwrap_or(Ordering::Equal)
        } else {
            first_index.cmp(second_index)
        }
    });
    let route_segments: Vec<(Point, Point)> = routes.iter().flat_map(|other| segments(other)).collect();
    let fits = |bounds: &Bounds| {
        let inflated = bounds.inflate(CLEARANCE);
        route_segments
            .iter()
            .all(|(from, to)| !segment_intersects_interior(*from, *to, &inflated))
            && blocked
                .iter()
                .all(|other| intersection(bounds, &other.inflate(4.0)).is_none())
            && containers.iter().all(|container| {
                let inside = bounds.x >= container.x + 4.0
                    && bounds.y >= container.y + 4.0
                    && bounds.right() <= container.right() - 4.0
                    && bounds.bottom() <= container.bottom() - 4.0;
                inside || intersection(bounds, &container.inflate(4.0)).is_none()
            })
    };
    // Other connectors passing close make it unclear which one a label
    // belongs to: a spot clear of them is preferred when there is one, and
    // among those, one further from them.
    let own = segments(route);
    let others: Vec<Vec<(Point, Point)>> = routes
        .iter()
        .map(|other| {
            segments(other)
                .into_iter()
                .filter(|segment| !own.contains(segment))
                .collect::<Vec<_>>()
        })
        .filter(|other| !other.is_empty())
        .collect();
    // How near each other connector comes to a label beside `beside`.
    let distances = |bounds: &Bounds, beside: (Point, Point)| {
        others
            .iter()
            .map(|other| {
                other
                    .iter()
                    .filter(|segment| !runs_across(beside, bounds, **segment))
                    .map(|(c, d)| distance_to_route(bounds, &[*c, *d]))
                    .fold(f64::INFINITY, f64::min)
            })
            .collect::<Vec<f64>>()
    };
    let clear_of_others = |bounds: &Bounds, beside: (Point, Point)| {
        distances(bounds, beside)
            .iter()
            .all(|distance| *distance >= LABEL_CLEAR_OF_OTHERS)
    };
    let crowding = |bounds: &Bounds, beside: (Point, Point)| {
        distances(bounds, beside)
            .iter()
            .map(|distance| (LABEL_COMFORT - distance).max(0.0))
            .sum::<f64>()
            * CROWDED_LABEL_WEIGHT
    };
    let landmarks = label_landmarks(route, routes);
    let clearance = |bounds: &Bounds| {
        landmarks
            .iter()
            .map(|p| {
                let dx = (bounds.x - p.x).max(p.x - bounds.right()).max(0.0);
                let dy = (bounds.y - p.y).max(p.y - bounds.bottom()).max(0.0);
                hypot(dx, dy)
            })
            .fold(f64::INFINITY, f64::min)
    };
    // Every spot that fits, in order of preference: the longest segment
    // first, from its middle outwards. The one furthest from the nearest
    // crossing, bend or end wins, less for other connectors near it; ties
    // keep that order.
    let find = |fits: &dyn Fn(&Bounds, (Point, Point)) -> bool| {
        ordered
            .iter()
            .flat_map(|(_, (from, to))| {
                let (from, to) = (*from, *to);
                let horizontal = from.y == to.y;
                (0..=20).flat_map(move |step| {
                    let fraction = 0.5 + if step % 2 == 0 { 1.0 } else { -1.0 } * ((step as f64) / 2.0).ceil() * 0.05;
                    let along = point(
                        round(from.x + (to.x - from.x) * fraction),
                        round(from.y + (to.y - from.y) * fraction),
                    );
                    let beside = round(along.y - height / 2.0);
                    let candidates: [(Bounds, &'static str); 2] = if horizontal {
                        [
                            (
                                bounds_at(along.x - width / 2.0, along.y - gap - height, width, height),
                                "middle",
                            ),
                            (bounds_at(along.x - width / 2.0, along.y + gap, width, height), "middle"),
                        ]
                    } else {
                        [
                            (bounds_at(along.x + gap, beside, width, height), "start"),
                            (bounds_at(along.x - gap - width, beside, width, height), "end"),
                        ]
                    };
                    candidates
                        .into_iter()
                        .map(move |(bounds, anchor)| (bounds, anchor, (from, to)))
                        .filter(move |_| (0.0..=1.0).contains(&fraction))
                        .filter(move |(bounds, _, _)| {
                            if horizontal {
                                bounds.x >= from.x.min(to.x) && bounds.right() <= from.x.max(to.x)
                            } else {
                                bounds.y >= from.y.min(to.y) && bounds.bottom() <= from.y.max(to.y)
                            }
                        })
                })
            })
            .filter(|(bounds, _, beside)| fits(bounds, *beside))
            .fold(None, |best: Option<(f64, Placement)>, (bounds, anchor, beside)| {
                let score = clearance(&bounds) - crowding(&bounds, beside);
                match best {
                    Some((best_score, _)) if best_score >= score => best,
                    _ => Some((score, Placement { anchor, bounds })),
                }
            })
            .map(|(_, placement)| placement)
    };
    let clear = find(&|bounds, beside| fits(bounds) && clear_of_others(bounds, beside));
    if clear_only {
        clear
    } else {
        clear.or_else(|| find(&|bounds, _| fits(bounds)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Given a route that has turned zero to four times
    /// When its next turn is costed
    /// Then a third or later turn costs more than the first two
    /// Given two obstacles whose facing edges lie 4px apart
    /// When the grid lines along that axis are drawn
    /// Then the two edges share one line between them, each obstacle
    ///   pushed outwards onto it, and no line runs within the merge
    ///   distance of another
    #[test]
    fn draws_one_line_for_obstacle_edges_close_together() {
        let (lines, spans) = axis_lines(&[(300.0, 388.0), (392.0, 500.0)], &[], &[], &[], true);

        assert_eq!(spans[0].1, spans[1].0);
        assert!(spans[0].1 >= 388.0 && spans[1].0 <= 392.0);
        assert!(lines.windows(2).all(|pair| pair[1] - pair[0] > LINE_MERGE), "{lines:?}");
        assert!(lines.contains(&spans[0].1));
    }

    /// Given an obstacle edge on a terminal's line and another obstacle's
    ///   edge 2px from it
    /// When the grid lines are drawn
    /// Then the terminal's line stays where it is and both edges meet on it,
    ///   so the terminal never ends up inside an obstacle
    #[test]
    fn keeps_a_terminal_line_and_merges_edges_onto_it() {
        let (lines, spans) = axis_lines(&[(60.0, 140.0), (142.0, 206.0)], &[142.0], &[], &[], true);

        assert!(lines.contains(&142.0));
        assert_eq!(spans[0].1, 142.0);
        assert_eq!(spans[1].0, 142.0);
    }

    /// Given an obstacle edge on a terminal's line and an overlapping
    ///   obstacle whose near edge is 2px outside it
    /// When the grid lines are drawn
    /// Then the edge on the terminal's line is not pushed past the terminal
    #[test]
    fn does_not_push_an_edge_past_a_terminal() {
        let (_, spans) = axis_lines(&[(140.0, 300.0), (142.0, 206.0)], &[142.0], &[], &[], true);

        assert_eq!(spans[1].0, 142.0);
    }

    /// Given a gap of 5px between two obstacles, and a connector whose lane
    ///   would fall 1px from an obstacle edge
    /// When the grid lines are drawn
    /// Then the narrow gap gets no channel midline, the lane next to the
    ///   edge is left out, and lanes clear of other lines are kept
    #[test]
    fn leaves_out_lines_crowding_others() {
        let (lines, _) = axis_lines(
            &[(0.0, 100.0), (105.0, 200.0), (241.0, 260.0)],
            &[],
            &[250.0],
            &[250.0],
            true,
        );

        assert!(!lines.contains(&102.0) && !lines.contains(&103.0), "{lines:?}");
        assert!(lines.contains(&50.0));
        assert!(!lines.contains(&240.0), "{lines:?}");
        assert!(lines.contains(&270.0));
    }

    /// Given a cost that is full up to 10px and reaches nothing at 30px
    /// When it is taken at lengths from 0 to past its reach
    /// Then it stays full up to 10px, is half at 20px, falls without a jump
    ///   or a kink (flat as it leaves the full cost and as it reaches
    ///   nothing), and is nothing from 30px on
    #[test]
    fn fades_a_cost_smoothly_to_nothing_at_its_reach() {
        let fade = Fade {
            start: 10.0,
            reach: 30.0,
            full: 300.0,
        };

        assert_eq!(fade.cost(0.0), 300.0);
        assert_eq!(fade.cost(10.0), 300.0);
        assert!((fade.cost(20.0) - 150.0).abs() < 1e-9);
        assert_eq!(fade.cost(30.0), 0.0);
        assert_eq!(fade.cost(45.0), 0.0);
        assert!(300.0 - fade.cost(10.1) < 0.01);
        assert!(fade.cost(29.9) < 0.01);
        assert!((11..=30).all(|length| fade.cost(f64::from(length)) < fade.cost(f64::from(length - 1))));
    }

    /// Given a port on a box's left side with room for every stub
    /// When the ends a route may arrive at are offered
    /// Then every stub shorter than a short end costs, on top of the length
    ///   it gives up, what arranging charges for an end that short
    #[test]
    fn costs_a_stub_too_short_to_see_the_arrival_like_arranging_does() {
        let target = Bounds {
            x: 100.0,
            y: 0.0,
            width: 100.0,
            height: 60.0,
        };
        let ends = end_terminals(point(100.0, 30.0), &target, &[]);

        for end in &ends {
            let length = 100.0 - end.point.x;
            let expected = END_STUBS[0] - length + SHORT_END_FADE.cost(length);
            assert!((end.cost - expected).abs() < 1e-9, "{end:?}");
        }
        assert!(
            ends.iter()
                .any(|end| 100.0 - end.point.x < SHORT_END_FADE.start && end.cost > 100.0)
        );
    }

    /// Given routes with no bend up to four bends, all 300px long, and a
    ///   straight one with a point in the middle of its line
    /// When each is costed on its own
    /// Then it costs its length plus its bends, each from the third on
    ///   costing more than the first two, as the search prices them, and
    ///   the point in the middle of a line is no bend
    #[test]
    fn costs_a_drawn_route_by_its_length_and_bends() {
        let straight = [point(0.0, 0.0), point(300.0, 0.0)];
        let one_bend = [point(0.0, 0.0), point(200.0, 0.0), point(200.0, 100.0)];
        let stairs = [
            point(0.0, 0.0),
            point(60.0, 0.0),
            point(60.0, 60.0),
            point(120.0, 60.0),
            point(120.0, 120.0),
            point(180.0, 120.0),
        ];

        assert_eq!(route_cost(&straight), 300.0);
        assert_eq!(
            route_cost(&[point(0.0, 0.0), point(150.0, 0.0), point(300.0, 0.0)]),
            300.0
        );
        assert_eq!(route_cost(&one_bend), 300.0 + BEND_PENALTY);
        assert_eq!(
            route_cost(&stairs),
            300.0 + 2.0 * BEND_PENALTY + 2.0 * BEND_PENALTY * LATE_TURN_FACTOR
        );
    }

    /// Given a pair of routes crossing once, pairs running alongside 5px
    ///   apart for 100px and for 400px, a pair alongside for only 5px, a
    ///   pair alongside 14px apart, a pair where one turns 3px beside the
    ///   other, and a pair far apart
    /// When each pair is costed
    /// Then a crossing and a turn beside cost what the search charges for
    ///   them, running alongside costs the same however long it lasts, less
    ///   for a mere touch of overlap and less further apart, and the pair far
    ///   apart costs nothing
    #[test]
    fn costs_two_drawn_routes_as_the_search_prices_them() {
        let across = [point(50.0, -50.0), point(50.0, 50.0)];
        let along = [point(0.0, 0.0), point(100.0, 0.0)];
        let beside = [point(0.0, 5.0), point(100.0, 5.0)];
        let long_along = [point(0.0, 0.0), point(400.0, 0.0)];
        let long_beside = [point(0.0, 5.0), point(400.0, 5.0)];
        let touching = [point(95.0, 5.0), point(200.0, 5.0)];
        let wider = [point(0.0, 14.0), point(100.0, 14.0)];
        let turning = [point(30.0, 30.0), point(30.0, 3.0), point(90.0, 3.0)];
        let far = [point(0.0, 200.0), point(100.0, 200.0)];

        assert_eq!(routes_cost(&along, &across), CROSSING_PENALTY);
        assert_eq!(routes_cost(&along, &beside), CLOSE_RUN.full);
        assert_eq!(routes_cost(&long_along, &long_beside), CLOSE_RUN.full);
        assert!(routes_cost(&along, &touching) > 0.0 && routes_cost(&along, &touching) < CLOSE_RUN.full / 2.0);
        assert!(routes_cost(&along, &wider) > 0.0 && routes_cost(&along, &wider) < CLOSE_RUN.full / 2.0);
        assert!(routes_cost(&along, &turning) >= NEAR_MISS_PENALTY);
        assert_eq!(routes_cost(&along, &far), 0.0);
        assert_eq!(routes_cost(&across, &along), routes_cost(&along, &across));
    }

    /// Given a route crossing one it runs 8px beside for 80px first, one
    ///   crossing it after running 30px beside, and one crossing it without
    ///   running beside it
    /// When each pair is costed
    /// Then crossing the route it ran right beside costs 2.6 crossings,
    ///   crossing after running further off costs between one and that,
    ///   and crossing alone costs one
    #[test]
    fn prices_crossing_a_route_running_alongside_higher() {
        let along = [point(0.0, 0.0), point(200.0, 0.0)];
        let after_running_at = |gap: f64| {
            [
                point(0.0, gap),
                point(80.0, gap),
                point(80.0, 40.0),
                point(120.0, 40.0),
                point(120.0, -40.0),
            ]
        };
        let apart = [point(0.0, 80.0), point(120.0, 80.0), point(120.0, -40.0)];
        let wider = routes_cost(&along, &after_running_at(30.0));

        assert_eq!(
            routes_cost(&along, &after_running_at(8.0)),
            CLOSE_RUN.full + 2.6 * CROSSING_PENALTY
        );
        assert!(wider > CROSSING_PENALTY && wider < 2.6 * CROSSING_PENALTY, "{wider}");
        assert_eq!(routes_cost(&along, &apart), CROSSING_PENALTY);
    }

    /// Given a box reaching across the left and lower edges of an area, and
    ///   a terminal on the area's lower edge
    /// When the grid for a search is built over that area only
    /// Then the points on that edge inside the box cannot be passed, so a
    ///   route kept to the area cannot slip through the box along its edge
    #[test]
    fn keeps_a_box_cut_by_the_area_s_edge_blocked_on_that_edge() {
        let terminal = |x: f64, y: f64| Terminal {
            point: point(x, y),
            axis: 0,
            port: point(x + 24.0, y),
            cost: 0.0,
        };
        let area = Bounds {
            x: 50.0,
            y: -50.0,
            width: 200.0,
            height: 100.0,
        };
        let boxed = Bounds {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
        };
        let built = grid(
            &[terminal(200.0, 50.0)],
            &[terminal(200.0, -40.0)],
            &[boxed],
            &[],
            &[],
            &[],
            Lines::Inside(area),
        );
        let row = index_of(&built.ys, 50.0).unwrap();
        let inside: Vec<usize> = (0..built.xs.len())
            .filter(|xi| area.x <= built.xs[*xi] && built.xs[*xi] < boxed.right())
            .collect();

        assert!(!inside.is_empty(), "{:?}", built.xs);
        assert!(inside.iter().all(|xi| built.blocked_cell[row * built.xs.len() + xi]));
    }

    /// Given a connector drawn along y = 0 and terminals on y = 6
    /// When the grid for a search is built
    /// Then the steps on y = 6 beside the connector cost running alongside
    ///   it in full, and those on the lane at y = 20 cost nothing
    #[test]
    fn marks_the_steps_running_alongside_a_drawn_connector() {
        let terminal = |x: f64| Terminal {
            point: point(x, 6.0),
            axis: 0,
            port: point(x - 24.0, 6.0),
            cost: 0.0,
        };
        let built = grid(
            &[terminal(-50.0)],
            &[terminal(200.0)],
            &[],
            &[],
            &[vec![point(0.0, 0.0), point(100.0, 0.0)]],
            &[],
            Lines::All,
        );
        let step = |y: f64| built.near_horizontal[index_of(&built.ys, y).unwrap() * built.xs.len()];

        assert_eq!(step(6.0), CLOSE_RUN.full);
        assert_eq!(step(20.0), 0.0);
    }

    /// Given a connector drawn along y = 0 and a route to find straight
    ///   along y = 6, with a row of far-off boxes splitting that line into
    ///   many grid steps
    /// When the route is searched
    /// Then it runs straight alongside the connector, paying for running
    ///   alongside once rather than on every step, which would make
    ///   stepping away and back with four bends cheaper
    #[test]
    fn pays_once_for_running_alongside_a_connector_however_many_steps() {
        let terminal = |x: f64, port: f64| Terminal {
            point: point(x, 6.0),
            axis: 0,
            port: point(port, 6.0),
            cost: 0.0,
        };
        let boxes: Vec<Bounds> = (0..8)
            .map(|index| Bounds {
                x: 20.0 + 50.0 * f64::from(index),
                y: -400.0,
                width: 20.0,
                height: 20.0,
            })
            .collect();
        let drawn = vec![vec![point(0.0, 0.0), point(400.0, 0.0)]];

        let found = search(
            &[terminal(-20.0, -44.0)],
            &[terminal(420.0, 444.0)],
            &boxes,
            &[],
            &drawn,
            &[],
            BEND_PENALTY,
        )
        .unwrap();

        assert_eq!(found.path, vec![point(-20.0, 6.0), point(420.0, 6.0)]);
    }

    #[test]
    fn counts_the_turns_a_route_cannot_avoid() {
        // A stub ending 24px above a port at (100, 100), arrived at moving down.
        let terminal = Terminal {
            point: point(100.0, 76.0),
            axis: 1,
            port: point(100.0, 100.0),
            cost: 0.0,
        };

        assert_eq!(fewest_turns(point(100.0, 20.0), 1, &terminal), 0);
        assert_eq!(fewest_turns(point(40.0, 20.0), 1, &terminal), 2);
        assert_eq!(fewest_turns(point(40.0, 20.0), 0, &terminal), 1);
        assert_eq!(fewest_turns(point(100.0, 90.0), 1, &terminal), 2);
    }

    #[test]
    fn costs_a_route_s_third_and_later_turns_more() {
        assert_eq!(turn_cost(0, BEND_PENALTY), turn_cost(1, BEND_PENALTY));
        assert!(turn_cost(2, BEND_PENALTY) > turn_cost(1, BEND_PENALTY));
        assert_eq!(turn_cost(3, BEND_PENALTY), turn_cost(2, BEND_PENALTY));
    }
}
