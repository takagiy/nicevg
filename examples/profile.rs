//! Profiles fixing or arranging an SVG diagram: where the time goes, span
//! by span, with the sizes each span worked on.
//!
//! ```text
//! cargo run --release --example profile -- tests/fixtures/loan-origination-dfd.svg
//! cargo run --release --example profile -- input.svg --fix --chrome trace.json --folded stacks.folded
//! ```
//!
//! It prints a table of spans: how often each ran, its total and self time
//! (time not spent in its child spans), and the sums and maxima of the
//! numbers recorded on it, such as grid sizes. Time in spans entered on
//! several threads at once (the arrange evaluations) is summed over the
//! threads, so it can exceed the wall-clock time.
//!
//! `--chrome` writes a trace to open in https://ui.perfetto.dev, and
//! `--folded` writes folded stacks for a flame graph (for example with
//! `inferno-flamegraph stacks.folded > flame.svg`).

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use clap::Parser;
use tracing::Subscriber;
use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id, Record};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::{Context, SubscriberExt};
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::util::SubscriberInitExt;

#[derive(Parser)]
struct Args {
    /// The SVG diagram to process.
    input: PathBuf,
    /// Only fix the diagram; arranging is the default, as in the CLI.
    #[arg(long)]
    fix: bool,
    /// Write a Chrome trace (JSON) here.
    #[arg(long)]
    chrome: Option<PathBuf>,
    /// Write folded stacks for a flame graph here.
    #[arg(long)]
    folded: Option<PathBuf>,
    /// How many spans to list, slowest self time first.
    #[arg(long, default_value_t = 40)]
    top: usize,
}

/// Timing and recorded numbers of one span while it is open.
#[derive(Default)]
struct Open {
    entered: Option<Instant>,
    busy: Duration,
    children: Duration,
    numbers: BTreeMap<&'static str, u64>,
}

/// Totals for every span with one name.
#[derive(Default)]
struct Totals {
    calls: u64,
    busy: Duration,
    own: Duration,
    longest: Duration,
    numbers: BTreeMap<&'static str, (u64, u64)>,
}

struct Numbers<'a>(&'a mut BTreeMap<&'static str, u64>);

impl Visit for Numbers<'_> {
    fn record_u64(&mut self, field: &Field, value: u64) {
        self.0.insert(field.name(), value);
    }
    fn record_i64(&mut self, field: &Field, value: i64) {
        self.0.insert(field.name(), value.max(0) as u64);
    }
    fn record_debug(&mut self, _: &Field, _: &dyn std::fmt::Debug) {}
}

/// Sums each span's busy and self time by span name.
#[derive(Clone, Default)]
struct Summary {
    totals: Arc<Mutex<BTreeMap<&'static str, Totals>>>,
}

impl<S> Layer<S> for Summary
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        let mut open = Open::default();
        attrs.record(&mut Numbers(&mut open.numbers));
        if let Some(span) = ctx.span(id) {
            span.extensions_mut().insert(open);
        }
    }

    fn on_record(&self, id: &Id, values: &Record<'_>, ctx: Context<'_, S>) {
        if let Some(span) = ctx.span(id)
            && let Some(open) = span.extensions_mut().get_mut::<Open>()
        {
            values.record(&mut Numbers(&mut open.numbers));
        }
    }

    fn on_enter(&self, id: &Id, ctx: Context<'_, S>) {
        if let Some(span) = ctx.span(id)
            && let Some(open) = span.extensions_mut().get_mut::<Open>()
        {
            open.entered = Some(Instant::now());
        }
    }

    fn on_exit(&self, id: &Id, ctx: Context<'_, S>) {
        if let Some(span) = ctx.span(id)
            && let Some(open) = span.extensions_mut().get_mut::<Open>()
            && let Some(entered) = open.entered.take()
        {
            open.busy += entered.elapsed();
        }
    }

    fn on_close(&self, id: Id, ctx: Context<'_, S>) {
        let Some(span) = ctx.span(&id) else { return };
        let Some(open) = span.extensions_mut().remove::<Open>() else {
            return;
        };
        if let Some(parent) = span.parent()
            && let Some(parent_open) = parent.extensions_mut().get_mut::<Open>()
        {
            parent_open.children += open.busy;
        }
        let mut totals = self.totals.lock().expect("summary lock");
        let entry = totals.entry(span.name()).or_default();
        entry.calls += 1;
        entry.busy += open.busy;
        entry.own += open.busy.saturating_sub(open.children);
        entry.longest = entry.longest.max(open.busy);
        for (name, value) in open.numbers {
            let (sum, max) = entry.numbers.entry(name).or_default();
            *sum += value;
            *max = (*max).max(value);
        }
    }
}

fn milliseconds(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

fn main() {
    let args = Args::parse();
    let svg = std::fs::read_to_string(&args.input).expect("the input SVG is readable");

    let summary = Summary::default();
    let (chrome, _chrome_guard) = match &args.chrome {
        Some(path) => {
            let (layer, guard) = tracing_chrome::ChromeLayerBuilder::new()
                .file(path)
                .include_args(true)
                .build();
            (Some(layer), Some(guard))
        }
        None => (None, None),
    };
    let (flame, flame_guard) = match &args.folded {
        Some(path) => {
            let (layer, guard) = tracing_flame::FlameLayer::with_file(path).expect("the folded stacks file opens");
            (Some(layer), Some(guard))
        }
        None => (None, None),
    };
    tracing_subscriber::registry()
        .with(summary.clone())
        .with(chrome)
        .with(flame)
        .init();

    let report = nicevg::analyze(&svg).expect("the input is an SVG diagram");
    let started = Instant::now();
    let result = if args.fix {
        nicevg::fix(&svg)
    } else {
        nicevg::arrange(&svg)
    }
    .expect("the input is an SVG diagram");
    let wall = started.elapsed();
    if let Some(guard) = flame_guard {
        guard.flush().expect("the folded stacks are written");
    }

    println!(
        "{} {}: {} nodes, {} connectors, {} labels; {:.1} ms wall, {} changes, {} issues left",
        if args.fix { "fix" } else { "arrange" },
        args.input.display(),
        report.diagram.nodes.len(),
        report.diagram.connectors.len(),
        report.diagram.labels.len(),
        milliseconds(wall),
        result.changes.len(),
        result.report.issues.len(),
    );
    println!();
    println!(
        "{:<22} {:>9} {:>11} {:>11} {:>9} {:>9}  numbers (sum / max)",
        "span", "calls", "total ms", "self ms", "mean ms", "max ms"
    );
    let totals = summary.totals.lock().expect("summary lock");
    let mut rows: Vec<_> = totals.iter().collect();
    rows.sort_by_key(|(_, totals)| std::cmp::Reverse(totals.own));
    for (name, totals) in rows.into_iter().take(args.top) {
        let numbers = totals
            .numbers
            .iter()
            .map(|(field, (sum, max))| format!("{field} {sum}/{max}"))
            .collect::<Vec<_>>()
            .join(", ");
        println!(
            "{:<22} {:>9} {:>11.1} {:>11.1} {:>9.3} {:>9.1}  {}",
            name,
            totals.calls,
            milliseconds(totals.busy),
            milliseconds(totals.own),
            milliseconds(totals.busy) / totals.calls as f64,
            milliseconds(totals.longest),
            numbers
        );
    }
}
