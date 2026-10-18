// Zero to JupyterHub with Kubernetes, based on https://z2jh.jupyter.org/en/stable/administrator/architecture.html,
// https://z2jh.jupyter.org/en/3.3.2/resources/tools.html, and https://jupyterhub.readthedocs.io/en/stable/reference/technical-overview.html.
// Shows the cluster placement and request/control paths for the chart's proxy, Hub, Kubernetes-spawned user server,
// Hub database, and per-user persistent storage, including the optional external identity provider connection.
import type { Sketch, SketchNode } from "nicevg";
import { svg, text } from "nicevg";

type Kind = "boundary" | "actor" | "service" | "process" | "record";
type Node = SketchNode & { class: Kind; label: string; width: number; height: number; fill?: string };

function card(node: Node, fill: string) {
  const lines = text(node.label, 12);
  const x = 12;
  const y = 11;
  const w = Math.max(node.width, lines.width + 24);
  const h = Math.max(node.height, lines.height + 22);
  const shape = node.class === "actor"
    ? svg`<ellipse data-bounds="true" cx="${w / 2}" cy="${h / 2}" rx="${w / 2}" ry="${h / 2}" fill="${node.fill ?? fill}" stroke="#334155"/>`
    : node.class === "service"
      ? svg`<rect data-bounds="true" width="${w}" height="${h}" rx="16" fill="${node.fill ?? fill}" stroke="#334155"/>`
      : node.class === "record"
        ? svg`<path data-bounds="true" d="M 0 0 H ${w - 10} L ${w} 10 V ${h} H 0 Z" fill="${node.fill ?? fill}" stroke="#334155"/>`
        : svg`<rect data-bounds="true" width="${w}" height="${h}" rx="5" fill="${node.fill ?? fill}" stroke="#334155"/>`;
  return svg`${shape}${lines.at({ cx: w / 2, cy: h / 2 })}`;
}
const boundary = (node: Node) => card(node, "#f1f5f9");
const actor = (node: Node) => card(node, "#fef3c7");
const service = (node: Node) => card(node, "#dbeafe");
const process = (node: Node) => card(node, "#dcfce7");
const record = (node: Node) => card(node, "#fefce8");

const sketch = {
  nodes: [
    { id: "cluster", class: "boundary", x: 270, y: 35, label: "Kubernetes cluster\nZero to JupyterHub Helm release", width: 920, height: 670 },
    { id: "browser", class: "actor", x: 20, y: 180, label: "User browser\nHub cookie", width: 160, height: 68 },
    { id: "idp", class: "actor", x: 20, y: 430, label: "External OAuth\nidentity provider", width: 160, height: 68 },
    { id: "proxy_public", class: "service", parent: "cluster", x: 320, y: 90, label: "proxy-public Service\npublic address · HTTP(S)", width: 220, height: 68 },
    { id: "proxy", class: "process", parent: "cluster", x: 650, y: 90, label: "proxy pod\nConfigurableHTTPProxy\npublic ingress · /hub and /user routes", width: 250, height: 90 },
    { id: "hub", class: "process", parent: "cluster", x: 650, y: 270, label: "hub pod\nJupyterHub · admin/API\ninternal HTTP", width: 250, height: 78 },
    { id: "auth", class: "process", parent: "cluster", x: 320, y: 260, label: "Authenticator\nOAuth login callback\nallowed user → username", width: 220, height: 78 },
    { id: "spawner", class: "process", parent: "cluster", x: 320, y: 440, label: "KubeSpawner\nHub-side spawner", width: 220, height: 68 },
    { id: "kube_api", class: "service", parent: "cluster", x: 650, y: 440, label: "Kubernetes API\ncreate / watch user Pod", width: 250, height: 68 },
    { id: "hub_db", class: "record", parent: "cluster", x: 320, y: 570, label: "hub-db PVC (default)\nSQLite: users · servers · tokens\nHub state survives restart", width: 220, height: 82 },
    { id: "user_pod", class: "process", parent: "cluster", x: 930, y: 440, label: "singleuser Pod\nJupyter Server :8888\none per active user", width: 220, height: 78 },
    { id: "user_pvc", class: "record", parent: "cluster", x: 930, y: 570, label: "user home PVC\n/home/jovyan · user files\nmounted into singleuser Pod", width: 220, height: 78 },
  ] satisfies Node[],
  edges: [
    { from: "browser", to: "proxy_public", label: "HTTPS" },
    { from: "proxy_public", to: "proxy", label: "Service forwards traffic" },
    { from: "proxy", to: "hub", label: "/hub/* HTTP" },
    { from: "proxy", to: "user_pod", label: "/user/{name}/* HTTP/WebSocket" },
    { from: "hub", to: "auth", label: "validate login" },
    { from: "auth", to: "idp", label: "OAuth authorize / token / user info" },
    { from: "idp", to: "browser", label: "redirect with authorization code" },
    { from: "browser", to: "proxy_public", label: "OAuth callback via /hub/oauth_callback" },
    { from: "hub", to: "spawner", label: "spawn(user)" },
    { from: "spawner", to: "kube_api", label: "Pod create + readiness watch" },
    { from: "kube_api", to: "user_pod", label: "schedule Pod from singleuser image" },
    { from: "hub", to: "proxy", label: "proxy API: add /user/{name} route" },
    { from: "hub", to: "hub_db", label: "read/write Hub state" },
    { from: "user_pod", to: "user_pvc", label: "mount home storage" },
  ],
  style: { boundary, actor, service, process, record },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<Node>;

export default sketch;
