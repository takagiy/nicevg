// Linux kernel scheduler: https://docs.kernel.org/scheduler/sched-design-CFS.html
// https://docs.kernel.org/scheduler/sched-eevdf.html
// https://github.com/torvalds/linux/blob/master/kernel/sched/fair.c
// Tracks the fair-class task selection change from CFS's minimum-vruntime choice
// to EEVDF's eligibility and virtual-deadline choice, including sleeper handling.
import type { Sketch, SketchNode } from "nicevg";
import { svg, text } from "nicevg";

type Part = SketchNode & { class: "module" | "decision" | "version"; label: string; detail?: string };
const node = ({ label, detail, class: kind }: Part) => {
  const a = text(label, 13);
  const b = detail ? text(detail, 11) : undefined;
  const width = Math.max(a.width, b?.width ?? 0) + 24;
  const height = a.height + (b ? b.height + 8 : 0) + 20;
  const shape = kind === "decision"
    ? svg`<polygon data-bounds="true" points="${width / 2},0 ${width},${height / 2} ${width / 2},${height} 0,${height / 2}" fill="#fef3c7" stroke="#92400e"/>`
    : kind === "version"
      ? svg`<rect data-bounds="true" width="${width}" height="${height}" rx="18" fill="#e0e7ff" stroke="#4338ca"/>`
      : svg`<rect data-bounds="true" width="${width}" height="${height}" rx="3" fill="#e0f2fe" stroke="#075985"/>`;
  return svg`${shape}${a.at({ cx: width / 2, cy: b ? 12 + a.height / 2 : height / 2 })}${b?.at({ cx: width / 2, cy: 12 + a.height + 4 + b.height / 2 }) ?? ""}`;
};

const nodes: Part[] = [
  { id: "v6_5", class: "version", x: 20, y: 20, label: "旧: Linux 6.5 / CFS" },
  { id: "fair_old", class: "module", x: 40, y: 100, label: "kernel/sched/fair.c", detail: "pick_next_entity()" },
  { id: "cfs_rq", class: "module", x: 260, y: 100, label: "struct cfs_rq", detail: "min_vruntime / tasks_timeline" },
  { id: "vruntime", class: "module", x: 260, y: 220, label: "sched_entity.vruntime", detail: "実行時間を重みで正規化" },
  { id: "leftmost", class: "decision", x: 500, y: 150, label: "最小 vruntime を選択" },
  { id: "run_old", class: "module", x: 740, y: 150, label: "次の runnable task", detail: "より左の entity が実行" },
  { id: "v6_6", class: "version", x: 20, y: 360, label: "新: Linux 6.6+ / EEVDF" },
  { id: "fair_new", class: "module", x: 40, y: 440, label: "kernel/sched/fair.c", detail: "pick_eevdf()" },
  { id: "lag", class: "module", x: 260, y: 380, label: "entity_lag()", detail: "平均仮想時間との差" },
  { id: "eligible", class: "decision", x: 500, y: 390, label: "lag ≥ 0 ?", detail: "公平分を受け取る権利" },
  { id: "deadline", class: "module", x: 260, y: 520, label: "虚拟 deadline", detail: "request slice と重みから算出" },
  { id: "tree", class: "module", x: 740, y: 360, label: "仮想 deadline 順の RB tree", detail: "eligible entity を探索" },
  { id: "earliest", class: "decision", x: 980, y: 390, label: "eligible 中の最早 VD" },
  { id: "run_new", class: "module", x: 1210, y: 390, label: "次の runnable task", detail: "先行 VD なら実行中を preempt" },
  { id: "sleep", class: "module", x: 740, y: 540, label: "deferred dequeue", detail: "sleep 中も rq に残し lag を減衰" },
  { id: "api", class: "module", x: 980, y: 540, label: "sched_setattr()", detail: "要求する time slice" },
];

const edges = [
  { from: "v6_5", to: "fair_old" }, { from: "fair_old", to: "cfs_rq" },
  { from: "cfs_rq", to: "vruntime" }, { from: "vruntime", to: "leftmost", label: "順序キー" },
  { from: "leftmost", to: "run_old" }, { from: "v6_6", to: "fair_new" },
  { from: "fair_new", to: "lag" }, { from: "fair_new", to: "tree" },
  { from: "lag", to: "eligible", label: "lag" }, { from: "eligible", to: "tree", label: "eligible のみ" },
  { from: "deadline", to: "tree", label: "VD" }, { from: "tree", to: "earliest" },
  { from: "earliest", to: "run_new" }, { from: "run_new", to: "fair_new", label: "実行後に再選択" },
  { from: "sleep", to: "fair_new", label: "長く眠れば lag reset" },
  { from: "api", to: "deadline", label: "slice 指定" },
];

export default { nodes, edges, style: { module: node, decision: node, version: node }, rootAttrs: { "font-family": "sans-serif" } } satisfies Sketch<Part>;
