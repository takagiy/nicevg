// Ansible community.general contributor and release guides:
// https://github.com/ansible-collections/community.general/blob/main/CONTRIBUTING.md
// https://docs.ansible.com/projects/ansible/latest/community/collection_development_process.html
// https://docs.ansible.com/projects/ansible/latest/community/collection_contributors/collection_releasing.html
// https://docs.ansible.com/projects/ansible/latest/community/collection_contributors/collection_release_with_branches.html
// Shows the records a functional change leaves behind: a fork branch and PR, its fragment,
// the merged repository state, release outputs, and the tag-triggered publication to Galaxy.
import type { Sketch, SketchNode } from "nicevg";
import { svg, text } from "nicevg";

type Kind = "branch" | "record" | "file" | "commit" | "tag" | "external";
type Node = SketchNode & {
  class: Kind;
  title: string;
  rows: string[];
  width?: number;
  height?: number;
};

function card(node: Node, shape: "rect" | "round" | "file" | "hex" | "ellipse") {
  const { title: heading, rows, width = 220 } = node;
  const title = text(heading, 14);
  const lines = rows.map((row) => text(row, 12));
  const height = 32 + lines.reduce((sum, line) => sum + line.height + 7, 0) + 8;
  const fill = node.class === "external" ? "#f4f1e9" : "#fffdf7";
  const body = shape === "round"
    ? svg`<rect data-bounds="true" width="${width}" height="${height}" rx="14" fill="${fill}" stroke="#514b40"/>`
    : shape === "ellipse"
      ? svg`<ellipse data-bounds="true" cx="${width / 2}" cy="${height / 2}" rx="${width / 2}" ry="${height / 2}" fill="${fill}" stroke="#514b40"/>`
      : shape === "hex"
        ? svg`<path data-bounds="true" d="M14 0 H${width - 14} L${width} ${height / 2} L${width - 14} ${height} H14 L0 ${height / 2} Z" fill="${fill}" stroke="#514b40"/>`
        : shape === "file"
          ? svg`<path data-bounds="true" d="M0 0 H${width - 22} L${width} 22 V${height} H0 Z M${width - 22} 0 V22 H${width}" fill="${fill}" stroke="#514b40"/>`
          : svg`<rect data-bounds="true" width="${width}" height="${height}" fill="${fill}" stroke="#514b40"/>`;
  return svg`${body}${title.at({ left: 12, top: 8 })}${lines.map((line, i) => line.at({ left: 12, top: 31 + lines.slice(0, i).reduce((sum, item) => sum + item.height + 7, 0) }))}`;
}

const branch = (node: Node) => card(node, "file");
const record = (node: Node) => card(node, "round");
const file = (node: Node) => card(node, "file");
const commit = (node: Node) => card(node, "rect");
const tag = (node: Node) => card(node, "hex");
const external = (node: Node) => card(node, "ellipse");

export default {
  nodes: [
    { id: "fork", class: "branch", x: 20, y: 150, title: "Fork branch", rows: ["origin: contributor fork", "commit(s): code + tests", "changelogs/fragments/", "12345-fix.yml"] },
    { id: "pr", class: "record", x: 300, y: 110, title: "GitHub pull request", rows: ["base: community/general:main", "head: fork:topic-branch", "commits · review · issue link", "fragment required for bugfix/feature"] },
    { id: "ci", class: "external", x: 590, y: 35, title: "CI / ansible-test", rows: ["sanity + unit/integration", "status on each PR update"] },
    { id: "fragment", class: "file", x: 590, y: 250, title: "Fragment YAML", rows: ["changelogs/fragments/…yml", "bugfixes: / minor_changes:", "entry: scope + user-facing text", "issue or PR URL"] },
    { id: "main", class: "commit", x: 900, y: 150, title: "community.general main", rows: ["merged PR (GitHub squashes)", "plugin/module + tests", "fragment retained for release"] },
    { id: "release-files", class: "file", x: 1200, y: 80, title: "Release commit", rows: ["antsibull-changelog release", "changelogs/changelog.yaml", "CHANGELOG.rst", "galaxy.yml: version"] },
    { id: "tag", class: "tag", x: 1510, y: 175, title: "Annotated Git tag", rows: ["version = galaxy.yml", "tagged source commit", "push upstream tag"] },
    { id: "galaxy", class: "external", x: 1815, y: 130, title: "Ansible Galaxy", rows: ["Zuul sees upstream tag", "builds and publishes collection", "version tarball + install metadata"] },
  ],
  edges: [
    { from: "fork", to: "pr", label: "push branch; open PR" },
    { from: "pr", to: "fork", label: "review feedback; push revisions" },
    { from: "pr", to: "ci", label: "PR commit triggers checks" },
    { from: "ci", to: "pr", label: "status; failure feedback" },
    { from: "fork", to: "fragment", label: "functional change's release note" },
    { from: "fragment", to: "pr", label: "commit alongside change" },
    { from: "pr", to: "main", label: "checks + review pass; committer merges" },
    { from: "main", to: "release-files", label: "release tool consumes fragments" },
    { from: "release-files", to: "tag", label: "versioned release commit" },
    { from: "tag", to: "galaxy", label: "push annotated tag; Zuul publishes" },
  ],
  style: { branch, record, file, commit, tag, external },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<Node>;
