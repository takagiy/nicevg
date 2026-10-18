// Nextcloud All-in-One deployment, based on https://github.com/nextcloud/all-in-one/blob/main/readme.md
// and https://github.com/nextcloud/all-in-one/blob/main/Containers/mastercontainer/README.md.
// This shows the one-host Docker deployment: an operator starts AIO, its mastercontainer exposes
// the admin UI and controls sibling containers through Docker, while clients reach Apache and data persists in volumes.
import type { Sketch } from "nicevg";
import { box, circle } from "nicevg/style/simple";

type Node = {
  id: string;
  class: "box" | "circle";
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
    { id: "operator", class: "circle", x: 35, y: 90, label: "Server operator", width: 145, height: 85, fill: "#ffedd5" },
    { id: "host", class: "box", x: 240, y: 20, label: "Linux host · Docker Engine\nports 80, 443, 8080, 8443 published", width: 610, height: 580, fill: "#f1f5f9" },
    { id: "browser", class: "circle", x: 35, y: 395, label: "Browser / Nextcloud user", width: 170, height: 90, fill: "#ffedd5" },
    { id: "master", class: "box", x: 275, y: 100, parent: "host", label: "nextcloud-aio-mastercontainer\nAIO UI · PHP-FPM · Caddy\nDocker API orchestration", width: 260, height: 100, fill: "#dbeafe" },
    { id: "config", class: "box", x: 580, y: 100, parent: "host", label: "nextcloud_aio_mastercontainer\n/mnt/docker-aio-config", width: 230, height: 76, fill: "#fef3c7" },
    { id: "apache", class: "box", x: 310, y: 300, parent: "host", label: "nextcloud-aio-apache\nNextcloud web / PHP", width: 215, height: 76, fill: "#dcfce7" },
    { id: "database", class: "box", x: 580, y: 285, parent: "host", label: "nextcloud-aio-database\nPostgreSQL", width: 220, height: 70, fill: "#ede9fe" },
    { id: "redis", class: "box", x: 310, y: 445, parent: "host", label: "nextcloud-aio-redis\ncache + file locking", width: 215, height: 70, fill: "#fce7f3" },
    { id: "datadir", class: "box", x: 580, y: 430, parent: "host", label: "nextcloud-aio-nextcloud\n/mnt/ncdata persistent volume", width: 220, height: 76, fill: "#fef3c7" },
  ] satisfies Node[],
  edges: [
    { from: "operator", to: "master", label: "AIO setup / start via :8080 or :8443" },
    { from: "master", to: "host", label: "read-only /var/run/docker.sock\ncreate, start, update sibling containers" },
    { from: "master", to: "config", label: "configuration + AIO state" },
    { from: "browser", to: "master", label: "HTTPS admin UI :8080 / :8443" },
    { from: "browser", to: "apache", label: "HTTPS Nextcloud :443 → Apache :443" },
    { from: "apache", to: "database", label: "PostgreSQL metadata" },
    { from: "apache", to: "redis", label: "cache + transactional file locks" },
    { from: "apache", to: "datadir", label: "user files + appdata" },
  ],
  style: { box, circle },
} satisfies Sketch<Node>;

export default sketch;
