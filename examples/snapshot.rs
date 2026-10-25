//! The diagram at each step of recorded arrangements, for learning that
//! reads the drawing itself rather than features made from it.
//!
//! ```text
//! cargo run --release --features trace-candidates --example snapshot -- diagrams.txt out.jsonl steps/a.jsonl steps/b.jsonl
//! ```
//!
//! `diagrams.txt` names the diagrams to write, one a line; each written
//! line is one step of one diagram (`candidate_trace::snapshots`).
//! Diagrams are rebuilt side by side, a batch at a time, as the replay
//! example does.

use std::collections::HashSet;
use std::io::{BufRead as _, Write as _};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [names, out, logs @ ..] = args.as_slice() else {
        panic!("usage: snapshot <diagrams.txt> <out.jsonl> <steps.jsonl>...");
    };
    let wanted: HashSet<String> = std::fs::read_to_string(names)
        .expect("the names are readable")
        .lines()
        .map(|line| line.trim().to_owned())
        .filter(|line| !line.is_empty())
        .collect();
    let threads = std::thread::available_parallelism().map_or(1, |count| count.get());
    let mut sink = std::io::BufWriter::new(std::fs::File::create(out).expect("the output can be written"));
    for path in logs {
        let mut lines = std::io::BufReader::new(std::fs::File::open(path).expect("the log is readable"))
            .lines()
            .map(|line| line.expect("a log line"))
            // Parse only the wanted ones, read by the name field alone.
            .filter(|line| {
                line.rfind("\"diagram\":\"").is_some_and(|at| {
                    let rest = &line[at + "\"diagram\":\"".len()..];
                    rest.find('"').is_some_and(|end| wanted.contains(&rest[..end]))
                })
            });
        loop {
            let batch: Vec<String> = lines.by_ref().take(threads * 2).collect();
            if batch.is_empty() {
                break;
            }
            let done: Vec<(String, Result<Vec<serde_json::Value>, String>)> = std::thread::scope(|scope| {
                let jobs: Vec<_> = batch
                    .iter()
                    .map(|line| {
                        scope.spawn(move || {
                            let log: serde_json::Value = serde_json::from_str(line).expect("a log is JSON");
                            let name = log["diagram"].as_str().unwrap_or_default().to_owned();
                            (name, nicevg::candidate_trace::snapshots(&log))
                        })
                    })
                    .collect();
                jobs.into_iter()
                    .map(|job| job.join().expect("a snapshot thread"))
                    .collect()
            });
            for (name, steps) in done {
                match steps {
                    Ok(steps) => {
                        for mut step in steps {
                            step["diagram"] = serde_json::Value::String(name.clone());
                            writeln!(sink, "{step}").expect("the step is written");
                        }
                    }
                    Err(error) => eprintln!("{name}: {error}"),
                }
            }
        }
    }
}
