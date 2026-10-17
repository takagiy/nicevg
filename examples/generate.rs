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
//! `--grow` grows each kind from a few nodes by rewriting a part at a time
//! (a step split, a check that branches, a store, a junction table, a
//! service in a boundary of its own), instead of drawing it from one fixed
//! plan; their files are `<kind>-<size>-<mess>-grown-<seed>.svg`.
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
    /// Grow diagrams from a few nodes by rewriting a part at a time,
    /// instead of drawing them from a fixed plan; laid out as drawn specs.
    #[arg(long)]
    grow: bool,
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
    /// The container it sits in, if any, by its place in the draft's.
    parent: Option<usize>,
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
        parent: None,
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
            parent: None,
        });
        d.containers.push(Container {
            id: "fulfilment".into(),
            title: "Fulfilment".into(),
            members: ts[split..].to_vec(),
            parent: None,
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
    } else if let Some(stem) = table.strip_suffix("uses").filter(|stem| !stem.ends_with('o')) {
        // statuses, not warehouses
        format!("{stem}us")
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
            parent: None,
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
            parent: None,
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

/// A container with its nodes, and the containers in it inside it.
fn container_svg(
    out: &mut String,
    d: &Draft,
    frames: &[(f64, f64, f64, f64)],
    c: usize,
    indent: &str,
    placed: &mut [bool],
) {
    let (x, y, w, h) = frames[c];
    let _ = writeln!(out, r#"{indent}<g data-node="{}" class="group">"#, d.containers[c].id);
    let _ = writeln!(
        out,
        r#"{indent}  <rect x="{x}" y="{y}" width="{w}" height="{h}" rx="8"/>"#
    );
    let _ = writeln!(
        out,
        r#"{indent}  <text x="{}" y="{}">{}</text>"#,
        x + 12.0,
        y + 18.0,
        escape(&d.containers[c].title)
    );
    let inner = format!("{indent}  ");
    for m in &d.containers[c].members {
        node_svg(out, &d.nodes[*m], &inner);
        placed[*m] = true;
    }
    for nested in (0..d.containers.len()).filter(|n| d.containers[*n].parent == Some(c)) {
        container_svg(out, d, frames, nested, &inner, placed);
    }
    let _ = writeln!(out, "{indent}</g>");
}

fn write_svg(rng: &mut Rng, d: &Draft) -> String {
    let routes: Vec<Vec<(f64, f64)>> = (0..d.edges.len()).map(|index| route(rng, d, index)).collect();
    // Containers fitted round their members, sometimes with the title
    // squeezed against the first row.
    let pads: Vec<f64> = d
        .containers
        .iter()
        .map(|_| if rng.chance(d.mess.tight_frame) { 6.0 } else { 24.0 })
        .collect();
    // Inner containers first: an outer one is fitted round them as well.
    let depth = |mut c: usize| {
        let mut depth = 0;
        while let Some(parent) = d.containers[c].parent {
            (c, depth) = (parent, depth + 1);
        }
        depth
    };
    let mut order: Vec<usize> = (0..d.containers.len()).collect();
    order.sort_by_key(|c| std::cmp::Reverse(depth(*c)));
    let mut frames = vec![(0.0, 0.0, 0.0, 0.0); d.containers.len()];
    for c in order {
        let members = d.containers[c].members.iter().map(|m| {
            let n = &d.nodes[*m];
            (n.x, n.y, n.x + n.w, n.y + n.h)
        });
        let inner = (0..d.containers.len())
            .filter(|inner| d.containers[*inner].parent == Some(c))
            .map(|inner| {
                let (x, y, w, h) = frames[inner];
                (x, y, x + w, y + h)
            });
        let (mut left, mut top, mut right, mut bottom) =
            (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
        for (l, t, r, b) in members.chain(inner) {
            (left, top, right, bottom) = (left.min(l), top.min(t), right.max(r), bottom.max(b));
        }
        let (left, top) = (left - pads[c], top - pads[c] - 18.0);
        frames[c] = (left, top, right + pads[c] - left, bottom + pads[c] - top);
    }
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
    for c in (0..d.containers.len()).filter(|c| d.containers[*c].parent.is_none()) {
        container_svg(&mut out, d, &frames, c, "  ", &mut placed);
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
    /// An event: a small circle with its name written under it.
    Circle,
    /// A data flow diagram's process: a circle with its name inside.
    Process,
    Decision,
    Table,
}

#[derive(Deserialize)]
struct SpecGroup {
    id: String,
    title: String,
    /// The group it sits in, when groups nest; drawn only where nodes are
    /// laid out as drawn.
    #[serde(default)]
    parent: Option<String>,
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

/// The group the group `id` sits in, if any.
fn parent_of(groups: &[SpecGroup], id: &str) -> Option<String> {
    groups.iter().find(|g| g.id == id).and_then(|g| g.parent.clone())
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

/// Layers running top to bottom, a few small ones after another sharing a
/// row, so a long chain does not stand as one tall column.
fn packed(rows: Vec<Vec<usize>>) -> Vec<Vec<usize>> {
    let mut out: Vec<Vec<usize>> = Vec::new();
    for row in rows {
        match out.last_mut() {
            Some(last) if last.len() + row.len() <= PER_ROW => last.extend(row),
            _ => out.push(row),
        }
    }
    out
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
    // Groups nest: following parents must come out of the groups, or
    // every walk up them below would go round for ever.
    for group in &spec.groups {
        let mut at = group.parent.clone();
        for _ in 0..=spec.groups.len() {
            at = at.and_then(|id| parent_of(&spec.groups, &id));
        }
        assert!(at.is_none(), "{}: group parents loop", group.id);
    }
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
    let rows = if !flow && layout == Layout::Wrapped {
        packed(rows)
    } else {
        rows
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
    // A wrapped flow lays its layers out in blocks, one under another.
    let block_of_layer = |l: usize| if wrap { l / WRAP } else { 0 };
    let blocks = block_of_layer(rows.len().saturating_sub(1)) + 1;
    // In each block, each lane is as tall as its most crowded layer there;
    // a lane with nothing in a block takes no room in it.
    let mut lane_rows = vec![vec![0usize; lanes]; blocks];
    for (l, row) in rows.iter().enumerate() {
        let mut counts = vec![0usize; lanes];
        for v in row {
            counts[lane_of(&spec.nodes[*v])] += 1;
        }
        for (most, count) in lane_rows[block_of_layer(l)].iter_mut().zip(counts) {
            *most = (*most).max(count);
        }
    }
    let mut lane_top = vec![vec![0.0; lanes]; blocks];
    let mut top = 120.0;
    for (tops, rows) in lane_top.iter_mut().zip(&lane_rows) {
        for (lane_top, rows) in tops.iter_mut().zip(rows) {
            *lane_top = top;
            if *rows > 0 {
                top += *rows as f64 * 120.0 + 50.0;
            }
        }
        top += 60.0;
    }
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
                    lane_top[block][lane] + 50.0 + sub as f64 * 120.0,
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
                SpecShape::Process => ("process", Shape::Circle, wrapped(&n.label)),
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
        // A group holds a node when the node sits in it or in a group in it.
        let holds = |g: usize, n: &SpecNode| {
            let mut at = n.group.clone();
            while let Some(id) = at {
                if spec.groups[g].id == id {
                    return true;
                }
                at = parent_of(&spec.groups, &id);
            }
            false
        };
        let mut container_of: BTreeMap<(usize, usize), usize> = BTreeMap::new();
        for (g, group) in spec.groups.iter().enumerate() {
            for block in 0..blocks {
                let members: Vec<usize> = spec
                    .nodes
                    .iter()
                    .enumerate()
                    .filter(|(v, n)| lane_of(n) == g && block_of[*v] == block)
                    .filter_map(|(v, _)| placed[v])
                    .collect();
                let nested = block == 0 && spec.nodes.iter().any(|n| lane_of(n) != g && holds(g, n));
                if !members.is_empty() || nested {
                    container_of.insert((g, block), d.containers.len());
                    d.containers.push(Container {
                        id: format!("g{}-{}", g + 1, block + 1),
                        title: group.title.chars().take(40).collect(),
                        members,
                        parent: None,
                    });
                }
            }
        }
        for ((g, block), c) in &container_of {
            let parent = spec.groups[*g]
                .parent
                .as_ref()
                .and_then(|p| spec.groups.iter().position(|x| &x.id == p));
            d.containers[*c].parent = parent.and_then(|p| container_of.get(&(p, *block)).copied());
        }
    }
    d
}

/// A diagram grown from a few nodes by rewriting a part of it at a time,
/// the way a draft gains detail: a step split in two, a branch where a
/// check can fail, a store a process writes to. Each rewrite goes more
/// likely to a simple part (few connections), so detail spreads over the
/// diagram; now and then to any part, so hubs grow as well.
struct Grower {
    spec: Spec,
    /// The manner of the hand growing it.
    hand: Hand,
    /// Sizes given to nodes that stand for something else than their
    /// label: a group laid out as one box.
    sizes: BTreeMap<String, (f64, f64)>,
}

impl Grower {
    fn new(title: &str, kind: &str) -> Grower {
        Grower {
            spec: Spec {
                title: title.to_owned(),
                kind: kind.to_owned(),
                nodes: Vec::new(),
                groups: Vec::new(),
                edges: Vec::new(),
            },
            sizes: BTreeMap::new(),
            hand: Hand::PLAIN,
        }
    }

    fn group(&mut self, title: &str, parent: Option<String>) -> String {
        let id = format!("g{}", self.spec.groups.len() + 1);
        self.spec.groups.push(SpecGroup {
            id: id.clone(),
            title: title.to_owned(),
            parent,
        });
        id
    }

    fn node(&mut self, label: String, shape: SpecShape, group: Option<String>, columns: Vec<String>) -> String {
        let id = format!("n{}", self.spec.nodes.len() + 1);
        self.spec.nodes.push(SpecNode {
            id: id.clone(),
            label,
            shape,
            columns,
            group,
            x: None,
            y: None,
        });
        id
    }

    /// How big a node comes out: the size it was given, or as
    /// `Draft::add` sizes it.
    fn size_of(&self, n: &SpecNode) -> (f64, f64) {
        if let Some(size) = self.sizes.get(&n.id) {
            return *size;
        }
        let lines = match n.shape {
            SpecShape::Circle => vec![String::new()],
            SpecShape::Table => std::iter::once(n.label.clone())
                .chain(n.columns.iter().cloned())
                .collect(),
            _ => wrapped(&n.label),
        };
        let widest = lines.iter().map(|l| l.chars().count()).max().unwrap_or(1) as f64;
        match n.shape {
            SpecShape::Circle | SpecShape::Process => {
                let d = (widest * FONT * 0.62 + 24.0).max(64.0);
                (d, d)
            }
            SpecShape::Table => (
                (widest * 12.0 * 0.62 + 70.0).max(140.0),
                HEAD + (lines.len() - 1) as f64 * ROW,
            ),
            _ => (
                (widest * FONT * 0.62 + 32.0).max(96.0),
                (lines.len() as f64 * 18.0 + 26.0).max(48.0),
            ),
        }
    }

    fn at(&self, id: &str) -> Option<(f64, f64)> {
        let n = self.get(id);
        Some((n.x?, n.y?))
    }

    fn put(&mut self, id: &str, at: (f64, f64)) {
        let n = self.get_mut(id);
        (n.x, n.y) = (Some(at.0), Some(at.1));
    }

    /// Whether a node of `size` centred at `at` keeps `margin` clear of
    /// every placed node but `id` itself.
    fn free(&self, id: &str, at: (f64, f64), size: (f64, f64), margin: f64) -> bool {
        self.spec.nodes.iter().filter(|n| n.id != id).all(|n| {
            let (Some(x), Some(y)) = (n.x, n.y) else { return true };
            let (w, h) = self.size_of(n);
            (x - at.0).abs() >= (w + size.0) / 2.0 + margin || (y - at.1).abs() >= (h + size.1) / 2.0 + margin
        })
    }

    /// Puts `id` beside `anchor`: the first free place in the directions
    /// given, in order, each as far out as the two nodes' halves and a gap,
    /// keeping the anchor's line; then a step farther out, and so on.
    fn beside(&mut self, rng: &mut Rng, id: &str, anchor: &str, directions: &[(f64, f64)]) {
        let size = self.size_of(self.get(id));
        let (from, (aw, ah)) = (self.at(anchor).unwrap_or((0.0, 0.0)), self.size_of(self.get(anchor)));
        // How roomy this hand is with this node.
        // Never under the clearance `free` asks, or the first ring is lost.
        let gap = ([50.0, 70.0, 90.0, 130.0][rng.below(4)] * self.hand.room).max(32.0);
        for farther in 0..16 {
            let extra = gap + farther as f64 * (60.0 * self.hand.room).max(20.0);
            let free: Vec<(f64, f64)> = directions
                .iter()
                .map(|(dx, dy)| {
                    (
                        from.0 + dx * ((aw + size.0) / 2.0 + extra),
                        from.1 + dy * ((ah + size.1) / 2.0 + extra),
                    )
                })
                .filter(|at| self.free(id, *at, size, 30.0))
                .collect();
            // The first free place that keeps the diagram from growing
            // into a long strip, else the first free place.
            if let Some(at) = free.iter().find(|at| self.keeps_shape(**at, size)).or(free.first()) {
                self.put(id, *at);
                return;
            }
        }
        // Nowhere near: past everything on the right.
        let right = self
            .spec
            .nodes
            .iter()
            .filter_map(|n| Some(n.x? + self.size_of(n).0 / 2.0))
            .fold(0.0, f64::max);
        self.put(id, (right + gap + size.0 / 2.0, from.1));
    }

    /// Whether the placed nodes and one of `size` at `at` together stay
    /// between half as wide as tall and twice as wide.
    fn keeps_shape(&self, at: (f64, f64), size: (f64, f64)) -> bool {
        (0.5..=2.0).contains(&self.spread_ratio(Some((at, size))))
    }

    /// Puts `id` between `a` and `b` if there is room there, else off to
    /// one side of the line between them, else beside `a`.
    fn between(&mut self, rng: &mut Rng, id: &str, a: &str, b: &str, ahead: (f64, f64)) {
        let size = self.size_of(self.get(id));
        if let (Some(p), Some(q)) = (self.at(a), self.at(b)) {
            let mid = ((p.0 + q.0) / 2.0, (p.1 + q.1) / 2.0);
            let side = (ahead.1, ahead.0);
            for k in [0.0, 1.0, -1.0, 2.0, -2.0] {
                let at = (
                    mid.0 + side.0 * k * (size.0 + 60.0),
                    mid.1 + side.1 * k * (size.1 + 60.0),
                );
                if self.free(id, at, size, 30.0) {
                    self.put(id, at);
                    return;
                }
            }
        }
        let side = (ahead.1, ahead.0);
        self.beside(rng, id, a, &[ahead, side, (-side.0, -side.1)]);
    }

    /// The one of the eight compass directions closest to the way from
    /// `id` to the middle of `others`, if they are placed.
    fn toward(&self, id: &str, others: &[String]) -> Option<(f64, f64)> {
        let here = self.at(id)?;
        let placed: Vec<(f64, f64)> = others.iter().filter_map(|o| self.at(o)).collect();
        if placed.is_empty() {
            return None;
        }
        let count = placed.len() as f64;
        let (dx, dy) = (
            placed.iter().map(|p| p.0).sum::<f64>() / count - here.0,
            placed.iter().map(|p| p.1).sum::<f64>() / count - here.1,
        );
        let angle = (dy.atan2(dx) / std::f64::consts::FRAC_PI_4).round() * std::f64::consts::FRAC_PI_4;
        Some((angle.cos().round(), angle.sin().round()))
    }

    /// Where a flow's lane band starts: lanes lie one under another.
    fn lane_top(&self, group: &Option<String>) -> f64 {
        let index = group
            .as_ref()
            .and_then(|g| self.spec.groups.iter().position(|x| &x.id == g))
            .unwrap_or(0);
        index as f64 * (LANE_ROWS as f64 * ROW_GAP + 80.0)
    }

    /// Which row of its lane `id` sits in.
    fn row_of(&self, id: &str) -> usize {
        let n = self.get(id);
        let top = self.lane_top(&n.group);
        n.y.map_or(0, |y| {
            ((y - top - 60.0) / ROW_GAP).round().clamp(0.0, (LANE_ROWS - 1) as f64) as usize
        })
    }

    /// Where a node of `id` goes just after `a` along a flow, with `gap`.
    fn after(&self, a: &str, id: &str, gap: f64) -> f64 {
        let (aw, w) = (self.size_of(self.get(a)).0, self.size_of(self.get(id)).0);
        self.at(a).map_or(0.0, |p| p.0) + (aw + w) / 2.0 + gap
    }

    /// Moves every placed node whose centre is right of `x` along by `by`.
    fn make_room(&mut self, x: f64, by: f64) {
        for n in &mut self.spec.nodes {
            if let Some(nx) = n.x.filter(|nx| *nx >= x) {
                n.x = Some(nx + by);
            }
        }
    }

    /// Puts `id` in its lane at `x`: in `row` if free there, else in
    /// another row of the lane, else in `row` once the nodes from there
    /// on are moved along to make room, as a hand inserts a step.
    fn in_lane(&mut self, id: &str, x: f64, row: usize) {
        let size = self.size_of(self.get(id));
        let top = self.lane_top(&self.get(id).group);
        let y = |r: usize| top + 60.0 + r as f64 * ROW_GAP;
        let mut rows: Vec<usize> = (0..LANE_ROWS).collect();
        rows.sort_by_key(|r| r.abs_diff(row));
        for r in rows {
            if self.free(id, (x, y(r)), size, 30.0) {
                self.put(id, (x, y(r)));
                return;
            }
        }
        self.make_room(x - size.0 / 2.0 - 20.0, size.0 + 70.0);
        self.put(id, (x, y(row)));
    }

    /// Puts `id` at `x` in `row` of its lane, moving the nodes from there
    /// on along first if they are in the way: a step put between two.
    fn in_row(&mut self, id: &str, x: f64, row: usize) {
        let size = self.size_of(self.get(id));
        let y = self.lane_top(&self.get(id).group) + 60.0 + row as f64 * ROW_GAP;
        if !self.free(id, (x, y), size, 30.0) {
            self.make_room(x - size.0 / 2.0 - 20.0, size.0 + 70.0);
        }
        self.put(id, (x, y));
    }

    /// Moves each node of a flow right of the nodes that lead to it (but
    /// for the edges that close a loop), so the flow reads left to right;
    /// then moves along any node that now crowds the one before it in its
    /// row.
    fn forward(&mut self) {
        let index: BTreeMap<String, usize> = self
            .spec
            .nodes
            .iter()
            .enumerate()
            .map(|(i, n)| (n.id.clone(), i))
            .collect();
        let edges: Vec<(usize, usize)> = self.spec.edges.iter().map(|e| (index[&e.from], index[&e.to])).collect();
        let layer = layers(self.spec.nodes.len(), &edges);
        let mut order: Vec<usize> = (0..self.spec.nodes.len()).collect();
        order.sort_by_key(|v| layer[*v]);
        let size: Vec<(f64, f64)> = self.spec.nodes.iter().map(|n| self.size_of(n)).collect();
        for v in order {
            let least = edges
                .iter()
                .filter(|(a, b)| *b == v && layer[*a] < layer[v])
                .filter_map(|(a, _)| Some(self.spec.nodes[*a].x? + (size[*a].0 + size[v].0) / 2.0 + 40.0))
                .fold(f64::MIN, f64::max);
            let n = &mut self.spec.nodes[v];
            n.x = n.x.map(|x| x.max(least));
        }
        // Then, row by row, each node keeps clear of the one before it.
        let mut rows: BTreeMap<i64, Vec<usize>> = BTreeMap::new();
        for (v, n) in self.spec.nodes.iter().enumerate() {
            if let (Some(_), Some(y)) = (n.x, n.y) {
                rows.entry(y.round() as i64).or_default().push(v);
            }
        }
        for row in rows.values_mut() {
            row.sort_by(|a, b| {
                self.spec.nodes[*a]
                    .x
                    .unwrap_or(0.0)
                    .total_cmp(&self.spec.nodes[*b].x.unwrap_or(0.0))
            });
            for pair in row.windows(2) {
                let (a, b) = (pair[0], pair[1]);
                let least = self.spec.nodes[a].x.unwrap_or(0.0) + (size[a].0 + size[b].0) / 2.0 + 30.0;
                let n = &mut self.spec.nodes[b];
                n.x = n.x.map(|x| x.max(least));
            }
        }
    }

    /// How many times wider than tall the placed nodes spread, with one
    /// more of a size at a place when given.
    fn spread_ratio(&self, extra: Option<((f64, f64), (f64, f64))>) -> f64 {
        let boxes = self
            .spec
            .nodes
            .iter()
            .filter_map(|n| Some(((n.x?, n.y?), self.size_of(n))))
            .chain(extra);
        let (mut left, mut top, mut right, mut bottom) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
        for ((x, y), (w, h)) in boxes {
            (left, right) = (left.min(x - w / 2.0), right.max(x + w / 2.0));
            (top, bottom) = (top.min(y - h / 2.0), bottom.max(y + h / 2.0));
        }
        if right < left {
            1.0
        } else {
            (right - left) / (bottom - top).max(1.0)
        }
    }

    /// The directions from `id` that lead away from the middle of the
    /// diagram, most outward first.
    fn outward(&self, id: &str) -> Vec<(f64, f64)> {
        let placed: Vec<(f64, f64)> = self.spec.nodes.iter().filter_map(|n| Some((n.x?, n.y?))).collect();
        let count = placed.len().max(1) as f64;
        let middle = (
            placed.iter().map(|p| p.0).sum::<f64>() / count,
            placed.iter().map(|p| p.1).sum::<f64>() / count,
        );
        let here = self.at(id).unwrap_or(middle);
        let away = (here.0 - middle.0, here.1 - middle.1);
        let mut directions = vec![(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)];
        directions.sort_by(|a, b| (b.0 * away.0 + b.1 * away.1).total_cmp(&(a.0 * away.0 + a.1 * away.1)));
        directions
    }

    fn get(&self, id: &str) -> &SpecNode {
        self.spec
            .nodes
            .iter()
            .find(|n| n.id == id)
            .expect("a grown edge joins grown nodes")
    }

    fn get_mut(&mut self, id: &str) -> &mut SpecNode {
        self.spec
            .nodes
            .iter_mut()
            .find(|n| n.id == id)
            .expect("a grown edge joins grown nodes")
    }

    fn edge(&mut self, from: &str, to: &str, label: Option<String>) {
        self.edge_ports(from, to, label, None, None);
    }

    fn edge_ports(
        &mut self,
        from: &str,
        to: &str,
        label: Option<String>,
        from_port: Option<String>,
        to_port: Option<String>,
    ) {
        self.spec.edges.push(SpecEdge {
            from: from.to_owned(),
            to: to.to_owned(),
            label,
            from_port,
            to_port,
        });
    }

    fn joined(&self, a: &str, b: &str) -> bool {
        self.spec
            .edges
            .iter()
            .any(|e| (e.from == a && e.to == b) || (e.from == b && e.to == a))
    }

    fn degree(&self, id: &str) -> usize {
        self.spec.edges.iter().filter(|e| e.from == id || e.to == id).count()
    }

    /// A node two or three connections away from `from` (two likelier),
    /// not joined to it yet, that `keep` lets through, simple ones
    /// likelier: joining it closes a small loop.
    fn close_by(&self, rng: &mut Rng, from: &str, keep: impl Fn(&SpecNode) -> bool) -> Option<String> {
        let mut hops: BTreeMap<&str, usize> = BTreeMap::from([(from, 0)]);
        let mut frontier = vec![from];
        for hop in 1..=3 {
            let mut next = Vec::new();
            for at in frontier {
                for e in &self.spec.edges {
                    let other = if e.from == at {
                        e.to.as_str()
                    } else if e.to == at {
                        e.from.as_str()
                    } else {
                        continue;
                    };
                    if !hops.contains_key(other) {
                        hops.insert(other, hop);
                        next.push(other);
                    }
                }
            }
            frontier = next;
        }
        let candidates: Vec<(String, f64)> = hops
            .iter()
            .filter(|(id, hop)| **hop >= 2 && !self.joined(from, id) && keep(self.get(id)))
            .map(|(id, hop)| {
                let degree = self.degree(id) as f64 + 1.0;
                (
                    id.to_string(),
                    if *hop == 2 { self.hand.reach } else { 1.0 } / (degree * degree),
                )
            })
            .collect();
        if candidates.is_empty() {
            return None;
        }
        let weights: Vec<f64> = candidates.iter().map(|(_, w)| *w).collect();
        let at = weighted(rng, &weights);
        candidates.into_iter().nth(at).map(|(id, _)| id)
    }

    /// One of the nodes `keep` lets through, simple ones likelier.
    fn simple_node(&self, rng: &mut Rng, keep: impl Fn(&SpecNode) -> bool) -> Option<String> {
        let candidates: Vec<(String, usize)> = self
            .spec
            .nodes
            .iter()
            .filter(|n| keep(n))
            .map(|n| (n.id.clone(), self.degree(&n.id)))
            .collect();
        simple(rng, candidates, &self.hand)
    }

    /// One of the nodes `keep` lets through, simple ones and ones near
    /// `near` likelier: a hand joins what lies close by.
    fn nearby_node(&self, rng: &mut Rng, near: &str, keep: impl Fn(&SpecNode) -> bool) -> Option<String> {
        let here = self.at(near)?;
        let candidates: Vec<(String, f64)> = self
            .spec
            .nodes
            .iter()
            .filter(|n| n.id != near && keep(n))
            .filter_map(|n| {
                let d = ((n.x? - here.0).powi(2) + (n.y? - here.1).powi(2)).sqrt() / 250.0;
                let degree = self.degree(&n.id) as f64 + 1.0;
                Some((n.id.clone(), 1.0 / (degree * degree * (1.0 + d * d).powi(2))))
            })
            .collect();
        if candidates.is_empty() {
            return None;
        }
        let weights: Vec<f64> = candidates.iter().map(|(_, w)| *w).collect();
        let at = weighted(rng, &weights);
        candidates.into_iter().nth(at).map(|(id, _)| id)
    }

    /// The place of one of the edges `keep` lets through, those between
    /// simple nodes likelier.
    fn simple_edge(&self, rng: &mut Rng, keep: impl Fn(&SpecNode, &SpecNode) -> bool) -> Option<usize> {
        let candidates: Vec<(usize, usize)> = self
            .spec
            .edges
            .iter()
            .enumerate()
            .filter(|(_, e)| e.from != e.to && keep(self.get(&e.from), self.get(&e.to)))
            .map(|(i, e)| (i, self.degree(&e.from) + self.degree(&e.to)))
            .collect();
        simple(rng, candidates, &self.hand)
    }

    /// The finished spec, checked: ids unique, every edge between nodes
    /// it has, and every group a node names among its groups.
    fn done(self) -> Spec {
        let mut spec = self.spec;
        // Placed in drawn pixels; a spec's drawn positions are spread by
        // DRAWN_SCALE when laid out.
        for n in &mut spec.nodes {
            n.x = n.x.map(|x| x / DRAWN_SCALE);
            n.y = n.y.map(|y| y / DRAWN_SCALE);
        }
        let ids: std::collections::BTreeSet<&str> = spec.nodes.iter().map(|n| n.id.as_str()).collect();
        assert_eq!(ids.len(), spec.nodes.len(), "grown node ids are unique");
        for e in &spec.edges {
            assert!(
                ids.contains(e.from.as_str()) && ids.contains(e.to.as_str()),
                "grown edges join grown nodes"
            );
        }
        for n in &spec.nodes {
            if let Some(group) = &n.group {
                assert!(
                    spec.groups.iter().any(|g| &g.id == group),
                    "grown nodes sit in grown groups"
                );
            }
        }
        spec
    }
}

/// How one hand grows a diagram, drawn once per diagram so drafts of a
/// kind differ in manner as well as in detail: which rewrites it reaches
/// for, how much it keeps to simple parts, how roomy it is, and how close
/// what it joins up lies.
#[derive(Clone, Copy)]
struct Hand {
    /// Each rewrite's weight is multiplied by its own of these.
    tastes: [f64; 8],
    /// A part is likelier the fewer connections it has, as one over one
    /// more than them to this power.
    focus: f64,
    /// The share of picks that take any part, simple or not.
    wander: f64,
    /// Gaps between neighbours are multiplied by this: tight hands are
    /// commoner than roomy ones.
    room: f64,
    /// How much likelier a node two links away is joined than one three
    /// away.
    reach: f64,
}

impl Hand {
    /// The manner every grown diagram had before hands differed.
    const PLAIN: Hand = Hand {
        tastes: [1.0; 8],
        focus: 2.0,
        wander: 1.0 / 7.0,
        room: 1.0,
        reach: 2.0,
    };

    fn new(rng: &mut Rng) -> Hand {
        // Between a third and three times as keen, evenly on a log scale.
        let mut tastes = [1.0; 8];
        for taste in &mut tastes {
            *taste = 3f64.powf(rng.unit() * 2.0 - 1.0);
        }
        Hand {
            tastes,
            focus: 0.5 + rng.unit() * 2.5,
            wander: 0.05 + rng.unit() * 0.25,
            // From 0.4 to 1.3, evenly on a log scale but leaning low.
            room: 0.4 * (1.3f64 / 0.4).powf(rng.unit().powf(1.6)),
            reach: 1.0 + rng.unit() * 3.0,
        }
    }

    /// One of the rewrites with these weights, as this hand leans.
    fn pick(&self, rng: &mut Rng, weights: &[f64]) -> usize {
        let leaned: Vec<f64> = weights
            .iter()
            .zip(self.tastes.iter().cycle())
            .map(|(w, t)| w * t)
            .collect();
        weighted(rng, &leaned)
    }
}

/// One of `candidates`, each with how many connections it has: the fewer,
/// the likelier, as the hand's focus has it. Its wander takes any, so busy
/// parts grow too.
fn simple<T>(rng: &mut Rng, candidates: Vec<(T, usize)>, hand: &Hand) -> Option<T> {
    if candidates.is_empty() {
        return None;
    }
    if rng.chance(hand.wander) {
        let at = rng.below(candidates.len());
        return candidates.into_iter().nth(at).map(|(item, _)| item);
    }
    let weights: Vec<f64> = candidates
        .iter()
        .map(|(_, d)| 1.0 / ((d + 1) as f64).powf(hand.focus))
        .collect();
    let mut left = rng.unit() * weights.iter().sum::<f64>();
    let last = candidates.len() - 1;
    for (i, ((item, _), w)) in candidates.into_iter().zip(&weights).enumerate() {
        if left < *w || i == last {
            return Some(item);
        }
        left -= w;
    }
    None
}

/// The place of one of `weights`, each as likely as its weight.
fn weighted(rng: &mut Rng, weights: &[f64]) -> usize {
    let mut left = rng.unit() * weights.iter().sum::<f64>();
    for (i, w) in weights.iter().enumerate() {
        if left < *w {
            return i;
        }
        left -= w;
    }
    weights.len() - 1
}

/// Names drawn in turn from a shuffled pool, numbered once it runs out.
struct Pool {
    names: Vec<String>,
    next: usize,
}

impl Pool {
    fn new(rng: &mut Rng, pool: &[&str]) -> Pool {
        Pool {
            names: names(rng, pool, 400),
            next: 0,
        }
    }

    fn take(&mut self) -> String {
        self.next += 1;
        self.names[(self.next - 1) % self.names.len()].clone()
    }
}

/// Rows a grown flow's lane has room for, and how far apart they are.
const LANE_ROWS: usize = 3;
const ROW_GAP: f64 = 110.0;

/// How many nodes a grown diagram of `size` stops at.
fn grown_nodes(size: Size) -> usize {
    size.scale(8)
}

/// Rewrites a diagram until it has `target` nodes; `rewrite` tries one
/// rewrite, which may find nothing to rewrite.
fn grow_until(g: &mut Grower, rng: &mut Rng, target: usize, mut rewrite: impl FnMut(&mut Grower, &mut Rng)) {
    let mut tries = 0;
    while g.spec.nodes.len() < target && tries < 10 * target {
        tries += 1;
        rewrite(g, rng);
    }
}

const QUESTIONS: &[&str] = &[
    "Valid?",
    "In stock?",
    "Paid?",
    "Approved?",
    "Complete?",
    "Urgent?",
    "Signed?",
    "Accepted?",
];
const FAILED_ENDS: &[&str] = &["Cancelled", "Rejected", "Expired", "Withdrawn"];

/// A process flow in lanes: steps split, checks that branch and merge or
/// end early, retries, work done in parallel, and steps handed to another
/// department.
fn grow_flow(rng: &mut Rng, size: Size) -> Spec {
    let mut g = Grower::new("Order to delivery — process flow", "flow");
    g.hand = Hand::new(rng);
    let mut steps = Pool::new(rng, STEPS);
    let lanes = names(rng, LANES, LANES.len());
    let mut lanes_used = 0;
    let mut new_lane = |g: &mut Grower| -> Option<String> {
        let title = lanes.get(lanes_used)?.clone();
        lanes_used += 1;
        let id = format!("lane{lanes_used}");
        g.spec.groups.push(SpecGroup {
            id: id.clone(),
            title,
            parent: None,
        });
        Some(id)
    };
    let first = new_lane(&mut g);
    let start = g.node("Start".into(), SpecShape::Circle, first.clone(), Vec::new());
    let step = g.node(steps.take(), SpecShape::Rect, first.clone(), Vec::new());
    let end = g.node("Done".into(), SpecShape::Circle, first, Vec::new());
    g.put(&start, (0.0, 60.0));
    let room = g.hand.room;
    let gap = |rng: &mut Rng| ([40.0, 60.0, 80.0, 110.0][rng.below(4)] * room).max(32.0);
    let x = g.after(&start, &step, gap(rng));
    g.in_lane(&step, x, 0);
    let x = g.after(&step, &end, gap(rng));
    g.in_lane(&end, x, 0);
    g.edge(&start, &step, None);
    g.edge(&step, &end, None);
    // The row a node takes after `a`: a's own in a's lane, else the top.
    let row_after = |g: &Grower, a: &str, id: &str| {
        if g.get(a).group == g.get(id).group {
            g.row_of(a)
        } else {
            0
        }
    };
    let is_step = |n: &SpecNode| n.shape == SpecShape::Rect;
    // Mostly the lane of the work before; now and then another, or a new
    // one, more often once the lanes run long.
    let mut lane_for = |g: &mut Grower, rng: &mut Rng, near: &str| -> Option<String> {
        let here = g.get(near).group.clone();
        let weights = if g.spread_ratio(None) > 3.0 {
            [4.0, 3.0, 3.0]
        } else {
            [7.0, 2.0, 1.0]
        };
        match weighted(rng, &weights) {
            0 => here,
            1 => {
                let at = rng.below(g.spec.groups.len());
                Some(g.spec.groups[at].id.clone())
            }
            _ => new_lane(g).or(here),
        }
    };
    grow_until(&mut g, rng, grown_nodes(size), |g, rng| {
        match g.hand.pick(rng, &[3.0, 2.0, 1.0, 1.0, 1.0]) {
            // A step split in two.
            0 => {
                let Some(i) = g.simple_edge(rng, |_, _| true) else {
                    return;
                };
                let e = g.spec.edges.remove(i);
                let lane = lane_for(g, rng, &e.from);
                let t = g.node(steps.take(), SpecShape::Rect, lane, Vec::new());
                let (x, row) = (g.after(&e.from, &t, gap(rng)), row_after(g, &e.from, &t));
                g.in_row(&t, x, row);
                g.edge(&e.from, &t, e.label);
                g.edge(&t, &e.to, None);
            }
            // A check after a step: "no" takes another way back in, or ends early.
            1 => {
                let Some(i) = g.simple_edge(rng, |a, _| is_step(a)) else {
                    return;
                };
                let e = g.spec.edges.remove(i);
                let lane = g.get(&e.from).group.clone();
                let d = g.node(
                    rng.pick(QUESTIONS).into(),
                    SpecShape::Decision,
                    lane.clone(),
                    Vec::new(),
                );
                let yes = g.node(steps.take(), SpecShape::Rect, lane, Vec::new());
                let other = lane_for(g, rng, &e.from);
                let no = g.node(steps.take(), SpecShape::Rect, other.clone(), Vec::new());
                // "Yes" goes on along the row, "no" drops to the row below.
                let row = g.row_of(&e.from);
                let x = g.after(&e.from, &d, gap(rng));
                g.in_lane(&d, x, row);
                let x = g.after(&d, &yes, gap(rng));
                g.in_lane(&yes, x, row);
                let x = g.after(&d, &no, gap(rng));
                let below = if g.get(&no).group == g.get(&d).group {
                    (g.row_of(&d) + 1).min(LANE_ROWS - 1)
                } else {
                    0
                };
                g.in_lane(&no, x, below);
                g.edge(&e.from, &d, e.label);
                g.edge(&d, &yes, Some("yes".into()));
                g.edge(&yes, &e.to, None);
                g.edge(&d, &no, Some("no".into()));
                if rng.chance(0.3) {
                    let failed = g.node(rng.pick(FAILED_ENDS).into(), SpecShape::Circle, other, Vec::new());
                    let (x, row) = (g.after(&no, &failed, gap(rng)), g.row_of(&no));
                    g.in_lane(&failed, x, row);
                    g.edge(&no, &failed, None);
                } else {
                    g.edge(&no, &e.to, None);
                }
            }
            // A step checked after it and done again until it passes.
            2 => {
                let Some(i) = g.simple_edge(rng, |a, _| is_step(a)) else {
                    return;
                };
                let e = g.spec.edges.remove(i);
                let lane = g.get(&e.from).group.clone();
                let d = g.node(rng.pick(QUESTIONS).into(), SpecShape::Decision, lane, Vec::new());
                let (x, row) = (g.after(&e.from, &d, gap(rng)), g.row_of(&e.from));
                g.in_lane(&d, x, row);
                g.edge(&e.from, &d, e.label);
                g.edge(&d, &e.to, Some("yes".into()));
                g.edge(&d, &e.from, Some("no".into()));
            }
            // Two steps done side by side between a split and a join.
            3 => {
                let Some(i) = g.simple_edge(rng, |a, b| is_step(a) || is_step(b)) else {
                    return;
                };
                let e = g.spec.edges.remove(i);
                let lane = g.get(&e.from).group.clone();
                let split = g.node("+".into(), SpecShape::Decision, lane.clone(), Vec::new());
                let join = g.node("+".into(), SpecShape::Decision, lane.clone(), Vec::new());
                let one = g.node(steps.take(), SpecShape::Rect, lane, Vec::new());
                let other = lane_for(g, rng, &e.from);
                let two = g.node(steps.take(), SpecShape::Rect, other, Vec::new());
                // One above the other between the split and the join.
                let row = g.row_of(&e.from);
                let x = g.after(&e.from, &split, gap(rng));
                g.in_lane(&split, x, row);
                let x = g.after(&split, &one, gap(rng));
                g.in_lane(&one, x, row);
                let below = if g.get(&two).group == g.get(&split).group {
                    (row + 1).min(LANE_ROWS - 1)
                } else {
                    0
                };
                g.in_lane(&two, x, below);
                let last = if g.at(&one).map_or(0.0, |p| p.0) >= g.at(&two).map_or(0.0, |p| p.0) {
                    &one
                } else {
                    &two
                };
                let x = g.after(last, &join, gap(rng));
                g.in_lane(&join, x, row);
                g.edge(&e.from, &split, e.label);
                for t in [&one, &two] {
                    g.edge(&split, t, None);
                    g.edge(t, &join, None);
                }
                g.edge(&join, &e.to, None);
            }
            // A step handed to another department.
            _ => {
                let Some(t) = g.simple_node(rng, is_step) else {
                    return;
                };
                let lane = lane_for(g, rng, &t);
                if lane == g.get(&t).group {
                    return;
                }
                g.get_mut(&t).group = lane;
                let x = g.at(&t).map_or(0.0, |p| p.0);
                g.in_lane(&t, x, 0);
            }
        }
    });
    g.forward();
    g.done()
}

/// A data flow diagram: flows split by a process between, processes given
/// the stores they keep and the outside parties they deal with, fanning
/// out, and a process broken down into numbered sub-processes.
fn grow_dfd(rng: &mut Rng, size: Size) -> Spec {
    let mut g = Grower::new("Order handling — data flow diagram", "dfd");
    g.hand = Hand::new(rng);
    let mut processes = Pool::new(rng, PROCESSES);
    let mut entities = Pool::new(rng, ENTITIES);
    let mut stores = Pool::new(rng, STORES);
    let mut numbered = 0;
    let mut stored = 0;
    let mut process = |g: &mut Grower, name: String| {
        numbered += 1;
        g.node(format!("{numbered} {name}"), SpecShape::Process, None, Vec::new())
    };
    let mut store = |g: &mut Grower, name: String| {
        stored += 1;
        g.node(format!("D{stored} {name}"), SpecShape::Rect, None, Vec::new())
    };
    let flow = |rng: &mut Rng| Some(rng.pick(FLOWS).to_owned());
    // Which way this diagram reads: work moves ahead, stores sit to the side.
    let ahead = if rng.chance(0.5) { (1.0, 0.0) } else { (0.0, 1.0) };
    let side = (ahead.1, ahead.0);
    let back = (-ahead.0, -ahead.1);
    let other_side = (-side.0, -side.1);
    // To the side, but off the line work runs along.
    let aside = [
        side,
        other_side,
        (side.0 + ahead.0, side.1 + ahead.1),
        (other_side.0 + ahead.0, other_side.1 + ahead.1),
        (side.0 - ahead.0, side.1 - ahead.1),
        (other_side.0 - ahead.0, other_side.1 - ahead.1),
    ];
    let first = process(&mut g, processes.take());
    g.put(&first, (0.0, 0.0));
    let source = g.node(entities.take(), SpecShape::Rect, None, Vec::new());
    g.beside(rng, &source, &first, &[back, side, other_side]);
    let sink = g.node(entities.take(), SpecShape::Rect, None, Vec::new());
    g.beside(rng, &sink, &first, &[ahead, side, other_side]);
    let kept = store(&mut g, stores.take());
    g.beside(rng, &kept, &first, &aside);
    g.edge(&source, &first, flow(rng));
    g.edge(&first, &kept, flow(rng));
    g.edge(&first, &sink, flow(rng));
    let is_process = |n: &SpecNode| n.shape == SpecShape::Process;
    // Stores are named "D1 ...", "D2 ...".
    let is_store = |n: &SpecNode| {
        n.shape == SpecShape::Rect
            && n.label
                .strip_prefix('D')
                .is_some_and(|rest| rest.starts_with(|c: char| c.is_ascii_digit()))
    };
    let is_party = |n: &SpecNode| n.shape == SpecShape::Rect && !is_store(n);
    grow_until(&mut g, rng, grown_nodes(size), |g, rng| {
        match g.hand.pick(rng, &[3.0, 3.0, 2.0, 1.0, 1.0]) {
            // A store a process writes, and now and then another reads.
            0 => {
                let Some(p) = g.simple_node(rng, is_process) else {
                    return;
                };
                let s = store(g, stores.take());
                g.beside(rng, &s, &p, &aside);
                g.edge(&p, &s, flow(rng));
                if rng.chance(0.5)
                    && let Some(q) = g.nearby_node(rng, &s, |n| is_process(n) && n.id != p)
                {
                    g.edge(&s, &q, flow(rng));
                }
            }
            // A process between the ends of a flow.
            1 => {
                let Some(i) = g.simple_edge(rng, |a, b| is_process(a) || is_process(b)) else {
                    return;
                };
                let e = g.spec.edges.remove(i);
                let q = process(g, processes.take());
                g.between(rng, &q, &e.from, &e.to, ahead);
                g.edge(&e.from, &q, e.label);
                g.edge(&q, &e.to, flow(rng));
            }
            // An outside party a process hears from or answers.
            2 => {
                let Some(p) = g.simple_node(rng, is_process) else {
                    return;
                };
                let e = g.node(entities.take(), SpecShape::Rect, None, Vec::new());
                let outward = g.outward(&p);
                g.beside(rng, &e, &p, &outward);
                if rng.chance(0.5) {
                    g.edge(&e, &p, flow(rng));
                } else {
                    g.edge(&p, &e, flow(rng));
                }
            }
            // A process handing work on to two others.
            3 => {
                let Some(p) = g.simple_node(rng, is_process) else {
                    return;
                };
                let mut next_to = p.clone();
                for k in 0..2 {
                    let q = process(g, processes.take());
                    if k == 0 {
                        g.beside(rng, &q, &p, &[ahead, side, other_side]);
                    } else {
                        g.beside(rng, &q, &next_to, &[side, other_side, ahead]);
                    }
                    next_to.clone_from(&q);
                    g.edge(&p, &q, flow(rng));
                    let end = if rng.chance(0.5) {
                        g.nearby_node(rng, &q, is_store)
                    } else {
                        g.nearby_node(rng, &q, is_party)
                    };
                    if let Some(end) = end {
                        g.edge(&q, &end, flow(rng));
                    }
                }
            }
            // A process broken down: n becomes n.1, n.2 and n.3 in a row,
            // the first taking what came in, the last sending what went out.
            _ => {
                let Some(p) = g.simple_node(rng, is_process) else {
                    return;
                };
                let label = g.get(&p).label.clone();
                let (number, name) = label.split_once(' ').unwrap_or((label.as_str(), ""));
                let (number, name) = (number.to_owned(), name.to_owned());
                g.get_mut(&p).label = format!("{number}.1 {name}");
                let second = g.node(
                    format!("{number}.2 {}", processes.take()),
                    SpecShape::Process,
                    None,
                    Vec::new(),
                );
                let third = g.node(
                    format!("{number}.3 {}", processes.take()),
                    SpecShape::Process,
                    None,
                    Vec::new(),
                );
                // Laid out the way what it sends goes.
                let sent: Vec<String> = g
                    .spec
                    .edges
                    .iter()
                    .filter(|e| e.from == p)
                    .map(|e| e.to.clone())
                    .collect();
                let way = g.toward(&p, &sent).unwrap_or(ahead);
                let across = (way.1, way.0);
                g.beside(rng, &second, &p, &[way, across, (-across.0, -across.1)]);
                g.beside(rng, &third, &second, &[way, across, (-across.0, -across.1)]);
                for e in g.spec.edges.iter_mut().filter(|e| e.from == p) {
                    e.from.clone_from(&third);
                }
                g.edge(&p, &second, flow(rng));
                g.edge(&second, &third, flow(rng));
            }
        }
    });
    g.done()
}

/// Suffixes for tables made from another when the pool runs out.
const DERIVED: &[&str] = &["history", "notes", "events", "tags", "logs", "versions", "settings"];
const LOOKUPS: &[&str] = &["statuses", "types", "categories", "regions", "currencies"];

/// An entity relationship diagram: tables gaining child tables, junction
/// tables for many to many, lookup tables, one to one details, keys to
/// tables already near them, and now and then a table referring to itself.
fn grow_er(rng: &mut Rng, size: Size) -> Spec {
    let mut g = Grower::new("Shop — entity relationship diagram", "er");
    g.hand = Hand::new(rng);
    let mut order: Vec<usize> = (0..TABLES.len()).collect();
    for i in (1..order.len()).rev() {
        order.swap(i, rng.below(i + 1));
    }
    let mut pool = order.into_iter().map(|i| TABLES[i]);
    let table = |g: &mut Grower, name: String, own: &[&str]| {
        let columns = std::iter::once("id".to_owned())
            .chain(own.iter().map(|c| (*c).to_owned()))
            .collect();
        g.node(name, SpecShape::Table, None, columns)
    };
    let taken = |g: &Grower, name: &str| g.spec.nodes.iter().any(|n| n.label == name);
    // `child` gains a key to `parent`, after its id and the keys before.
    let refer = |g: &mut Grower, child: &str, parent: &str, column: String| {
        let at = g
            .get(child)
            .columns
            .iter()
            .take_while(|c| *c == "id" || c.ends_with("_id"))
            .count();
        g.get_mut(child).columns.insert(at, column.clone());
        g.edge_ports(child, parent, None, Some(column), Some("id".into()));
    };
    let (name, own) = pool.next().expect("the table pool is not empty");
    let first = table(&mut g, name.into(), own);
    g.put(&first, (0.0, 0.0));
    let (name, own) = pool.next().expect("the table pool has two tables");
    let second = table(&mut g, name.into(), own);
    let around = [(1.0, 0.0), (0.0, 1.0), (-1.0, 0.0), (0.0, -1.0)];
    g.beside(rng, &second, &first, &around);
    let key = format!("{}_id", singular(&g.get(&first).label));
    refer(&mut g, &second, &first, key);
    // Tables made for another's sake (junctions, lookups, details) are not
    // grown on, as a designer would not key a junction of junctions.
    let mut made_for: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    grow_until(&mut g, rng, grown_nodes(size), |g, rng| {
        let tables = |n: &SpecNode| n.shape == SpecShape::Table && !made_for.contains(&n.id);
        match g.hand.pick(rng, &[3.0, 2.0, 1.0, 1.0, 0.5, 1.0]) {
            // A child table, from the pool or named after its parent.
            0 => {
                let Some(parent) = g.simple_node(rng, tables) else {
                    return;
                };
                let parent_name = g.get(&parent).label.clone();
                let (name, own) = match pool.next() {
                    Some((name, own)) => (name.to_owned(), own),
                    None => (
                        format!("{}_{}", singular(&parent_name), rng.pick(DERIVED)),
                        &["created_at"][..],
                    ),
                };
                if taken(g, &name) {
                    return;
                }
                let child = table(g, name, own);
                let outward = g.outward(&parent);
                g.beside(rng, &child, &parent, &outward);
                refer(g, &child, &parent, format!("{}_id", singular(&parent_name)));
            }
            // Many to many: a junction table keyed to both.
            1 => {
                let Some(a) = g.simple_node(rng, tables) else {
                    return;
                };
                let Some(b) = g.nearby_node(rng, &a, |n| tables(n) && n.id != a) else {
                    return;
                };
                let (an, bn) = (g.get(&a).label.clone(), g.get(&b).label.clone());
                let name = format!("{}_{bn}", singular(&an));
                if g.joined(&a, &b) || taken(g, &name) {
                    return;
                }
                let j = table(g, name, &["created_at"]);
                g.between(rng, &j, &a, &b, (1.0, 0.0));
                refer(g, &j, &a, format!("{}_id", singular(&an)));
                refer(g, &j, &b, format!("{}_id", singular(&bn)));
                made_for.insert(j);
            }
            // A lookup table the table points at.
            2 => {
                let Some(t) = g.simple_node(rng, tables) else {
                    return;
                };
                let what = rng.pick(LOOKUPS);
                let name = format!("{}_{what}", singular(&g.get(&t).label));
                if taken(g, &name) {
                    return;
                }
                // status_id, or order_status_id when the table has one.
                let mut column = format!("{}_id", singular(what));
                if g.get(&t).columns.contains(&column) {
                    column = format!("{}_id", singular(&name));
                }
                let lookup = table(g, name, &["code", "label"]);
                g.beside(rng, &lookup, &t, &[(1.0, 0.0), (-1.0, 0.0), (1.0, 1.0), (-1.0, 1.0)]);
                refer(g, &t, &lookup, column);
                made_for.insert(lookup);
            }
            // One to one: details split out of a table.
            3 => {
                let Some(t) = g.simple_node(rng, tables) else {
                    return;
                };
                let base = g.get(&t).label.clone();
                let name = format!("{}_details", singular(&base));
                if taken(g, &name) {
                    return;
                }
                let details = table(g, name, &["notes", "updated_at"]);
                g.beside(rng, &details, &t, &[(0.0, 1.0), (0.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]);
                refer(g, &details, &t, format!("{}_id", singular(&base)));
                made_for.insert(details);
            }
            // A table referring to itself, as a tree.
            4 => {
                let Some(t) = g.simple_node(rng, |n| tables(n) && !n.columns.iter().any(|c| c == "parent_id")) else {
                    return;
                };
                refer(g, &t, &t, "parent_id".into());
            }
            // A table gaining a key to one near it, so two ways lead from
            // it to the same table (an order line to its product as well
            // as through its order).
            _ => {
                let Some(t) = g.simple_node(rng, tables) else { return };
                let Some(other) = g.close_by(rng, &t, tables) else {
                    return;
                };
                let column = format!("{}_id", singular(&g.get(&other).label));
                if g.get(&t).columns.contains(&column) {
                    return;
                }
                refer(g, &t, &other, column);
            }
        }
    });
    g.done()
}

const DOMAINS: &[&str] = &[
    "Ordering", "Payments", "Catalog", "Accounts", "Delivery", "Search", "Support", "Pricing",
];
const QUEUES: &[&str] = &[
    "Event bus [Kafka]",
    "Order events [SQS]",
    "Jobs [RabbitMQ]",
    "Notifications [SNS]",
    "Stream [Kinesis]",
];
const PROTOCOLS: &[&str] = &["HTTPS", "gRPC", "SQL", "AMQP", "REST", "events"];

/// The directions a diagram reads in: ahead, to one side and the other,
/// and back.
fn reading(rng: &mut Rng) -> [(f64, f64); 4] {
    let ahead = if rng.chance(0.5) { (1.0, 0.0) } else { (0.0, 1.0) };
    let side = (ahead.1, ahead.0);
    [ahead, side, (-side.0, -side.1), (-ahead.0, -ahead.1)]
}

/// Inner space round a group's contents, and the room its title takes, as
/// `write_svg` draws a group.
const GROUP_PAD: f64 = 24.0;
const GROUP_TITLE: f64 = 18.0;

/// Lays out a grown diagram whose groups nest, from the inside out: in
/// each group, its nodes and the groups in it are placed in the order
/// they were made, each beside one it joins (ahead of what leads to it,
/// back from what it leads to); then the group is one box at the level
/// above. Boxes are placed as nodes are, so none runs into another.
fn place_boxed(g: &mut Grower, rng: &mut Rng, [ahead, side, other_side, back]: [(f64, f64); 4]) {
    let depth = |g: &Grower, id: &str| {
        let mut depth = 0;
        let mut at = parent_of(&g.spec.groups, id);
        while let Some(p) = at {
            depth += 1;
            at = parent_of(&g.spec.groups, &p);
        }
        depth
    };
    let mut levels: Vec<Option<String>> = g.spec.groups.iter().map(|x| Some(x.id.clone())).collect();
    levels.sort_by_key(|level| std::cmp::Reverse(level.as_deref().map_or(0, |id| depth(g, id))));
    levels.push(None);
    // What stands for a node at a level: itself, or the group in the level
    // it sits in.
    let item_at = |g: &Grower, node: &SpecNode, level: &Option<String>| -> Option<String> {
        if &node.group == level {
            return Some(node.id.clone());
        }
        let mut at = node.group.clone();
        while let Some(id) = at {
            let parent = parent_of(&g.spec.groups, &id);
            if &parent == level {
                return Some(id);
            }
            at = parent;
        }
        None
    };
    let mut sizes: BTreeMap<String, (f64, f64)> = BTreeMap::new();
    let mut middles: BTreeMap<String, (f64, f64)> = BTreeMap::new();
    // Each level's items, with where they sit in it.
    type Items = Vec<(String, (f64, f64))>;
    let mut placed: BTreeMap<Option<String>, Items> = BTreeMap::new();
    for level in levels {
        // The items, in the order their first node was made.
        let mut items: Vec<(usize, String)> = Vec::new();
        for (index, n) in g.spec.nodes.iter().enumerate() {
            if let Some(item) = item_at(g, n, &level)
                && !items.iter().any(|(_, x)| *x == item)
            {
                items.push((index, item));
            }
        }
        let mut t = Grower::new("", "");
        t.hand = g.hand;
        for (_, item) in &items {
            let node = g.spec.nodes.iter().find(|n| &n.id == item);
            t.spec.nodes.push(SpecNode {
                id: item.clone(),
                label: node.map_or_else(String::new, |n| n.label.clone()),
                shape: node.map_or(SpecShape::Rect, |n| n.shape),
                columns: node.map_or_else(Vec::new, |n| n.columns.clone()),
                group: None,
                x: None,
                y: None,
            });
            if let Some(size) = sizes.get(item) {
                t.sizes.insert(item.clone(), *size);
            }
        }
        for e in &g.spec.edges {
            let (Some(a), Some(b)) = (item_at(g, g.get(&e.from), &level), item_at(g, g.get(&e.to), &level)) else {
                continue;
            };
            if a != b && !t.joined(&a, &b) {
                t.edge(&a, &b, None);
            }
        }
        for (k, (_, item)) in items.iter().enumerate() {
            if k == 0 {
                t.put(item, (0.0, 0.0));
                continue;
            }
            let done: Vec<&String> = items[..k].iter().map(|(_, x)| x).collect();
            let into = t.spec.edges.iter().find(|e| &e.to == item && done.contains(&&e.from));
            let out = t.spec.edges.iter().find(|e| &e.from == item && done.contains(&&e.to));
            let (anchor, mut directions) = match (into, out) {
                (Some(e), _) => (e.from.clone(), [ahead, side, other_side, back]),
                (None, Some(e)) => (e.to.clone(), [back, side, other_side, ahead]),
                (None, None) => (items[k - 1].1.clone(), [side, ahead, other_side, back]),
            };
            // Now and then off to a side first, so what hangs off one box
            // does not all line up on one side of it.
            if rng.chance(0.35) {
                directions.swap(0, 1 + rng.below(2));
            }
            t.beside(rng, item, &anchor, &directions);
        }
        let at: Vec<(String, (f64, f64))> = items
            .iter()
            .map(|(_, item)| (item.clone(), t.at(item).unwrap_or((0.0, 0.0))))
            .collect();
        if let Some(group) = &level {
            let (mut left, mut top, mut right, mut bottom) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
            for n in &t.spec.nodes {
                let ((x, y), (w, h)) = (t.at(&n.id).unwrap_or((0.0, 0.0)), t.size_of(n));
                (left, right) = (left.min(x - w / 2.0), right.max(x + w / 2.0));
                (top, bottom) = (top.min(y - h / 2.0), bottom.max(y + h / 2.0));
            }
            sizes.insert(
                group.clone(),
                (
                    right - left + 2.0 * GROUP_PAD,
                    bottom - top + 2.0 * GROUP_PAD + GROUP_TITLE,
                ),
            );
            middles.insert(group.clone(), ((left + right) / 2.0, (top + bottom) / 2.0));
        }
        placed.insert(level, at);
    }
    // From the outside in: each group's items moved to where its box went,
    // below its title.
    let mut todo: Vec<(Option<String>, (f64, f64))> = vec![(None, (0.0, 0.0))];
    while let Some((level, centre)) = todo.pop() {
        let middle = level.as_ref().map_or((0.0, 0.0), |id| middles[id]);
        let shift = if level.is_some() { GROUP_TITLE / 2.0 } else { 0.0 };
        for (item, (x, y)) in placed.get(&level).cloned().unwrap_or_default() {
            let at = (centre.0 + x - middle.0, centre.1 + y - middle.1 + shift);
            if sizes.contains_key(&item) {
                todo.push((Some(item), at));
            } else {
                g.put(&item, at);
            }
        }
    }
}

/// A C4 container diagram: people outside a system boundary, apps and
/// services inside it, some in domain boundaries of their own, with their
/// databases, queues between them, outside systems they call, and calls
/// to services and databases already near them.
fn grow_c4(rng: &mut Rng, size: Size) -> Spec {
    let mut g = Grower::new("Platform — container diagram", "c4");
    g.hand = Hand::new(rng);
    let apps: Vec<String> = APPS.iter().map(|(name, tech)| format!("{name} [{tech}]")).collect();
    let apps: Vec<&str> = apps.iter().map(String::as_str).collect();
    let databases: Vec<String> = DATABASES
        .iter()
        .map(|(name, tech)| format!("{name} [{tech}]"))
        .collect();
    let databases: Vec<&str> = databases.iter().map(String::as_str).collect();
    let (mut apps, mut databases) = (Pool::new(rng, &apps), Pool::new(rng, &databases));
    let mut people = Pool::new(rng, PEOPLE);
    let mut systems = Pool::new(rng, SYSTEMS);
    let mut domains = Pool::new(rng, DOMAINS);
    let reads = reading(rng);
    let verb = |rng: &mut Rng| Some(rng.pick(VERBS).to_owned());
    let system = g.group("Platform", None);
    let web = g.node(apps.take(), SpecShape::Rect, Some(system.clone()), Vec::new());
    let api = g.node(apps.take(), SpecShape::Rect, Some(system.clone()), Vec::new());
    let db = g.node(databases.take(), SpecShape::Rect, Some(system), Vec::new());
    let person = g.node(people.take(), SpecShape::Rect, None, Vec::new());
    g.edge(&person, &web, Some("Uses".into()));
    g.edge(&web, &api, verb(rng));
    g.edge(&api, &db, verb(rng));
    let mut queues = Pool::new(rng, QUEUES);
    // Apps are what sits inside but is no database and no queue.
    let kept: std::collections::BTreeSet<String> = databases.names.iter().chain(&queues.names).cloned().collect();
    let is_app = |n: &SpecNode| n.group.is_some() && !kept.contains(&n.label);
    grow_until(&mut g, rng, grown_nodes(size), |g, rng| {
        match g.hand.pick(rng, &[3.0, 2.0, 1.0, 1.5, 1.0, 2.0]) {
            // A service another calls, now and then in a domain of its own.
            0 => {
                let Some(caller) = g.simple_node(rng, is_app) else {
                    return;
                };
                let group = if rng.chance(0.3) {
                    let parent = g.get(&caller).group.clone();
                    Some(g.group(&domains.take(), parent))
                } else {
                    g.get(&caller).group.clone()
                };
                let s = g.node(apps.take(), SpecShape::Rect, group, Vec::new());
                g.edge(&caller, &s, verb(rng));
            }
            // A database a service keeps, beside it in its boundary.
            1 => {
                let Some(owner) = g.simple_node(rng, is_app) else {
                    return;
                };
                let group = g.get(&owner).group.clone();
                let d = g.node(databases.take(), SpecShape::Rect, group, Vec::new());
                g.edge(&owner, &d, verb(rng));
            }
            // Someone using an app.
            2 => {
                let Some(app) = g.simple_node(rng, is_app) else { return };
                let p = g.node(people.take(), SpecShape::Rect, None, Vec::new());
                g.edge(&p, &app, Some("Uses".into()));
            }
            // An outside system a service calls.
            3 => {
                let Some(caller) = g.simple_node(rng, is_app) else {
                    return;
                };
                let x = g.node(systems.take(), SpecShape::Rect, None, Vec::new());
                g.edge(&caller, &x, verb(rng));
            }
            // A queue between two services.
            4 => {
                let Some(i) = g.simple_edge(rng, |a, b| is_app(a) && is_app(b)) else {
                    return;
                };
                let e = g.spec.edges.remove(i);
                let group = g.get(&e.from).group.clone();
                let q = g.node(queues.take(), SpecShape::Rect, group, Vec::new());
                g.edge(&e.from, &q, Some("Sends events to".into()));
                g.edge(&q, &e.to, Some("Delivers to".into()));
            }
            // A service calling another near it, or using its database.
            _ => {
                let Some(from) = g.simple_node(rng, is_app) else { return };
                let Some(to) = g.close_by(rng, &from, |n| n.group.is_some()) else {
                    return;
                };
                g.edge(&from, &to, verb(rng));
            }
        }
    });
    place_boxed(&mut g, rng, reads);
    g.done()
}

/// A cloud architecture: clients outside a network, edge services in a
/// public subnet, services in a private subnet (some in clusters of their
/// own in it), data stores in a data subnet, and outside services; and
/// links between what is there, so the parts form loops, not a tree.
fn grow_arch(rng: &mut Rng, size: Size) -> Spec {
    let mut g = Grower::new("Shop — cloud architecture", "arch");
    g.hand = Hand::new(rng);
    let mut clients = Pool::new(rng, LAYERS[0].1);
    let mut doors = Pool::new(rng, LAYERS[1].1);
    let mut services = Pool::new(rng, LAYERS[2].1);
    let mut stores = Pool::new(rng, LAYERS[3].1);
    let mut outside = Pool::new(rng, SYSTEMS);
    let reads = reading(rng);
    let protocol = |rng: &mut Rng| Some(rng.pick(PROTOCOLS).to_owned());
    let network = g.group("VPC", None);
    let public = g.group("Public subnet", Some(network.clone()));
    let private = g.group("Private subnet", Some(network.clone()));
    let data = g.group("Data subnet", Some(network.clone()));
    let entry = g.node(doors.take(), SpecShape::Rect, Some(public.clone()), Vec::new());
    let first = g.node(services.take(), SpecShape::Rect, Some(private.clone()), Vec::new());
    let store = g.node(stores.take(), SpecShape::Rect, Some(data.clone()), Vec::new());
    let client = g.node(clients.take(), SpecShape::Rect, None, Vec::new());
    g.edge(&client, &entry, protocol(rng));
    g.edge(&entry, &first, protocol(rng));
    g.edge(&first, &store, protocol(rng));
    let in_group = |n: &SpecNode, group: &str| n.group.as_deref() == Some(group);
    let is_service = |n: &SpecNode| {
        n.group
            .as_ref()
            .is_some_and(|x| *x != public && *x != data && *x != network)
    };
    let mut clusters = 0;
    grow_until(&mut g, rng, grown_nodes(size), |g, rng| {
        match g.hand.pick(rng, &[3.0, 2.0, 1.0, 1.0, 1.0, 1.0, 2.5]) {
            // A service another calls, now and then in a cluster of its own.
            0 => {
                let Some(caller) = g.simple_node(rng, is_service) else {
                    return;
                };
                let group = if rng.chance(0.2) {
                    clusters += 1;
                    Some(g.group(&format!("Cluster {clusters}"), Some(private.clone())))
                } else {
                    g.get(&caller).group.clone()
                };
                let s = g.node(services.take(), SpecShape::Rect, group, Vec::new());
                g.edge(&caller, &s, protocol(rng));
            }
            // A data store a service uses, among the other stores.
            1 => {
                let Some(user) = g.simple_node(rng, is_service) else {
                    return;
                };
                let d = g.node(stores.take(), SpecShape::Rect, Some(data.clone()), Vec::new());
                g.edge(&user, &d, protocol(rng));
            }
            // A client coming in at the edge.
            2 => {
                let Some(door) = g.simple_node(rng, |n| in_group(n, &public)) else {
                    return;
                };
                let c = g.node(clients.take(), SpecShape::Rect, None, Vec::new());
                g.edge(&c, &door, protocol(rng));
            }
            // Another edge service in front of what a client reaches.
            3 => {
                let Some(i) = g.simple_edge(rng, |a, b| a.group.is_none() && in_group(b, &public)) else {
                    return;
                };
                let e = g.spec.edges.remove(i);
                let front = g.node(doors.take(), SpecShape::Rect, Some(public.clone()), Vec::new());
                g.edge(&e.from, &front, e.label);
                g.edge(&front, &e.to, protocol(rng));
            }
            // A queue and a worker behind a service.
            4 => {
                let Some(producer) = g.simple_node(rng, is_service) else {
                    return;
                };
                let group = g.get(&producer).group.clone();
                let q = g.node(
                    format!("{} queue", services.take()),
                    SpecShape::Rect,
                    group.clone(),
                    Vec::new(),
                );
                let w = g.node(
                    format!("{} worker", services.take()),
                    SpecShape::Rect,
                    group,
                    Vec::new(),
                );
                g.edge(&producer, &q, Some("events".into()));
                g.edge(&q, &w, Some("events".into()));
            }
            // An outside service a service calls.
            5 => {
                let Some(caller) = g.simple_node(rng, is_service) else {
                    return;
                };
                let x = g.node(outside.take(), SpecShape::Rect, None, Vec::new());
                g.edge(&caller, &x, protocol(rng));
            }
            // A link between what is already there, closing a small loop: a
            // service calling another near it, sharing another's store, or
            // an edge service sending to a second service.
            _ => {
                let Some(from) = g.simple_node(rng, |n| is_service(n) || in_group(n, &public)) else {
                    return;
                };
                let door = in_group(g.get(&from), &public);
                let Some(to) = g.close_by(rng, &from, |n| is_service(n) || (!door && in_group(n, &data))) else {
                    return;
                };
                g.edge(&from, &to, protocol(rng));
            }
        }
    });
    place_boxed(&mut g, rng, reads);
    g.done()
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

/// A grown diagram of `kind` drafted at `level`, laid out as a spec is.
fn generate_grown(kind: Kind, size: Size, level: Level, seed: u64, layout: Option<Layout>) -> String {
    let mut rng = Rng::new(seed ^ (1 << 32) ^ ((kind as u64) << 40) ^ ((size as u64) << 48) ^ ((level as u64) << 56));
    let spec = match kind {
        Kind::Flow => grow_flow(&mut rng, size),
        Kind::Dfd => grow_dfd(&mut rng, size),
        Kind::Er => grow_er(&mut rng, size),
        Kind::C4 => grow_c4(&mut rng, size),
        Kind::Arch => grow_arch(&mut rng, size),
    };
    let mut draft = from_spec(&mut rng, &spec, level.mess(), layout);
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
                if args.grow {
                    let name = format!("{kind:?}-{size:?}-{level:?}-grown-{seed}.svg").to_lowercase();
                    write_checked(
                        &args.out,
                        &name,
                        &generate_grown(*kind, *size, level, seed, args.layout),
                    );
                } else {
                    let name = format!("{kind:?}-{size:?}-{level:?}-{seed}.svg").to_lowercase();
                    write_checked(&args.out, &name, &generate(*kind, *size, level, seed));
                }
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
