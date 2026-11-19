//! Times arranging each fixture and prints a fingerprint of the result, so a
//! change can be checked to leave every output as it was:
//!
//! ```text
//! cargo run --release --example bench -- 5
//! ```

use std::hash::{Hash, Hasher};

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[path = "../tests/support/mod.rs"]
mod support;

fn main() {
    let runs: usize = std::env::args()
        .nth(1)
        .map_or(5, |n| n.parse().expect("a number of runs"));
    let mut paths: Vec<_> = std::fs::read_dir("tests/fixtures")
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    paths.sort();
    let mut total = 0.0;
    for path in &paths {
        let svg = std::fs::read_to_string(path).unwrap();
        let mut best = f64::INFINITY;
        let mut output = String::new();
        for _ in 0..runs {
            let start = std::time::Instant::now();
            let result = nicevg::arrange(&svg).unwrap();
            best = best.min(start.elapsed().as_secs_f64() * 1000.0);
            output = result.svg;
        }
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        output.hash(&mut hasher);
        let report = nicevg::analyze(&output).unwrap();
        let quality = support::quality_of(&report);
        total += best;
        println!(
            "{:40} {:8.1} ms  hash {:016x}  badness {:.2}",
            path.file_name().unwrap().to_string_lossy(),
            best,
            hasher.finish(),
            quality.badness
        );
    }
    println!("{:40} {:8.1} ms", "total", total);
}
