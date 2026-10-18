// Immich Docker Compose deployment, based on https://docs.immich.app/overview/quick-start/ and
// https://github.com/immich-app/immich/blob/main/docker/docker-compose.yml. It shows the four
// containers on one Compose host, the published API port, service calls, and persistent mounts.
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
    { id: "host", class: "box", x: 240, y: 30, label: "Docker Compose host\ndocker-compose.yml + .env", width: 520, height: 470, fill: "#f1f5f9" },
    { id: "client", class: "box", x: 20, y: 190, label: "Web / mobile client", width: 150, height: 58, fill: "#fff7ed" },
    { id: "server", class: "box", x: 290, y: 100, parent: "host", label: "immich-server\nREST API :2283", width: 190, height: 76, fill: "#dbeafe" },
    { id: "ml", class: "box", x: 530, y: 100, parent: "host", label: "immich-machine-learning\nFastAPI inference", width: 190, height: 76, fill: "#ede9fe" },
    { id: "redis", class: "box", x: 290, y: 250, parent: "host", label: "redis (Valkey)\nBullMQ job queues", width: 190, height: 70, fill: "#fce7f3" },
    { id: "db", class: "box", x: 530, y: 250, parent: "host", label: "database (Postgres)\nmetadata + config", width: 190, height: 70, fill: "#dcfce7" },
    { id: "upload", class: "box", x: 290, y: 390, parent: "host", label: "${UPLOAD_LOCATION}\n→ /data", width: 190, height: 58, fill: "#fef3c7" },
    { id: "modelcache", class: "box", x: 530, y: 390, parent: "host", label: "model-cache volume\n→ /cache", width: 190, height: 58, fill: "#fef3c7" },
  ] satisfies Node[],
  edges: [
    { from: "client", to: "server", label: "HTTP :2283 published" },
    { from: "server", to: "ml", label: "HTTP model requests" },
    { from: "server", to: "redis", label: "BullMQ jobs" },
    { from: "redis", to: "server", label: "queued work" },
    { from: "server", to: "db", label: "Postgres metadata/config" },
    { from: "server", to: "upload", label: "media files bind mount" },
    { from: "ml", to: "modelcache", label: "downloaded model cache" },
  ],
  style: { box },
} satisfies Sketch<Node>;

export default sketch;
