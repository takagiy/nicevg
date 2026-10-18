// Homebrew contributor and release process:
// https://docs.brew.sh/How-To-Open-a-Homebrew-Pull-Request
// https://docs.brew.sh/Homebrew-brew-Maintainer-Guide
// https://docs.brew.sh/Releases
// Shows a Homebrew/brew change from a contributor fork through reviewed, passing CI,
// then the maintainer release gates, draft/tag publication, release notes and announcements.
import type { Sketch, SketchNode } from "nicevg";
import { svg, text } from "nicevg";

type Kind = "person" | "repo" | "review" | "automation" | "gate" | "artifact" | "public";
type Node = SketchNode & { class: Kind; title: string; rows: string[]; width?: number };

function card(node: Node) {
  const width = node.width ?? 220;
  const heading = text(node.title, 14);
  const rows = node.rows.map((row) => text(row, 12));
  const height = 36 + rows.reduce((sum, row) => sum + row.height + 6, 0);
  const fill = node.class === "person" ? "#f8efd9" :
    node.class === "automation" ? "#e8f1fa" :
      node.class === "public" ? "#e9f3e7" : "#fffdf7";
  const shape = node.class === "person"
    ? svg`<ellipse data-bounds="true" cx="${width / 2}" cy="${height / 2}" rx="${width / 2}" ry="${height / 2}" fill="${fill}" stroke="#514b40"/>`
    : node.class === "gate"
      ? svg`<path data-bounds="true" d="M14 0 H${width - 14} L${width} ${height / 2} L${width - 14} ${height} H14 L0 ${height / 2} Z" fill="${fill}" stroke="#514b40"/>`
      : node.class === "artifact"
        ? svg`<path data-bounds="true" d="M0 0 H${width - 20} L${width} 20 V${height} H0 Z M${width - 20} 0 V20 H${width}" fill="${fill}" stroke="#514b40"/>`
        : node.class === "review"
          ? svg`<rect data-bounds="true" width="${width}" height="${height}" rx="12" fill="${fill}" stroke="#514b40"/>`
          : svg`<rect data-bounds="true" width="${width}" height="${height}" fill="${fill}" stroke="#514b40"/>`;
  return svg`${shape}${heading.at({ left: 12, top: 8 })}${rows.map((row, i) => row.at({ left: 12, top: 32 + rows.slice(0, i).reduce((sum, prior) => sum + prior.height + 6, 0) }))}`;
}

const person = card, repo = card, review = card, automation = card, gate = card, artifact = card, publicNode = card;

export default {
  nodes: [
    { id: "contributor", class: "person", x: 20, y: 270, title: "Contributor", rows: ["works on a topic branch", "responds to review"] },
    { id: "fork", class: "repo", x: 290, y: 270, title: "Contributor fork", rows: ["Homebrew/brew clone", "topic branch + commits"] },
    { id: "pr", class: "review", x: 580, y: 260, title: "Homebrew/brew pull request", rows: ["fork branch → upstream main", "human review; discussion stays here"] },
    { id: "ci", class: "automation", x: 900, y: 60, title: "GitHub Actions CI", rows: ["required checks pass", "type/style, macOS + Linux tests", "docs, dependencies, packaging as applicable"] },
    { id: "maintainer", class: "person", x: 900, y: 290, title: "Homebrew maintainer", rows: ["at least one approval", "standard Merge button", "preserves signed commit history"] },
    { id: "feedback", class: "review", x: 580, y: 480, title: "PR revisions", rows: ["review comments / requested changes", "new commits pushed to fork branch"] },
    { id: "main", class: "repo", x: 1210, y: 265, title: "Homebrew/brew main", rows: ["merged change", "stable users receive updates by release"] },
    { id: "core-pr", class: "repo", x: 1210, y: 40, title: "homebrew/core recent PR", rows: ["at least one recent PR", "must have completed CI successfully"] },
    { id: "blockers", class: "gate", x: 1530, y: 30, title: "Release blocker check", rows: ["open brew issue/PR labels:", "release blocker", "major/minor release blocker"] },
    { id: "readiness", class: "gate", x: 1530, y: 280, title: "Release readiness", rows: ["brew main workflows passing", "allow regression-detection time", "current main is suitable"] },
    { id: "preview", class: "automation", x: 1830, y: 275, title: "brew release preview", rows: ["default patch; --minor / --major", "proposed version + release notes", "major/minor: previous release ≥ 1 month"] },
    { id: "release-workflow", class: "automation", x: 2130, y: 275, title: "Release workflow", rows: ["brew release --force", "creates draft GitHub release", "refuses while blocker labels remain"] },
    { id: "draft", class: "artifact", x: 2430, y: 275, title: "Draft GitHub release", rows: ["review version and notes", "maintainer publishes"] },
    { id: "tag", class: "artifact", x: 2730, y: 275, title: "GitHub release tag", rows: ["published Homebrew/brew version", "users receive updates from release tags"] },
    { id: "notes", class: "artifact", x: 1830, y: 510, title: "Release notes post", rows: ["based on generated notes", "edited for purpose + user impact"] },
    { id: "website", class: "public", x: 2130, y: 510, title: "Homebrew website", rows: ["release post published"] },
    { id: "channels", class: "public", x: 2430, y: 510, title: "Homebrew communication channels", rows: ["announce after release + post publish"] },
    { id: "users", class: "public", x: 3030, y: 275, title: "Homebrew users", rows: ["new Homebrew/brew versions", "arrive via GitHub release tags"] },
  ],
  edges: [
    { from: "contributor", to: "fork", label: "edit + commit change" },
    { from: "fork", to: "pr", label: "push branch; open pull request" },
    { from: "pr", to: "ci", label: "PR update triggers checks" },
    { from: "ci", to: "pr", label: "check status + CI logs" },
    { from: "pr", to: "maintainer", label: "review request" },
    { from: "maintainer", to: "pr", label: "comments / approval" },
    { from: "pr", to: "feedback", label: "requested changes" },
    { from: "feedback", to: "fork", label: "commit revisions; push update" },
    { from: "pr", to: "main", label: "approved + passing CI; merge" },
    { from: "main", to: "readiness", label: "current main + passing workflows" },
    { from: "core-pr", to: "readiness", label: "successful recent CI evidence" },
    { from: "blockers", to: "readiness", label: "no blocking labels" },
    { from: "readiness", to: "preview", label: "release candidate is ready" },
    { from: "preview", to: "release-workflow", label: "review preview; run --force" },
    { from: "blockers", to: "release-workflow", label: "workflow rechecks open blockers" },
    { from: "release-workflow", to: "draft", label: "draft release + tag/version" },
    { from: "draft", to: "tag", label: "maintainer reviews and publishes" },
    { from: "preview", to: "notes", label: "generated version + notes basis" },
    { from: "notes", to: "website", label: "publish edited release post" },
    { from: "draft", to: "channels", label: "release published" },
    { from: "website", to: "channels", label: "post published" },
    { from: "tag", to: "users", label: "GitHub release tag delivers new version" },
  ],
  style: { person, repo, review, automation, gate, artifact, public: publicNode },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<Node>;
