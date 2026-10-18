//! Makes the records `examples/record.rs` writes again from its logs,
//! without arranging: each step's document is rebuilt from the log and
//! what could be known of each candidate is worked out anew, with the
//! gains, moves taken and times the log kept.
//!
//! ```text
//! cargo run --release --features trace-candidates --example replay -- steps/gen.jsonl gen.jsonl
//! ```
//!
//! Diagrams replay side by side, a batch at a time, and their records are
//! written in the logs' order.

use std::io::{BufRead as _, Write as _};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [logs, out] = args.as_slice() else {
        panic!("usage: replay <steps.jsonl> <out.jsonl>");
    };
    let threads = std::thread::available_parallelism().map_or(1, |count| count.get());
    let mut lines = std::io::BufReader::new(std::fs::File::open(logs).expect("the logs are readable")).lines();
    let mut sink = std::io::BufWriter::new(std::fs::File::create(out).expect("the output can be written"));
    let mut failed = 0;
    loop {
        let batch: Vec<String> = lines
            .by_ref()
            .take(threads * 4)
            .map(|line| line.expect("a log line"))
            .collect();
        if batch.is_empty() {
            break;
        }
        let replayed: Vec<(String, Result<Vec<serde_json::Value>, String>)> = std::thread::scope(|scope| {
            let jobs: Vec<_> = batch
                .chunks(batch.len().div_ceil(threads))
                .map(|chunk| {
                    scope.spawn(move || {
                        chunk
                            .iter()
                            .map(|line| {
                                let log: serde_json::Value = serde_json::from_str(line).expect("a log is JSON");
                                let name = log["diagram"].as_str().unwrap_or_default().to_owned();
                                (name, nicevg::candidate_trace::replay(&log))
                            })
                            .collect::<Vec<_>>()
                    })
                })
                .collect();
            jobs.into_iter()
                .flat_map(|job| job.join().expect("a replay thread"))
                .collect()
        });
        for (name, records) in replayed {
            match records {
                Ok(records) => {
                    for mut record in records {
                        record["diagram"] = serde_json::Value::String(name.clone());
                        writeln!(sink, "{record}").expect("the record is written");
                    }
                }
                Err(error) => {
                    eprintln!("{name}: {error}");
                    failed += 1;
                }
            }
        }
    }
    assert_eq!(failed, 0, "every log replays");
}
