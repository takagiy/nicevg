//! Where the gain of chosen moves comes from: for each move a request
//! names (by diagram, step, node ids and step), the score's terms before
//! and after evaluating it, the connectors the one pass of fixing
//! rerouted, and the trial rerouting's change.
//!
//! ```text
//! cargo run --release --features trace-candidates --example explain -- requests.json out.jsonl steps/a.jsonl steps/b.jsonl
//! ```
//!
//! `requests.json` maps a diagram to its moves: `{"d": [[step, "id", dx, dy], ...]}`.

use std::io::{BufRead as _, Write as _};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [requests, out, logs @ ..] = args.as_slice() else {
        panic!("usage: explain <requests.json> <out.jsonl> <steps.jsonl>...");
    };
    let requests: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(requests).expect("requests readable"))
            .expect("requests are JSON");
    let mut sink = std::io::BufWriter::new(std::fs::File::create(out).expect("output writable"));
    for path in logs {
        for line in std::io::BufReader::new(std::fs::File::open(path).expect("log readable")).lines() {
            let line = line.expect("a log line");
            let log: serde_json::Value = serde_json::from_str(&line).expect("a log is JSON");
            let name = log["diagram"].as_str().unwrap_or_default().to_owned();
            let Some(moves) = requests[&name].as_array() else {
                continue;
            };
            let wanted: Vec<(usize, String, f64, f64)> = moves
                .iter()
                .map(|m| {
                    (
                        m[0].as_u64().expect("step") as usize,
                        m[1].as_str().expect("id").to_owned(),
                        m[2].as_f64().expect("dx"),
                        m[3].as_f64().expect("dy"),
                    )
                })
                .collect();
            match nicevg::candidate_trace::explain(&log, &wanted) {
                Ok(rows) => {
                    for mut row in rows {
                        row["diagram"] = serde_json::Value::String(name.clone());
                        writeln!(sink, "{row}").expect("written");
                    }
                }
                Err(error) => eprintln!("{name}: {error}"),
            }
        }
    }
}
