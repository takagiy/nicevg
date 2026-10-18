// Docker Buildx / BuildKit — https://docs.docker.com/build/policies/usage/
// https://docs.docker.com/build/policies/input-reference/
// Shows how a Dockerfile.rego policy decides whether each build source is allowed:
// local context is allowed, an image is allowed only when its repo is alpine, and other sources deny.
import type { Sketch, SketchNode } from "nicevg";
import { box } from "nicevg/style/simple";
import { svg, text } from "nicevg";

type Node = SketchNode & { class: "source" | "policy" | "result"; label: string; fill?: string };

const source = ({ label, fill = "#e0f2fe" }: Node) => {
  const lines = text(label, 12);
  const w = Math.max(178, lines.width + 28), h = lines.height + 24;
  return svg`<path data-bounds="true" d="M 0 0 H ${w - 18} L ${w} 18 V ${h} H 0 Z" fill="${fill}" stroke="#334155"/>${lines.at({ cx: w / 2, cy: h / 2 })}`;
};

const policy = ({ label, fill = "#fef3c7" }: Node) => {
  const lines = text(label, 12);
  const w = Math.max(220, lines.width + 30), h = lines.height + 26;
  return svg`<path data-bounds="true" d="M 0 0 H ${w - 18} L ${w} 18 V ${h - 18} L ${w - 18} ${h} H 18 L 0 ${h - 18} V 18 Z" fill="${fill}" stroke="#92400e" stroke-width="1.5"/>${lines.at({ cx: w / 2, cy: h / 2 })}`;
};

const result = ({ label, fill = "#dcfce7" }: Node) => {
  const lines = text(label, 12);
  const w = Math.max(190, lines.width + 28), h = lines.height + 24;
  return svg`<ellipse data-bounds="true" cx="${w / 2}" cy="${h / 2}" rx="${w / 2}" ry="${h / 2}" fill="${fill}" stroke="#334155"/>${lines.at({ cx: w / 2, cy: h / 2 })}`;
};

const sketch = {
  nodes: [
    { id: "buildx", class: "source", x: 30, y: 210, label: "Docker Buildx\ndocker buildx build" },
    { id: "sources", class: "source", x: 285, y: 190, label: "Dockerfile inputs\nlocal context · image · Git · HTTP", fill: "#e0e7ff" },
    { id: "loader", class: "source", x: 580, y: 35, label: "Buildx policy loading\nDockerfile.rego beside Dockerfile\n(or named -f file)" },
    { id: "eval", class: "policy", x: 605, y: 210, label: "BuildKit policy evaluation\ninput.local / input.image.repo\nper source, before build" },
    { id: "denydefault", class: "policy", x: 930, y: 40, label: "default allow := false\nunknown source stays denied", fill: "#fee2e2" },
    { id: "allowlocal", class: "policy", x: 930, y: 190, label: "allow if input.local\nlocal build context", fill: "#dcfce7" },
    { id: "allowimage", class: "policy", x: 930, y: 350, label: "allow if input.image.repo == \"alpine\"\nimage repo field", fill: "#dcfce7" },
    { id: "decision", class: "result", x: 1260, y: 215, label: "policy decision\nALLOW / DENY" },
    { id: "build", class: "result", x: 1550, y: 85, label: "Build proceeds\nsource accepted", fill: "#bbf7d0" },
    { id: "error", class: "result", x: 1550, y: 385, label: "Build fails\nsource not allowed by policy", fill: "#fecaca" },
  ] satisfies Node[],
  edges: [
    { from: "buildx", to: "sources", label: "Dockerfile source references" },
    { from: "buildx", to: "loader", label: "Dockerfile name + directory" },
    { from: "loader", to: "eval", label: "automatically loaded .rego rules" },
    { from: "sources", to: "eval", label: "source-specific input; e.g. image.repo" },
    { from: "eval", to: "denydefault", label: "no matching allow rule" },
    { from: "eval", to: "allowlocal", label: "input.local is true" },
    { from: "eval", to: "allowimage", label: "input.image.repo == alpine" },
    { from: "denydefault", to: "decision", label: "otherwise: DENY" },
    { from: "allowlocal", to: "decision", label: "rule allows: ALLOW" },
    { from: "allowimage", to: "decision", label: "rule allows: ALLOW" },
    { from: "decision", to: "build", label: "ALLOW: continue build" },
    { from: "decision", to: "error", label: "DENY: source not allowed" },
  ],
  style: { source, policy, result },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<Node>;

export default sketch;
