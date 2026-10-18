import { box } from "nicevg/style/simple";
import type { Sketch } from "nicevg";

type Node = { id: string; class: "box"; x: number; y: number; label: string; parent?: string; width?: number; height?: number; fill?: string };

// GitLab Self-Managed 3,000-user Linux package reference architecture:
// https://docs.gitlab.com/administration/reference_architectures/3k_users/
// https://docs.gitlab.com/development/architecture/
// Shows the documented VM roles on the private network, the external entry point,
// GitLab's internal service paths, and its Runner, object storage, and monitoring integrations.

const nodes: Node[] = [
  { id: "clients", class: "box", x: 20, y: 210, label: "Users / Git clients", width: 170 },
  { id: "runner", class: "box", x: 20, y: 490, label: "GitLab Runner fleet", width: 170, fill: "#fef3c7" },
  { id: "registry", class: "box", x: 20, y: 650, label: "Container registry", width: 170 },
  { id: "object-store", class: "box", x: 20, y: 800, label: "Object storage\n(S3-compatible / cloud)", width: 190 },
  { id: "network", class: "box", x: 250, y: 20, label: "Private network · 10.6.0.0/24", width: 1810, height: 980, fill: "#f8fafc" },
  { id: "edge", class: "box", x: 280, y: 100, parent: "network", label: "External load balancer\n10.6.0.10", width: 230, fill: "#dbeafe" },
  { id: "internal-lb", class: "box", x: 680, y: 100, parent: "network", label: "Internal load balancer\n10.6.0.20", width: 230, fill: "#dbeafe" },
  { id: "app", class: "box", x: 1100, y: 60, parent: "network", label: "GitLab application VMs", width: 850, height: 245, fill: "#eff6ff" },
  { id: "app1", class: "box", x: 1140, y: 130, parent: "app", label: "Rails app 1\nPuma · Workhorse · Shell\n10.6.0.41", width: 220, height: 95 },
  { id: "app2", class: "box", x: 1400, y: 130, parent: "app", label: "Rails app 2\nPuma · Workhorse · Shell\n10.6.0.42", width: 220, height: 95 },
  { id: "app3", class: "box", x: 1660, y: 130, parent: "app", label: "Rails app 3\nPuma · Workhorse · Shell\n10.6.0.43", width: 220, height: 95 },
  { id: "db", class: "box", x: 280, y: 360, parent: "network", label: "PostgreSQL HA", width: 300, height: 210, fill: "#fefce8" },
  { id: "pg1", class: "box", x: 305, y: 410, parent: "db", label: "Primary · 10.6.0.21", width: 245 },
  { id: "pg2", class: "box", x: 305, y: 465, parent: "db", label: "Standby 1 · 10.6.0.22", width: 245 },
  { id: "pg3", class: "box", x: 305, y: 520, parent: "db", label: "Standby 2 · 10.6.0.23", width: 245 },
  { id: "pgbouncer", class: "box", x: 680, y: 360, parent: "network", label: "PgBouncer pool", width: 270, height: 210, fill: "#fefce8" },
  { id: "pgb1", class: "box", x: 705, y: 410, parent: "pgbouncer", label: "10.6.0.31", width: 220 },
  { id: "pgb2", class: "box", x: 705, y: 465, parent: "pgbouncer", label: "10.6.0.32", width: 220 },
  { id: "pgb3", class: "box", x: 705, y: 520, parent: "pgbouncer", label: "10.6.0.33", width: 220 },
  { id: "redis", class: "box", x: 1100, y: 360, parent: "network", label: "Redis · sessions, cache, queues", width: 360, height: 190, fill: "#fee2e2" },
  { id: "redis1", class: "box", x: 1130, y: 415, parent: "redis", label: "Primary · 10.6.0.61", width: 300 },
  { id: "redis2", class: "box", x: 1130, y: 470, parent: "redis", label: "Replica · 10.6.0.62", width: 300 },
  { id: "redis3", class: "box", x: 1130, y: 525, parent: "redis", label: "Replica · 10.6.0.63", width: 300 },
  { id: "coord", class: "box", x: 1550, y: 360, parent: "network", label: "Consul / Sentinel quorum\n10.6.0.11 · .12 · .13", width: 390, height: 80, fill: "#ede9fe" },
  { id: "sidekiq", class: "box", x: 1550, y: 470, parent: "network", label: "Sidekiq workers\n10.6.0.71 · .72", width: 390, height: 80, fill: "#fce7f3" },
  { id: "gitaly-lb", class: "box", x: 280, y: 650, parent: "network", label: "Praefect TCP load balancer", width: 300, fill: "#ffedd5" },
  { id: "praefect", class: "box", x: 680, y: 620, parent: "network", label: "Praefect routers\n10.6.0.131 · .132 · .133", width: 330, height: 90, fill: "#ffedd5" },
  { id: "praefect-db", class: "box", x: 1100, y: 620, parent: "network", label: "Praefect PostgreSQL\nmetadata · 10.6.0.141", width: 330, fill: "#fefce8" },
  { id: "gitaly", class: "box", x: 1550, y: 600, parent: "network", label: "Gitaly replicas\n10.6.0.51 · .52 · .93\nGit repositories", width: 390, height: 130, fill: "#ffedd5" },
  { id: "prom", class: "box", x: 680, y: 810, parent: "network", label: "Prometheus\n10.6.0.81", width: 260, fill: "#dcfce7" },
  { id: "ssh", class: "box", x: 280, y: 850, parent: "network", label: "SSH ingress · TCP 22", width: 300 },
];

