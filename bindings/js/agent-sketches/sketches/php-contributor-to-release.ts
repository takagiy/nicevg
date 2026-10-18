// PHP — from a contributor's php-src change to a public GA release.
// Sources: https://github.com/php/policies/blob/main/release-process.rst
//          https://github.com/php/php-src/blob/master/docs/release-process.md
// The slice follows a core feature through its RFC / PR path, the yearly freeze and RC gates,
// then the release manager's test, signed source package and public downloads.
import type { Sketch, SketchNode } from "nicevg";
import { svg, text } from "nicevg";

type Node = SketchNode & { class: "person" | "change" | "repo" | "gate" | "release"; label: string };

const person = ({ label }: Node) => {
  const t = text(label, 12);
  const w = Math.max(82, t.width + 24), h = Math.max(46, t.height + 20);
  return svg`<ellipse data-bounds="true" cx="${w / 2}" cy="${h / 2}" rx="${w / 2}" ry="${h / 2}" fill="#fef3c7" stroke="#92400e"/>${t.at({ cx: w / 2, cy: h / 2 })}`;
};
const change = ({ label }: Node) => {
  const t = text(label, 12);
  const w = Math.max(105, t.width + 28), h = t.height + 22;
  return svg`<polygon data-bounds="true" points="14,0 ${w - 4},0 ${w},${h - 14} ${w - 14},${h} 0,${h} 0,14" fill="#dbeafe" stroke="#1d4ed8"/>${t.at({ cx: w / 2, cy: h / 2 })}`;
};
const repo = ({ label }: Node) => {
  const t = text(label, 12);
  const w = Math.max(118, t.width + 28), h = t.height + 26;
  return svg`<path data-bounds="true" d="M0 9 Q${w / 2} -3 ${w} 9 V${h - 7} Q${w / 2} ${h + 5} 0 ${h - 7} Z" fill="#dcfce7" stroke="#166534"/><path d="M0 9 Q${w / 2} 21 ${w} 9" fill="none" stroke="#166534"/>${t.at({ cx: w / 2, cy: h / 2 })}`;
};
const gate = ({ label }: Node) => {
  const t = text(label, 12);
  const w = Math.max(120, t.width + 36), h = t.height + 30;
  return svg`<polygon data-bounds="true" points="${w / 2},0 ${w},${h / 2} ${w / 2},${h} 0,${h / 2}" fill="#f3e8ff" stroke="#7e22ce"/>${t.at({ cx: w / 2, cy: h / 2 })}`;
};
const release = ({ label }: Node) => {
  const t = text(label, 12);
  const w = Math.max(122, t.width + 28), h = t.height + 22;
  return svg`<path data-bounds="true" d="M0 0 H${w - 15} L${w} ${h / 2} L${w - 15} ${h} H0 Z" fill="#fee2e2" stroke="#b91c1c"/>${t.at({ cx: (w - 8) / 2, cy: h / 2 })}`;
};

const sketch = {
  nodes: [
    { id: "author", class: "person", x: 24, y: 135, label: "Contributor" },
    { id: "patch", class: "change", x: 168, y: 46, label: "php-src change\ncode + tests" },
    { id: "rfc", class: "change", x: 166, y: 238, label: "RFC\nnew core feature" },
    { id: "vote", class: "gate", x: 354, y: 238, label: "RFC vote\naccepted" },
    { id: "pr", class: "change", x: 364, y: 62, label: "GitHub PR\nphp/php-src" },
    { id: "internals", class: "person", x: 558, y: 220, label: "Core devs" },
    { id: "master", class: "repo", x: 584, y: 48, label: "php-src\nmaster" },
    { id: "freeze", class: "gate", x: 790, y: 190, label: "Feature freeze\nRFC passed + merged" },
    { id: "rm", class: "person", x: 976, y: 42, label: "Release managers" },
    { id: "branch", class: "repo", x: 1002, y: 232, label: "PHP-X.Y\nrelease branch" },
    { id: "rc", class: "release", x: 1210, y: 74, label: "RC1 tag\nAPI numbers bumped" },
    { id: "qa", class: "gate", x: 1396, y: 220, label: "make test\nZTS + non-ZTS" },
    { id: "dist", class: "change", x: 1588, y: 50, label: "makedist\n3 source tarballs" },
    { id: "signed", class: "release", x: 1778, y: 226, label: "GPG signatures\n*.manifest" },
    { id: "downloads", class: "release", x: 1988, y: 84, label: "downloads.php.net\nsource release" },
    { id: "windows", class: "release", x: 1994, y: 264, label: "windows.php.net\nQA binaries" },
  ] satisfies Node[],
  edges: [
    { from: "author", to: "patch", label: "implement + test" },
    { from: "author", to: "rfc", label: "propose feature" },
    { from: "rfc", to: "vote", label: "RFC page + public vote" },
    { from: "vote", to: "pr", label: "accepted proposal" },
    { from: "patch", to: "pr", label: "open pull request" },
    { from: "pr", to: "internals", label: "review / objection" },
    { from: "internals", to: "pr", label: "requested changes" },
    { from: "internals", to: "master", label: "merge approved change" },
    { from: "pr", to: "master", label: "implementation + tests" },
    { from: "master", to: "freeze", label: "feature merged before freeze" },
    { from: "freeze", to: "rm", label: "release schedule / gates" },
    { from: "rm", to: "branch", label: "first RC: create branch" },
    { from: "branch", to: "rc", label: "tag release candidate" },
    { from: "rc", to: "qa", label: "build + run test suites" },
    { from: "qa", to: "branch", label: "failure: fix on branch; retry" },
    { from: "qa", to: "dist", label: "pass; makedist from tag" },
    { from: "dist", to: "signed", label: "tar.gz / .bz2 / .xz" },
    { from: "signed", to: "downloads", label: "upload tarballs + manifest" },
    { from: "rc", to: "windows", label: "ask for Windows builds" },
    { from: "downloads", to: "windows", label: "RC testing; last RC becomes GA tag" },
  ],
  style: { person, change, repo, gate, release },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<Node>;

export default sketch;
