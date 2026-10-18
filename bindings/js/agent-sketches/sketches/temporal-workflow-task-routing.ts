import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";

// Temporal — https://docs.temporal.io/temporal, https://docs.temporal.io/tasks,
// https://docs.temporal.io/encyclopedia/event-history, https://docs.temporal.io/references/service-configuration
// Shows how a Workflow start and Workflow Task move through the Temporal Service,
// how a Worker replays Event History and returns Commands, and where state is stored.

type Node = {
  id: string;
  class: "box";
  x: number;
  y: number;
  label: string;
  fill?: string;
};

const nodes: Node[] = [
  { id: "starter", class: "box", x: 20, y: 180, label: "Starter program\nTemporal SDK Client", fill: "#dbeafe" },
  { id: "frontend", class: "box", x: 260, y: 180, label: "Frontend Service\npublic gRPC API", fill: "#e0e7ff" },
  { id: "history", class: "box", x: 530, y: 70, label: "History Service\nWorkflow state machine\nHistory Shard", fill: "#dcfce7" },
  { id: "matching", class: "box", x: 530, y: 330, label: "Matching Service\nTask Queue dispatch", fill: "#fef3c7" },
  { id: "worker", class: "box", x: 850, y: 200, label: "Worker Process\nWorkflow Task Poller\nWorkflow Definition", fill: "#fae8ff" },
  { id: "persistence", class: "box", x: 850, y: 30, label: "Default Persistence Store\nWorkflow mutable state\nEvent History", fill: "#fee2e2" },
  { id: "visibility", class: "box", x: 850, y: 440, label: "Visibility Store\nlist / filter executions", fill: "#ffedd5" },
  { id: "workflowtask", class: "box", x: 260, y: 430, label: "Workflow Task\nTask Queue: named by starter", fill: "#fef9c3" },
  { id: "historyevents", class: "box", x: 1150, y: 100, label: "Event History\nordered durable Events", fill: "#f1f5f9" },
  { id: "commands", class: "box", x: 1150, y: 300, label: "Commands\nSchedule Activity / Timer", fill: "#f1f5f9" },
  { id: "activityqueue", class: "box", x: 530, y: 520, label: "Activity Task Queue\nWorkflow-scheduled work", fill: "#fef3c7" },
  { id: "activityworker", class: "box", x: 850, y: 590, label: "Activity Worker\nregistered Activity code", fill: "#fae8ff" },
];

const sketch = {
  nodes,
  edges: [
    { from: "starter", to: "frontend", label: "StartWorkflowExecution" },
    { from: "frontend", to: "history", label: "start request" },
    { from: "history", to: "persistence", label: "append start Event; create state" },
    { from: "history", to: "visibility", label: "execution visibility record" },
    { from: "history", to: "matching", label: "schedule first Workflow Task" },
    { from: "matching", to: "workflowtask", label: "place on Task Queue" },
    { from: "worker", to: "matching", label: "long-poll Task Queue" },
    { from: "workflowtask", to: "worker", label: "deliver Workflow Task" },
    { from: "worker", to: "persistence", label: "load Event History for replay" },
    { from: "persistence", to: "historyevents", label: "durable Events" },
    { from: "historyevents", to: "worker", label: "replay from beginning" },
    { from: "worker", to: "commands", label: "Workflow code produces" },
    { from: "commands", to: "frontend", label: "complete Workflow Task" },
    { from: "frontend", to: "history", label: "record completion" },
    { from: "history", to: "persistence", label: "append Events; update state" },
    { from: "history", to: "matching", label: "schedule Activity Task" },
    { from: "matching", to: "activityqueue", label: "enqueue Activity Task" },
    { from: "activityworker", to: "matching", label: "poll Activity Task Queue" },
    { from: "matching", to: "activityworker", label: "deliver Activity Task" },
    { from: "activityworker", to: "frontend", label: "Activity result / failure" },
    { from: "frontend", to: "history", label: "record Activity outcome" },
    { from: "history", to: "matching", label: "schedule next Workflow Task" },
  ],
  style: { box },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<Node>;

export default sketch;
