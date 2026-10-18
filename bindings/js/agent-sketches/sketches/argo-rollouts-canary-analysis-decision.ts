// Argo Rollouts canary analysis, based on:
// https://github.com/argoproj/argo-rollouts/blob/master/docs/features/analysis.md
// https://github.com/argoproj/argo-rollouts/blob/master/analysis/analysis.go
// https://github.com/argoproj/argo-rollouts/blob/master/pkg/apis/rollouts/v1alpha1/analysis_types.go
// It follows an inline canary Analysis step through metric evaluation to the AnalysisRun phase
// that controls rollout progression. Provider errors are kept distinct from failed measurements:
// they increment error counters, use the error retry interval, and can end as Error.

import type { Sketch, SketchNode } from "nicevg";
import { svg, text } from "nicevg";

type Area = SketchNode & { class: "area"; label: string; width: number; height: number };
type RecordNode = SketchNode & { class: "record"; label: string; width?: number; color?: string };
type Decision = SketchNode & { class: "decision"; label: string; color?: string };
type Note = SketchNode & { class: "note"; label: string; color?: string };
type Node = Area | RecordNode | Decision | Note;

const area = ({ label, width, height }: Area) => svg`
  <rect data-bounds="true" width="${width}" height="${height}" fill="#f8fafc" stroke="#94a3b8" stroke-dasharray="5 3"/>
  ${text(label, 13).at({ left: 12, top: 8 })}`;

const record = ({ label, width = 230, color = "#dbeafe" }: RecordNode) => {
  const lines = label.split("\n");
  const body = text(lines.slice(1).join("\n"), 12);
  const head = text(lines[0]!, 13);
  const w = Math.max(width, head.width + 24, body.width + 24);
  const h = 18 + body.height + 20;
  return svg`<rect data-bounds="true" width="${w}" height="${h}" fill="white" stroke="#334155"/>
    <path d="M 0 28 H ${w}" stroke="#64748b"/><rect width="${w}" height="28" fill="${color}" stroke="none"/>
    ${head.at({ cx: w / 2, cy: 14 })}${body.at({ left: 10, top: 34 })}`;
};

const decision = ({ label, color = "#fef3c7" }: Decision) => {
  const name = text(label, 12);
  const w = name.width + 42, h = name.height + 34;
  return svg`<path data-bounds="true" d="M ${w / 2} 0 L ${w} ${h / 2} L ${w / 2} ${h} L 0 ${h / 2} Z" fill="${color}" stroke="#92400e"/>
    ${name.at({ cx: w / 2, cy: h / 2 })}`;
};

const note = ({ label, color = "#fff7ed" }: Note) => {
  const name = text(label, 12);
  const w = Math.max(160, name.width + 24), h = name.height + 20;
  return svg`<path data-bounds="true" d="M 0 0 H ${w - 12} L ${w} 12 V ${h} H 0 Z" fill="${color}" stroke="#9a3412"/>
    ${name.at({ left: 10, top: 10 })}`;
};

