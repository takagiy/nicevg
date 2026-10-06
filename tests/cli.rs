//! The command line: SVG in on stdin, fixed SVG out on stdout.

mod support;

use std::io::Write;
use std::process::{Command, Stdio};

use support::*;

struct Run {
    status: i32,
    stdout: String,
    stderr: String,
}

fn run(input: &str, arguments: &[&str]) -> Run {
    let mut child = Command::new(env!("CARGO_BIN_EXE_nicevg"))
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the CLI starts");
    // The CLI may reject its arguments before reading stdin.
    let _ = child.stdin.take().expect("stdin").write_all(input.as_bytes());
    let output = child.wait_with_output().expect("the CLI finishes");
    Run {
        status: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

/// Given the installed command
/// When its help is displayed
/// Then the public tool name is nicevg
#[test]
fn help_uses_nicevg_as_the_command_name() {
    let result = run("", &["--help"]);

    assert_eq!(result.status, 0);
    assert!(result.stdout.contains("Usage: nicevg"));
}

/// Given a clipped SVG diagram on standard input
/// When the CLI runs
/// Then the fixed SVG, now free of issues, is written to standard output
///   and the command succeeds
#[test]
fn fixes_svg_from_stdin_and_writes_it_to_stdout() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 120">
      <g data-node="checkout">
        <rect x="20" y="20" width="100" height="56"/>
        <text x="70" y="53" text-anchor="middle" font-size="14">Checkout</text>
      </g>
    </svg>"#;

    let result = run(svg, &[]);

    assert_eq!(result.status, 0);
    assert_eq!(result.stderr, "");
    assert!(!analyze(svg).issues.is_empty());
    assert!(analyze(&result.stdout).issues.is_empty());
}

/// Given two free labels that overlap, which fix does not move
/// When the CLI runs
/// Then it still writes the SVG to standard output, lists the remaining
///   issue on standard error and exits with status 1
#[test]
fn reports_issues_fix_cannot_resolve_and_exits_with_status_1() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 300 120">
      <text data-label="first" x="40" y="60" font-size="14">Overlap</text>
      <text data-label="second" x="50" y="62" font-size="14">Overlap</text>
    </svg>"#;

    let result = run(svg, &[]);

    assert_eq!(result.status, 1);
    assert_eq!(issue_codes(&analyze(&result.stdout)), ["label-overlap"]);
    assert!(result.stderr.contains("label-overlap"));
}

/// Given malformed SVG on standard input
/// When the CLI runs
/// Then it reports an input error without a stack trace and exits with
///   status 2
#[test]
fn invalid_svg_exits_with_status_2_and_a_concise_input_error() {
    let result = run("<svg><g></svg>", &[]);

    assert_eq!(result.status, 2);
    assert_eq!(result.stdout, "");
    assert!(result.stderr.starts_with("Invalid SVG:"));
    assert!(!result.stderr.contains("panicked"));
}

/// Given a valid diagram on standard input
/// When the CLI is also given an argument, as the old fix and check
///   commands and the old --arrange option took
/// Then it fails without writing anything to standard output
#[test]
fn rejects_file_and_subcommand_arguments_because_input_is_stdin_only() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"/>"#;

    for argument in ["fix", "diagram.svg", "--arrange"] {
        let result = run(svg, &[argument]);

        assert_ne!(result.status, 0);
        assert_eq!(result.stdout, "");
        assert_ne!(result.stderr, "");
    }
}

/// Given a diagram whose connector bends only because one node sits 24px
///   off, on standard input
/// When the CLI runs
/// Then it writes the arranged SVG, with the connector straight, and
///   succeeds
#[test]
fn arranges_nodes_by_default() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 420 200">
      <g data-node="notify"><circle cx="100" cy="110" r="54"/><text x="100" y="115" text-anchor="middle" font-size="13">7 Notify</text></g>
      <g data-node="mail"><rect x="280" y="60" width="130" height="52"/><text x="345" y="91" text-anchor="middle" font-size="13">Email service</text></g>
      <path id="email" data-from="notify" data-to="mail" d="M 154 110 L 220 110 L 220 86 L 280 86"/>
    </svg>"#;

    let result = run(svg, &[]);

    assert_eq!(result.status, 0);
    assert_eq!(bend_count(&connector_points(&analyze(svg), "email")), 2);
    assert_eq!(bend_count(&connector_points(&analyze(&result.stdout), "email")), 0);
}
