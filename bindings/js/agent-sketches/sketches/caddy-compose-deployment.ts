// Caddy Docker Compose deployment, based on https://caddyserver.com/docs/running and
// https://hub.docker.com/_/caddy. It shows the Compose host, published entry ports, the mounted
// Caddyfile and static site, and the named volumes that persist Caddy's data and config.
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
    { id: "host", class: "box", x: 250, y: 30, label: "Docker Compose host\ncompose.yml", width: 600, height: 470, fill: "#f1f5f9" },
    { id: "client", class: "box", x: 20, y: 180, label: "Web client", width: 150, height: 58, fill: "#fff7ed" },
    { id: "caddy", class: "box", x: 460, y: 110, parent: "host", label: "caddy:<version>\nCaddy server", width: 200, height: 74, fill: "#dbeafe" },
    { id: "conf", class: "box", x: 285, y: 285, parent: "host", label: "./conf\nCaddyfile", width: 150, height: 60, fill: "#ede9fe" },
    { id: "site", class: "box", x: 465, y: 285, parent: "host", label: "./site\nstatic files", width: 150, height: 60, fill: "#dcfce7" },
    { id: "data", class: "box", x: 645, y: 285, parent: "host", label: "caddy_data\n/data", width: 150, height: 60, fill: "#fef3c7" },
    { id: "config", class: "box", x: 465, y: 395, parent: "host", label: "caddy_config\n/config", width: 150, height: 60, fill: "#fef3c7" },
  ] satisfies Node[],
  edges: [
    { from: "client", to: "caddy", label: "HTTP :80 / HTTPS :443\nHTTP/3 :443/udp" },
    { from: "conf", to: "caddy", label: "bind mount -> /etc/caddy" },
    { from: "site", to: "caddy", label: "bind mount -> /srv" },
    { from: "caddy", to: "data", label: "named volume -> /data\nTLS certificates + keys" },
    { from: "caddy", to: "config", label: "named volume -> /config" },
  ],
  style: { box },
} satisfies Sketch<Node>;

export default sketch;
