import type { Sketch, SketchNode } from "nicevg";
import { box } from "nicevg/style/simple";

// Actions Runner Controller webhook scaling decision:
// https://github.com/actions/actions-runner-controller/blob/master/docs/automatically-scaling-runners.md
// https://docs.github.com/en/actions/how-tos/write-workflows/choose-where-workflows-run/choose-the-runner-for-a-job
// Shows how a workflow_job webhook is matched to one HRA, reserved while queued, capped by maxReplicas,
// and later released by completion, cancellation, or timeout after checking runner occupancy.

type Node = SketchNode & { class: "box"; label: string; fill?: string };

export default {
  nodes: [
    { id: "github", class: "box", x: 30, y: 220, label: "GitHub Actions\nworkflow_job webhook\nstatus + labels", fill: "#dbeafe" },
    { id: "webhook_server", class: "box", x: 290, y: 220, label: "ARC webhook server\nreceives workflow_job\nevent", fill: "#e0e7ff" },
    { id: "status", class: "box", x: 550, y: 70, label: "Event status\nqueued / completed / cancelled", fill: "#fef3c7" },
    { id: "find_hra", class: "box", x: 820, y: 220, label: "Find HorizontalRunnerAutoscaler\nwebhook trigger backs target\nand runner labels match", fill: "#e0e7ff" },
    { id: "no_match", class: "box", x: 1100, y: 30, label: "No matching HRA\nno scale action", fill: "#fee2e2" },
    { id: "ambiguous", class: "box", x: 1100, y: 160, label: "More than one match\nevent ignored", fill: "#fee2e2" },
    { id: "queued", class: "box", x: 1100, y: 340, label: "status = queued\nadd capacityReservation\nexpires at now + duration", fill: "#ffedd5" },
    { id: "cap", class: "box", x: 1370, y: 340, label: "Running replicas < maxReplicas?\nHRA scale-up decision", fill: "#fef3c7" },
    { id: "scale_up", class: "box", x: 1650, y: 230, label: "Add replica\nEffectiveTime = now", fill: "#dcfce7" },
    { id: "wait_cap", class: "box", x: 1650, y: 430, label: "At maxReplicas\nreservation waits in queue\noldest becomes active next", fill: "#ffedd5" },
    { id: "release", class: "box", x: 820, y: 570, label: "completed / cancelled\nor reservation duration expires\nremove oldest reservation", fill: "#fef3c7" },
    { id: "busy_check", class: "box", x: 1100, y: 570, label: "Query GitHub API\nbefore choosing runner\nfor termination", fill: "#e0e7ff" },
    { id: "terminate", class: "box", x: 1370, y: 570, label: "Runner busy?\nkeep it; otherwise\nterminate scale-up runner", fill: "#dcfce7" },
  ],
  edges: [
    { from: "github", to: "webhook_server", label: "workflow_job payload" },
    { from: "webhook_server", to: "status", label: "status" },
    { from: "status", to: "find_hra", label: "queued event" },
    { from: "find_hra", to: "no_match", label: "zero matches" },
    { from: "find_hra", to: "ambiguous", label: "multiple matches" },
    { from: "find_hra", to: "queued", label: "exactly one HRA" },
    { from: "queued", to: "cap", label: "reservation + expiry" },
    { from: "cap", to: "scale_up", label: "below cap" },
    { from: "cap", to: "wait_cap", label: "at cap" },
    { from: "status", to: "release", label: "completed / cancelled" },
    { from: "queued", to: "release", label: "duration elapsed" },
    { from: "wait_cap", to: "release", label: "job finishes; oldest reservation advances" },
    { from: "release", to: "busy_check", label: "remove oldest reservation" },
    { from: "busy_check", to: "terminate", label: "GitHub runner status" },
    { from: "terminate", to: "wait_cap", label: "runner busy: retain capacity" },
  ],
  style: { box },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<Node>;
