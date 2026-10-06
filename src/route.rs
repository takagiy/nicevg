//! Orthogonal routing: choosing node sides and ports, searching a sparse
//! grid for the cheapest route, and finding label slots beside routes.

use std::cmp::{Ordering, Reverse};
use std::collections::{BinaryHeap, HashMap};

use crate::diagram::DiagramNode;
use crate::geometry::{
    Bounds, Point, intersection, point, round, route_is_clear, routes_overlap, segment_intersects_interior,
    segment_length, segments, without_redundant_points,
};

pub const BEND_PENALTY: f64 = 40.0;
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
/// Running beside another connector costs this much extra per unit of
/// length: a lot within a lane of it, a little within two lanes.
const CLOSE_RUN_COST: f64 = 2.0;
const NEAR_RUN_COST: f64 = 0.2;
/// Crossing another connector costs about as much as a bend.
const CROSSING_PENALTY: f64 = 40.0;
pub const CLEARANCE: f64 = 8.0;
/// How far a route runs straight into its target before the last bend,
/// longest first. An arrowhead is about 10px long, so the clearance alone
/// leaves the last bend under it; shorter stubs are the fallbacks when
/// another node is in the way.
pub const END_STUBS: [f64; 3] = [24.0, 16.0, CLEARANCE];

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
/// the obstacles; every pixel of stub given up costs a pixel of route.
fn end_terminals(port: Point, target: &Bounds, obstacles: &[Bounds]) -> Vec<Terminal> {
    END_STUBS
        .iter()
        .filter(|length| {
            **length == CLEARANCE || route_is_clear(&[port, outward(port, side_of(port, target), **length)], obstacles)
        })
        .map(|length| stub_terminal(port, target, *length, END_STUBS[0] - length))
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
    lines.iter().position(|line| *line == value)
}

/// Indices of the grid steps (between lines[i] and lines[i + 1]) that share
/// some length with the span from a to b; lines are sorted ascending.
fn steps_covering(lines: &[f64], a: f64, b: f64) -> (usize, isize) {
    let (low, high) = (a.min(b), a.max(b));
    let (mut first, mut last) = (0usize, lines.len().saturating_sub(1));
    while first < last {
        let middle = (first + last) / 2;
        if lines.get(middle + 1).copied().unwrap_or(f64::INFINITY) > low {
            last = middle;
        } else {
            first = middle + 1;
        }
    }
    let mut end = first as isize - 1;
    while ((end + 1) as usize) < lines.len().saturating_sub(1)
        && lines.get((end + 1) as usize).copied().unwrap_or(f64::INFINITY) < high
    {
        end += 1;
    }
    (first, end)
}

#[derive(PartialEq)]
struct Entry(f64, usize);

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

