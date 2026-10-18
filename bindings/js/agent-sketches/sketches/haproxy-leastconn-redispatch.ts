// HAProxy backend server selection and recovery, based on the configuration
// manual and the HAProxy introduction: https://docs.haproxy.org/3.2/configuration.html
// https://github.com/haproxy/haproxy/blob/master/doc/intro.txt
// It follows a request through a leastconn backend, including a failed server
// connection, the configured retry/redispatch path, health state, and exhaustion.
import type { Sketch, SketchNode } from "nicevg";
import { svg, text } from "nicevg";

type Kind = "request" | "module" | "decision" | "server" | "result";
type Node = SketchNode & { class: Kind; label: string; width: number; height: number; fill: string };

function panel({ label, width, height, fill }: Node) {
  const name = text(label, 13);
  return svg`<rect data-bounds="true" width="${width}" height="${height}" rx="8" fill="${fill}" stroke="#334155" stroke-width="1.5"/>${name.at({ cx: width / 2, cy: height / 2 })}`;
}

function request({ label, width, height, fill }: Node) {
  const name = text(label, 13);
  return svg`<path data-bounds="true" d="M 18 0 H ${width - 18} Q ${width} 0 ${width} 18 V ${height - 18} Q ${width} ${height} ${width - 18} ${height} H 18 Q 0 ${height} 0 ${height - 18} V 18 Q 0 0 18 0Z" fill="${fill}" stroke="#334155" stroke-width="1.5"/>${name.at({ cx: width / 2, cy: height / 2 })}`;
}

function decision({ label, width, height, fill }: Node) {
  const name = text(label, 13);
  return svg`<path data-bounds="true" d="M ${width / 2} 0 L ${width} ${height / 2} L ${width / 2} ${height} L 0 ${height / 2} Z" fill="${fill}" stroke="#334155" stroke-width="1.5"/>${name.at({ cx: width / 2, cy: height / 2 })}`;
}

function server({ label, width, height, fill }: Node) {
  const name = text(label, 13);
  return svg`<rect data-bounds="true" width="${width}" height="${height}" rx="22" fill="${fill}" stroke="#334155" stroke-width="1.5"/>${name.at({ cx: width / 2, cy: height / 2 })}`;
}

function result({ label, width, height, fill }: Node) {
  const name = text(label, 13);
  return svg`<path data-bounds="true" d="M 12 0 H ${width - 12} L ${width} 12 V ${height - 12} L ${width - 12} ${height} H 12 L 0 ${height - 12} V 12 Z" fill="${fill}" stroke="#334155" stroke-width="1.5"/>${name.at({ cx: width / 2, cy: height / 2 })}`;
}

const sketch = {
  nodes: [
    { id: "client", class: "request", x: 20, y: 190, label: "Client\nHTTP request", width: 155, height: 64, fill: "#fff7ed" },
    { id: "frontend", class: "module", x: 220, y: 185, label: "frontend\nbind + default_backend", width: 200, height: 74, fill: "#dbeafe" },
    { id: "backend", class: "module", x: 475, y: 175, label: "backend app\nbalance leastconn", width: 200, height: 82, fill: "#ede9fe" },
    { id: "eligible", class: "decision", x: 735, y: 175, label: "active, non-backup\nservers available?", width: 210, height: 100, fill: "#fef3c7" },
    { id: "server-a", class: "server", x: 1035, y: 40, label: "server app1\ncurrent conns = 8", width: 190, height: 72, fill: "#dcfce7" },
    { id: "server-b", class: "server", x: 1035, y: 175, label: "server app2\ncurrent conns = 3", width: 190, height: 72, fill: "#dcfce7" },
    { id: "server-c", class: "server", x: 1035, y: 310, label: "server app3\ncurrent conns = 5", width: 190, height: 72, fill: "#dcfce7" },
    { id: "check", class: "module", x: 755, y: 395, label: "server check\nfall / rise health state", width: 190, height: 72, fill: "#cffafe" },
    { id: "connect", class: "module", x: 1300, y: 175, label: "connect to selected\nserver app2", width: 200, height: 72, fill: "#e0f2fe" },
    { id: "retry", class: "decision", x: 1300, y: 310, label: "retries remain?\nretries N", width: 200, height: 84, fill: "#ffedd5" },
    { id: "redispatch", class: "module", x: 1015, y: 500, label: "option redispatch\ndefault: last retry\nselect another server", width: 230, height: 90, fill: "#fae8ff" },
    { id: "response", class: "result", x: 1560, y: 185, label: "response\nfrom server", width: 180, height: 72, fill: "#dcfce7" },
    { id: "unavailable", class: "result", x: 1560, y: 345, label: "no usable server /\nretry budget exhausted\nHTTP 503", width: 190, height: 90, fill: "#fee2e2" },
  ] satisfies Node[],
  edges: [
    { from: "client", to: "frontend", label: "request" },
    { from: "frontend", to: "backend", label: "default_backend app" },
    { from: "backend", to: "eligible", label: "candidate set; skip down / maintenance" },
    { from: "eligible", to: "server-a", label: "choose lowest active connection count" },
    { from: "eligible", to: "server-b" },
    { from: "eligible", to: "server-c" },
    { from: "check", to: "server-a", label: "health checks update UP / DOWN" },
    { from: "check", to: "server-b" },
    { from: "check", to: "server-c" },
    { from: "server-b", to: "connect", label: "first selection" },
    { from: "connect", to: "response", label: "connection succeeds" },
    { from: "connect", to: "retry", label: "connection failure" },
    { from: "retry", to: "redispatch", label: "retry allowed; final retry redispatches" },
    { from: "redispatch", to: "backend", label: "run selection again, excluding failed server" },
    { from: "retry", to: "unavailable", label: "no retry remains" },
    { from: "eligible", to: "unavailable", label: "no active server" },
  ],
  style: { request, module: panel, decision, server, result },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<Node>;

export default sketch;
