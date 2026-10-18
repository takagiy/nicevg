import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";

// Blender Developer Documentation:
// https://developer.blender.org/docs/handbook/contributing/
// https://developer.blender.org/docs/handbook/contributing/pull_requests/
// https://developer.blender.org/docs/handbook/release_process/release_cycle/
// https://developer.blender.org/docs/handbook/release_process/release_checklist/
// A code change is proposed and reviewed on projects.blender.org, then enters main.
// At Beta, main is forked for stabilization; approved release builds are tagged and published.

export default {
  nodes: [
    { id: "contributor", class: "box", x: 30, y: 160, label: "Contributor" },
    { id: "agreement", class: "box", x: 30, y: 300, label: "Accept blender-contributor-agreement" },
    { id: "feature-branch", class: "box", x: 230, y: 160, label: "Local feature branch\ncommit change" },
    { id: "pull-request", class: "box", x: 450, y: 160, label: "projects.blender.org\nPull Request to main" },
    { id: "module-review", class: "box", x: 700, y: 160, label: "Module owner / reviewers\nfunctional + technical review" },
    { id: "revision", class: "box", x: 700, y: 320, label: "Address review feedback\npush updated commits" },
    { id: "main", class: "box", x: 950, y: 160, label: "blender.git: main\nmerged change" },
    { id: "beta", class: "box", x: 1170, y: 160, label: "Beta: fork\nblender-v{VERSION}-release" },
    { id: "rc-builds", class: "box", x: 1380, y: 160, label: "Release Candidate\nfreeze; make candidate builds" },
    { id: "build-approval", class: "box", x: 1380, y: 330, label: "Build approved" },
    { id: "tags", class: "box", x: 1150, y: 430, label: "Tag repositories\nv{VERSION} release" },
    { id: "platform-builds", class: "box", x: 900, y: 430, label: "Final platform builds\npackage for release" },
    { id: "blender-org", class: "box", x: 650, y: 430, label: "blender.org\nrelease shown on Downloads" },
  ],
  edges: [
    { from: "contributor", to: "agreement", label: "one-time account setup" },
    { from: "contributor", to: "feature-branch", label: "implement change" },
    { from: "feature-branch", to: "pull-request", label: "push branch; describe rationale" },
    { from: "agreement", to: "pull-request", label: "required contributor terms" },
    { from: "pull-request", to: "module-review", label: "CODEOWNERS may add reviewers" },
    { from: "module-review", to: "revision", label: "request changes" },
    { from: "revision", to: "pull-request", label: "updated commits / review iteration" },
    { from: "module-review", to: "main", label: "approve and merge" },
    { from: "main", to: "beta", label: "branch at Beta; change is included" },
    { from: "beta", to: "rc-builds", label: "bug fixing, then freeze" },
    { from: "rc-builds", to: "build-approval", label: "candidate build review" },
    { from: "build-approval", to: "tags", label: "only after approval" },
    { from: "tags", to: "platform-builds", label: "tagged source and libraries" },
    { from: "platform-builds", to: "blender-org", label: "final release publication" },
  ],
  style: { box },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<{ id: string; class: "box"; x: number; y: number; label: string }>;
