// NATS JetStream. Sources: https://docs.nats.io/concepts/jetstream and
// https://github.com/nats-io/nats.docs/blob/master/nats-concepts/jetstream/consumers.md
// Shows how a published subject is stored in a Stream, then delivered by a stateful Consumer;
// the client acknowledgment advances its position, while a missing acknowledgment causes redelivery.
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
    { id: "publisher", class: "box", x: 40, y: 120, label: "Publisher\nsubject: orders.created" },
    { id: "stream", class: "box", x: 280, y: 105, label: "Stream ORDERS\nsubject filter: orders.>\nmessage + stream sequence" },
    { id: "consumer", class: "box", x: 570, y: 105, label: "Consumer\ndelivery state + ack floor\nfilter / delivery policy" },
    { id: "client", class: "box", x: 860, y: 105, label: "Application client\nprocess message" },
    { id: "ack", class: "box", x: 860, y: 290, label: "Ack or timeout" },
    { id: "redelivery", class: "box", x: 570, y: 290, label: "Redeliver unacked\nmessage" },
  ],
  edges: [
    { from: "publisher", to: "stream", label: "publish message; subject matches" },
    { from: "stream", to: "consumer", label: "stored message by stream sequence" },
    { from: "consumer", to: "client", label: "deliver message" },
    { from: "client", to: "ack", label: "ack after processing" },
    { from: "ack", to: "consumer", label: "ack advances consumer position" },
    { from: "ack", to: "redelivery", label: "ack deadline expires" },
    { from: "redelivery", to: "client", label: "same message; at-least-once" },
  ],
  style: { box },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<Node>;

export default sketch;
