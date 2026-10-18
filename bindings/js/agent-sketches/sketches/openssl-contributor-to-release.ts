// OpenSSL — a normal contribution from a GitHub pull request through the scheduled release process.
// Sources: https://github.com/openssl/openssl/blob/master/CONTRIBUTING.md
//          https://www.openssl-library.org/policies/technical/release-requirements/
//          https://www.openssl-library.org/policies/general/release-policy/
//          https://github.com/openssl/tools/blob/master/HOWTO-make-a-release.md
//          https://github.com/openssl/tools/blob/master/HOWTO-stage-a-release.md
//          https://github.com/openssl/tools/blob/master/HOWTO-publish-a-release.md
// Follows a non-security change into a time-based public release; a review revision loop and the release freeze/validation gates are shown.
import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";

type Node = {
  id: string;
  class: "box";
  x: number;
  y: number;
  label: string;
  parent?: string;
};

const sketch = {
  nodes: [
    { id: "contributor", class: "box", x: 20, y: 170, label: "Contributor\nimplement one logical change; build + test" },
    { id: "change", class: "box", x: 270, y: 30, label: "Patch\ncode + tests; docs / CHANGES.md if applicable" },
    { id: "pull", class: "box", x: 270, y: 250, label: "GitHub pull request\nopenssl/openssl; CLA or CLA: trivial" },
    { id: "committers", class: "box", x: 550, y: 30, label: "OpenSSL Committers\ntriage; review; ≥2 approvals" },
    { id: "checks", class: "box", x: 550, y: 250, label: "GitHub Actions\nclean builds on PR create/update" },
    { id: "revise", class: "box", x: 550, y: 430, label: "Contributor revises\nfixup commits; push update" },
    { id: "merge", class: "box", x: 830, y: 150, label: "Merge-ready PR\napproval: done → 24h → ready to merge" },
    { id: "source", class: "box", x: 1080, y: 150, label: "OpenSSL source tree\nmerged commit; public GitHub mirror" },
    { id: "release_gate", class: "box", x: 1330, y: 30, label: "Release requirements\nrelease milestone closed; CI + daily CI pass" },
    { id: "freeze", class: "box", x: 1330, y: 270, label: "Source freeze\n≥3 business days; regressions / blockers / security only" },
    { id: "stage", class: "box", x: 1580, y: 150, label: "release-tools\nstage-release.sh; update NEWS.md, dates, version" },
    { id: "review_stage", class: "box", x: 1830, y: 30, label: "Release staging review\ninspect commits, announcement, metadata" },
    { id: "tag", class: "box", x: 1830, y: 270, label: "Signed release tag\npush staged source commits + tag" },
    { id: "draft", class: "box", x: 2080, y: 150, label: "GitHub draft release\nautomation creates draft on tag push" },
    { id: "publish", class: "box", x: 2330, y: 30, label: "Publish release\nverify signed .asc; tarball hashes; release notes" },
    { id: "artifacts", class: "box", x: 2580, y: 150, label: "OpenSSL release\nsource tarball + signatures; announcement; download page" },
  ],
  edges: [
    { from: "contributor", to: "change", label: "edit, test, describe rationale" },
    { from: "change", to: "pull", label: "push branch; open PR" },
    { from: "contributor", to: "pull", label: "submit patch and CLA status" },
    { from: "pull", to: "committers", label: "approval: review pending; triage label" },
    { from: "pull", to: "checks", label: "PR commit triggers clean builds" },
    { from: "committers", to: "revise", label: "review comments / requested changes" },
    { from: "checks", to: "revise", label: "failed build / test feedback" },
    { from: "revise", to: "pull", label: "push revision; re-approval required" },
    { from: "committers", to: "merge", label: "2+ approvals; approval: done" },
    { from: "checks", to: "merge", label: "clean build checks pass" },
    { from: "merge", to: "source", label: "after 24h ready-to-merge delay, merge PR" },
    { from: "source", to: "release_gate", label: "change accumulates on release branch" },
    { from: "release_gate", to: "freeze", label: "milestone PRs/issues closed; coverage not lower" },
    { from: "release_gate", to: "source", label: "failed CI / gate: fix before release commits" },
    { from: "freeze", to: "stage", label: "scheduled release preparation" },
    { from: "stage", to: "review_stage", label: "local staged commits + announcement text" },
    { from: "review_stage", to: "stage", label: "correct staging output" },
    { from: "review_stage", to: "tag", label: "approved staged result; publish source tag" },
    { from: "tag", to: "draft", label: "tag push triggers draft creation" },
    { from: "draft", to: "publish", label: "check signature, hashes, and notes; publish" },
    { from: "publish", to: "artifacts", label: "release publicly available; announce" },
  ],
  style: { box },
} satisfies Sketch<Node>;

export default sketch;
