use std::io::{Read, Write};
use std::process::ExitCode;

use clap::Parser;

/// Repair mechanical layout errors in an SVG diagram read from stdin and
/// write the result to stdout.
#[derive(Parser)]
#[command(name = "nicevg", version)]
struct Cli {}

fn main() -> ExitCode {
    Cli::parse();
    let mut input = String::new();
    if let Err(error) = std::io::stdin().read_to_string(&mut input) {
        eprintln!("Invalid SVG: {error}");
        return ExitCode::from(2);
    }
    match nicevg::fix(&input) {
        Ok(result) => {
            let mut stdout = std::io::stdout().lock();
            let _ = writeln!(stdout, "{}", result.svg);
            for issue in &result.report.issues {
                eprintln!("{}: {}", issue.code, issue.message);
            }
            if result.report.valid {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        Err(error) => {
            eprintln!("Invalid SVG: {error}");
            ExitCode::from(2)
        }
    }
}
