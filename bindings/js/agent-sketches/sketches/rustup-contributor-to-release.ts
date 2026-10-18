/**
 * Rustup release workflow, based on https://github.com/rust-lang/rustup/blob/main/doc/dev-guide/src/release-process.md and https://github.com/rust-lang/rustup/blob/main/CONTRIBUTING.md.
 * It follows a contributor change through PR validation, release preparation, stable validation and publication; CI and installer checks are the concern running across the path.
 */
import type { Sketch } from "nicevg";
import { area, table, type ErNode } from "../styles/er.ts";

export default {
  nodes: [
    { id: "release-workflow", class: "area", x: 28, y: 24, title: "rustup · contributor to release / validation", width: 760, height: 620 },
    { id: "contributor-pr", class: "table", x: 48, y: 82, parent: "release-workflow", title: "Contributor PR", columns: [["base", "main | release/X.Y"], ["commit", "change"], ["label", "backport?"], ["status", "review"]] },
    { id: "main", class: "table", x: 286, y: 68, parent: "release-workflow", title: "main", columns: [["tip", "commit SHA", "PK"], ["version", "Cargo.toml"], ["CI", "ci.yaml"]] },
    { id: "cargo", class: "table", x: 500, y: 82, parent: "release-workflow", title: "Cargo.toml + Cargo.lock", columns: [["version", "X.Y.0"], ["dependencies", "locked"], ["build", "cargo build"]] },
    { id: "changelog", class: "table", x: 92, y: 248, parent: "release-workflow", title: "CHANGELOG.md", columns: [["entry", "release notes"], ["version", "$VER_NUM"]] },
    { id: "release-branch", class: "table", x: 328, y: 256, parent: "release-workflow", title: "release/X.Y", columns: [["tip", "backported commit", "PK"], ["PR label", "backport"], ["CI", "manual ci.yaml"]] },
    { id: "stable", class: "table", x: 554, y: 270, parent: "release-workflow", title: "stable", columns: [["tip", "expected commit", "PK"], ["version", "$VER_NUM"], ["CI", "all green"]] },
    { id: "installer", class: "table", x: 36, y: 440, parent: "release-workflow", title: "rustup-init.sh + help snapshot", columns: [["version", "$VER_NUM"], ["commit shasum", "branch tip"], ["snapshot", "--help"], ["check", "cargo test"]] },
    { id: "ci-run", class: "table", x: 290, y: 434, parent: "release-workflow", title: "CI / release qualification", columns: [["workflow", "ci.yaml"], ["ref", "main | release/X.Y | stable"], ["result", "green"], ["checks", "installer + toolchains"]] },
    { id: "release-team", class: "table", x: 532, y: 452, parent: "release-workflow", title: "Release team · ci/sync-dist.py", columns: [["mode", "beta | official"], ["branch", "main | release/X.Y"], ["manual test", "approve / abort"]] },
    { id: "published", class: "table", x: 586, y: 570, parent: "release-workflow", title: "static.rust-lang.org/rustup", columns: [["dist", "platform rustup-init"], ["channel", "dev-static beta | static official"], ["tag", "$VER_NUM · official"]] },
  ],
  edges: [
    { from: "contributor-pr", to: "main", label: "reviewed merge · main" },
    { from: "contributor-pr", to: "release-branch", label: "backport PR · rebased commits" },
    { from: "main", to: "cargo", label: "minor version bump · build / review lock" },
    { from: "main", to: "release-branch", label: "branch after X.Y.0 beta" },
    { from: "release-branch", to: "cargo", label: "patch version bump · build / review lock" },
    { from: "cargo", to: "changelog", label: "$VER_NUM release entry" },
    { from: "cargo", to: "installer", label: "version + branch-tip SHA" },
    { from: "installer", to: "ci-run", label: "--help snapshot · cargo test" },
    { from: "changelog", to: "stable", label: "release-prep PR merged" },
    { from: "release-branch", to: "stable", label: "sync expected release tip" },
    { from: "stable", to: "ci-run", label: "full CI + reported --version" },
    { from: "installer", to: "release-team", label: "check script + rustup-init" },
    { from: "ci-run", to: "release-team", label: "green CI; toolchain / rust-analyzer ecosystem check" },
    { from: "release-team", to: "published", label: "sync-dist.py · beta or official artifacts" },
    { from: "published", to: "stable", label: "manual testing failure → abort / rollback" },
  ],
  style: { table, area },
} satisfies Sketch<ErNode>;
