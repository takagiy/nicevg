//! Runs the fix tests with recording on, then writes a self-contained HTML
//! gallery of every recorded fix call to `test-results/fix-gallery.html`.
//!
//! ```sh
//! cargo run --example fix-gallery
//! ```

use std::collections::HashMap;
use std::path::Path;
use std::process::{Command, ExitCode};

use regex::Regex;
use serde_json::{Value, json};

fn main() -> ExitCode {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let results = root.join("test-results");
    let records = results.join("fix-records");
    let _ = std::fs::remove_dir_all(&records);
    std::fs::create_dir_all(&records).expect("results directory");

    let run = Command::new(env!("CARGO"))
        .args(["test", "--test", "fix", "--", "--color", "never"])
        .current_dir(root)
        .env("NICEVG_RECORD_DIR", &records)
        .env("CI", "true")
        .output()
        .expect("cargo test runs");
    let log = String::from_utf8_lossy(&run.stdout);
    print!("{log}");
    eprint!("{}", String::from_utf8_lossy(&run.stderr));

    let outcomes = outcomes(&log);
    let failures = failure_messages(&log);
    let source = std::fs::read_to_string(root.join("tests/fix.rs")).expect("fix tests");
    let scenario = Regex::new(r"((?:/// [^\n]*\n)+)#\[test\]\nfn (\w+)\(").expect("valid pattern");
    let cases: Vec<Value> = scenario
        .captures_iter(&source)
        .map(|captures| {
            let name = &captures[2];
            let text: Vec<&str> = captures[1]
                .lines()
                .map(|line| line.trim_start_matches("/// "))
                .collect();
            let status = outcomes.get(name).map_or("skipped", String::as_str);
            json!({
                "title": name.replace('_', " "),
                "scenario": text.join("\n"),
                "outcome": match failures.get(name) {
                    Some(message) => json!({ "status": status, "message": message }),
                    None => json!({ "status": status }),
                },
                "calls": calls(&records.join(format!("{name}.jsonl"))),
            })
        })
        .collect();

    let generated = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_millis());
    let data = json!({ "generatedAt": generated, "exitCode": run.status.code(), "cases": cases })
        .to_string()
        .replace('<', "\\u003c");
    let template = std::fs::read_to_string(root.join("tools/fix-gallery/template.html")).expect("template");
    let output = results.join("fix-gallery.html");
    std::fs::write(&output, template.replace("__GALLERY_DATA__", &data)).expect("gallery written");

    let failed = cases
        .iter()
        .filter(|case| case["outcome"]["status"] == "failed")
        .count();
    println!(
        "\nfix gallery: {} cases, {failed} failed -> {}",
        cases.len(),
        output.display()
    );
    if run.status.success() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// `test name ... ok` lines from the test log.
fn outcomes(log: &str) -> HashMap<String, String> {
    let line = Regex::new(r"(?m)^test (\w+) \.\.\. (ok|FAILED|ignored)").expect("valid pattern");
    line.captures_iter(log)
        .map(|captures| {
            let status = match &captures[2] {
                "ok" => "passed",
                "FAILED" => "failed",
                _ => "skipped",
            };
            (captures[1].to_owned(), status.to_owned())
        })
        .collect()
}

/// The panic output printed for each failing test.
fn failure_messages(log: &str) -> HashMap<String, String> {
    let section = Regex::new(r"(?ms)^---- (\w+) stdout ----\n(.*?)(?:\n\n|\z)").expect("valid pattern");
    section
        .captures_iter(log)
        .map(|captures| (captures[1].to_owned(), captures[2].to_owned()))
        .collect()
}

fn calls(file: &Path) -> Vec<Value> {
    std::fs::read_to_string(file)
        .map(|text| {
            text.lines()
                .filter_map(|line| serde_json::from_str(line).ok())
                .collect()
        })
        .unwrap_or_default()
}
