// Zulip message sending, based on https://zulip.readthedocs.io/en/8.1/subsystems/sending-messages.html,
// https://github.com/zulip/zulip/blob/main/docs/subsystems/events-system.md, and
// https://github.com/zulip/zulip/blob/main/docs/subsystems/queuing.md.
// It follows a channel message from compose validation through the Django send path and commit,
// then through RabbitMQ and Tornado to the active clients for its recipients (including the sender).
import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";

type Node = {
  id: string;
  class: "box";
  x: number;
  y: number;
  label: string;
};

const sketch = {
  nodes: [
    { id: "compose", class: "box", x: 30, y: 130, label: "Compose box\nclient-side checks; local echo" },
    { id: "send-view", class: "box", x: 300, y: 130, label: "POST /messages\nsend_message_backend" },
    { id: "check-message", class: "box", x: 570, y: 130, label: "check_message\nrecipient permission + Markdown" },
    { id: "invalid", class: "box", x: 570, y: 330, label: "Validation error\nHTTP error response" },
    { id: "database", class: "box", x: 850, y: 130, label: "do_send_messages\ncreate Message in transaction" },
    { id: "notify-queue", class: "box", x: 1140, y: 130, label: "RabbitMQ notify_tornado\nmessage event + recipient users" },
    { id: "event-queue", class: "box", x: 1440, y: 130, label: "Tornado event_queue.py\nprocess_message_event" },
    { id: "clients", class: "box", x: 1740, y: 130, label: "Active recipient clients\napply message event" },
  ],
  edges: [
    { from: "compose", to: "send-view", label: "POST content, stream, topic" },
    { from: "send-view", to: "check-message", label: "validated endpoint arguments" },
    { from: "check-message", to: "invalid", label: "invalid content or recipient" },
    { from: "invalid", to: "compose", label: "error response; show failure" },
    { from: "check-message", to: "database", label: "checked message data" },
    { from: "database", to: "notify-queue", label: "on commit: message event" },
    { from: "notify-queue", to: "event-queue", label: "queued event JSON" },
    { from: "event-queue", to: "clients", label: "message event to subscribed users" },
  ],
  style: { box },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<Node>;

export default sketch;
