// Kubernetes — an enhancement contribution through KEP tracking, PR review, freeze, and a minor release.
// The slice follows one feature from SIG planning and production-readiness review through k/k CI,
// code freeze, the release branch, and publication of the official release artifacts.
// Sources: https://kubernetes.io/releases/release/
//          https://kubernetes.io/blog/2022/08/11/enhancing-kubernetes-one-kep-at-a-time/
//          https://kubernetes.dev/docs/guide/release-notes/
//          https://github.com/kubernetes/sig-release/blob/master/release-engineering/handbooks/release-manager.md
import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";

type Node = {
  id: string;
  class: "box";
  x: number;
  y: number;
  label: string;
  parent?: string;
  fill?: string;
};

const sketch = {
  nodes: [
    { id: "contributor", class: "box", x: 20, y: 150, label: "Contributor\nproposes user-visible enhancement", fill: "#eff6ff" },
    { id: "sig", class: "box", x: 250, y: 60, label: "Relevant SIG\ndiscuss design + ownership" },
    { id: "enhancements", class: "box", x: 250, y: 200, label: "kubernetes/enhancements\nKEP PR + issue" },
    { id: "kep", class: "box", x: 480, y: 60, label: "KEP\nproblem, proposal, test plan, graduation criteria" },
    { id: "prr", class: "box", x: 480, y: 210, label: "Production Readiness Review\noperational + support questions" },
    { id: "lead", class: "box", x: 720, y: 60, label: "Release Team\nEnhancements Lead checks artifacts" },
    { id: "efreeze", class: "box", x: 960, y: 60, label: "Enhancements Freeze\n~week 4; KEP complete to opt in" },
    { id: "tracked", class: "box", x: 1190, y: 20, label: "KEP tracked\nstatus meets requirements" },
    { id: "atrisk", class: "box", x: 1190, y: 130, label: "At risk\nmissing requirement; author notified" },
    { id: "prune", class: "box", x: 1420, y: 130, label: "Pruned from milestone\nnot ready by freeze" },
    { id: "tasks", class: "box", x: 1190, y: 260, label: "GitHub task issue(s)\nimplementation work in k/k" },
    { id: "milestone", class: "box", x: 1420, y: 260, label: "Release milestone v1.y\nProw /milestone; SIG triages" },
    { id: "branch", class: "box", x: 250, y: 440, label: "Contributor branch\ncode + regression tests", fill: "#eff6ff" },
    { id: "pr", class: "box", x: 480, y: 440, label: "kubernetes/kubernetes PR\nuser-facing release note or NONE" },
    { id: "labels", class: "box", x: 720, y: 440, label: "Prow metadata\n/sig, /kind, milestone hint" },
    { id: "prow", class: "box", x: 960, y: 440, label: "Prow + Tide\nlabels and merge requirements" },
    { id: "review", class: "box", x: 720, y: 590, label: "SIG reviewers\n/lgtm then /approved" },
    { id: "presubmit", class: "box", x: 960, y: 590, label: "Presubmit CI\nrequired tests on PR" },
    { id: "revise", class: "box", x: 480, y: 590, label: "Contributor revises\naddress review / test failures", fill: "#eff6ff" },
    { id: "merge", class: "box", x: 1190, y: 440, label: "k/k master\nPR merged; commit available" },
    { id: "auto", class: "box", x: 1420, y: 440, label: "Milestone automation\nauto-assign merged PR milestone" },
    { id: "enhancecheck", class: "box", x: 1650, y: 260, label: "Enhancements subteam\nfind all KEP implementation PRs" },
    { id: "codefreeze", class: "box", x: 1880, y: 260, label: "Code Freeze\n~week 12; implementation must land" },
    { id: "allmerged", class: "box", x: 2110, y: 220, label: "All KEP PRs merged\nstatus updated for release" },
    { id: "incomplete", class: "box", x: 2110, y: 350, label: "Incomplete / unstable\nfeature removed from release" },
    { id: "stabilize", class: "box", x: 2340, y: 220, label: "Stabilization\nflake fixes, regressions, docs finalized" },
    { id: "critical", class: "box", x: 2340, y: 380, label: "Critical bug PR only\nfreeze gate: /milestone v1.y + bug/failing-test" },
    { id: "thaw", class: "box", x: 2570, y: 220, label: "Code Thaw\nmaster reopens for next release" },
    { id: "rc", class: "box", x: 1650, y: 60, label: "v1.y-rc.0\nrelease candidate cut" },
    { id: "releasebranch", class: "box", x: 1880, y: 60, label: "release-1.y\nbranch created at rc.0" },
    { id: "cherrypick", class: "box", x: 2570, y: 400, label: "Remaining current-release fix\ncherry-pick master → release-1.y" },
    { id: "build", class: "box", x: 2800, y: 220, label: "Release build\ncut from release-1.y" },
    { id: "artifacts", class: "box", x: 3030, y: 220, label: "GitHub kubernetes/kubernetes Releases\ntag + release artifacts" },
    { id: "changelog", class: "box", x: 3030, y: 380, label: "CHANGELOG-X.Y.md\nautomatically loaded" },
    { id: "announce", class: "box", x: 3260, y: 220, label: "Kubernetes v1.y official release\nusers install / upgrade" },
  ] satisfies Node[],
  edges: [
    { from: "contributor", to: "sig", label: "proposal + design discussion" },
    { from: "sig", to: "enhancements", label: "SIG opts feature into release" },
    { from: "enhancements", to: "kep", label: "KEP design and implementation plan" },
    { from: "kep", to: "prr", label: "operational readiness review" },
    { from: "kep", to: "lead", label: "planning artifacts for target release" },
    { from: "prr", to: "lead", label: "PRR completion" },
    { from: "lead", to: "efreeze", label: "check at ~week 4 freeze" },
    { from: "efreeze", to: "tracked", label: "requirements complete" },
    { from: "efreeze", to: "atrisk", label: "requirements missing" },
    { from: "atrisk", to: "contributor", label: "GitHub issue comment: update KEP" },
    { from: "atrisk", to: "prune", label: "still at risk at freeze" },
    { from: "tracked", to: "tasks", label: "tracked enhancement implementation" },
    { from: "tasks", to: "milestone", label: "issue marked for v1.y" },
    { from: "contributor", to: "branch", label: "implement feature + tests" },
    { from: "branch", to: "pr", label: "push and open PR" },
    { from: "pr", to: "labels", label: "PR title/body, user-facing release note" },
    { from: "labels", to: "prow", label: "/sig + /kind; normal-dev checks" },
    { from: "pr", to: "review", label: "code and tests for SIG review" },
    { from: "pr", to: "presubmit", label: "PR event starts CI" },
    { from: "review", to: "prow", label: "/lgtm and /approved" },
    { from: "presubmit", to: "prow", label: "required jobs pass" },
    { from: "review", to: "revise", label: "requested changes" },
    { from: "presubmit", to: "revise", label: "failed test feedback" },
    { from: "revise", to: "pr", label: "push revision; rerun review + CI" },
    { from: "prow", to: "merge", label: "Tide merges when requirements pass" },
    { from: "merge", to: "auto", label: "merged PR into master" },
    { from: "auto", to: "milestone", label: "automated milestone tracking" },
    { from: "milestone", to: "enhancecheck", label: "PRs associated with KEP" },
    { from: "enhancecheck", to: "codefreeze", label: "verify all implementation merged by deadline" },
    { from: "codefreeze", to: "allmerged", label: "all enhancement PRs landed" },
    { from: "codefreeze", to: "incomplete", label: "code missing or unstable" },
    { from: "allmerged", to: "stabilize", label: "freeze complete; stabilize codebase" },
    { from: "incomplete", to: "prune", label: "drop enhancement from milestone" },
    { from: "stabilize", to: "critical", label: "remaining critical issue?" },
    { from: "critical", to: "prow", label: "only critical bug fix PR may merge" },
    { from: "critical", to: "cherrypick", label: "late fix for current release" },
    { from: "stabilize", to: "thaw", label: "stable; next release development resumes" },
    { from: "rc", to: "releasebranch", label: "create release-1.y at rc.0" },
    { from: "merge", to: "cherrypick", label: "remaining fix already on master" },
    { from: "cherrypick", to: "releasebranch", label: "picked commit enters release branch" },
    { from: "releasebranch", to: "build", label: "release built from branch" },
    { from: "build", to: "artifacts", label: "tag and publish binaries / artifacts" },
    { from: "build", to: "changelog", label: "release note data" },
    { from: "artifacts", to: "announce", label: "official release available" },
    { from: "changelog", to: "announce", label: "versioned user-facing changes" },
  ],
  style: { box },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<Node>;

export default sketch;
