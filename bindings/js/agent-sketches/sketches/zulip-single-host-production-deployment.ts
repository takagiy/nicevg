// Zulip production deployment and architecture:
// https://github.com/zulip/zulip/blob/main/docs/production/deployment.md
// https://github.com/zulip/zulip/blob/main/docs/overview/architecture-overview.md
// https://github.com/zulip/zulip/blob/main/docs/subsystems/queuing.md
// https://github.com/zulip/zulip/blob/main/docs/subsystems/events-system.md
// A standard single-machine installation, with the externally visible HTTP routes,
// app processes, internal RabbitMQ work queues, persistent/cache services, and outbound integrations.

import type { Sketch, SketchNode } from "nicevg";
import { box } from "nicevg/style/simple";

type BoxNode = SketchNode & { class: "box"; label: string; width?: number; height?: number };

const nodes = [
  { id: "client", class: "box", x: 20, y: 170, label: "Web / API clients" },
  { id: "host", class: "box", x: 330, y: 20, label: "Zulip application host\nUbuntu or Debian" },
  { id: "nginx", parent: "host", class: "box", x: 390, y: 100, label: "nginx\nHTTPS frontend" },
  { id: "static", parent: "host", class: "box", x: 700, y: 30, label: "/home/zulip/prod-static/\ncompiled static assets" },
  { id: "uploads", parent: "host", class: "box", x: 700, y: 115, label: "LOCAL_UPLOADS_DIR\navatars, emoji, files" },
  { id: "supervisor", parent: "host", class: "box", x: 390, y: 310, label: "supervisord\nstarts, restarts, logs" },
  { id: "uwsgi", parent: "host", class: "box", x: 700, y: 250, label: "uWSGI\nDjango application server" },
  { id: "django", parent: "host", class: "box", x: 980, y: 250, label: "Django app\nall non-event HTTP paths" },
  { id: "tornado", parent: "host", class: "box", x: 700, y: 390, label: "Tornado\n/json/events, /api/v1/events" },
  { id: "rabbit", parent: "host", class: "box", x: 1290, y: 330, label: "RabbitMQ\ninternal queues" },
  { id: "notify", parent: "host", class: "box", x: 980, y: 390, label: "notify_tornado\nqueue consumer" },
  { id: "emailworker", parent: "host", class: "box", x: 980, y: 490, label: "events_email_senders\nqueue worker" },
  { id: "activityworker", parent: "host", class: "box", x: 980, y: 590, label: "user_activity\nqueue worker" },
  { id: "pushworker", parent: "host", class: "box", x: 980, y: 780, label: "mobile push\nqueue worker" },
  { id: "postgres", parent: "host", class: "box", x: 1290, y: 30, label: "PostgreSQL\npersistent Zulip data" },
  { id: "memcached", parent: "host", class: "box", x: 1290, y: 110, label: "Memcached\nmodel-object cache" },
  { id: "redis", parent: "host", class: "box", x: 1290, y: 190, label: "Redis\nshort-term data, rate limits" },
  { id: "smokescreen", parent: "host", class: "box", x: 1290, y: 500, label: "Smokescreen\noutgoing HTTP CONNECT proxy" },
  { id: "smtp", class: "box", x: 1650, y: 30, label: "SMTP server" },
  { id: "push", class: "box", x: 1650, y: 120, label: "Mobile push services" },
  { id: "webhooks", class: "box", x: 1650, y: 210, label: "Webhook destinations\nand public Internet" },
  { id: "camo", parent: "host", class: "box", x: 980, y: 690, label: "Camo image proxy" },
  { id: "s3", class: "box", x: 1650, y: 690, label: "S3-compatible\nfile storage (optional)" },
  { id: "supervisorconfig", parent: "host", class: "box", x: 390, y: 500, label: "supervisor/zulip.conf.template.erb\nprocess definitions" },
  { id: "nginxconfig", parent: "host", class: "box", x: 390, y: 600, label: "nginx app frontend config\npath routing" },
  { id: "deployments", parent: "host", class: "box", x: 700, y: 530, label: "/home/zulip/deployments/current\nrelease code symlink" },
  { id: "cron", parent: "host", class: "box", x: 700, y: 650, label: "cron jobs\nperiodic maintenance" },
  { id: "emailqueue", parent: "host", class: "box", x: 1290, y: 590, label: "email queue\nreliable deferred work" },
  { id: "pushqueue", parent: "host", class: "box", x: 1290, y: 670, label: "mobile push queue" },
  { id: "analyticsqueue", parent: "host", class: "box", x: 1290, y: 750, label: "analytics / user_activity queue" },
  { id: "clientfiles", class: "box", x: 20, y: 320, label: "Uploaded file requests" },
  { id: "browserstatic", class: "box", x: 20, y: 440, label: "Static asset requests" },
  { id: "clientpoll", class: "box", x: 20, y: 560, label: "Long-poll event requests" },
  { id: "events", parent: "host", class: "box", x: 390, y: 750, label: "Tornado event queues\nin-memory per-client queues" },
] satisfies BoxNode[];