/// A* over a sparse orthogonal grid built from obstacle edges, the channels
/// between them, lanes beside drawn connectors and the terminals. Starts
/// from whichever start terminal and stops at whichever end terminal is
/// cheapest overall.
pub fn search(
    starts: &[Terminal],
    ends: &[Terminal],
    obstacles: &[Bounds],
    occupied: &[Vec<Point>],
    bend_penalty: f64,
) -> Option<Found> {
    let occupied_segments: Vec<(Point, Point)> = occupied.iter().flat_map(|route| segments(route)).collect();
    let axis = |pick: fn(&Point) -> f64, low: fn(&Bounds) -> f64, high: fn(&Bounds) -> f64| {
        // Lanes one and two lanes beside connectors already drawn let new
        // routes run alongside them when every other line through a
        // channel is taken, keeping clear of them when there is room.
        let lanes = occupied_segments.iter().flat_map(|(from, to)| {
            let at = pick(from);
            (at == pick(to))
                .then_some([
                    at - 2.0 * LANE_SPACING,
                    at - LANE_SPACING,
                    at + LANE_SPACING,
                    at + 2.0 * LANE_SPACING,
                ])
                .into_iter()
                .flatten()
        });
        let edges = sorted_distinct(
            starts
                .iter()
                .chain(ends)
                .map(|terminal| pick(&terminal.point))
                .chain(obstacles.iter().flat_map(|bounds| [low(bounds), high(bounds)])),
        );
        // Midlines of the gaps between edges, rounded to whole units.
        let channels = edges.windows(2).map(|pair| round((pair[0] + pair[1]) / 2.0));
        sorted_distinct(edges.iter().copied().chain(channels).chain(lanes))
    };
    let xs = axis(|p| p.x, |b| b.x, Bounds::right);
    let ys = axis(|p| p.y, |b| b.y, Bounds::bottom);
    let width = xs.len();
    let cells = width * ys.len();
    let key = |xi: usize, yi: usize| yi * width + xi;

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
    for bounds in obstacles {
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
    // Grid edges that share any length with another connector's segment are
    // taken, including segments shorter than one grid step.
    for (from, to) in &occupied_segments {
        if from.y == to.y {
            let Some(yi) = index_of(&ys, from.y) else { continue };
            let (first, last) = steps_covering(&xs, from.x, to.x);
            for xi in first as isize..=last {
                blocked_horizontal[key(xi as usize, yi)] = true;
            }
        } else if from.x == to.x {
            let Some(xi) = index_of(&xs, from.x) else { continue };
            let (first, last) = steps_covering(&ys, from.y, to.y);
            for yi in first as isize..=last {
                blocked_vertical[key(xi, yi as usize)] = true;
            }
        }
    }

    // Grid edges running parallel to another connector's segment, closer
    // than two lanes, cost extra: closely packed lines read as one band.
    let mut near_horizontal = vec![0.0f64; cells];
    let mut near_vertical = vec![0.0f64; cells];
    let near_cost = |distance: f64| {
        if distance <= 0.0 || distance >= 2.0 * LANE_SPACING {
            0.0
        } else if distance < LANE_SPACING {
            CLOSE_RUN_COST
        } else {
            NEAR_RUN_COST
        }
    };
    for (from, to) in &occupied_segments {
        if from.y == to.y {
            let (first, last) = steps_covering(&xs, from.x, to.x);
            for (yi, y) in ys.iter().enumerate() {
                let extra = near_cost((y - from.y).abs());
                for xi in (first as isize..=last).filter(|_| extra > 0.0) {
                    let index = key(xi as usize, yi);
                    near_horizontal[index] = near_horizontal[index].max(extra);
                }
            }
        } else if from.x == to.x {
            let (first, last) = steps_covering(&ys, from.y, to.y);
            for (xi, x) in xs.iter().enumerate() {
                let extra = near_cost((x - from.x).abs());
                for yi in (first as isize..=last).filter(|_| extra > 0.0) {
                    let index = key(xi, yi as usize);
                    near_vertical[index] = near_vertical[index].max(extra);
                }
            }
        }
    }

    // Grid edges that cross another connector's segment, or reach a point
    // inside it, cost a crossing. An edge ending on the segment counts, the
    // one leaving it does not, so passing straight through counts once.
    let mut crossing_horizontal = vec![false; cells];
    let mut crossing_vertical = vec![false; cells];
    for (from, to) in &occupied_segments {
        if from.x == to.x {
            let (low, high) = (from.y.min(to.y), from.y.max(to.y));
            for (yi, _) in ys.iter().enumerate().filter(|(_, y)| low < **y && **y < high) {
                for xi in (0..width.saturating_sub(1)).filter(|xi| xs[*xi] < from.x && from.x <= xs[xi + 1]) {
                    crossing_horizontal[key(xi, yi)] = true;
                }
            }
        } else if from.y == to.y {
            let (low, high) = (from.x.min(to.x), from.x.max(to.x));
            for (xi, _) in xs.iter().enumerate().filter(|(_, x)| low < **x && **x < high) {
                for yi in (0..ys.len().saturating_sub(1)).filter(|yi| ys[*yi] < from.y && from.y <= ys[yi + 1]) {
                    crossing_vertical[key(xi, yi)] = true;
                }
            }
        }
    }

    let cell_of = |target: &Point| Some(key(index_of(&xs, target.x)?, index_of(&ys, target.y)?));
    let end_cells: HashMap<usize, (usize, Terminal)> = ends
        .iter()
        .enumerate()
        .filter_map(|(index, terminal)| Some((cell_of(&terminal.point)?, (index, *terminal))))
        .collect();
    let point_of = |cell: usize| point(xs[cell % width], ys[cell / width]);
    // A*: every edge costs at least its length, so the Manhattan distance to
    // the nearest end never overestimates and the cheapest route is found.
    let estimate = |cell: usize| {
        let here = point_of(cell);
        ends.iter()
            .map(|terminal| (terminal.point.x - here.x).abs() + (terminal.point.y - here.y).abs())
            .fold(f64::INFINITY, f64::min)
    };

    // state = cell * 2 + (0: arrived horizontally, 1: arrived vertically)
    let mut distance = vec![f64::INFINITY; cells * 2];
    let mut previous: Vec<Option<usize>> = vec![None; cells * 2];
    let mut queue = BinaryHeap::new();
    for terminal in starts {
        let Some(cell) = cell_of(&terminal.point) else { continue };
        let state = cell * 2 + terminal.axis;
        if blocked_cell[cell] || terminal.cost >= distance[state] {
            continue;
        }
        distance[state] = terminal.cost;
        queue.push(Reverse(Entry(terminal.cost + estimate(cell), state)));
    }
    let mut reached = None;
    while let Some(Reverse(Entry(priority, state))) = queue.pop() {
        let cell = state / 2;
        let cost = distance[state];
        if priority > cost + estimate(cell) {
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
            if end.is_some_and(|(_, terminal)| arrives_backwards(here, point_of(next_cell), terminal)) {
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
            let bend = (if state % 2 != vertical { bend_penalty } else { 0.0 })
                + end.map_or(0.0, |(_, terminal)| {
                    terminal.cost + if vertical != terminal.axis { bend_penalty } else { 0.0 }
                });
            let next_state = next_cell * 2 + vertical;
            let next = point_of(next_cell);
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
            let next_cost = cost
                + length * (if outline[edge] { 1.01 } else { 1.0 } + near)
                + bend
                + if crossing { CROSSING_PENALTY } else { 0.0 };
            if next_cost >= distance[next_state] {
                continue;
            }
            distance[next_state] = next_cost;
            previous[next_state] = Some(state);
            queue.push(Reverse(Entry(next_cost + estimate(next_cell), next_state)));
        }
    }
    let reached = reached?;
    let path: Vec<Point> = std::iter::successors(Some(reached), |state| previous[*state])
        .map(|state| point_of(state / 2))
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    let first = path.first().copied();
    Some(Found {
        start: starts.iter().position(|terminal| Some(terminal.point) == first),
        end: end_cells.get(&(reached / 2)).map(|(index, _)| *index),
        path: without_redundant_points(&path),
    })
}

/// A route between two ports: straight when the ports line up and nothing
/// is in the way, otherwise the cheapest detour leaving and entering the
/// node sides at right angles.
pub fn route_connector(
    source: &Bounds,
    target: &Bounds,
    raw_obstacles: &[Bounds],
    ports: Ports,
    occupied: &[Vec<Point>],
) -> Vec<Point> {
    let obstacles: Vec<Bounds> = raw_obstacles.iter().map(|bounds| bounds.inflate(CLEARANCE)).collect();
    let direct = vec![ports.start, ports.end];
    if (ports.start.x == ports.end.x || ports.start.y == ports.end.y)
        && route_is_clear(&direct, &obstacles)
        && occupied.iter().all(|other| !routes_overlap(&direct, other))
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
        occupied,
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
pub fn choose_sides(
    source: &Bounds,
    target: &Bounds,
    raw_obstacles: &[Bounds],
    occupied: &[Vec<Point>],
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
    let found = search(
        &terminals(source, fallback.start),
        &terminals(target, fallback.end),
        &obstacles,
        occupied,
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
pub fn spread_ports(requests: &[PortRequest]) -> HashMap<String, Ports> {
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
        .fold(midpoints.clone(), |ports, (key, users)| spread_side(ports, key, users));
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
/// the position of the node at their other end.
fn spread_side(ports: HashMap<String, Ports>, key: &str, users: &[SideUser]) -> HashMap<String, Ports> {
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
    let count = ordered.len() as f64;
    ordered.iter().enumerate().fold(ports, |mut ports, (index, (_, user))| {
        let Some(port) = ports.get(&user.id).copied() else {
            return ports;
        };
        let fraction = (index as f64 + 1.0) / (count + 1.0);
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
/// also fits on the other end's side, away from its corners and at least a
/// lane away from other ports there, both ends share it.
fn align_facing_ports(requests: &[PortRequest], ports: HashMap<String, Ports>) -> HashMap<String, Ports> {
    let margin = 8.0;
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
        let fits = |bounds: &Bounds, value: f64| {
            if along_y {
                value >= bounds.y + margin && value <= bounds.bottom() - margin
            } else {
                value >= bounds.x + margin && value <= bounds.right() - margin
            }
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
        if fits(&request.target, start_value) && !taken(&ports, &request.target_id, end_side, start_value) {
            ports.insert(
                request.id.clone(),
                Ports {
                    end: moved(port.end, start_value),
                    ..port
                },
            );
        } else if fits(&request.source, end_value) && !taken(&ports, &request.source_id, start_side, end_value) {
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
pub fn place_label(
    width: f64,
    height: f64,
    route: &[Point],
    routes: &[Vec<Point>],
    blocked: &[Bounds],
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
    };
    ordered.iter().find_map(|(_, (from, to))| {
        let horizontal = from.y == to.y;
        (0..=20).find_map(|step| {
            let fraction = 0.5 + if step % 2 == 0 { 1.0 } else { -1.0 } * ((step as f64) / 2.0).ceil() * 0.05;
            if !(0.0..=1.0).contains(&fraction) {
                return None;
            }
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
            candidates.into_iter().find_map(|(bounds, anchor)| {
                let within = if horizontal {
                    bounds.x >= from.x.min(to.x) && bounds.right() <= from.x.max(to.x)
                } else {
                    bounds.y >= from.y.min(to.y) && bounds.bottom() <= from.y.max(to.y)
                };
                (within && fits(&bounds)).then_some(Placement { anchor, bounds })
            })
        })
    })
}
