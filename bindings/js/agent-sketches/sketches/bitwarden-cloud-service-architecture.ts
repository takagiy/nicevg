import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";

type Node = { id: string; class: "box"; x: number; y: number; label: string; parent?: string; width?: number; height?: number; fill?: string };

// Bitwarden server: https://github.com/bitwarden/server
// Service and event paths: https://contributing.bitwarden.com/architecture/deep-dives/event-logs/
// Push paths: https://contributing.bitwarden.com/architecture/deep-dives/push-notifications/
// Authentication: https://github.com/bitwarden/contributing-docs/blob/main/docs/architecture/deep-dives/authentication/index.md
// This is the cloud-hosted Bitwarden service set, including the separate web client, identity,
// notifications and event processing paths, plus their database and Azure-backed queues/storage.
const nodes: Node[] = [
  { id: "browser", class: "box", x: 25, y: 90, label: "Browser extension", parent: "bitwarden", fill: "#dbeafe" },
  { id: "mobile", class: "box", x: 35, y: 200, label: "Mobile apps", parent: "bitwarden", fill: "#dbeafe" },
  { id: "cli", class: "box", x: 25, y: 320, label: "CLI", parent: "bitwarden", fill: "#dbeafe" },
  { id: "web", class: "box", x: 190, y: 55, label: "Web Vault", parent: "bitwarden" },
  { id: "api", class: "box", x: 350, y: 140, label: "API", parent: "bitwarden" },
  { id: "identity", class: "box", x: 335, y: 255, label: "Identity\n/connect/token", parent: "bitwarden" },
  { id: "notifications", class: "box", x: 555, y: 90, label: "Notifications API", parent: "bitwarden" },
  { id: "events", class: "box", x: 550, y: 235, label: "Events service\n/collect", parent: "bitwarden" },
  { id: "processor", class: "box", x: 755, y: 330, label: "EventsProcessor", parent: "bitwarden" },
  { id: "admin", class: "box", x: 190, y: 390, label: "Admin", parent: "bitwarden" },
  { id: "billing", class: "box", x: 360, y: 410, label: "Billing", parent: "bitwarden" },
  { id: "sso", class: "box", x: 555, y: 410, label: "SSO", parent: "bitwarden" },
  { id: "scim", class: "box", x: 740, y: 430, label: "SCIM", parent: "bitwarden" },
  { id: "sql", class: "box", x: 765, y: 90, label: "SQL Server\nBitwarden records", parent: "bitwarden", fill: "#f3e8ff" },
  { id: "event-queue", class: "box", x: 755, y: 200, label: "Azure Queue\nevent logs", parent: "bitwarden", fill: "#f3e8ff" },
  { id: "event-table", class: "box", x: 970, y: 310, label: "Azure Table Storage\nevent records", parent: "bitwarden", fill: "#f3e8ff" },
  { id: "push-queue", class: "box", x: 970, y: 85, label: "Azure Queue\npush notifications", parent: "bitwarden", fill: "#f3e8ff" },
  { id: "push-hub", class: "box", x: 1170, y: 185, label: "Azure Notification Hub\nmobile push", parent: "bitwarden", fill: "#f3e8ff" },
  { id: "bitwarden", class: "box", x: 5, y: 15, label: "Bitwarden cloud service", width: 1385, height: 510, fill: "#fffbeb" },
];

const edges = [
  { from: "browser", to: "api", label: "HTTPS API / sync" },
  { from: "mobile", to: "api", label: "HTTPS API / sync" },
  { from: "cli", to: "api", label: "HTTPS API" },
  { from: "web", to: "api", label: "vault API" },
  { from: "browser", to: "identity", label: "POST /connect/token" },
  { from: "mobile", to: "identity", label: "POST /connect/token" },
  { from: "api", to: "identity", label: "internal client-credentials token" },
  { from: "api", to: "sql", label: "vault / user records" },
  { from: "identity", to: "sql", label: "users / auth data" },
  { from: "api", to: "notifications", label: "enqueue push message" },
  { from: "notifications", to: "push-queue", label: "AzureQueuePushNotificationService" },
  { from: "push-queue", to: "notifications", label: "AzureQueueHostedService reads" },
  { from: "notifications", to: "browser", label: "SignalR WebSocket updates" },
  { from: "notifications", to: "mobile", label: "mobile push" },
  { from: "push-hub", to: "mobile", label: "APNs / FCM delivery" },
  { from: "api", to: "push-hub", label: "mobile notification relay" },
  { from: "browser", to: "events", label: "POST /collect" },
  { from: "mobile", to: "events", label: "POST /collect" },
  { from: "api", to: "event-queue", label: "server-side events" },
  { from: "events", to: "event-queue", label: "AzureQueueEventWriteService" },
  { from: "event-queue", to: "processor", label: "dequeue event logs" },
  { from: "processor", to: "event-table", label: "EventRepository writes" },
  { from: "api", to: "event-table", label: "EventsController queries" },
  { from: "admin", to: "api", label: "admin operations" },
  { from: "billing", to: "api", label: "billing operations" },
  { from: "sso", to: "identity", label: "authorization-code flow" },
  { from: "scim", to: "api", label: "organization provisioning" },
];

export default { nodes, edges, style: { box }, rootAttrs: { "font-family": "sans-serif" } } satisfies Sketch<Node>;
