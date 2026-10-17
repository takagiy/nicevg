//! Arranges diagrams and records each candidate move arrange tries, for
//! learning to screen candidates: what could be known of a candidate
//! before trying it, and the gain trying it showed, one JSON object a line.
//!
//! ```text
//! cargo run --release --features trace-candidates --example record -- drafts candidates.jsonl
//! ```
//!
//! Takes SVG files or directories of them, and writes every record with
//! the diagram it came from. Trying every candidate (none is skipped as
//! unchanged) makes arranging slower than usual.

use std::io::Write as _;
use std::path::PathBuf;
use std::time::Instant;

fn main() {
    let mut args: Vec<PathBuf> = std::env::args().skip(1).map(PathBuf::from).collect();
    let out = args.pop().expect("usage: record <svg or directory>... <out.jsonl>");
    let mut files: Vec<PathBuf> = args
        .iter()
        .flat_map(|path| {
            if path.is_dir() {
                std::fs::read_dir(path)
                    .expect("the directory is readable")
                    .filter_map(|entry| Some(entry.ok()?.path()))
                    .filter(|p| p.extension().is_some_and(|e| e == "svg"))
                    .filter(|p| !p.to_string_lossy().ends_with(".arranged.svg"))
                    .collect()
            } else {
                vec![path.clone()]
            }
        })
        .collect();
    files.sort();
    let mut sink = std::io::BufWriter::new(std::fs::File::create(&out).expect("the output can be written"));
    for (index, file) in files.iter().enumerate() {
        let name = file
            .file_stem()
            .map_or_else(String::new, |s| s.to_string_lossy().into_owned());
        let svg = std::fs::read_to_string(file).expect("the SVG is readable");
        let started = Instant::now();
        let result = nicevg::arrange(&svg);
        let records = nicevg::candidate_trace::take();
        if let Err(error) = result {
            eprintln!("{name}: {error}");
            continue;
        }
        for mut record in records {
            record["diagram"] = serde_json::Value::String(name.clone());
            writeln!(sink, "{record}").expect("the record is written");
        }
        eprintln!(
            "[{}/{}] {name}: {:.1}s",
            index + 1,
            files.len(),
            started.elapsed().as_secs_f64()
        );
    }
}
