// Docker Compose Quickstart: https://docs.docker.com/compose/gettingstarted/
// The sketch shows the Compose demo's host, published Flask port, service network,
// Redis readiness gate, and persistent Redis data mount.
import { box } from "nicevg/style/simple";
import type { Sketch } from "nicevg";

type Node = { id: string; class: "box"; x: number; y: number; label: string; parent?: string; width?: number; height?: number };

const sketch = {
  nodes: [
    { id: "host", class: "box", x: 30, y: 30, width: 660, height: 385, label: "Docker host · compose-demo" },
    { id: "browser", class: "box", x: 65, y: 105, label: "Browser" },
    { id: "web", class: "box", x: 280, y: 95, label: "web container\nFlask :5000" },
    { id: "redis", class: "box", x: 475, y: 245, label: "redis container\nredis:alpine :6379" },
    { id: "volume", class: "box", x: 475, y: 340, label: "named volume\nredis-data" },
    { id: "compose", class: "box", x: 55, y: 275, label: "Compose\ncompose.yaml" },
    { id: "dockerhub", class: "box", x: 475, y: 75, label: "Docker Hub\nRedis image" },
  ] satisfies Node[],
  edges: [
    { from: "browser", to: "web", label: "http://localhost:8000 → :5000" },
    { from: "web", to: "redis", label: "REDIS_HOST=redis :6379\nINCR hits" },
    { from: "redis", to: "web", label: "counter value / reply" },
    { from: "redis", to: "volume", label: "/data" },
    { from: "dockerhub", to: "redis", label: "pull redis:alpine" },
    { from: "compose", to: "web", label: "build .; publish 8000:5000" },
    { from: "compose", to: "redis", label: "start after healthy\nredis-cli ping → PONG" },
  ],
  style: { box },
} satisfies Sketch<Node>;

export default sketch;
