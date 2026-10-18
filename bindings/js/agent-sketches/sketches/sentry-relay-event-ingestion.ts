// Sentry Relay. Sources:
// https://github.com/getsentry/relay
// https://github.com/getsentry/relay/blob/master/relay/relay/src/main.rs
// https://github.com/getsentry/relay/blob/master/relay/relay-server/src/services/store.rs
// https://github.com/getsentry/relay/blob/master/relay/relay-server/src/statsd.rs
// https://github.com/getsentry/sentry/blob/master/src/sentry/tasks/store.py
// Traces an SDK error event through Relay's processing mode, Kafka, and Sentry's event store;
// the HTTP acknowledgement returns to the SDK while downstream consumers process the event.

import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";

type Node = {
  id: string;
  class: "box";
  x: number;
  y: number;
  label: string;
  width?: number;
  height?: number;
};

const sketch = {
  nodes: [
    { id: "sdk", class: "box", x: 30, y: 180, label: "Sentry SDK\nPOST /api/{project_id}/envelope/\nEnvelope: event + attachments", width: 230, height: 78 },
    { id: "lb", class: "box", x: 300, y: 150, label: "Load balancer / ingress\nroute project DSN to Relay", width: 220, height: 68 },
    { id: "relay", class: "box", x: 570, y: 160, label: "Relay service\nprocessing mode enabled\nHTTP store endpoint", width: 230, height: 78 },
    { id: "project", class: "box", x: 570, y: 340, label: "Project config cache\nkeys, quotas, filters,\nscrubbing rules", width: 220, height: 74 },
    { id: "validate", class: "box", x: 860, y: 150, label: "Envelope processing\nvalidate + normalize\ninbound filters + rate limits", width: 250, height: 78 },
    { id: "scrub", class: "box", x: 1160, y: 150, label: "Data scrubbing\nremove configured PII\nnormalize event payload", width: 235, height: 74 },
    { id: "ack", class: "box", x: 860, y: 350, label: "HTTP response\naccepted item IDs / status", width: 220, height: 66 },
    { id: "store", class: "box", x: 1450, y: 150, label: "relay-server Store service\nStoreEvent + attachments\nselect ingest topic", width: 260, height: 78 },
    { id: "kafka", class: "box", x: 1760, y: 150, label: "Kafka\ningest-events\ningest-attachments", width: 230, height: 76 },
    { id: "consumer", class: "box", x: 2050, y: 150, label: "Sentry ingest consumer\ndecode Kafka message\nroute event item", width: 250, height: 76 },
    { id: "preprocess", class: "box", x: 2350, y: 150, label: "sentry.tasks.store.save_event\npreprocessors + StoreNormalizer\nproject / event context", width: 270, height: 78 },
    { id: "snuba", class: "box", x: 2670, y: 70, label: "Snuba\ningest event for search\nKafka-backed analytics path", width: 250, height: 72 },
    { id: "db", class: "box", x: 2670, y: 270, label: "Sentry event store\nPostgreSQL metadata + event\nissue grouping / event_id", width: 250, height: 76 },
  ],
  edges: [
    { from: "sdk", to: "lb", label: "DSN-authenticated envelope request" },
    { from: "lb", to: "relay", label: "HTTP envelope" },
    { from: "relay", to: "project", label: "project key → project config" },
    { from: "project", to: "validate", label: "project settings + quotas" },
    { from: "relay", to: "validate", label: "envelope items" },
    { from: "validate", to: "scrub", label: "valid, allowed event" },
    { from: "validate", to: "ack", label: "rejected / rate limited → status" },
    { from: "scrub", to: "store", label: "normalized, scrubbed event + attachments" },
    { from: "store", to: "kafka", label: "event → ingest-events; attachment → ingest-attachments" },
    { from: "store", to: "ack", label: "successful Kafka publish" },
    { from: "ack", to: "sdk", label: "HTTP acknowledgement" },
    { from: "kafka", to: "consumer", label: "event message (async)" },
    { from: "consumer", to: "preprocess", label: "decoded event + project scope" },
    { from: "preprocess", to: "snuba", label: "processed event for search / analytics" },
    { from: "preprocess", to: "db", label: "event data + grouping / persistence work" },
  ],
  style: { box },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<Node>;

export default sketch;
