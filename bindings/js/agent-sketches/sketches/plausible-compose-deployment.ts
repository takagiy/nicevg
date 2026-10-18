/**
 * Plausible Community Edition deployment, based on https://plausible.io/docs/self-hosting and https://github.com/plausible/community-edition/blob/v3.2.1/compose.yml.
 * Shows the documented single-host Docker Compose setup: the public entry point, Plausible container, its two databases and persistent data, plus configured outbound email.
 */
import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";

type Node = { id: string; class: "box"; x: number; y: number; parent?: string; label: string; width?: number; height?: number; fill?: string };

const sketch = {
  nodes: [
    { id: "dns", class: "box", x: 40, y: 170, label: "DNS\nBASE_URL host", fill: "#e0f2fe" },
    { id: "visitor", class: "box", x: 40, y: 340, label: "Site visitor\nbrowser", fill: "#fef3c7" },
    { id: "website", class: "box", x: 250, y: 340, label: "Tracked website\nplausible.js", fill: "#fef3c7" },
    { id: "host", class: "box", x: 260, y: 80, label: "Docker host\nDocker Compose", width: 800, height: 500, fill: "#f1f5f9" },
    { id: "env", class: "box", parent: "host", x: 300, y: 120, label: ".env\nBASE_URL, SECRET_KEY_BASE", fill: "#ede9fe" },
    { id: "plausible", class: "box", parent: "host", x: 590, y: 150, label: "plausible\nHTTP :8000 / configured ports", fill: "#dcfce7" },
    { id: "pg", class: "box", parent: "host", x: 340, y: 340, label: "plausible_db\nPostgreSQL 16", fill: "#dbeafe" },
    { id: "pgdata", class: "box", parent: "host", x: 340, y: 460, label: "db-data volume\n/var/lib/postgresql/data", fill: "#eff6ff" },
    { id: "ch", class: "box", parent: "host", x: 720, y: 340, label: "plausible_events_db\nClickHouse :8123", fill: "#ffedd5" },
    { id: "eventdata", class: "box", parent: "host", x: 700, y: 460, label: "event-data volume\n/var/lib/clickhouse", fill: "#fff7ed" },
    { id: "eventlogs", class: "box", parent: "host", x: 900, y: 460, label: "event-logs volume\nClickHouse logs", fill: "#fff7ed" },
    { id: "smtp", class: "box", x: 1100, y: 360, label: "SMTP provider\nconfigured externally", fill: "#fce7f3" },
  ],
  edges: [
    { from: "visitor", to: "website", label: "loads site" },
    { from: "website", to: "plausible", label: "POST /api/event" },
    { from: "visitor", to: "dns", label: "opens BASE_URL" },
    { from: "dns", to: "plausible", label: "resolves host; HTTP(S)" },
    { from: "env", to: "plausible", label: "runtime configuration" },
    { from: "plausible", to: "pg", label: "DATABASE_URL; app data" },
    { from: "plausible", to: "ch", label: "CLICKHOUSE_DATABASE_URL; stats" },
    { from: "pg", to: "pgdata", label: "persist database files" },
    { from: "ch", to: "eventdata", label: "persist event data" },
    { from: "ch", to: "eventlogs", label: "persist logs" },
    { from: "plausible", to: "smtp", label: "SMTP verification email" },
  ],
  style: { box },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<Node>;

export default sketch;


