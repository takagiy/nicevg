import type { Sketch, SketchNode } from "nicevg";
import { box } from "nicevg/style/simple";

// Sources: https://docs.celeryq.dev/en/stable/userguide/routing.html
// https://docs.celeryq.dev/en/stable/userguide/configuration.html
// Shows a task named feeds.tasks.import_feed routed by task_routes to feed_tasks;
// the named Queue expands routing defaults, while an unmatched task uses the default queue.

type Node = SketchNode & { class: "box"; label: string; fill?: string };

export default {
  nodes: [
    { id: "apply", class: "box", x: 20, y: 150, label: "Task.apply_async()\nname: feeds.tasks.import_feed\noptions: args, kwargs", fill: "#dbeafe" },
    { id: "routers", class: "box", x: 280, y: 150, label: "app.conf.task_routes\nordered router configuration\nroute_task(name, args, kwargs, options)", fill: "#e0e7ff" },
    { id: "match", class: "box", x: 570, y: 40, label: "Router matches task name\nfeeds.tasks.import_feed\nreturns {queue: feed_tasks}", fill: "#fef3c7" },
    { id: "queue", class: "box", x: 850, y: 40, label: "task_queues: Queue('feed_tasks')\nExchange('feeds', type='direct')\nrouting_key='feed.import'", fill: "#dcfce7" },
    { id: "default", class: "box", x: 570, y: 300, label: "No router result\nuse task_default_queue\n+ default exchange and routing key", fill: "#f1f5f9" },
    { id: "options", class: "box", x: 1160, y: 150, label: "Resolved publish options\nqueue + exchange + routing_key\nexplicit apply_async options can override", fill: "#ffedd5" },
    { id: "broker", class: "box", x: 1470, y: 150, label: "Broker publish\nmessage carries task name, args, kwargs\nrouted with resolved exchange/key", fill: "#fae8ff" },
  ],
  edges: [
    { from: "apply", to: "routers", label: "task name + publish options" },
    { from: "routers", to: "match", label: "first router returning a route" },
    { from: "routers", to: "default", label: "no route returned" },
    { from: "match", to: "queue", label: "queue name lookup" },
    { from: "queue", to: "options", label: "fill exchange + routing_key" },
    { from: "default", to: "options", label: "default queue settings" },
    { from: "options", to: "broker", label: "publish task message" },
  ],
  style: { box },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<Node>;
