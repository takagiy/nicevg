import type { Sketch, SketchNode } from "nicevg";
import { box } from "nicevg/style/simple";

// Ray documentation and implementation:
// https://docs.ray.io/en/latest/ray-core/scheduling/index.html
// https://github.com/ray-project/ray/blob/master/src/ray/raylet/scheduling/policy/hybrid_scheduling_policy.h
// https://github.com/ray-project/ray/blob/master/src/ray/raylet/scheduling/policy/hybrid_scheduling_policy.cc
// This sketch follows one task or actor using Ray's DEFAULT hybrid policy. It shows hard resource
// eligibility, availability before utilization ranking, the top-k random choice, and wait outcomes.

type RayNode = SketchNode & { class: "box"; label: string };

export default {
  nodes: [
    { id: "work", class: "box", x: 30, y: 210, label: "Task / actor\nresource request\nargs + DEFAULT" },
    { id: "raylet", class: "box", x: 250, y: 210, label: "Local raylet\nClusterResourceScheduler" },
    { id: "hybrid", class: "box", x: 490, y: 210, label: "HybridSchedulingPolicy\nScheduleImpl" },
    { id: "nodes", class: "box", x: 735, y: 210, label: "Cluster nodes\nNodeResources + alive" },
    { id: "feasible", class: "box", x: 980, y: 80, label: "Feasible nodes\nrequired total resources fit" },
    { id: "infeasible", class: "box", x: 980, y: 380, label: "Infeasible nodes\nresource capacity too small\nor node not alive" },
    { id: "available", class: "box", x: 1210, y: 80, label: "Available set\nrequested resources free now" },
    { id: "busy", class: "box", x: 1210, y: 250, label: "Feasible, unavailable\ncapacity fits; resources busy" },
    { id: "locality", class: "box", x: 1450, y: 80, label: "Argument locality\nprefer node holding large args" },
    { id: "score", class: "box", x: 1450, y: 250, label: "ComputeNodeScore\ncritical utilization < threshold\nbecomes 0" },
    { id: "topk", class: "box", x: 1690, y: 170, label: "Priority order → top k\nk = max(absolute, fraction × nodes)" },
    { id: "pick", class: "box", x: 1930, y: 170, label: "GetBestNode\nrandom from top k\npreferred best node can bypass k" },
    { id: "schedule", class: "box", x: 2170, y: 80, label: "Chosen node\nrun task / actor" },
    { id: "wait", class: "box", x: 2170, y: 300, label: "No current placement\nwait for feasible resources\nor nodes to be added" },
  ],
  edges: [
    { from: "work", to: "raylet", label: "resource request, args, strategy" },
    { from: "raylet", to: "hybrid", label: "DEFAULT policy call" },
    { from: "hybrid", to: "nodes", label: "ScheduleImpl scans node state" },
    { from: "nodes", to: "feasible", label: "alive + total capacity fits" },
    { from: "nodes", to: "infeasible", label: "dead or capacity cannot fit" },
    { from: "feasible", to: "available", label: "free resources satisfy request" },
    { from: "feasible", to: "busy", label: "only total capacity satisfies" },
    { from: "available", to: "locality", label: "rank locality first" },
    { from: "locality", to: "score", label: "then load balance" },
    { from: "score", to: "topk", label: "lower score wins" },
    { from: "topk", to: "pick", label: "candidate nodes" },
    { from: "pick", to: "schedule", label: "node id" },
    { from: "busy", to: "wait", label: "selected only if no available node" },
    { from: "infeasible", to: "wait", label: "no eligible node" },
  ],
  style: { box },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<RayNode>;