const edges = [
  { from: "client", to: "nginx", label: "HTTPS requests" },
  { from: "nginx", to: "static", label: "/static/* file reads" },
  { from: "nginx", to: "uploads", label: "local uploaded content" },
  { from: "nginx", to: "uwsgi", label: "other paths; Unix uwsgi-socket" },
  { from: "uwsgi", to: "django", label: "WSGI requests" },
  { from: "nginx", to: "tornado", label: "/json/events and /api/v1/events" },
  { from: "clientpoll", to: "nginx", label: "GET /json/events; queue_id, last_event_id" },
  { from: "tornado", to: "clientpoll", label: "held HTTP response with events" },
  { from: "clientfiles", to: "nginx", label: "uploaded-content URL" },
  { from: "browserstatic", to: "nginx", label: "/static/*" },
  { from: "django", to: "postgres", label: "persistent reads and writes" },
  { from: "django", to: "memcached", label: "model-object cache get / invalidate" },
  { from: "django", to: "redis", label: "rate-limit and short-lived data" },
  { from: "django", to: "rabbit", label: "queue_event_on_commit; deferred work" },
  { from: "django", to: "rabbit", label: "send_event_on_commit → notify_tornado" },
  { from: "rabbit", to: "notify", label: "notify_tornado messages (JSON events)" },
  { from: "notify", to: "tornado", label: "event, target user IDs / data" },
  { from: "tornado", to: "events", label: "register queues; enqueue events; ack by last_event_id" },
  { from: "events", to: "tornado", label: "waiting handler / queued event" },
  { from: "rabbit", to: "emailqueue", label: "email work" },
  { from: "emailqueue", to: "emailworker", label: "queued email notifications" },
  { from: "emailworker", to: "smokescreen", label: "outbound email-related HTTP" },
  { from: "emailworker", to: "smtp", label: "SMTP delivery" },
  { from: "rabbit", to: "pushqueue", label: "mobile push work" },
  { from: "pushqueue", to: "pushworker", label: "queued mobile notifications" },
  { from: "pushworker", to: "smokescreen", label: "HTTP CONNECT" },
  { from: "smokescreen", to: "push", label: "push provider requests" },
  { from: "django", to: "smokescreen", label: "webhooks, previews, mobile push" },
  { from: "smokescreen", to: "webhooks", label: "CONNECT; public destinations allowed" },
  { from: "django", to: "camo", label: "remote image proxy requests" },
  { from: "camo", to: "webhooks", label: "fetch remote images" },
  { from: "django", to: "s3", label: "optional S3 storage API" },
  { from: "rabbit", to: "analyticsqueue", label: "analytics events" },
  { from: "analyticsqueue", to: "activityworker", label: "user activity work" },
  { from: "activityworker", to: "postgres", label: "update analytics tables" },
  { from: "supervisorconfig", to: "supervisor", label: "generated process configuration" },
  { from: "supervisor", to: "uwsgi", label: "start / restart; log output" },
  { from: "supervisor", to: "tornado", label: "start / restart; log output" },
  { from: "supervisor", to: "notify", label: "start queue processors" },
  { from: "supervisor", to: "emailworker", label: "start / restart; log output" },
  { from: "supervisor", to: "activityworker", label: "start / restart; log output" },
  { from: "nginxconfig", to: "nginx", label: "frontend routing rules" },
  { from: "deployments", to: "uwsgi", label: "Django release code" },
  { from: "deployments", to: "tornado", label: "Tornado release code" },
  { from: "cron", to: "postgres", label: "scheduled maintenance / database tasks" },
] satisfies NonNullable<Sketch<BoxNode>["edges"]>;

export default {
  nodes,
  edges,
  style: { box },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<BoxNode>;
