// Argo CD sync phases and waves: https://argo-cd.readthedocs.io/en/stable/user-guide/sync-waves/
// Annotation reference: https://argo-cd.readthedocs.io/en/stable/user-guide/annotations-and-labels/
// Shows how a sync operation orders resources and chooses the next phase/wave, then waits for health before advancing.

import type { Sketch, SketchNode } from "nicevg";
import { box } from "nicevg/style/simple";

type Node = SketchNode & { class: "box"; label: string; fill?: string };

export default {
  nodes: [
    { id: "sync", class: "box", x: 30, y: 220, label: "Application sync\noperation", fill: "#dbeafe" },
    { id: "manifests", class: "box", x: 250, y: 220, label: "Desired manifests\nKubernetes resources + hooks" },
    { id: "skip", class: "box", x: 500, y: 35, label: "hook: Skip\nDo not apply" },
    { id: "phase", class: "box", x: 500, y: 200, label: "Resolve phase\nhook annotation; default Sync" },
    { id: "wave", class: "box", x: 760, y: 200, label: "Resolve wave\nsync-wave integer; default 0" },
    { id: "order", class: "box", x: 1020, y: 200, label: "Order resources\nphase → wave → kind → name" },
    { id: "pending", class: "box", x: 1280, y: 200, label: "Find first phase/wave\nwith out-of-sync OR unhealthy resource" },
    { id: "done", class: "box", x: 1530, y: 35, label: "No pending phase/wave\nSync complete" },
    { id: "apply", class: "box", x: 1530, y: 200, label: "Apply resources\nin selected wave" },
    { id: "delay", class: "box", x: 1530, y: 380, label: "Wait between waves\n2 seconds by default\nARGOCD_SYNC_WAVE_DELAY" },
    { id: "health", class: "box", x: 1280, y: 380, label: "Assess resource health\nand sync status" },
    { id: "healthy", class: "box", x: 1020, y: 380, label: "Selected wave\nin-sync AND healthy?" },
    { id: "blocked", class: "box", x: 760, y: 470, label: "Remain at this wave\nLater waves wait; unhealthy first wave can block sync" },
    { id: "hooks", class: "box", x: 500, y: 380, label: "Failure path\nSyncFail hooks run on sync failure" },
    { id: "post", class: "box", x: 250, y: 380, label: "PostSync hooks\nrun after successful apply\nand all resources Healthy" },
  ] satisfies Node[],
  edges: [
    { from: "sync", to: "manifests", label: "desired resource set" },
    { from: "manifests", to: "skip", label: "hook annotation is Skip" },
    { from: "manifests", to: "phase", label: "other resources" },
    { from: "phase", to: "wave", label: "phase + resource" },
    { from: "wave", to: "order", label: "phase and integer wave" },
    { from: "order", to: "pending", label: "ordered set" },
    { from: "pending", to: "done", label: "none out of sync or unhealthy" },
    { from: "pending", to: "apply", label: "earliest pending wave" },
    { from: "apply", to: "delay", label: "spec changes" },
    { from: "delay", to: "health", label: "controller reaction time" },
    { from: "health", to: "healthy", label: "health + sync status" },
    { from: "healthy", to: "pending", label: "yes: select next pending wave" },
    { from: "healthy", to: "blocked", label: "no: wait / operation cannot advance" },
    { from: "blocked", to: "health", label: "reassess status" },
    { from: "apply", to: "hooks", label: "sync fails; SyncFail hook" },
    { from: "done", to: "post", label: "successful sync; resources Healthy" },
  ],
  style: { box },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<Node>;
