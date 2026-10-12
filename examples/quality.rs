//! Prints the quality measures the snapshots record for each SVG diagram
//! given, one JSON object per line, with the diagram as read:
//!
//! ```text
//! cargo run --release --example quality -- a.svg b.svg
//! ```

#[path = "../tests/support/mod.rs"]
mod support;

fn main() {
    for path in std::env::args().skip(1) {
        let svg = std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{path}: {error}"));
        let report = nicevg::analyze(&svg).unwrap_or_else(|error| panic!("{path}: {error}"));
        let line = serde_json::json!({
            "file": path,
            "quality": support::quality_of(&report),
            "diagram": report.diagram,
            "issues": report.issues,
        });
        println!("{line}");
    }
}
