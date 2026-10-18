// Nextflow run and task execution, based on https://www.nextflow.io/docs/latest/config.html,
// https://www.nextflow.io/docs/latest/executor.html, and https://training.nextflow.io/latest/config_exec/02_resources_and_retries/.
// It follows a user's `nextflow run` through configuration, task dispatch and a failed attempt's
// retry, including the work files and run records that explain what happened.
import type { Sketch } from "nicevg";
import { svg, text } from "nicevg";

type Node = { id: string; class: "system" | "step" | "decision"; x: number; y: number; label: string; width: number; height: number; fill?: string };
const system = ({ label, width, height, fill = "#e0f2fe" }: Node) => {
  const t = text(label, 13); const w = Math.max(width, t.width + 28); const h = Math.max(height, t.height + 22);
  return svg`<rect data-bounds="true" width="${w}" height="${h}" rx="8" fill="${fill}" stroke="#334155"/>${t.at({ cx: w / 2, cy: h / 2 })}`;
};
const step = ({ label, width, height, fill = "#fef3c7" }: Node) => {
  const t = text(label, 13); const w = Math.max(width, t.width + 26); const h = Math.max(height, t.height + 20);
  return svg`<rect data-bounds="true" width="${w}" height="${h}" rx="18" fill="${fill}" stroke="#92400e"/>${t.at({ cx: w / 2, cy: h / 2 })}`;
};
const decision = ({ label, width, height, fill = "#fee2e2" }: Node) => {
  const t = text(label, 12); const w = Math.max(width, t.width + 42); const h = Math.max(height, t.height + 28);
  return svg`<path data-bounds="true" d="M ${w / 2} 0 L ${w} ${h / 2} L ${w / 2} ${h} L 0 ${h / 2} Z" fill="${fill}" stroke="#991b1b"/>${t.at({ cx: w / 2, cy: h / 2 })}`;
};

const sketch = {
  nodes: [
    { id: "user", class: "system", x: 40, y: 210, label: "Researcher\nstarts a run", width: 145, height: 60 },
    { id: "cli", class: "step", x: 235, y: 210, label: "nextflow run main.nf\n--input reads.fastq", width: 185, height: 62 },
    { id: "project", class: "system", x: 220, y: 55, label: "Pipeline project\nmain.nf + nextflow.config", width: 205, height: 64 },
    { id: "params", class: "step", x: 465, y: 205, label: "Workflow parameters\nparams.input", width: 185, height: 62 },
    { id: "config", class: "system", x: 465, y: 55, label: "Config resolution\nCLI > project > home > defaults", width: 215, height: 66 },
    { id: "profile", class: "system", x: 720, y: 55, label: "Selected profile\nprocess.executor = slurm", width: 195, height: 64 },
    { id: "workflow", class: "step", x: 710, y: 205, label: "Workflow entry\nvalidate and emit reads", width: 195, height: 62 },
    { id: "channel", class: "step", x: 955, y: 205, label: "Input channel\nreads.fastq item", width: 175, height: 62 },
    { id: "process", class: "step", x: 1170, y: 205, label: "Process task\nFASTQC(reads)", width: 170, height: 62 },
    { id: "hash", class: "step", x: 1140, y: 55, label: "Task hash\nscript + inputs + config", width: 195, height: 62 },
    { id: "cache", class: "system", x: 1400, y: 55, label: "$HOME/.nextflow/cache\nsession task records", width: 195, height: 62 },
    { id: "work", class: "system", x: 1400, y: 205, label: "work/<hash>\n.command.sh / .command.run", width: 205, height: 62 },
    { id: "executor", class: "system", x: 1650, y: 205, label: "Nextflow Slurm executor\nsubmit + poll job", width: 205, height: 62 },
    { id: "slurm", class: "system", x: 1650, y: 55, label: "Slurm scheduler\nsbatch job ID", width: 185, height: 62 },
    { id: "compute", class: "system", x: 1900, y: 205, label: "Compute node\nrun task wrapper", width: 180, height: 62 },
    { id: "files", class: "system", x: 1900, y: 355, label: ".command.out / .command.err\n.command.exitcode", width: 215, height: 62, fill: "#f3e8ff" },
    { id: "exit", class: "decision", x: 1650, y: 365, label: "exit status\n0?", width: 130, height: 90 },
    { id: "strategy", class: "system", x: 1400, y: 365, label: "process.errorStrategy\nretry / finish / terminate", width: 205, height: 62, fill: "#fee2e2" },
    { id: "retry", class: "step", x: 1160, y: 505, label: "maxRetries permits?\nnew task attempt", width: 190, height: 62 },
    { id: "log", class: "system", x: 920, y: 365, label: ".nextflow.log\nsubmission, exit, retry events", width: 215, height: 62, fill: "#dcfce7" },
    { id: "publish", class: "step", x: 710, y: 365, label: "workflow output\npublishDir results/", width: 185, height: 62, fill: "#dcfce7" },
    { id: "report", class: "system", x: 465, y: 365, label: "-with-trace / -with-report\ntask status + timing", width: 215, height: 62, fill: "#dcfce7" },
  ] satisfies Node[],
  edges: [
    { from: "user", to: "cli" }, { from: "cli", to: "project", label: "pipeline source" },
    { from: "cli", to: "params", label: "--input" }, { from: "project", to: "workflow", label: "workflow + process definitions" },
    { from: "project", to: "config", label: "nextflow.config" }, { from: "cli", to: "config", label: "-profile" },
    { from: "config", to: "profile" }, { from: "profile", to: "process", label: "executor selection" },
    { from: "params", to: "workflow" }, { from: "workflow", to: "channel" }, { from: "channel", to: "process" },
    { from: "process", to: "hash" }, { from: "hash", to: "cache", label: "lookup by session + task hash" },
    { from: "cache", to: "process", label: "hit: reuse outputs; skip execution" }, { from: "cache", to: "work", label: "miss: allocate task directory" },
    { from: "process", to: "work", label: "render task wrapper" }, { from: "work", to: "executor" }, { from: "executor", to: "slurm" },
    { from: "slurm", to: "compute", label: "scheduled job" }, { from: "compute", to: "files", label: "stdout, stderr, exit code" },
    { from: "files", to: "exit" }, { from: "exit", to: "strategy", label: "nonzero" }, { from: "strategy", to: "retry" },
    { from: "retry", to: "process", label: "retry: rerun task with attempt incremented" },
    { from: "exit", to: "publish", label: "success: declared outputs" }, { from: "publish", to: "report", label: "published files + task metadata" },
    { from: "executor", to: "log", label: "submit / poll / completion" }, { from: "exit", to: "log", label: "failure or success" },
    { from: "retry", to: "log", label: "retry attempt recorded" }, { from: "report", to: "user", label: "inspect run" },
  ],
  style: { system, step, decision },
} satisfies Sketch<Node>;

export default sketch;
