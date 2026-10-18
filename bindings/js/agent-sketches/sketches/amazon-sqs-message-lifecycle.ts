// Amazon SQS message lifecycle, based on https://docs.aws.amazon.com/AWSSimpleQueueService/latest/SQSDeveloperGuide/welcome.html,
// https://docs.aws.amazon.com/AWSSimpleQueueService/latest/APIReference/API_ReceiveMessage.html,
// and https://docs.aws.amazon.com/AWSSimpleQueueService/latest/SQSDeveloperGuide/sqs-configure-dead-letter-queue.html.
// This sketch follows one standard-queue message from SendMessage through processing, retry/redrive,
// and deletion or retention expiry, including the receipt handle used by message-level operations.

import type { Sketch } from "nicevg";
import { svg, text } from "nicevg";

type Node = {
  id: string; class: "frame" | "service" | "state" | "terminal"; x: number; y: number;
  label: string; parent?: string; width?: number; height?: number;
};

const service = ({ label }: Node) => {
  const t = text(label, 13); const w = Math.max(108, t.width + 28); const h = 48;
  return svg`<rect data-bounds="true" width="${w}" height="${h}" rx="8" fill="#dbeafe" stroke="#2563eb"/><path d="M 12 0 V ${h}" stroke="#2563eb" stroke-width="3"/>${t.at({ cx: w / 2 + 5, cy: h / 2 })}`;
};
const state = ({ label }: Node) => {
  const t = text(label, 12); const w = Math.max(112, t.width + 24); const h = 42;
  return svg`<path data-bounds="true" d="M 14 0 H ${w - 14} L ${w} 21 L ${w - 14} 42 H 14 L 0 21 Z" fill="#fef3c7" stroke="#d97706"/>${t.at({ cx: w / 2, cy: h / 2 })}`;
};
const terminal = ({ label }: Node) => {
  const t = text(label, 12); const w = Math.max(100, t.width + 26); const h = 42;
  return svg`<rect data-bounds="true" width="${w}" height="${h}" rx="21" fill="#dcfce7" stroke="#15803d"/>${t.at({ cx: w / 2, cy: h / 2 })}`;
};
const frame = ({ label, width = 500, height = 260 }: Node) => svg`
  <rect data-bounds="true" width="${width}" height="${height}" rx="14" fill="#f8fafc" fill-opacity="0.65" stroke="#94a3b8" stroke-dasharray="5 4"/>
  ${text(label, 13).at({ left: 12, top: 8 })}`;

const nodes: Node[] = [
  { id: "consumer", class: "frame", x: 370, y: 80, label: "Consumer process", width: 360, height: 210 },
  { id: "main", class: "frame", x: 60, y: 60, label: "Source queue", width: 470, height: 380 },
  { id: "dlq", class: "frame", x: 720, y: 410, label: "Dead-letter queue", width: 260, height: 150 },
  { id: "producer", class: "service", x: 30, y: 185, label: "Producer" },
  { id: "send", class: "service", x: 175, y: 160, label: "SendMessage", parent: "main" },
  { id: "stored", class: "state", x: 340, y: 130, label: "Stored message", parent: "main" },
  { id: "delay", class: "state", x: 120, y: 270, label: "Delivery delay", parent: "main" },
  { id: "visible", class: "state", x: 290, y: 255, label: "Visible", parent: "main" },
  { id: "receive", class: "service", x: 430, y: 145, label: "ReceiveMessage", parent: "consumer" },
  { id: "handle", class: "state", x: 565, y: 120, label: "ReceiptHandle", parent: "consumer" },
  { id: "inflight", class: "state", x: 400, y: 325, label: "In flight", parent: "main" },
  { id: "work", class: "service", x: 535, y: 240, label: "Process message", parent: "consumer" },
  { id: "extend", class: "service", x: 640, y: 165, label: "ChangeMessageVisibility", parent: "consumer" },
  { id: "success", class: "state", x: 775, y: 190, label: "Processing succeeds", parent: "consumer" },
  { id: "delete", class: "service", x: 680, y: 300, label: "DeleteMessage", parent: "consumer" },
  { id: "gone", class: "terminal", x: 830, y: 295, label: "Deleted", parent: "main" },
  { id: "timeout", class: "state", x: 245, y: 370, label: "Visibility timeout", parent: "main" },
  { id: "count", class: "state", x: 80, y: 475, label: "ReceiveCount", parent: "main" },
  { id: "redrive", class: "service", x: 370, y: 485, label: "RedrivePolicy", parent: "main" },
  { id: "dlqmsg", class: "state", x: 600, y: 470, label: "Moved after maxReceiveCount", parent: "dlq" },
  { id: "retention", class: "service", x: 140, y: 405, label: "MessageRetentionPeriod", parent: "main" },
  { id: "expired", class: "terminal", x: 360, y: 435, label: "Expired", parent: "main" },
];

const edges = [
  { from: "producer", to: "send" }, { from: "send", to: "stored" },
  { from: "stored", to: "delay" }, { from: "delay", to: "visible" },
  { from: "visible", to: "receive" }, { from: "receive", to: "handle" },
  { from: "receive", to: "inflight" }, { from: "handle", to: "work" },
  { from: "inflight", to: "work" }, { from: "work", to: "success" },
  { from: "work", to: "extend" }, { from: "extend", to: "inflight" },
  { from: "success", to: "delete" }, { from: "delete", to: "gone" },
  { from: "inflight", to: "timeout" }, { from: "timeout", to: "count" },
  { from: "count", to: "redrive" }, { from: "redrive", to: "visible" },
  { from: "redrive", to: "dlqmsg" }, { from: "dlqmsg", to: "dlq" },
  { from: "stored", to: "retention" }, { from: "retention", to: "expired" },
];

export default { nodes, edges, style: { frame, service, state, terminal }, rootAttrs: { "font-family": "sans-serif" } } satisfies Sketch<Node>;
