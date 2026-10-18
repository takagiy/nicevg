import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";

// systemd — https://www.freedesktop.org/software/systemd/man/258/systemctl.html,
// https://github.com/systemd/systemd/blob/main/man/systemd.service.xml,
// https://github.com/systemd/systemd/blob/main/man/systemd.unit.xml
// Follows a service unit's ActiveState through start, running, stop, and failure;
// Restart=on-failure can schedule another activation, subject to start rate limiting.

const nodes = [
  { id: "inactive", class: "box", x: 40, y: 220, label: "inactive\nservice SubState: dead", fill: "#e2e8f0" },
  { id: "activating", class: "box", x: 350, y: 70, label: "activating\nstart job / ExecStart\nSubState varies by service", fill: "#fef3c7" },
  { id: "active", class: "box", x: 690, y: 70, label: "active\nservice SubState: running", fill: "#dcfce7" },
  { id: "deactivating", class: "box", x: 1010, y: 220, label: "deactivating\nstop job / ExecStop", fill: "#ffedd5" },
  { id: "failed", class: "box", x: 690, y: 420, label: "failed\nservice SubState: failed\nResult records the failure", fill: "#fee2e2" },
  { id: "unit_file", class: "box", x: 40, y: 450, label: "service unit file\nRestart=on-failure\nStartLimitIntervalSec= / Burst=", fill: "#dbeafe" },
  { id: "restart", class: "box", x: 350, y: 420, label: "restart scheduled\nnew start job\nactivating SubState: auto-restart", fill: "#fae8ff" },
] as const;

const sketch = {
  nodes,
  edges: [
    { from: "inactive", to: "activating", label: "start job; systemctl start" },
    { from: "activating", to: "active", label: "startup succeeds" },
    { from: "activating", to: "failed", label: "ExecStart / startup fails" },
    { from: "active", to: "deactivating", label: "stop job; systemctl stop" },
    { from: "deactivating", to: "inactive", label: "stop operation completes" },
    { from: "active", to: "inactive", label: "service exits cleanly on its own" },
    { from: "active", to: "failed", label: "nonzero exit, signal, or operation timeout" },
    { from: "unit_file", to: "failed", label: "Restart=on-failure; start limit exhausted" },
    { from: "failed", to: "restart", label: "failure matches Restart=; rate limit allows" },
    { from: "restart", to: "activating", label: "restart timer expires; start job" },
    { from: "failed", to: "inactive", label: "systemctl reset-failed" },
  ],
  style: { box },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<(typeof nodes)[number]>;

export default sketch;
