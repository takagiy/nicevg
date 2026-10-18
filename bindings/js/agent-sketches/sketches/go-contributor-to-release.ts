// Go — a contribution from issue and Gerrit CL through maintainer submission, the release freeze and stable publication.
// Sources: https://go.dev/doc/contribute
//          https://go.dev/wiki/Go-Release-Cycle
//          https://go.dev/doc/devel/release
// This follows one normal main-repository change as it enters the six-month release cycle. Review, TryBot failures, and freeze policy can send the change back for revision or defer it.
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
    { id: "contributor", class: "box", x: 20, y: 150, label: "Contributor\nedit Go source; gofmt; test" },
    { id: "issue", class: "box", x: 250, y: 30, label: "GitHub issue\ndesign / bug consensus" },
    { id: "commit", class: "box", x: 250, y: 260, label: "git-codereview change\nsingle commit + Change-Id" },
    { id: "gerrit", class: "box", x: 500, y: 150, label: "Gerrit CL\ngit-codereview mail" },
    { id: "review", class: "box", x: 750, y: 30, label: "Maintainer review\ncomments; revise same CL" },
    { id: "trybots", class: "box", x: 750, y: 260, label: "TryBots\nfull tests on architectures" },
    { id: "submit", class: "box", x: 1000, y: 150, label: "Maintainer submit\nCode-Review +2; no Hold +1" },
    { id: "main", class: "box", x: 1230, y: 30, label: "go.googlesource.com/go\nsubmitted commit on main" },
    { id: "freeze", class: "box", x: 1230, y: 260, label: "Release freeze\nonly fixes, tests, docs accepted" },
    { id: "rc", class: "box", x: 1470, y: 30, label: "Release candidate\ncritical fixes only after RC1" },
    { id: "stable", class: "box", x: 1470, y: 260, label: "Go stable release\ngoX.Y.0; tagged source + binaries" },
    { id: "fix", class: "box", x: 990, y: 400, label: "Revise / retest\nnew patch set to same Gerrit CL" },
  ],
  edges: [
    { from: "contributor", to: "issue", label: "link existing issue; agree scope" },
    { from: "issue", to: "commit", label: "issue understood; start branch from master" },
    { from: "contributor", to: "commit", label: "stage edits; codereview change" },
    { from: "commit", to: "gerrit", label: "codereview mail" },
    { from: "gerrit", to: "review", label: "CL for code review" },
    { from: "gerrit", to: "trybots", label: "maintainers trigger full test run" },
    { from: "review", to: "fix", label: "comments / failed checks" },
    { from: "trybots", to: "fix", label: "failure logs; fix and upload again" },
    { from: "fix", to: "gerrit", label: "amend commit; mail updated CL" },
    { from: "review", to: "submit", label: "+2 approval; no Hold +1" },
    { from: "trybots", to: "submit", label: "tests pass; submit when ready" },
    { from: "submit", to: "main", label: "cherry-pick into repository" },
    { from: "main", to: "freeze", label: "development window closes" },
    { from: "freeze", to: "rc", label: "test / polish; issue RC1" },
    { from: "rc", to: "stable", label: "stabilized bits; scheduled release" },
    { from: "freeze", to: "fix", label: "feature held for next cycle; fixes remain eligible" },
  ],
  style: { box },
} satisfies Sketch<Node>;

export default sketch;
