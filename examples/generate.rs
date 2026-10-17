//! Generates diagrams the way an agent drafts them: laid out with intent
//! (layers, grids, lanes), with the small mistakes such drafts have. Labels
//! measured too short for their boxes, nodes a few pixels off their line or
//! too close, connectors drawn diagonally, centre to centre, through other
//! nodes or stopping short, labels away from their route, a viewBox cut a
//! little short. Material for nicevg to repair and to learn from.
//!
//! ```text
//! cargo run --release --example generate -- --out drafts
//! cargo run --release --example generate -- --kind er --size large --mess messy --count 20 --seed 7 --out drafts
//! ```
//!
//! `--spec` drafts the diagrams JSON specs describe (nodes, groups and
//! edges, as converters write them from datasets) instead of made-up ones.
//! `--mess` picks how rough: `tidy`, `sloppy`, `messy` or `wild`. Each file
//! is `<kind>-<size>-<mess>-<seed>.svg`. The same arguments always give
//! the same files. Each draft is read back with `nicevg::analyze`, and its
//! issues are printed by code.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::PathBuf;

use clap::{Parser, ValueEnum};
use serde::Deserialize;

#[derive(Parser)]
struct Args {
    /// The kind of diagram; all kinds by default.
    #[arg(long)]
    kind: Option<Kind>,
    /// The size; all sizes by default.
    #[arg(long)]
    size: Option<Size>,
    /// How rough the drafts are; all levels by default.
    #[arg(long)]
    mess: Option<Level>,
    /// How many drafts of each kind, size and level.
    #[arg(long, default_value_t = 3)]
    count: u64,
    /// The first seed; the drafts take this one and the ones after it.
    #[arg(long, default_value_t = 1)]
    seed: u64,
    /// Where to write the drafts.
    #[arg(long)]
    out: PathBuf,
    /// Draft the diagrams these specs describe (a JSON file, or a
    /// directory of them) instead of made-up ones; kind and size are then
    /// the spec's own.
    #[arg(long)]
    spec: Option<PathBuf>,
    /// How to lay a spec out: where its nodes were drawn, when it says,
    /// else in layers wrapped onto rows when a flow runs long.
    #[arg(long)]
    layout: Option<Layout>,
}

#[derive(Clone, Copy, Debug, PartialEq, ValueEnum)]
enum Layout {
    /// Where the spec's own positions put each node.
    Drawn,
    /// In layers, a flow left to right in one long band.
    Layered,
    /// In layers, a long flow wrapped onto rows that turn back and forth.
    Wrapped,
}

#[derive(Clone, Copy, Debug, PartialEq, ValueEnum)]
enum Kind {
    Dfd,
    C4,
    Er,
    Flow,
    Arch,
}

/// How carefully the agent drew: from a few slips to a sketch of straight
/// lines thrown between rough positions.
#[derive(Clone, Copy, Debug, PartialEq, ValueEnum)]
enum Level {
    Tidy,
    Sloppy,
    Messy,
    Wild,
}

/// The chances and sizes of the slips at one level.
#[derive(Clone, Copy)]
struct Mess {
    /// A label's width guessed too short for its box.
    short_label: f64,
    /// A node off the line it was meant on, and by how much at most.
    nudge: f64,
    nudge_px: i64,
    /// A node pulled closer to its right-hand neighbour than 20px, and how
    /// far past the gap it may go.
    crowd: f64,
    crowd_px: i64,
    /// A connector drawn centre to centre, diagonally, or straight along
    /// one axis stopping wherever that lands.
    centre: f64,
    diagonal: f64,
    short: f64,
    /// A container fitted too tightly round its members.
    tight_frame: f64,
    /// A canvas cut short, and by how much at most.
    cut: f64,
    cut_px: i64,
    /// A label placed off its route, and how far it drifts at most.
    label_off: f64,
    label_px: i64,
    /// How far above its line a label sits.
    label_gap: f64,
    /// How much room the layout leaves: the intended spacing times this.
    spread: f64,
    /// How far apart the agent staggers the elbows of connectors that
    /// would otherwise turn on the same line.
    lane_px: f64,
}