const edges = [
  { from: "clients", to: "edge", label: "HTTPS 443 · UI / API / Git HTTP" },
  { from: "edge", to: "app1", label: "balance HTTP requests" },
  { from: "edge", to: "app2" }, { from: "edge", to: "app3" },
  { from: "clients", to: "ssh", label: "Git over SSH" },
  { from: "ssh", to: "app1", label: "GitLab Shell" },
  { from: "ssh", to: "app2" }, { from: "ssh", to: "app3" },
  { from: "app1", to: "internal-lb", label: "internal component connections" },
  { from: "app2", to: "internal-lb" }, { from: "app3", to: "internal-lb" },
  { from: "app1", to: "pgbouncer", label: "Rails queries" }, { from: "app2", to: "pgbouncer" }, { from: "app3", to: "pgbouncer" },
  { from: "sidekiq", to: "pgbouncer", label: "job persistence" },
  { from: "pgbouncer", to: "db", label: "pooled PostgreSQL sessions" },
  { from: "pg1", to: "pg2", label: "replication" }, { from: "pg1", to: "pg3", label: "replication" },
  { from: "app1", to: "redis", label: "sessions · cache · enqueue jobs" }, { from: "app2", to: "redis" }, { from: "app3", to: "redis" },
  { from: "sidekiq", to: "redis", label: "dequeue jobs" },
  { from: "redis", to: "coord", label: "Sentinel monitoring / failover" },
  { from: "internal-lb", to: "sidekiq", label: "internal service traffic" },
  { from: "app1", to: "gitaly-lb", label: "repository RPC via Praefect" }, { from: "app2", to: "gitaly-lb" }, { from: "app3", to: "gitaly-lb" },
  { from: "sidekiq", to: "gitaly-lb", label: "repository jobs" },
  { from: "gitaly-lb", to: "praefect", label: "gRPC balance" },
  { from: "praefect", to: "gitaly", label: "route Git RPC; replicate repositories" },
  { from: "praefect", to: "praefect-db", label: "cluster status metadata" },
  { from: "app1", to: "object-store", label: "uploads / artifacts / LFS" },
  { from: "sidekiq", to: "object-store", label: "background object reads / writes" },
  { from: "runner", to: "edge", label: "HTTPS API / clone / artifact exchange" },
  { from: "runner", to: "registry", label: "push / pull images" },
  { from: "app1", to: "registry", label: "registry API / auth" },
  { from: "prom", to: "app1", label: "scrape metrics" }, { from: "prom", to: "sidekiq" },
  { from: "prom", to: "gitaly" }, { from: "prom", to: "redis" },
];

export default { nodes, edges, style: { box }, rootAttrs: { "font-family": "sans-serif" } } satisfies Sketch<Node>;
