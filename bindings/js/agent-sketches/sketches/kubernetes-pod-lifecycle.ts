// Kubernetes Pod lifecycle, based on https://kubernetes.io/docs/concepts/workloads/pods/pod-lifecycle/
// and https://kubernetes.io/docs/concepts/workloads/pods/. It traces one Pod API object's phase
// and its containers from admission and scheduling through restart, graceful deletion, and cleanup.

import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";

const nodes = [
  { id: "accepted", class: "box", x: 40, y: 80, label: "API server accepts Pod\nmetadata.uid assigned" },
  { id: "pending", class: "box", x: 270, y: 80, label: "Pod phase: Pending\nunscheduled / setup / image pull" },
  { id: "scheduler", class: "box", x: 270, y: 230, label: "kube-scheduler\nbinds Pod to one Node" },
  { id: "kubelet", class: "box", x: 550, y: 230, label: "kubelet\nsandbox, network, volumes" },
  { id: "waiting", class: "box", x: 550, y: 80, label: "Container state: Waiting\nimage pull, Secret setup" },
  { id: "running", class: "box", x: 830, y: 80, label: "Pod phase: Running\nat least one container active" },
  { id: "container", class: "box", x: 830, y: 230, label: "Container state: Running\npostStart complete" },
  { id: "terminated", class: "box", x: 1110, y: 230, label: "Container state: Terminated\nreason, exitCode, times" },
  { id: "restart", class: "box", x: 830, y: 400, label: "restartPolicy\nAlways / OnFailure / Never" },
  { id: "unknown", class: "box", x: 40, y: 400, label: "Pod phase: Unknown\nkubelet status unavailable" },
  { id: "terminal", class: "box", x: 1110, y: 80, label: "Pod phase: Succeeded or Failed\nall containers terminated" },
  { id: "delete", class: "box", x: 40, y: 570, label: "kubectl delete\nAPI sets deletionTimestamp\nTerminating is kubectl display status" },
  { id: "shutdown", class: "box", x: 340, y: 570, label: "kubelet graceful shutdown\npreStop, then runtime sends TERM\nterminationGracePeriodSeconds (default 30s)" },
  { id: "forced", class: "box", x: 680, y: 570, label: "Grace expires with processes alive\nruntime sends KILL" },
  { id: "api_remove", class: "box", x: 1110, y: 570, label: "API server removes Pod object\nforce delete may skip node confirmation" },
  { id: "podgc", class: "box", x: 1110, y: 400, label: "PodGC\ncleans terminated Pods by threshold\nand orphan / unscheduled cases" },
] as const;

const sketch = {
  nodes,
  edges: [
    { from: "accepted", to: "pending", label: "Pod spec accepted" },
    { from: "pending", to: "scheduler", label: "schedule / bind once" },
    { from: "scheduler", to: "kubelet", label: "Pod assigned to Node" },
    { from: "kubelet", to: "waiting", label: "create containers" },
    { from: "waiting", to: "running", label: "container starts" },
    { from: "running", to: "container", label: "status in PodStatus" },
    { from: "container", to: "terminated", label: "exit or system termination" },
    { from: "terminated", to: "restart", label: "container exit" },
    { from: "restart", to: "waiting", label: "policy allows restart; backoff on repeated crashes" },
    { from: "restart", to: "terminal", label: "no restart; all containers done" },
    { from: "terminated", to: "terminal", label: "all done; success → Succeeded, failure → Failed" },
    { from: "running", to: "unknown", label: "node communication lost" },
    { from: "unknown", to: "terminal", label: "lost-node policy marks Failed" },
    { from: "delete", to: "shutdown", label: "deletion timestamp + grace period" },
    { from: "shutdown", to: "container", label: "preStop / TERM while draining" },
    { from: "shutdown", to: "forced", label: "deadline reached" },
    { from: "forced", to: "terminal", label: "kubelet records exit outcome" },
    { from: "terminal", to: "api_remove", label: "kubelet requests API deletion" },
    { from: "terminal", to: "podgc", label: "Succeeded / Failed remains in API" },
    { from: "podgc", to: "api_remove", label: "delete object" },
  ],
  style: { box },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<(typeof nodes)[number]>;

export default sketch;
