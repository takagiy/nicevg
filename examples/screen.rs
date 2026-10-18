//! Arranges diagrams with a screening model and without, to see what the
//! model costs in badness and saves in time.
//!
//! ```text
//! cargo run --release --features screening --example screen -- eval model.json 24 out.jsonl drafts/a drafts/b.svg
//! cargo run --release --features screening --example screen -- plain out.jsonl drafts/a
//! cargo run --release --features screening --example screen -- screened model.json 24 out.jsonl drafts/a
//! cargo run --release --features screening,trace-candidates --example screen -- features model.json steps.jsonl out.jsonl
//! ```
//!
//! `eval` arranges each SVG (files or directories of them) as `arrange` does
//! and as `screening::arrange_screened` does with the model (an XGBoost
//! model saved as JSON) evaluating `keep` candidates a step, and writes a
//! line per diagram: each one's wall time and badness (the quality measure
//! the tests snapshot).
//!
//! `plain` and `screened` write the same lines with only one of the two:
//! the arrangement without the model does not change with the model, so
//! it is arranged once and kept.
//!
//! `draw` writes each SVG arranged both ways, as `<name>.plain.svg` and
//! `<name>.screened.svg` in the directory given, to look at.
//!
//! `features` writes, for each candidate of each step of the record
//! example's logs, the features the model would be given, to check them
//! against the ones learning computes from the same records.

#[path = "../tests/support/mod.rs"]
#[allow(unused_imports)]
mod support;

use std::io::Write as _;
use std::path::PathBuf;
use std::time::Instant;

use nicevg::screening::{Screen, arrange_screened};

fn svgs(paths: &[String]) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = paths
        .iter()
        .map(PathBuf::from)
        .flat_map(|path| {
            if path.is_dir() {
                std::fs::read_dir(&path)
                    .expect("the directory is readable")
                    .filter_map(|entry| Some(entry.ok()?.path()))
                    .filter(|p| p.extension().is_some_and(|e| e == "svg"))
                    .collect()
            } else {
                vec![path]
            }
        })
        .collect();
    files.sort();
    files
}

/// Arranges each SVG without the model when `plain`, and with it when
/// there is a `screen`.
fn eval(screen: Option<&Screen>, plain: bool, out: &str, paths: &[String]) {
    let mut sink = std::io::BufWriter::new(std::fs::File::create(out).expect("the output can be written"));
    let files = svgs(paths);
    for (index, file) in files.iter().enumerate() {
        let name = file
            .file_stem()
            .map_or_else(String::new, |s| s.to_string_lossy().into_owned());
        let svg = std::fs::read_to_string(file).expect("the SVG is readable");
        let mut line = serde_json::json!({ "diagram": name });
        let mut timed = |key: &str, arrange: &dyn Fn() -> Result<nicevg::FixResult, nicevg::SvgInputError>| {
            let started = Instant::now();
            let arranged = arrange();
            let ms = started.elapsed().as_secs_f64() * 1000.0;
            let result = arranged.ok()?;
            line["nodes"] = result.report.diagram.nodes.len().into();
            line[format!("{key}_ms")] = ms.into();
            line[format!("{key}_badness")] = support::quality_of(&result.report).badness.into();
            Some(ms)
        };
        let plain_ms = if plain { timed("plain", &|| nicevg::arrange(&svg)) } else { Some(0.0) };
        let screened_ms = match screen {
            Some(screen) => timed("screened", &|| arrange_screened(&svg, screen)),
            None => Some(0.0),
        };
        let (Some(plain_ms), Some(screened_ms)) = (plain_ms, screened_ms) else {
            eprintln!("{name}: not readable");
            continue;
        };
        writeln!(sink, "{line}").expect("the line is written");
        sink.flush().expect("the line is written");
        eprintln!("[{}/{}] {name}: {plain_ms:.0} ms, {screened_ms:.0} ms", index + 1, files.len());
    }
}

fn draw(screen: &Screen, out: &str, paths: &[String]) {
    std::fs::create_dir_all(out).expect("the directory can be made");
    for file in svgs(paths) {
        let name = file
            .file_stem()
            .map_or_else(String::new, |s| s.to_string_lossy().into_owned());
        let svg = std::fs::read_to_string(&file).expect("the SVG is readable");
        let plain = nicevg::arrange(&svg).expect("the SVG reads");
        let screened = arrange_screened(&svg, screen).expect("the SVG reads");
        let dir = std::path::Path::new(out);
        std::fs::write(dir.join(format!("{name}.plain.svg")), plain.svg).expect("written");
        std::fs::write(dir.join(format!("{name}.screened.svg")), screened.svg).expect("written");
        eprintln!("{name}");
    }
}

#[cfg(feature = "trace-candidates")]
fn features(screen: &Screen, logs: &str, out: &str) {
    use std::io::BufRead as _;
    let mut sink = std::io::BufWriter::new(std::fs::File::create(out).expect("the output can be written"));
    let logs = std::io::BufReader::new(std::fs::File::open(logs).expect("the logs are readable"));
    for line in logs.lines() {
        let log: serde_json::Value = serde_json::from_str(&line.expect("a log line")).expect("a log is JSON");
        let name = log["diagram"].as_str().unwrap_or_default().to_owned();
        let rows = nicevg::screening::replay_features(&log, screen).unwrap_or_else(|error| panic!("{name}: {error}"));
        for mut row in rows {
            row["diagram"] = serde_json::Value::String(name.clone());
            writeln!(sink, "{row}").expect("the row is written");
        }
    }
    writeln!(sink, "{}", serde_json::json!({ "names": screen.names() })).expect("the names are written");
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let usage = "usage: screen eval|screened <model.json> <keep> <out.jsonl> <svg or directory>... | screen plain <out.jsonl> <svg or directory>... | screen draw <model.json> <keep> <out dir> <svg or directory>... | screen features <model.json> <steps.jsonl> <out.jsonl>";
    let (Some(command), Some(model)) = (args.first(), args.get(1)) else {
        panic!("{usage}");
    };
    if command == "plain" {
        eval(None, true, model, &args[2..]);
        return;
    }
    let text = std::fs::read_to_string(model).expect("the model is readable");
    match command.as_str() {
        "eval" | "screened" => {
            let keep: usize = args.get(2).and_then(|k| k.parse().ok()).expect(usage);
            let screen = Screen::from_xgboost_json(&text, keep).unwrap_or_else(|error| panic!("{error}"));
            eval(Some(&screen), command == "eval", args.get(3).expect(usage), &args[4..]);
        }
        "draw" => {
            let keep: usize = args.get(2).and_then(|k| k.parse().ok()).expect(usage);
            let screen = Screen::from_xgboost_json(&text, keep).unwrap_or_else(|error| panic!("{error}"));
            draw(&screen, args.get(3).expect(usage), &args[4..]);
        }
        #[cfg(feature = "trace-candidates")]
        "features" => {
            let screen = Screen::from_xgboost_json(&text, usize::MAX).unwrap_or_else(|error| panic!("{error}"));
            features(&screen, args.get(2).expect(usage), args.get(3).expect(usage));
        }
        _ => panic!("{usage}"),
    }
}