impl Level {
    fn mess(self) -> Mess {
        match self {
            Level::Tidy => Mess {
                short_label: 0.05,
                nudge: 0.1,
                nudge_px: 3,
                crowd: 0.02,
                crowd_px: 8,
                centre: 0.0,
                diagonal: 0.05,
                short: 0.02,
                tight_frame: 0.05,
                cut: 0.1,
                cut_px: 20,
                label_off: 0.05,
                label_px: 4,
                label_gap: 16.0,
                spread: 1.3,
                lane_px: 12.0,
            },
            Level::Sloppy => Mess {
                short_label: 0.2,
                nudge: 0.35,
                nudge_px: 6,
                crowd: 0.08,
                crowd_px: 14,
                centre: 0.08,
                diagonal: 0.22,
                short: 0.08,
                tight_frame: 0.15,
                cut: 0.33,
                cut_px: 50,
                label_off: 0.2,
                label_px: 8,
                label_gap: 6.0,
                spread: 1.0,
                lane_px: 6.0,
            },
            Level::Messy => Mess {
                short_label: 0.4,
                nudge: 0.6,
                nudge_px: 14,
                crowd: 0.2,
                crowd_px: 40,
                centre: 0.15,
                diagonal: 0.4,
                short: 0.15,
                tight_frame: 0.4,
                cut: 0.6,
                cut_px: 90,
                label_off: 0.4,
                label_px: 20,
                label_gap: 4.0,
                spread: 0.9,
                lane_px: 0.0,
            },
            Level::Wild => Mess {
                short_label: 0.5,
                nudge: 0.9,
                nudge_px: 30,
                crowd: 0.3,
                crowd_px: 70,
                centre: 0.7,
                diagonal: 0.25,
                short: 0.05,
                tight_frame: 0.6,
                cut: 0.8,
                cut_px: 140,
                label_off: 0.6,
                label_px: 36,
                label_gap: 2.0,
                spread: 0.8,
                lane_px: 0.0,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, ValueEnum)]
enum Size {
    Small,
    Medium,
    Large,
}

impl Size {
    /// A count that grows with the size: `small`, about twice for medium,
    /// about four times for large.
    fn scale(self, small: usize) -> usize {
        match self {
            Size::Small => small,
            Size::Medium => small * 2,
            Size::Large => small * 4,
        }
    }
}

/// SplitMix64: small, fast and the same everywhere.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Rng {
        Rng(seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) ^ 0x2545_f491_4f6c_dd1d)
    }

    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }

    fn chance(&mut self, p: f64) -> bool {
        self.unit() < p
    }

    /// A whole number in `low..=high`.
    fn int(&mut self, low: i64, high: i64) -> i64 {
        low + (self.next() % (high - low + 1) as u64) as i64
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    fn pick<'a>(&mut self, items: &'a [&'a str]) -> &'a str {
        items[self.below(items.len())]
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Shape {
    Rect,
    Circle,
    Table,
}

struct Node {
    id: String,
    class: &'static str,
    shape: Shape,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    /// The lines of its label, first the name; for a table, its columns.
    lines: Vec<String>,
}

impl Node {
    fn centre(&self) -> (f64, f64) {
        (self.x + self.w / 2.0, self.y + self.h / 2.0)
    }
}

struct Container {
    id: String,
    title: String,
    members: Vec<usize>,
}

struct Edge {
    from: usize,
    to: usize,
    label: Option<String>,
    /// For a table: the column of each end, whose row the connector meets.
    ports: [Option<usize>; 2],
}

struct Draft {
    mess: Mess,
    title: String,
    nodes: Vec<Node>,
    containers: Vec<Container>,
    edges: Vec<Edge>,
    /// Names written under a node rather than in it, as BPMN names events.
    captions: Vec<(usize, String)>,
}

const FONT: f64 = 13.0;
const ROW: f64 = 20.0;
const HEAD: f64 = 26.0;

impl Draft {
    fn new(title: &str, mess: Mess) -> Draft {
        Draft {
            mess,
            title: title.to_owned(),
            nodes: Vec::new(),
            containers: Vec::new(),
            edges: Vec::new(),
            captions: Vec::new(),
        }
    }

    /// A node sized the way an agent guesses text widths: about right, but
    /// now and then, more often the rougher the level, too short for its
    /// label.
    fn add(&mut self, rng: &mut Rng, class: &'static str, shape: Shape, lines: Vec<String>, at: (f64, f64)) -> usize {
        let m = self.mess;
        let guess = if rng.chance(m.short_label) { 0.42 } else { 0.62 };
        let widest = lines.iter().map(|line| line.chars().count()).max().unwrap_or(1) as f64;
        let (w, h) = match shape {
            Shape::Circle => {
                let d = (widest * FONT * guess + 24.0).max(64.0).round();
                (d, d)
            }
            Shape::Rect => (
                (widest * FONT * guess + 32.0).max(96.0).round(),
                (lines.len() as f64 * 18.0 + 26.0).max(48.0),
            ),
            Shape::Table => (
                (widest * 12.0 * guess + 70.0).max(140.0).round(),
                HEAD + (lines.len() - 1) as f64 * ROW,
            ),
        };
        // A few pixels off the line the agent meant.
        let nudge = |rng: &mut Rng| {
            if rng.chance(m.nudge) {
                rng.int(-m.nudge_px, m.nudge_px) as f64
            } else {
                0.0
            }
        };
        let id = format!("n{}", self.nodes.len() + 1);
        self.nodes.push(Node {
            id,
            class,
            shape,
            x: at.0 - w / 2.0 + nudge(rng),
            y: at.1 - h / 2.0 + nudge(rng),
            w,
            h,
            lines,
        });
        self.nodes.len() - 1
    }

    fn link(&mut self, from: usize, to: usize, label: Option<String>) {
        if from != to && !self.edges.iter().any(|e| e.from == from && e.to == to) {
            self.edges.push(Edge {
                from,
                to,
                label,
                ports: [None, None],
            });
        }
    }

    /// Spreads the layout out or packs it in about the origin, leaving the
    /// nodes their sizes.
    fn spread(&mut self) {
        let f = self.mess.spread;
        for n in &mut self.nodes {
            let (cx, cy) = n.centre();
            n.x = (cx * f - n.w / 2.0).round();
            n.y = (cy * f - n.h / 2.0).round();
        }
    }

    /// Now and then pulls a node towards its right-hand neighbour in the
    /// same row, closer than the 20px a gap should keep.
    fn crowd(&mut self, rng: &mut Rng) {
        for index in 0..self.nodes.len() {
            if !rng.chance(self.mess.crowd) {
                continue;
            }
            let (x, y, h) = (self.nodes[index].x, self.nodes[index].y, self.nodes[index].h);
            let right = (0..self.nodes.len())
                .filter(|other| {
                    let o = &self.nodes[*other];
                    o.x > x && o.y < y + h && o.y + o.h > y
                })
                .min_by(|a, b| self.nodes[*a].x.total_cmp(&self.nodes[*b].x));
            if let Some(right) = right {
                let gap = self.nodes[right].x - (x + self.nodes[index].w);
                self.nodes[index].x += gap - rng.int(4, self.mess.crowd_px) as f64;
            }
        }
    }
}

/// `count` names from `pool` in a shuffled order, each once while the
/// pool lasts, then again with a number.
fn names(rng: &mut Rng, pool: &[&str], count: usize) -> Vec<String> {
    let mut order: Vec<&str> = pool.to_vec();
    for i in (1..order.len()).rev() {
        order.swap(i, rng.below(i + 1));
    }
    (0..count)
        .map(|i| match i / order.len() {
            0 => order[i].to_owned(),
            round => format!("{} {}", order[i % order.len()], round + 1),
        })
        .collect()
}

const PROCESSES: &[&str] = &[
    "Validate order",
    "Reserve stock",
    "Take payment",
    "Ship parcel",
    "Send receipt",
    "Check credit",
    "Score risk",
    "Open account",
    "Verify identity",
    "Price quote",
    "Approve loan",
    "Schedule visit",
    "Match supplier",
    "Issue refund",
    "Track delivery",
    "Generate invoice",
    "Notify customer",
    "Reconcile ledger",
    "Assign courier",
    "Review claim",
];
const ENTITIES: &[&str] = &[
    "Customer",
    "Bank",
    "Courier",
    "Supplier",
    "Tax office",
    "Auditor",
    "Support agent",
    "Warehouse",
    "Payment provider",
    "Partner API",
];
const STORES: &[&str] = &[
    "Orders",
    "Customers",
    "Stock",
    "Payments",
    "Invoices",
    "Shipments",
    "Accounts",
    "Claims",
    "Prices",
    "Audit log",
];
const FLOWS: &[&str] = &[
    "order",
    "order details",
    "stock level",
    "payment",
    "receipt",
    "status",
    "invoice",
    "refund",
    "address",
    "approval",
    "quote",
    "risk score",
    "tracking id",
    "claim form",
];

fn dfd(rng: &mut Rng, size: Size, mess: Mess) -> Draft {
    let mut d = Draft::new("Order handling — data flow diagram", mess);
    let processes = size.scale(3);
    let cols = (processes as f64).sqrt().ceil() as usize + 1;
    let (cw, rh) = (230.0, 190.0);
    let process_names = names(rng, PROCESSES, processes);
    let ps: Vec<usize> = process_names
        .into_iter()
        .enumerate()
        .map(|(i, name)| {
            let at = (260.0 + (i % cols) as f64 * cw, 140.0 + (i / cols) as f64 * rh);
            d.add(rng, "process", Shape::Circle, vec![format!("{} {name}", i + 1)], at)
        })
        .collect();
    let rows = processes.div_ceil(cols);
    let entities = size.scale(2).min(ENTITIES.len());
    let es: Vec<usize> = names(rng, ENTITIES, entities)
        .into_iter()
        .enumerate()
        .map(|(i, name)| {
            let left = i % 2 == 0;
            let x = if left { 80.0 } else { 260.0 + cols as f64 * cw };
            let y = 140.0 + (i / 2) as f64 * rh * rows as f64 / entities.div_ceil(2) as f64;
            d.add(rng, "entity", Shape::Rect, vec![name], (x, y))
        })
        .collect();
    let stores = size.scale(2).min(STORES.len());
    let ss: Vec<usize> = names(rng, STORES, stores)
        .into_iter()
        .enumerate()
        .map(|(i, name)| {
            let at = (
                260.0 + cw / 2.0 + (i % cols) as f64 * cw,
                140.0 + rh / 2.0 + (i / cols % rows.max(1)) as f64 * rh,
            );
            d.add(rng, "store", Shape::Rect, vec![format!("D{} {name}", i + 1)], at)
        })
        .collect();
    for pair in ps.windows(2) {
        let label = Some(rng.pick(FLOWS).to_owned());
        d.link(pair[0], pair[1], label);
    }
    for (i, e) in es.iter().enumerate() {
        let p = ps[(i * 3) % ps.len()];
        let label = Some(rng.pick(FLOWS).to_owned());
        if i % 2 == 0 {
            d.link(*e, p, label);
        } else {
            d.link(p, *e, label);
        }
    }
    for (i, s) in ss.iter().enumerate() {
        let p = ps[i % ps.len()];
        let label = Some(rng.pick(FLOWS).to_owned());
        d.link(p, *s, label);
        if rng.chance(0.4) {
            let q = ps[(i + 1) % ps.len()];
            d.link(*s, q, Some(rng.pick(FLOWS).to_owned()));
        }
    }
    d
}

const PEOPLE: &[&str] = &[
    "Customer",
    "Courier",
    "Restaurant owner",
    "Support agent",
    "Admin",
    "Partner",
];
const APPS: &[(&str, &str)] = &[
    ("Web app", "React"),
    ("Mobile app", "Swift"),
    ("API gateway", "Envoy"),
    ("Order service", "Go"),
    ("Payment service", "Java"),
    ("Search service", "Rust"),
    ("Notification service", "Node.js"),
    ("Catalog service", "Kotlin"),
    ("Auth service", "Go"),
    ("Reporting service", "Python"),
    ("Scheduler", "Python"),
    ("Pricing service", "Rust"),
];
const DATABASES: &[(&str, &str)] = &[
    ("Orders DB", "PostgreSQL"),
    ("Cache", "Redis"),
    ("Search index", "OpenSearch"),
    ("Event bus", "Kafka"),
    ("Files", "S3"),
    ("Analytics", "BigQuery"),
];
const SYSTEMS: &[&str] = &[
    "Payment provider",
    "Maps API",
    "Email service",
    "SMS gateway",
    "Identity provider",
    "ERP",
];
const VERBS: &[&str] = &["Uses", "Reads from", "Writes to", "Sends events to", "Calls", "Queries"];

fn c4(rng: &mut Rng, size: Size, mess: Mess) -> Draft {
    let mut d = Draft::new("Platform — container diagram", mess);
    let people = size.scale(1).min(PEOPLE.len()) + 1;
    let ps: Vec<usize> = (0..people)
        .map(|i| {
            let lines = vec![PEOPLE[i].to_owned(), "[Person]".to_owned()];
            d.add(rng, "person", Shape::Rect, lines, (220.0 + i as f64 * 260.0, 90.0))
        })
        .collect();
    let apps = size.scale(3).min(APPS.len());
    let cols = 3 + usize::from(size == Size::Large);
    let cs: Vec<usize> = (0..apps)
        .map(|i| {
            let (name, tech) = APPS[i];
            let lines = vec![name.to_owned(), format!("[Container: {tech}]")];
            let at = (220.0 + (i % cols) as f64 * 260.0, 290.0 + (i / cols) as f64 * 170.0);
            d.add(rng, "container", Shape::Rect, lines, at)
        })
        .collect();
    let rows = apps.div_ceil(cols);
    let dbs = size.scale(1).min(DATABASES.len()) + 1;
    let ds: Vec<usize> = (0..dbs)
        .map(|i| {
            let (name, tech) = DATABASES[i];
            let lines = vec![name.to_owned(), format!("[Database: {tech}]")];
            d.add(
                rng,
                "database",
                Shape::Rect,
                lines,
                (220.0 + i as f64 * 260.0, 290.0 + rows as f64 * 170.0),
            )
        })
        .collect();
    let systems = size.scale(1).min(SYSTEMS.len());
    let xs: Vec<usize> = (0..systems)
        .map(|i| {
            let lines = vec![SYSTEMS[i].to_owned(), "[Software System]".to_owned()];
            let at = (220.0 + cols as f64 * 260.0 + 60.0, 290.0 + i as f64 * 170.0);
            d.add(rng, "external", Shape::Rect, lines, at)
        })
        .collect();
    d.containers.push(Container {
        id: "platform".into(),
        title: "Platform [Software System]".into(),
        members: cs.iter().chain(&ds).copied().collect(),
    });
    for (i, p) in ps.iter().enumerate() {
        d.link(*p, cs[i % 2.min(cs.len())], Some("Uses [HTTPS]".into()));
    }
    for i in 2..cs.len() {
        let from = cs[if i < cols { i % 2 } else { i - cols }];
        let label = format!("{} [JSON/HTTPS]", rng.pick(VERBS));
        d.link(from, cs[i], Some(label));
    }
    for (i, db) in ds.iter().enumerate() {
        let from = cs[(cs.len() - 1).saturating_sub(i * 2)];
        let label = rng
            .pick(&["Reads from and writes to", "Writes to", "Reads from"])
            .to_owned();
        d.link(from, *db, Some(label));
    }
    for (i, x) in xs.iter().enumerate() {
        let from = cs[(cs.len() / 2 + i) % cs.len()];
        d.link(from, *x, Some("Calls [HTTPS]".into()));
    }
    d
}

const TABLES: &[(&str, &[&str])] = &[
    ("users", &["email", "display_name", "created_at"]),
    ("orders", &["status", "total", "placed_at"]),
    ("order_items", &["quantity", "price"]),
    ("products", &["name", "sku", "price"]),
    ("categories", &["name", "slug"]),
    ("payments", &["amount", "method", "paid_at"]),
    ("shipments", &["carrier", "tracking", "shipped_at"]),
    ("addresses", &["line1", "city", "postcode"]),
    ("reviews", &["rating", "body"]),
    ("carts", &["updated_at"]),
    ("coupons", &["code", "percent"]),
    ("warehouses", &["name", "region"]),
    ("stock", &["quantity"]),
    ("suppliers", &["name", "country"]),
    ("refunds", &["amount", "reason"]),
    ("sessions", &["token", "expires_at"]),
];

fn er(rng: &mut Rng, size: Size, mess: Mess) -> Draft {
    let mut d = Draft::new("Shop — entity relationship diagram", mess);
    let count = size.scale(4).min(TABLES.len());
    let cols = (count as f64).sqrt().ceil() as usize;
    // Each table refers to one or two tables before it.
    let refs: Vec<Vec<usize>> = (0..count)
        .map(|i| match i {
            0 => vec![],
            _ if i > 2 && rng.chance(0.4) => vec![rng.below(i), rng.below(i)],
            _ => vec![rng.below(i)],
        })
        .collect();
    let rows = count.div_ceil(cols);
    // Areas take whole rows, the second a little further down.
    let split = if size == Size::Small {
        count
    } else {
        (rows.div_ceil(2) * cols).min(count)
    };
    let ts: Vec<usize> = (0..count)
        .map(|i| {
            let (name, own) = TABLES[i];
            let mut lines = vec![name.to_owned(), "id".to_owned()];
            let mut targets: Vec<usize> = refs[i].clone();
            targets.dedup();
            lines.extend(targets.iter().map(|t| format!("{}_id", singular(TABLES[*t].0))));
            lines.extend(own.iter().map(|c| (*c).to_owned()));
            let below = if i >= split { 60.0 } else { 0.0 };
            let at = (
                200.0 + (i % cols) as f64 * 300.0,
                160.0 + (i / cols) as f64 * 240.0 + below,
            );
            d.add(rng, "table", Shape::Table, lines, at)
        })
        .collect();
    for i in 0..count {
        let mut targets = refs[i].clone();
        targets.dedup();
        for (k, t) in targets.iter().enumerate() {
            d.edges.push(Edge {
                from: ts[i],
                to: ts[*t],
                label: None,
                // Row 0 is the key "id", foreign keys follow.
                ports: [Some(1 + k), Some(0)],
            });
        }
    }
    if split < count {
        d.containers.push(Container {
            id: "sales".into(),
            title: "Sales".into(),
            members: ts[..split].to_vec(),
        });
        d.containers.push(Container {
            id: "fulfilment".into(),
            title: "Fulfilment".into(),
            members: ts[split..].to_vec(),
        });
    }
    d
}

/// The singular of a table name, for its foreign key column.
fn singular(table: &str) -> String {
    if let Some(stem) = table.strip_suffix("ies") {
        format!("{stem}y")
    } else if let Some(stem) = table.strip_suffix("sses") {
        format!("{stem}ss")
    } else {
        table.strip_suffix('s').unwrap_or(table).to_owned()
    }
}

const LANES: &[&str] = &["Customer", "Sales", "Finance", "Warehouse", "Shipping", "Support"];
const STEPS: &[&str] = &[
    "Submit request",
    "Check details",
    "Prepare offer",
    "Approve",
    "Book payment",
    "Pick items",
    "Pack order",
    "Hand over",
    "Confirm delivery",
    "Close case",
    "Escalate",
    "Send reminder",
    "Archive",
];

fn flow(rng: &mut Rng, size: Size, mess: Mess) -> Draft {
    let mut d = Draft::new("Order to delivery — process flow", mess);
    let lanes = (size.scale(2) / 2 + 1).min(LANES.len());
    let steps = size.scale(4);
    let (lane_h, step_w) = (150.0, 190.0);
    let mut lane_members: Vec<Vec<usize>> = vec![Vec::new(); lanes];
    let mut lane = 0;
    let mut previous: Option<usize> = None;
    let mut decision: Option<usize> = None;
    let step_names = names(rng, STEPS, steps);
    for (i, step) in step_names.into_iter().enumerate() {
        let at = (180.0 + i as f64 * step_w, 120.0 + lane as f64 * lane_h);
        let node = if i > 0 && i < steps - 1 && rng.chance(0.25) {
            let question = format!("{}?", rng.pick(&["Valid", "In stock", "Paid", "Approved"]));
            d.add(rng, "decision", Shape::Rect, vec![question], at)
        } else {
            d.add(rng, "task", Shape::Rect, vec![step], at)
        };
        lane_members[lane].push(node);
        if let Some(previous) = previous {
            let label = (Some(previous) == decision).then(|| "yes".to_owned());
            d.link(previous, node, label);
        }
        if let Some(open) = decision.filter(|open| Some(*open) == previous) {
            // The "no" branch of the last decision loops back or skips ahead.
            let back = lane_members.iter().flatten().copied().filter(|n| *n + 1 < open).max();
            if let Some(back) = back {
                d.link(open, back, Some("no".into()));
            }
        }
        decision = (d.nodes[node].class == "decision").then_some(node);
        previous = Some(node);
        if rng.chance(0.45) {
            lane = rng.below(lanes);
        }
    }
    for (index, members) in lane_members.into_iter().enumerate() {
        d.containers.push(Container {
            id: format!("lane{}", index + 1),
            title: LANES[index].into(),
            members,
        });
    }
    d.containers.retain(|c| !c.members.is_empty());
    d
}

const LAYERS: &[(&str, &[&str])] = &[
    (
        "Clients",
        &["Browser", "iOS app", "Android app", "Partner portal", "CLI"],
    ),
    ("Edge", &["CDN", "Load balancer", "API gateway", "WAF"]),
    (
        "Services",
        &[
            "Accounts",
            "Orders",
            "Billing",
            "Search",
            "Inventory",
            "Messaging",
            "Reports",
            "Recommendations",
        ],
    ),
    (
        "Data",
        &[
            "PostgreSQL",
            "Redis",
            "Kafka",
            "Object storage",
            "Warehouse",
            "Elasticsearch",
        ],
    ),
];

fn arch(rng: &mut Rng, size: Size, mess: Mess) -> Draft {
    let mut d = Draft::new("Storefront — architecture overview", mess);
    let mut layers: Vec<Vec<usize>> = Vec::new();
    for (row, (title, pool)) in LAYERS.iter().enumerate() {
        let count = (size.scale(2) - usize::from(row == 1)).clamp(1, pool.len());
        let members: Vec<usize> = (0..count)
            .map(|i| {
                d.add(
                    rng,
                    "box",
                    Shape::Rect,
                    vec![pool[i].to_owned()],
                    (200.0 + i as f64 * 200.0, 130.0 + row as f64 * 170.0),
                )
            })
            .collect();
        d.containers.push(Container {
            id: title.to_lowercase(),
            title: (*title).to_owned(),
            members: members.clone(),
        });
        layers.push(members);
    }
    for pair in layers.windows(2) {
        let (upper, lower) = (&pair[0], &pair[1]);
        for (i, u) in upper.iter().enumerate() {
            let l = lower[(i * lower.len()) / upper.len()];
            d.link(*u, l, None);
            if rng.chance(0.3) {
                d.link(*u, lower[rng.below(lower.len())], None);
            }
        }
    }
    d
}

/// The point on `n`'s outline facing `towards`, or its centre's height on
/// the side towards it for a circle.
fn side_mid(n: &Node, towards: (f64, f64)) -> (f64, f64) {
    let (cx, cy) = n.centre();
    let (dx, dy) = (towards.0 - cx, towards.1 - cy);
    if dx.abs() >= dy.abs() {
        (if dx > 0.0 { n.x + n.w } else { n.x }, cy)
    } else {
        (cx, if dy > 0.0 { n.y + n.h } else { n.y })
    }
}

/// The point where the row of column `row` meets the side of a table
/// facing `towards`.
fn row_port(n: &Node, row: usize, towards: (f64, f64)) -> (f64, f64) {
    let y = n.y + HEAD + row as f64 * ROW + ROW / 2.0;
    (if towards.0 > n.centre().0 { n.x + n.w } else { n.x }, y)
}

/// A route as an agent draws one: mostly a straight line or one elbow
/// between facing sides, sometimes diagonal, centre to centre, or
/// straight along one axis until it stops short of the other end.
fn route(rng: &mut Rng, d: &Draft, index: usize) -> Vec<(f64, f64)> {
    let e = &d.edges[index];
    // Five lanes round the middle, taken in turn.
    let lane = (index % 5) as f64 - 2.0;
    let (a, b) = (&d.nodes[e.from], &d.nodes[e.to]);
    let (ac, bc) = (a.centre(), b.centre());
    let start = match e.ports[0] {
        Some(row) => row_port(a, row, bc),
        None => side_mid(a, bc),
    };
    let end = match e.ports[1] {
        Some(row) => row_port(b, row, ac),
        None => side_mid(b, ac),
    };
    let m = d.mess;
    let roll = rng.unit();
    if roll < m.centre {
        vec![ac, bc]
    } else if roll < m.centre + m.diagonal {
        vec![start, end]
    } else if roll < m.centre + m.diagonal + m.short {
        // Straight along the start's axis, ending wherever that lands.
        if (start.0 - ac.0).abs() > 0.0 {
            vec![start, (end.0, start.1)]
        } else {
            vec![start, (start.0, end.1)]
        }
    } else if (start.0 - end.0).abs() < 1.0 || (start.1 - end.1).abs() < 1.0 {
        vec![start, end]
    } else if (start.0 - ac.0).abs() > 0.0 {
        // Leaves sideways: across, then down or up into the target's side.
        let x = (start.0 + end.0) / 2.0 + lane * d.mess.lane_px;
        vec![start, (x, start.1), (x, end.1), end]
    } else {
        let y = (start.1 + end.1) / 2.0 + lane * d.mess.lane_px;
        vec![start, (start.0, y), (end.0, y), end]
    }
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn node_svg(out: &mut String, n: &Node, indent: &str) {
    let name = escape(&n.lines[0]);
    match n.shape {
        Shape::Circle => {
            let (cx, cy) = n.centre();
            let _ = writeln!(out, r#"{indent}<g data-node="{}" class="{}">"#, n.id, n.class);
            let _ = writeln!(out, r#"{indent}  <circle cx="{cx}" cy="{cy}" r="{}"/>"#, n.w / 2.0);
            let _ = writeln!(
                out,
                r#"{indent}  <text x="{cx}" y="{}" text-anchor="middle" font-size="{FONT}">{name}</text>"#,
                cy + 4.0
            );
            let _ = writeln!(out, "{indent}</g>");
        }
        Shape::Rect => {
            let (cx, cy) = n.centre();
            let _ = writeln!(out, r#"{indent}<g data-node="{}" class="{}">"#, n.id, n.class);
            let _ = writeln!(
                out,
                r#"{indent}  <rect x="{}" y="{}" width="{}" height="{}" rx="6"/>"#,
                n.x, n.y, n.w, n.h
            );
            let top = cy - (n.lines.len() as f64 - 1.0) * 9.0 + 4.0;
            let _ = write!(
                out,
                r#"{indent}  <text x="{cx}" y="{top}" text-anchor="middle" font-size="{FONT}">"#
            );
            for (i, line) in n.lines.iter().enumerate() {
                // A C4 tag such as [Person] is set small; a wrapped name keeps its size.
                let size = if line.starts_with('[') { 11.0 } else { FONT };
                let dy = if i == 0 { 0.0 } else { 18.0 };
                let _ = write!(
                    out,
                    r#"<tspan x="{cx}" dy="{dy}" font-size="{size}">{}</tspan>"#,
                    escape(line)
                );
            }
            let _ = writeln!(out, "</text>");
            let _ = writeln!(out, "{indent}</g>");
        }
        Shape::Table => {
            let _ = writeln!(
                out,
                r#"{indent}<g data-node="{}" class="{}" data-custom="true">"#,
                n.id, n.class
            );
            let _ = writeln!(out, r#"{indent}  <g transform="translate({} {})">"#, n.x, n.y);
            let _ = writeln!(
                out,
                r#"{indent}    <rect data-bounds="true" width="{}" height="{}"/>"#,
                n.w, n.h
            );
            let _ = writeln!(
                out,
                r#"{indent}    <rect class="head" width="{}" height="{HEAD}"/>"#,
                n.w
            );
            let _ = writeln!(
                out,
                r#"{indent}    <text x="{}" y="17" text-anchor="middle" font-size="{FONT}">{name}</text>"#,
                n.w / 2.0
            );
            for (row, column) in n.lines[1..].iter().enumerate() {
                let y = HEAD + row as f64 * ROW + ROW / 2.0;
                let _ = writeln!(
                    out,
                    r#"{indent}    <text x="10" y="{}" font-size="12">{}</text>"#,
                    y + 4.0,
                    escape(column)
                );
                for x in [0.0, n.w] {
                    let _ = writeln!(
                        out,
                        r#"{indent}    <circle data-port="{}" cx="{x}" cy="{y}" r="0"/>"#,
                        escape(column)
                    );
                }
            }
            let _ = writeln!(out, "{indent}  </g>");
            let _ = writeln!(out, "{indent}</g>");
        }
    }
}

const STYLE: &str = r##"  <style>
    text { font-family: system-ui, sans-serif; fill: #1f2937; }
    .title { font-size: 20px; font-weight: 700; }
    .group > rect { fill: #f8fafc; stroke: #94a3b8; stroke-dasharray: 6 4; }
    .group > text { font-size: 13px; font-weight: 600; fill: #475569; }
    .process circle { fill: #dbeafe; stroke: #1d4ed8; stroke-width: 1.5; }
    .entity rect, .person rect { fill: #fef3c7; stroke: #92400e; stroke-width: 1.5; }
    .store rect, .database rect { fill: #f0fdf4; stroke: #166534; }
    .container rect, .box rect, .task rect { fill: #e0f2fe; stroke: #0369a1; }
    .external rect { fill: #f1f5f9; stroke: #334155; stroke-width: 2; }
    .decision rect { fill: #fae8ff; stroke: #86198f; }
    .table rect { fill: #ffffff; stroke: #334155; }
    .table rect.head { fill: #e2e8f0; }
    .event circle { fill: #fef3c7; stroke: #92400e; stroke-width: 1.5; }
    .flow { fill: none; stroke: #475569; stroke-width: 1.4; marker-end: url(#arrow); }
    .flow-label { font-size: 11px; fill: #334155; font-style: italic; }
  </style>
  <defs>
    <marker id="arrow" viewBox="0 0 10 10" refX="10" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
      <path d="M 0 0 L 10 5 L 0 10 z" fill="#475569"/>
    </marker>
  </defs>
"##;

fn write_svg(rng: &mut Rng, d: &Draft) -> String {
    let routes: Vec<Vec<(f64, f64)>> = (0..d.edges.len()).map(|index| route(rng, d, index)).collect();
    // Containers fitted round their members, sometimes with the title
    // squeezed against the first row.
    let frames: Vec<(f64, f64, f64, f64)> = d
        .containers
        .iter()
        .map(|c| {
            let pad = if rng.chance(d.mess.tight_frame) { 6.0 } else { 24.0 };
            let left = c.members.iter().map(|m| d.nodes[*m].x).fold(f64::INFINITY, f64::min) - pad;
            let top = c.members.iter().map(|m| d.nodes[*m].y).fold(f64::INFINITY, f64::min) - pad - 18.0;
            let right = c
                .members
                .iter()
                .map(|m| d.nodes[*m].x + d.nodes[*m].w)
                .fold(f64::NEG_INFINITY, f64::max)
                + pad;
            let bottom = c
                .members
                .iter()
                .map(|m| d.nodes[*m].y + d.nodes[*m].h)
                .fold(f64::NEG_INFINITY, f64::max)
                + pad;
            (left, top, right - left, bottom - top)
        })
        .collect();
    let mut right = d.nodes.iter().map(|n| n.x + n.w).fold(0.0, f64::max);
    let mut bottom = d.nodes.iter().map(|n| n.y + n.h).fold(0.0, f64::max);
    for (x, y, w, h) in &frames {
        right = right.max(x + w);
        bottom = bottom.max(y + h);
    }
    // The agent sizes the canvas by eye, sometimes a little short.
    let (cut_w, cut_h) = if rng.chance(d.mess.cut) {
        (
            rng.int(10, d.mess.cut_px) as f64,
            rng.int(0, d.mess.cut_px * 3 / 5) as f64,
        )
    } else {
        (-40.0, -40.0)
    };
    let (w, h) = ((right - cut_w).round(), (bottom - cut_h).round());

    let mut out = String::new();
    let _ = writeln!(
        out,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}" width="{w}" height="{h}">"#
    );
    out.push_str(STYLE);
    let _ = writeln!(
        out,
        r##"  <rect x="0" y="0" width="100%" height="100%" fill="#ffffff"/>"##
    );
    let _ = writeln!(
        out,
        r#"  <text data-label="title" class="title" x="24" y="36" font-size="20">{}</text>"#,
        escape(&d.title)
    );
    let mut placed = vec![false; d.nodes.len()];
    for (c, (x, y, fw, fh)) in d.containers.iter().zip(&frames) {
        let _ = writeln!(out, r#"  <g data-node="{}" class="group">"#, c.id);
        let _ = writeln!(out, r#"    <rect x="{x}" y="{y}" width="{fw}" height="{fh}" rx="8"/>"#);
        let _ = writeln!(
            out,
            r#"    <text x="{}" y="{}">{}</text>"#,
            x + 12.0,
            y + 18.0,
            escape(&c.title)
        );
        for m in &c.members {
            node_svg(&mut out, &d.nodes[*m], "    ");
            placed[*m] = true;
        }
        let _ = writeln!(out, "  </g>");
    }
    for (n, done) in d.nodes.iter().zip(&placed) {
        if !done {
            node_svg(&mut out, n, "  ");
        }
    }
    for (node, caption) in &d.captions {
        let n = &d.nodes[*node];
        let _ = writeln!(
            out,
            r#"  <text data-label="{}-name" x="{}" y="{}" text-anchor="middle" font-size="11">{}</text>"#,
            n.id,
            n.centre().0,
            n.y + n.h + 14.0,
            escape(caption)
        );
    }
    for (i, (e, points)) in d.edges.iter().zip(&routes).enumerate() {
        let id = format!("f{}", i + 1);
        let path = points
            .iter()
            .enumerate()
            .map(|(k, (x, y))| format!("{} {x} {y}", if k == 0 { "M" } else { "L" }))
            .collect::<Vec<_>>()
            .join(" ");
        let (from, to) = (&d.nodes[e.from], &d.nodes[e.to]);
        let ports = e
            .ports
            .iter()
            .zip([from, to])
            .zip(["from", "to"])
            .filter_map(|((row, n), end)| Some(format!(r#" data-{end}-port="{}""#, escape(&n.lines[1 + (*row)?]))))
            .collect::<String>();
        let _ = writeln!(
            out,
            r#"  <path id="{id}" class="flow" data-from="{}" data-to="{}"{ports} d="{path}"/>"#,
            from.id, to.id
        );
        if let Some(label) = &e.label {
            // Beside the middle of the route, or now and then at the middle
            // of the straight line between the ends, off a bent route.
            let (lx, ly) = if rng.chance(d.mess.label_off) {
                let (a, b) = (points[0], points[points.len() - 1]);
                let drift = |rng: &mut Rng| rng.int(-d.mess.label_px, d.mess.label_px) as f64;
                ((a.0 + b.0) / 2.0 + drift(rng), (a.1 + b.1) / 2.0 + drift(rng))
            } else {
                let k = points.len() / 2;
                let (a, b) = (points[k - 1], points[k]);
                ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0)
            };
            let _ = writeln!(
                out,
                r#"  <text data-label="{id}-label" data-label-for="{id}" class="flow-label" x="{lx}" y="{}" text-anchor="middle" font-size="11">{}</text>"#,
                ly - d.mess.label_gap,
                escape(label)
            );
        }
    }
    out.push_str("</svg>\n");
    out
}

/// A diagram given as its structure only: nodes, the groups they sit in and
/// the edges between them, as a converter writes it from a dataset.
#[derive(Deserialize)]
struct Spec {
    title: String,
    /// `flow` lays the diagram out left to right, in a lane for each group;
    /// any other kind runs top to bottom, showing groups only as drawn.
    kind: String,
    nodes: Vec<SpecNode>,
    #[serde(default)]
    groups: Vec<SpecGroup>,
    edges: Vec<SpecEdge>,
}

#[derive(Deserialize)]
struct SpecNode {
    id: String,
    label: String,
    #[serde(default)]
    shape: SpecShape,
    /// A table's columns, each a port on both sides of its row.
    #[serde(default)]
    columns: Vec<String>,
    #[serde(default)]
    group: Option<String>,
    /// Where the node's centre was drawn, when the dataset says.
    #[serde(default)]
    x: Option<f64>,
    #[serde(default)]
    y: Option<f64>,
}

#[derive(Clone, Copy, Default, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
enum SpecShape {
    #[default]
    Rect,
    Circle,
    Decision,
    Table,
}

#[derive(Deserialize)]
struct SpecGroup {
    id: String,
    title: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SpecEdge {
    from: String,
    to: String,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    from_port: Option<String>,
    #[serde(default)]
    to_port: Option<String>,
}

/// Each node's layer: the longest way to it from a node nothing leads to,
/// leaving out the edges that close a cycle.
fn layers(count: usize, edges: &[(usize, usize)]) -> Vec<usize> {
    let mut out: Vec<Vec<usize>> = vec![Vec::new(); count];
    for (a, b) in edges {
        if a != b {
            out[*a].push(*b);
        }
    }
    let mut state = vec![0u8; count];
    let mut kept: Vec<Vec<usize>> = vec![Vec::new(); count];
    // A depth-first walk kept on a stack of (node, next edge to look at),
    // so a long chain cannot run out of call stack.
    for root in 0..count {
        if state[root] != 0 {
            continue;
        }
        state[root] = 1;
        let mut stack = vec![(root, 0usize)];
        while let Some((v, next)) = stack.last_mut() {
            let v = *v;
            let Some(&w) = out[v].get(*next) else {
                state[v] = 2;
                stack.pop();
                continue;
            };
            *next += 1;
            match state[w] {
                0 => {
                    kept[v].push(w);
                    state[w] = 1;
                    stack.push((w, 0));
                }
                // On the walk's own path: this edge closes a cycle.
                1 => {}
                _ => kept[v].push(w),
            }
        }
    }
    let mut layer = vec![0; count];
    for _ in 0..count {
        let mut changed = false;
        for v in 0..count {
            for &w in &kept[v] {
                if layer[w] < layer[v] + 1 {
                    layer[w] = layer[v] + 1;
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
    layer
}

/// The nodes of each layer, ordered to sit near what they join in the
/// layers beside: a sweep down and back up by mean position.
fn ordered(layer: &[usize], edges: &[(usize, usize)]) -> Vec<Vec<usize>> {
    let depth = layer.iter().max().map_or(0, |deepest| deepest + 1);
    let mut rows: Vec<Vec<usize>> = vec![Vec::new(); depth];
    for (v, l) in layer.iter().enumerate() {
        rows[*l].push(v);
    }
    let positions = |rows: &[Vec<usize>]| {
        let mut place = vec![0.0; layer.len()];
        for row in rows {
            for (i, v) in row.iter().enumerate() {
                place[*v] = i as f64;
            }
        }
        place
    };
    let sweep: Vec<usize> = (1..depth).chain((0..depth.saturating_sub(1)).rev()).collect();
    for l in sweep {
        let place = positions(&rows);
        let mean = |v: usize| {
            let near: Vec<f64> = edges
                .iter()
                .filter_map(|(a, b)| match (*a == v, *b == v) {
                    (true, false) => Some(*b),
                    (false, true) => Some(*a),
                    _ => None,
                })
                .filter(|w| layer[*w].abs_diff(l) == 1)
                .map(|w| place[w])
                .collect();
            if near.is_empty() {
                place[v]
            } else {
                near.iter().sum::<f64>() / near.len() as f64
            }
        };
        let mut keyed: Vec<(f64, usize)> = rows[l].iter().map(|v| (mean(*v), *v)).collect();
        keyed.sort_by(|a, b| a.0.total_cmp(&b.0));
        rows[l] = keyed.into_iter().map(|(_, v)| v).collect();
    }
    rows
}

/// A label broken into lines of about 22 characters, at most three.
fn wrapped(label: &str) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for word in label.split_whitespace() {
        match lines.last_mut() {
            Some(line) if line.chars().count() + word.chars().count() < 22 => {
                line.push(' ');
                line.push_str(word);
            }
            _ => lines.push(word.to_owned()),
        }
    }
    lines.truncate(3);
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

/// How many nodes of a layer go side by side when it runs top to bottom.
const PER_ROW: usize = 5;
/// How many layers of a flow go side by side before it wraps.
const WRAP: usize = 6;
/// How much farther apart drawn positions are put.
const DRAWN_SCALE: f64 = 1.6;

/// Lays a spec out in layers, the way an agent drawing it would.
fn from_spec(rng: &mut Rng, spec: &Spec, mess: Mess, layout: Option<Layout>) -> Draft {
    let mut d = Draft::new(&spec.title, mess);
    let index: BTreeMap<&str, usize> = spec.nodes.iter().enumerate().map(|(i, n)| (n.id.as_str(), i)).collect();
    let edges: Vec<(usize, usize)> = spec
        .edges
        .iter()
        .filter_map(|e| Some((*index.get(e.from.as_str())?, *index.get(e.to.as_str())?)))
        .collect();
    let layer = layers(spec.nodes.len(), &edges);
    let rows = ordered(&layer, &edges);
    let flow = spec.kind == "flow";
    let drawn = spec.nodes.iter().all(|n| n.x.is_some() && n.y.is_some());
    let layout = match layout {
        Some(Layout::Drawn) | None if drawn => Layout::Drawn,
        Some(Layout::Layered) => Layout::Layered,
        _ => Layout::Wrapped,
    };
    let left = spec.nodes.iter().filter_map(|n| n.x).fold(f64::INFINITY, f64::min);
    let top_drawn = spec.nodes.iter().filter_map(|n| n.y).fold(f64::INFINITY, f64::min);
    let wrap = layout == Layout::Wrapped && flow && rows.len() > WRAP;
    // Nodes in no group share a last lane.
    let lane_of = |n: &SpecNode| {
        n.group
            .as_ref()
            .and_then(|g| spec.groups.iter().position(|group| &group.id == g))
            .unwrap_or(spec.groups.len())
    };
    let lanes = spec.groups.len() + 1;
    // Each lane is as tall as its most crowded layer.
    let mut lane_rows = vec![1usize; lanes];
    for row in &rows {
        let mut counts = vec![0usize; lanes];
        for v in row {
            counts[lane_of(&spec.nodes[*v])] += 1;
        }
        for (most, count) in lane_rows.iter_mut().zip(counts) {
            *most = (*most).max(count);
        }
    }
    let mut lane_top = Vec::with_capacity(lanes);
    let mut top = 120.0;
    for rows in &lane_rows {
        lane_top.push(top);
        top += *rows as f64 * 120.0 + 50.0;
    }
    // A wrapped flow repeats its lanes on each row of layers.
    let band = top - 120.0 + 60.0;
    let mut block_of = vec![0usize; spec.nodes.len()];
    let mut row_top = 140.0;
    let mut placed: Vec<Option<usize>> = vec![None; spec.nodes.len()];
    for (l, row) in rows.iter().enumerate() {
        let mut in_lane = vec![0usize; lanes];
        let tallest = row
            .iter()
            .map(|v| HEAD + spec.nodes[*v].columns.len().max(1) as f64 * ROW)
            .fold(60.0, f64::max);
        for (i, v) in row.iter().enumerate() {
            let n = &spec.nodes[*v];
            let at = if layout == Layout::Drawn {
                // Spread out: the agent's boxes, sized to their text, are
                // larger than the modeller's.
                (
                    (n.x.unwrap_or(left) - left) * DRAWN_SCALE + 160.0,
                    (n.y.unwrap_or(top_drawn) - top_drawn) * DRAWN_SCALE + 140.0,
                )
            } else if flow {
                let lane = lane_of(n);
                let sub = in_lane[lane];
                in_lane[lane] += 1;
                let (block, column) = if wrap {
                    let column = l % WRAP;
                    // Odd rows run back the other way, as a hand would.
                    (l / WRAP, if (l / WRAP) % 2 == 1 { WRAP - 1 - column } else { column })
                } else {
                    (0, l)
                };
                block_of[*v] = block;
                (
                    160.0 + column as f64 * 210.0,
                    lane_top[lane] + 50.0 + sub as f64 * 120.0 + block as f64 * band,
                )
            } else {
                // A wide layer wraps onto the rows below it.
                let width = if n.shape == SpecShape::Table { 300.0 } else { 230.0 };
                let below = (i / PER_ROW) as f64 * (tallest + 80.0);
                (160.0 + (i % PER_ROW) as f64 * width, row_top + below + tallest / 2.0)
            };
            let (class, shape, lines) = match n.shape {
                SpecShape::Table => {
                    let lines = std::iter::once(n.label.clone())
                        .chain(n.columns.iter().cloned())
                        .collect();
                    ("table", Shape::Table, lines)
                }
                SpecShape::Circle => ("event", Shape::Circle, vec![String::new()]),
                SpecShape::Decision => {
                    let label = if n.label.is_empty() { "?" } else { n.label.as_str() };
                    ("decision", Shape::Rect, wrapped(label))
                }
                SpecShape::Rect => (if flow { "task" } else { "box" }, Shape::Rect, wrapped(&n.label)),
            };
            let node = d.add(rng, class, shape, lines, at);
            if n.shape == SpecShape::Circle && !n.label.is_empty() {
                d.captions.push((node, n.label.chars().take(30).collect()));
            }
            placed[*v] = Some(node);
        }
        let wraps = if flow { 1 } else { row.len().div_ceil(PER_ROW).max(1) };
        row_top += wraps as f64 * (tallest + 80.0);
    }
    let column = |node: usize, port: &Option<String>| {
        let port = port.as_ref()?;
        spec.nodes[node].columns.iter().position(|c| c == port)
    };
    for e in &spec.edges {
        let (Some(&a), Some(&b)) = (index.get(e.from.as_str()), index.get(e.to.as_str())) else {
            continue;
        };
        let (Some(from), Some(to)) = (placed[a], placed[b]) else {
            continue;
        };
        let ports = [column(a, &e.from_port), column(b, &e.to_port)];
        let label = e.label.as_ref().map(|l| l.chars().take(30).collect());
        // Two foreign keys between the same tables, or a table referring
        // to itself, are edges of their own; only an exact repeat is not.
        let repeat = d
            .edges
            .iter()
            .any(|x| x.from == from && x.to == to && x.ports == ports && x.label == label);
        if !repeat {
            d.edges.push(Edge { from, to, label, ports });
        }
    }
    // Lanes, or the boxes a modeller drew round nodes where they drew them.
    if flow || layout == Layout::Drawn {
        let blocks = block_of.iter().max().map_or(1, |last| last + 1);
        for (g, group) in spec.groups.iter().enumerate() {
            for block in 0..blocks {
                let members: Vec<usize> = spec
                    .nodes
                    .iter()
                    .enumerate()
                    .filter(|(v, n)| lane_of(n) == g && block_of[*v] == block)
                    .filter_map(|(v, _)| placed[v])
                    .collect();
                if !members.is_empty() {
                    d.containers.push(Container {
                        id: format!("g{}-{}", g + 1, block + 1),
                        title: group.title.chars().take(40).collect(),
                        members,
                    });
                }
            }
        }
    }
    d
}

/// A spec drafted at `level`: laid out, then given the level's slips.
fn generate_from(spec: &Spec, name: &str, level: Level, seed: u64, layout: Option<Layout>) -> String {
    // The name keeps each spec's drafts apart for the same seed.
    let salt = name.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x100_0000_01b3)
    });
    let mut rng = Rng::new(seed ^ salt ^ ((level as u64) << 56));
    let mut draft = from_spec(&mut rng, spec, level.mess(), layout);
    draft.spread();
    draft.crowd(&mut rng);
    write_svg(&mut rng, &draft)
}

fn generate(kind: Kind, size: Size, level: Level, seed: u64) -> String {
    let mut rng = Rng::new(seed ^ ((kind as u64) << 40) ^ ((size as u64) << 48) ^ ((level as u64) << 56));
    let mess = level.mess();
    let mut draft = match kind {
        Kind::Dfd => dfd(&mut rng, size, mess),
        Kind::C4 => c4(&mut rng, size, mess),
        Kind::Er => er(&mut rng, size, mess),
        Kind::Flow => flow(&mut rng, size, mess),
        Kind::Arch => arch(&mut rng, size, mess),
    };
    draft.spread();
    draft.crowd(&mut rng);
    write_svg(&mut rng, &draft)
}

fn main() {
    let args = Args::parse();
    std::fs::create_dir_all(&args.out).expect("the output directory can be made");
    let kinds = args
        .kind
        .map_or_else(|| Kind::value_variants().to_vec(), |kind| vec![kind]);
    let levels = args
        .mess
        .map_or_else(|| Level::value_variants().to_vec(), |level| vec![level]);
    let sizes = args
        .size
        .map_or_else(|| Size::value_variants().to_vec(), |size| vec![size]);
    if let Some(spec) = &args.spec {
        let mut files: Vec<PathBuf> = if spec.is_dir() {
            std::fs::read_dir(spec)
                .expect("the spec directory is readable")
                .filter_map(|entry| Some(entry.ok()?.path()))
                .filter(|path| path.extension().is_some_and(|e| e == "json"))
                .collect()
        } else {
            vec![spec.clone()]
        };
        files.sort();
        for file in files {
            let stem = file
                .file_stem()
                .map_or_else(String::new, |s| s.to_string_lossy().into_owned());
            let text = std::fs::read_to_string(&file).expect("the spec is readable");
            let parsed: Spec =
                serde_json::from_str(&text).unwrap_or_else(|error| panic!("{}: {error}", file.display()));
            for level in &levels {
                for seed in args.seed..args.seed + args.count {
                    let name = format!("{stem}-{level:?}-{seed}.svg").to_lowercase();
                    write_checked(
                        &args.out,
                        &name,
                        &generate_from(&parsed, &stem, *level, seed, args.layout),
                    );
                }
            }
        }
        return;
    }
    for kind in &kinds {
        for size in &sizes {
            for (level, seed) in levels
                .iter()
                .flat_map(|level| (args.seed..args.seed + args.count).map(move |seed| (*level, seed)))
            {
                let name = format!("{kind:?}-{size:?}-{level:?}-{seed}.svg").to_lowercase();
                write_checked(&args.out, &name, &generate(*kind, *size, level, seed));
            }
        }
    }
}

/// Writes a draft, reads it back with nicevg, which it must be able to,
/// and prints its issues by code.
fn write_checked(out: &std::path::Path, name: &str, svg: &str) {
    std::fs::write(out.join(name), svg).expect("the draft can be written");
    let report = nicevg::analyze(svg).unwrap_or_else(|error| panic!("{name}: {error}"));
    let mut codes: BTreeMap<&str, usize> = BTreeMap::new();
    for issue in &report.issues {
        *codes.entry(issue.code.as_str()).or_default() += 1;
    }
    println!(
        "{name}: {} nodes, {} connectors, {} issues {codes:?}",
        report.diagram.nodes.len(),
        report.diagram.connectors.len(),
        report.issues.len()
    );
}
