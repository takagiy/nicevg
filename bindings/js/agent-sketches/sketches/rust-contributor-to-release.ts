// Rust — a rust-lang/rust contribution from a forked branch through merge CI, the release train and publication.
// This follows the normal compiler contribution path and a scheduled stable release, including the release team's artifact promotion.
// Sources: https://rustc-dev-guide.rust-lang.org/src/git.html
//          https://rustc-dev-guide.rust-lang.org/src/tests/ci.html
//          https://forge.rust-lang.org/release/process.html
//          https://forge.rust-lang.org/infra/channel-layout.html
import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";

type Node = {
  id: string;
  class: "box";
  x: number;
  y: number;
  label: string;
};

const sketch = {
  nodes: [
    { id: "contributor", class: "box", x: 20, y: 100, label: "Contributor\nedit + test rust-lang/rust" },
    { id: "fork", class: "box", x: 20, y: 300, label: "Fork branch\nissue-12345-fix" },
    { id: "pr", class: "box", x: 270, y: 100, label: "Pull request\nfork → rust-lang/rust:main" },
    { id: "prci", class: "box", x: 270, y: 300, label: "PR CI\npr jobs: tests, tidy, cross-checks" },
    { id: "review", class: "box", x: 520, y: 100, label: "Rust reviewer\nr+ approval" },
    { id: "bors", class: "box", x: 520, y: 300, label: "bors merge queue\napproved PR enqueued" },
    { id: "mergeci", class: "box", x: 770, y: 100, label: "Auto build\nmerge commit + full CI suite" },
    { id: "main", class: "box", x: 770, y: 300, label: "rust-lang/rust main\nmerged code; nightly channel" },
    { id: "beta", class: "box", x: 1020, y: 100, label: "beta branch\ncut from main ahead of release" },
    { id: "stable", class: "box", x: 1020, y: 300, label: "stable branch\nrelease PR updates notes + channel" },
    { id: "dist", class: "box", x: 1270, y: 100, label: "CI dist jobs\n./x dist per platform" },
    { id: "ci2", class: "box", x: 1270, y: 300, label: "rust-lang-ci2 S3\nrelease build artifacts" },
    { id: "release", class: "box", x: 1520, y: 100, label: "release-team\nstart-release.py publish-rust-prod-stable" },
    { id: "static", class: "box", x: 1520, y: 300, label: "static.rust-lang.org\npublished + signed channel manifest" },
  ],
  edges: [
    { from: "contributor", to: "fork", label: "push commits" },
    { from: "fork", to: "pr", label: "open PR" },
    { from: "pr", to: "prci", label: "PR push triggers quick checks" },
    { from: "prci", to: "contributor", label: "failure: fix and push again" },
    { from: "prci", to: "review", label: "checks pass; request review" },
    { from: "review", to: "pr", label: "review feedback; revise PR" },
    { from: "review", to: "bors", label: "approve with r+" },
    { from: "bors", to: "mergeci", label: "front of queue; test merge commit" },
    { from: "mergeci", to: "contributor", label: "failure needs fix" },
    { from: "mergeci", to: "main", label: "all required CI passes; push commit" },
    { from: "main", to: "beta", label: "release train branch promotion" },
    { from: "beta", to: "stable", label: "next scheduled promotion" },
    { from: "stable", to: "dist", label: "stable branch commit builds" },
    { from: "dist", to: "ci2", label: "upload platform artifacts" },
    { from: "ci2", to: "release", label: "copy artifacts from staging" },
    { from: "release", to: "static", label: "promote to production + sign" },
  ],
  style: { box },
} satisfies Sketch<Node>;

export default sketch;
