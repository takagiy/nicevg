import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";

type Node = { id: string; class: "box"; x: number; y: number; label: string; parent?: string; width?: number; height?: number; fill?: string };

// Mattermost documentation: https://docs.mattermost.com/deployment-guide/application-architecture
// Mattermost deployment reference: https://docs.mattermost.com/deployment-guide/server/server-architecture.html
// Mattermost API WebSocket reference: https://github.com/mattermost/mattermost/blob/master/api/v4/source/introduction.yaml
// This sketch follows client requests through the load balancer and Mattermost Server to its
// persistent, real-time, notification and extension dependencies in a clustered deployment.
const nodes: Node[] = [
  { id: "web-client", class: "box", x: 40, y: 90, label: "Web client", parent: "access", fill: "#dbeafe" },
  { id: "desktop-mobile", class: "box", x: 40, y: 200, label: "Desktop / mobile apps", parent: "access", fill: "#dbeafe" },
  { id: "email-client", class: "box", x: 40, y: 330, label: "Email client", parent: "access", fill: "#dbeafe" },
  { id: "access", class: "box", x: 15, y: 45, label: "Access layer", width: 245, height: 400, fill: "#eff6ff" },
  { id: "reverse-proxy", class: "box", x: 310, y: 150, label: "NGINX / HAProxy\nTLS + load balancing", parent: "deployment", fill: "#fef3c7" },
  { id: "deployment", class: "box", x: 285, y: 40, label: "Mattermost deployment", width: 520, height: 650, fill: "#fffbeb" },
  { id: "server-a", class: "box", x: 470, y: 100, label: "Mattermost Server A", parent: "deployment", fill: "#dcfce7" },
  { id: "server-b", class: "box", x: 650, y: 100, label: "Mattermost Server B", parent: "deployment", fill: "#dcfce7" },
  { id: "rest-api", class: "box", x: 470, y: 200, label: "RESTful JSON Web Service", parent: "deployment" },
  { id: "auth-client", class: "box", x: 650, y: 200, label: "Authentication client", parent: "deployment" },
  { id: "data-management", class: "box", x: 470, y: 300, label: "Data Management Service", parent: "deployment" },
  { id: "notification-service", class: "box", x: 650, y: 300, label: "Notification Service", parent: "deployment" },
  { id: "websocket", class: "box", x: 470, y: 405, label: "WebSocket endpoint\n/api/v4/websocket", parent: "deployment" },
  { id: "cluster-gossip", class: "box", x: 650, y: 405, label: "Cluster gossip\nTCP/UDP 8074", parent: "deployment" },
  { id: "metrics", class: "box", x: 550, y: 505, label: "Metrics listener :8067", parent: "deployment" },
  { id: "postgres", class: "box", x: 900, y: 80, label: "PostgreSQL primary\nmessages · users · config\nteams / channels", parent: "backend", fill: "#f3e8ff" },
  { id: "replica", class: "box", x: 900, y: 205, label: "PostgreSQL read replica", parent: "backend", fill: "#f3e8ff" },
  { id: "file-storage", class: "box", x: 900, y: 345, label: "File storage\nlocal / NAS / S3", parent: "backend", fill: "#f3e8ff" },
  { id: "search-index", class: "box", x: 900, y: 465, label: "Elasticsearch / OpenSearch", parent: "backend", fill: "#f3e8ff" },
  { id: "backend", class: "box", x: 870, y: 35, label: "Backend infrastructure", width: 280, height: 590, fill: "#faf5ff" },
  { id: "idp", class: "box", x: 1175, y: 70, label: "LDAP / SSO / OAuth\nidentity provider", parent: "external", fill: "#fee2e2" },
  { id: "push-proxy", class: "box", x: 1175, y: 200, label: "Mattermost Push Proxy", parent: "external", fill: "#fee2e2" },
  { id: "apns", class: "box", x: 1390, y: 160, label: "Apple APNs", parent: "external", fill: "#f1f5f9" },
  { id: "fcm", class: "box", x: 1390, y: 260, label: "Google FCM", parent: "external", fill: "#f1f5f9" },
  { id: "smtp", class: "box", x: 1175, y: 360, label: "SMTP server", parent: "external", fill: "#f1f5f9" },
  { id: "integration", class: "box", x: 1175, y: 465, label: "Self-hosted integrations\nREST API / webhooks", parent: "external", fill: "#f1f5f9" },
  { id: "external", class: "box", x: 1150, y: 35, label: "External services", width: 470, height: 525, fill: "#fff1f2" },
  { id: "prometheus", class: "box", x: 550, y: 750, label: "Prometheus / monitoring", fill: "#f1f5f9" },
  { id: "channel-post", class: "box", x: 900, y: 700, label: "Post + channel context", fill: "#fef9c3" },
  { id: "client-users", class: "box", x: 40, y: 520, label: "Users", fill: "#f1f5f9" },
];

const edges = [
  { from: "web-client", to: "reverse-proxy", label: "HTTPS / WSS" },
  { from: "desktop-mobile", to: "reverse-proxy", label: "HTTPS / WSS APIs" },
  { from: "email-client", to: "smtp", label: "email notifications" },
  { from: "reverse-proxy", to: "server-a", label: "HTTP :8065" },
  { from: "reverse-proxy", to: "server-b", label: "HTTP :8065; persistent WS" },
  { from: "server-a", to: "server-b", label: "cluster events / gossip" },
  { from: "server-b", to: "server-a", label: "cluster events / gossip" },
  { from: "server-a", to: "rest-api", label: "dispatch API request" },
  { from: "server-b", to: "rest-api", label: "dispatch API request" },
  { from: "rest-api", to: "auth-client", label: "session / token validation" },
  { from: "auth-client", to: "idp", label: "LDAP / SSO exchange" },
  { from: "auth-client", to: "postgres", label: "user and session records" },
  { from: "rest-api", to: "data-management", label: "authorized operations" },
  { from: "data-management", to: "postgres", label: "read / write records" },
  { from: "postgres", to: "replica", label: "replication" },
  { from: "data-management", to: "file-storage", label: "upload / retrieve files" },
  { from: "data-management", to: "search-index", label: "index and query posts" },
  { from: "replica", to: "search-index", label: "search reads" },
  { from: "data-management", to: "channel-post", label: "persisted post + channel_id" },
  { from: "channel-post", to: "websocket", label: "posted event + broadcast scope" },
  { from: "websocket", to: "web-client", label: "WSS event: posted / typing" },
  { from: "websocket", to: "desktop-mobile", label: "WSS event: posted / typing" },
  { from: "desktop-mobile", to: "websocket", label: "user_typing(channel_id, parent_id)" },
  { from: "websocket", to: "cluster-gossip", label: "fan-out to connected users" },
  { from: "notification-service", to: "push-proxy", label: "mobile push payload" },
  { from: "push-proxy", to: "apns", label: "iOS push" },
  { from: "push-proxy", to: "fcm", label: "Android push" },
  { from: "notification-service", to: "smtp", label: "offline email alert" },
  { from: "rest-api", to: "integration", label: "integration API calls" },
  { from: "integration", to: "rest-api", label: "webhook callbacks" },
  { from: "metrics", to: "prometheus", label: "scrape metrics" },
  { from: "client-users", to: "web-client", label: "read / compose" },
  { from: "client-users", to: "desktop-mobile", label: "read / compose" },
];

export default { nodes, edges, style: { box }, rootAttrs: { "font-family": "sans-serif" } } satisfies Sketch<Node>;
