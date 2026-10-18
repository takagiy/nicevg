// Node.js — a contributor's change through the release-line proposal to published release artifacts.
// It follows a normal non-security patch release: reviewed code lands on main, is selected for a staging line,
// and reaches nodejs.org only after the proposal and builds are checked and the release commit is signed.
// Sources: https://github.com/nodejs/node/blob/main/doc/contributing/collaborator-guide.md
//          https://github.com/nodejs/node/blob/main/doc/contributing/releases.md
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
    { id: "pr", class: "box", x: 20, y: 100, label: "Contributor PR\nchange + test case" },
    { id: "main", class: "box", x: 270, y: 100, label: "nodejs/node main\nGitHub Actions + required CI" },
    { id: "staging", class: "box", x: 520, y: 100, label: "vN.x-staging\nselected fix; signed backport" },
    { id: "proposal", class: "box", x: 770, y: 100, label: "vX.Y.Z-proposal PR\nversion header + CHANGELOG_Vx.md" },
    { id: "citgm", class: "box", x: 1020, y: 20, label: "citgm-smoker\ncompare base vN.x vs proposal" },
    { id: "jenkins", class: "box", x: 1020, y: 190, label: "iojs+release Jenkins\nall platform workers succeed" },
    { id: "tag", class: "box", x: 1270, y: 100, label: "Release commit\nGPG signed tag vX.Y.Z" },
    { id: "dist", class: "box", x: 1520, y: 100, label: "nodejs.org/dist\nrelease builds + signed SHASUMS256.txt" },
  ],
  edges: [
    { from: "pr", to: "main", label: "2 collaborator approvals + passing CI; land" },
    { from: "main", to: "staging", label: "release team selects / backports fix" },
    { from: "staging", to: "proposal", label: "prepare version + changelog; PR targets vN.x" },
    { from: "proposal", to: "citgm", label: "smoke-test proposal against base" },
    { from: "citgm", to: "proposal", label: "regression? update or defer proposal" },
    { from: "proposal", to: "jenkins", label: "build release artifacts from proposal commit" },
    { from: "jenkins", to: "proposal", label: "failed worker: investigate and rebuild" },
    { from: "jenkins", to: "tag", label: "all green + build checked; sign release commit" },
    { from: "tag", to: "dist", label: "push tag, then tools/release.sh promotes + signs checksums" },
  ],
  style: { box },
} satisfies Sketch<Node>;

export default sketch;
