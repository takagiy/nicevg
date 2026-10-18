import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";

// Linux kernel — https://docs.kernel.org/next/scheduler/sched-eevdf.html,
// https://github.com/torvalds/linux/blob/master/kernel/sched/fair.c
// Shows how the fair scheduling class chooses a runnable sched_entity: eligibility
// is measured against the cfs_rq virtual-time average, then the earliest virtual deadline wins.

type Node = {
  id: string;
  class: "box";
  x: number;
  y: number;
  label: string;
  fill?: string;
};

const nodes: Node[] = [
  { id: "runnable", class: "box", x: 20, y: 210, label: "Runnable task\nSCHED_NORMAL", fill: "#dbeafe" },
  { id: "entity", class: "box", x: 260, y: 210, label: "sched_entity\nvruntime · vlag · slice\ndeadline", fill: "#e0e7ff" },
  { id: "runqueue", class: "box", x: 520, y: 60, label: "cfs_rq\navg_vruntime / avg_load\nrb-tree of runnable entities", fill: "#dcfce7" },
  { id: "eligible", class: "box", x: 790, y: 60, label: "entity_eligible()\nvruntime at or behind\nweighted average", fill: "#fef3c7" },
  { id: "deadline", class: "box", x: 520, y: 380, label: "update_deadline()\ndeadline = vruntime +\ncalc_delta_fair(slice, se)", fill: "#fae8ff" },
  { id: "pick", class: "box", x: 1060, y: 210, label: "pick_eevdf()\nearliest-deadline eligible\nentity in rb-tree", fill: "#fee2e2" },
  { id: "cpu", class: "box", x: 1320, y: 210, label: "Current task on CPU\nexec runtime updates\nvruntime", fill: "#ffedd5" },
];

const sketch = {
  nodes,
  edges: [
    { from: "runnable", to: "entity", label: "task's scheduling entity" },
    { from: "entity", to: "runqueue", label: "enqueue_entity(); insert by vruntime" },
    { from: "runqueue", to: "eligible", label: "average virtual time" },
    { from: "entity", to: "eligible", label: "vruntime candidate" },
    { from: "entity", to: "deadline", label: "slice request and vruntime" },
    { from: "deadline", to: "runqueue", label: "set virtual deadline; order tree" },
    { from: "runqueue", to: "pick", label: "rb-tree candidates; subtree min_vruntime" },
    { from: "eligible", to: "pick", label: "eligible entities only" },
    { from: "pick", to: "cpu", label: "set_next_entity() / run" },
    { from: "cpu", to: "entity", label: "task_tick_fair(): account runtime" },
    { from: "cpu", to: "pick", label: "earlier virtual deadline may preempt" },
  ],
  style: { box },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<Node>;

export default sketch;
