import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";

// CPython Developer's Guide:
// https://devguide.python.org/contrib/code/pull-request-lifecycle/
// https://devguide.python.org/getting-started/git-boot-camp/
// https://devguide.python.org/developer-workflow/development-cycle/
// A bug fix starts on main, is reviewed and merged, then can be backported by miss-islington
// to a maintenance branch; the release manager prepares that branch's next micro release.

export default {
  nodes: [
    { id: "contributor", class: "box", x: 30, y: 170, label: "Contributor\nbranch from upstream/main" },
    { id: "pull-request", class: "box", x: 250, y: 170, label: "GitHub PR to python/cpython\nCI + core review" },
    { id: "main", class: "box", x: 500, y: 170, label: "CPython main\nmerged fix" },
    { id: "backport-label", class: "box", x: 740, y: 170, label: "needs backport to X.Y\ncore developer labels PR" },
    { id: "miss-islington", class: "box", x: 990, y: 170, label: "miss-islington\nautomatic cherry-pick" },
    { id: "manual-cherry-pick", class: "box", x: 990, y: 360, label: "cherry_picker\nresolve conflicts; continue" },
    { id: "maintenance-pr", class: "box", x: 1240, y: 170, label: "PR to X.Y maintenance branch\nreview and merge backport" },
    { id: "micro-release", class: "box", x: 1490, y: 170, label: "Release Manager\nX.Y.Z micro release" },
  ],
  edges: [
    { from: "contributor", to: "pull-request", label: "push commits; open PR" },
    { from: "pull-request", to: "contributor", label: "review comments; revised commits" },
    { from: "pull-request", to: "main", label: "checks pass; core developer merges" },
    { from: "main", to: "backport-label", label: "fix warrants older bugfix branch" },
    { from: "backport-label", to: "miss-islington", label: "label names target X.Y" },
    { from: "miss-islington", to: "maintenance-pr", label: "clean cherry-pick opens backport PR" },
    { from: "miss-islington", to: "manual-cherry-pick", label: "conflict: report commit hash" },
    { from: "manual-cherry-pick", to: "maintenance-pr", label: "resolve, cherry_picker --continue" },
    { from: "maintenance-pr", to: "micro-release", label: "approved fix merged; branch ships micro releases" },
  ],
  style: { box },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<{ id: string; class: "box"; x: number; y: number; label: string }>;
