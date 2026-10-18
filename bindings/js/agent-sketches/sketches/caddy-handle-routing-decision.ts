// Caddy HTTP routing decision, based on https://caddyserver.com/docs/caddyfile/directives/handle,
// https://caddyserver.com/docs/caddyfile/directives/handle_path, and
// https://caddyserver.com/docs/caddyfile/directives. It shows the mutually exclusive handle
// selection for the documented /api/* example: the API path is stripped before proxying, while
// every request not claimed by that matcher reaches the static file server fallback.
import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";

type Node = {
  id: string;
  class: "box";
  x: number;
  y: number;
  parent?: string;
  label: string;
  width?: number;
  height?: number;
  fill?: string;
};

const sketch = {
  nodes: [
    { id: "request", class: "box", x: 20, y: 220, label: "HTTP request\npath + method", width: 155, height: 66, fill: "#fff7ed" },
    { id: "handles", class: "box", x: 235, y: 165, label: "Caddyfile handle blocks\nmutually exclusive", width: 205, height: 82, fill: "#dbeafe" },
    { id: "api", class: "box", x: 525, y: 60, label: "handle_path /api/*\nfirst matching block", width: 210, height: 66, fill: "#ede9fe" },
    { id: "strip", class: "box", x: 825, y: 40, label: "uri strip_prefix /api\n/api/users -> /users", width: 205, height: 66, fill: "#fef3c7" },
    { id: "proxy", class: "box", x: 1110, y: 40, label: "reverse_proxy\nlocalhost:9000", width: 190, height: 66, fill: "#dcfce7" },
    { id: "fallback", class: "box", x: 525, y: 300, label: "handle\nno matcher = fallback", width: 210, height: 66, fill: "#ede9fe" },
    { id: "files", class: "box", x: 825, y: 300, label: "root /srv\nfile_server", width: 190, height: 66, fill: "#dcfce7" },
  ] satisfies Node[],
  edges: [
    { from: "request", to: "handles", label: "path checked against handles" },
    { from: "handles", to: "api", label: "matches /api/*" },
    { from: "api", to: "strip", label: "matched prefix removed" },
    { from: "strip", to: "proxy", label: "rewritten request" },
    { from: "handles", to: "fallback", label: "no earlier handle matched" },
    { from: "fallback", to: "files", label: "remaining request" },
  ],
  style: { box },
} satisfies Sketch<Node>;

export default sketch;