const nodes: Node[] = [
  { id: "controller", class: "area", x: 20, y: 20, width: 1540, height: 740, label: "Argo Rollouts controller · analysis assessment and rollout reconciliation" },
  { id: "provider", class: "area", x: 1080, y: 105, width: 430, height: 285, label: "External metric provider" },
  { id: "canary-step", class: "record", parent: "controller", x: 55, y: 90, label: "Rollout.spec.strategy.canary.steps\nanalysis.templates[] · args[]" },
  { id: "template", class: "record", parent: "controller", x: 55, y: 230, label: "AnalysisTemplate.spec\nargs · metrics[]", color: "#e0e7ff" },
  { id: "metric", class: "record", parent: "controller", x: 340, y: 205, width: 280, label: "Metric\ninterval · count · initialDelay\nsuccessCondition · failureCondition\nfailureLimit · inconclusiveLimit\nconsecutiveErrorLimit · dryRun", color: "#e0e7ff" },
  { id: "run-spec", class: "record", parent: "controller", x: 340, y: 390, label: "AnalysisRun.spec\nargs[] · resolved metrics[]\nterminate · dryRun[]" },
  { id: "arg-resolution", class: "note", parent: "controller", x: 685, y: 105, label: "Resolve {{args.*}} in metric\nprovider query and conditions" },
  { id: "prom-config", class: "record", parent: "provider", x: 1110, y: 165, label: "Prometheus provider\naddress · query", color: "#dcfce7" },
  { id: "measurement-result", class: "note", parent: "provider", x: 1110, y: 275, label: "provider.Run(run, metric)\nvalue[] or provider error", color: "#dcfce7" },
  { id: "measurement", class: "record", parent: "controller", x: 685, y: 270, label: "Measurement\nphase · value · message\nstartedAt · finishedAt", color: "#f1f5f9" },
  { id: "metric-result", class: "record", parent: "controller", x: 685, y: 410, width: 245, label: "MetricResult\nphase · measurements[]\ncount · successful · failed\ninconclusive · error\nconsecutiveError / Success", color: "#f1f5f9" },
  { id: "assess-metric", class: "decision", parent: "controller", x: 990, y: 430, label: "assessMetricStatus\nlimits exceeded?" },
  { id: "retry", class: "note", parent: "controller", x: 930, y: 620, label: "Error measurement: retry after\nDefaultErrorRetryInterval\nConsecutiveError > limit → Error", color: "#fee2e2" },
  { id: "assess-run", class: "decision", parent: "controller", x: 1240, y: 445, label: "assessRunStatus\nworst metric phase" },
  { id: "run-status", class: "record", parent: "controller", x: 1260, y: 290, label: "AnalysisRun.status\nphase · message · runSummary\ndryRunSummary · completedAt", color: "#f1f5f9" },
  { id: "rollout-action", class: "decision", parent: "controller", x: 1280, y: 565, label: "Rollout action\nSuccessful → continue\nFailed / Error → abort\nInconclusive → pause" },
  { id: "events", class: "note", parent: "controller", x: 690, y: 600, label: "Kubernetes event\ncompletion: Error / Failed use Warning" },
  { id: "requeue", class: "note", parent: "controller", x: 55, y: 540, label: "calculateNextReconcileTime\ninterval / error retry → enqueue" },
  { id: "pause-condition", class: "record", parent: "controller", x: 1280, y: 680, label: "Rollout.status.pauseConditions[]\nreason: InconclusiveAnalysisRun", color: "#ffedd5" },
];

const edges = [
  { from: "canary-step", to: "template", label: "template reference + args" },
  { from: "template", to: "metric", label: "metric definitions" },
  { from: "metric", to: "run-spec", label: "instantiate AnalysisRun" },
  { from: "run-spec", to: "arg-resolution", label: "arguments" },
  { from: "arg-resolution", to: "prom-config", label: "substituted query" },
  { from: "prom-config", to: "measurement-result", label: "query response / error" },
  { from: "measurement-result", to: "measurement" },
  { from: "metric", to: "measurement", label: "conditions evaluate result[]" },
  { from: "measurement", to: "metric-result", label: "append + increment counters" },
  { from: "metric-result", to: "assess-metric" },
  { from: "assess-metric", to: "retry", label: "phase Error; not terminal yet" },
  { from: "retry", to: "prom-config", label: "re-run provider" },
  { from: "assess-metric", to: "assess-run", label: "metric terminal phase" },
  { from: "metric-result", to: "requeue", label: "unfinished interval or error retry" },
  { from: "requeue", to: "prom-config", label: "next reconcile" },
  { from: "assess-run", to: "run-status", label: "aggregate phase + message" },
  { from: "run-status", to: "events", label: "completed run" },
  { from: "run-status", to: "rollout-action", label: "phase" },
  { from: "rollout-action", to: "pause-condition", label: "Inconclusive" },
  { from: "rollout-action", to: "canary-step", label: "Successful: resume next step" },
];

export default {
  nodes,
  edges,
  style: { area, record, decision, note },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<Node>;
