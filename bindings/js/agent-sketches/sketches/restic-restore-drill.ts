// Restic restore procedure, based on https://github.com/restic/restic/blob/master/doc/050_restore.rst,
// https://github.com/restic/restic/blob/master/doc/040_backup.rst, and https://github.com/restic/restic/blob/master/doc/design.rst.
// It follows an operator selecting and dry-running a snapshot restore, Restic reading repository objects,
// writing the restored tree to a target, and the operator checking the recovered files.
import type { Sketch } from "nicevg";
import { box } from "nicevg/style/simple";

type Node = {
  id: string;
  class: "box";
  x: number;
  y: number;
  label: string;
  width?: number;
  height?: number;
  fill?: string;
};

const sketch = {
  nodes: [
    { id: "operator", class: "box", x: 35, y: 150, label: "Operator\nchoose snapshot ID / host / path", width: 240, height: 76, fill: "#dbeafe" },
    { id: "safety", class: "box", x: 335, y: 55, label: "Protect current target\nbackup it before in-place restore", width: 240, height: 76, fill: "#fef3c7" },
    { id: "inspect", class: "box", x: 335, y: 220, label: "restic snapshots / ls / find\nselect snapshot; locate paths", width: 240, height: 76, fill: "#ede9fe" },
    { id: "dryrun", class: "box", x: 655, y: 150, label: "restic restore --dry-run -v 2\nreview planned files and filters", width: 250, height: 76, fill: "#ffedd5" },
    { id: "repository", class: "box", x: 975, y: 55, label: "Restic repository\nsnapshots → index → packs\nconfig, keys, trees and data blobs", width: 255, height: 92, fill: "#dcfce7" },
    { id: "restore", class: "box", x: 975, y: 240, label: "restic restore <snapshot>\n--target <directory>\n--include /path (optional)", width: 255, height: 92, fill: "#ede9fe" },
    { id: "target", class: "box", x: 1295, y: 150, label: "Restore target\nrecreated paths, contents, metadata", width: 240, height: 76, fill: "#dcfce7" },
    { id: "verify", class: "box", x: 1610, y: 150, label: "Operator verifies recovered files\nthen resumes service / reports result", width: 255, height: 76, fill: "#fee2e2" },
  ] satisfies Node[],
  edges: [
    { from: "operator", to: "safety", label: "if restoring in place" },
    { from: "safety", to: "operator", label: "current-state snapshot" },
    { from: "operator", to: "inspect", label: "repository + password" },
    { from: "inspect", to: "dryrun", label: "chosen snapshot / include paths" },
    { from: "dryrun", to: "repository", label: "read metadata and planned tree" },
    { from: "repository", to: "dryrun", label: "snapshot paths and file list" },
    { from: "dryrun", to: "operator", label: "review plan; adjust selection if needed" },
    { from: "operator", to: "restore", label: "approved restore command" },
    { from: "restore", to: "repository", label: "load snapshot, then index and packs" },
    { from: "repository", to: "restore", label: "tree / data blobs and metadata" },
    { from: "restore", to: "target", label: "write restored files under target" },
    { from: "target", to: "verify", label: "recovered data" },
  ],
  style: { box },
} satisfies Sketch<Node>;

export default sketch;
