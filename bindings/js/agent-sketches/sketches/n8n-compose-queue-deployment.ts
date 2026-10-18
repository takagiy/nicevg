// n8n Docker Compose queue-mode deployment, based on
// https://github.com/n8n-io/n8n-hosting/blob/main/docker-compose/withPostgresAndWorker/docker-compose.yml
// and its README. It shows the Compose host, published editor/API endpoint, shared PostgreSQL and
// Redis services, and the external task-runner sidecars attached to the main and worker containers.

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
    { id: "compose", class: "box", x: 230, y: 25, label: "Docker Compose host\nwithPostgresAndWorker", width: 720, height: 550, fill: "#f1f5f9" },
    { id: "client", class: "box", x: 20, y: 160, label: "Browser / webhook caller", width: 175, height: 62, fill: "#fff7ed" },
    { id: "n8n", class: "box", x: 275, y: 95, parent: "compose", label: "n8n\nmain · :5678 published\neditor, API, webhook ingress", width: 215, height: 90, fill: "#dbeafe" },
    { id: "worker", class: "box", x: 620, y: 95, parent: "compose", label: "n8n-worker\ncommand: worker\nexecutes queued workflows", width: 215, height: 90, fill: "#dbeafe" },
    { id: "postgres", class: "box", x: 275, y: 285, parent: "compose", label: "postgres:18\nPostgreSQL · db_storage volume", width: 215, height: 72, fill: "#dcfce7" },
    { id: "redis", class: "box", x: 620, y: 285, parent: "compose", label: "redis:7-alpine\nBull queue · redis_storage volume", width: 215, height: 72, fill: "#fce7f3" },
    { id: "main-runner", class: "box", x: 275, y: 445, parent: "compose", label: "n8n-runner\nexternal Code node tasks", width: 215, height: 70, fill: "#ede9fe" },
    { id: "worker-runner", class: "box", x: 620, y: 445, parent: "compose", label: "n8n-worker-runner\nexternal Code node tasks", width: 215, height: 70, fill: "#ede9fe" },
  ] satisfies Node[],
  edges: [
    { from: "client", to: "n8n", label: "HTTP :5678" },
    { from: "n8n", to: "postgres", label: "DB_TYPE=postgresdb · metadata" },
    { from: "worker", to: "postgres", label: "same database config" },
    { from: "n8n", to: "redis", label: "Bull queue producer" },
    { from: "worker", to: "redis", label: "Bull queue consumer" },
    { from: "n8n", to: "main-runner", label: "HTTP task broker :5679\nshared RUNNERS_AUTH_TOKEN" },
    { from: "worker", to: "worker-runner", label: "HTTP task broker :5679\nshared RUNNERS_AUTH_TOKEN" },
  ],
  style: { box },
} satisfies Sketch<Node>;

export default sketch;
