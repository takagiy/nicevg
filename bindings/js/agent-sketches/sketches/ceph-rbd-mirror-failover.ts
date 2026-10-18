// Ceph RBD mirroring failover procedure, based on https://docs.ceph.com/en/latest/rbd/rbd-mirroring/.
// It follows an orderly site switch and the forced-promotion/split-brain recovery path for one mirrored image.

import type { Sketch, SketchNode } from "nicevg";
import { box, circle } from "nicevg/style/simple";

type DiagramNode = SketchNode & { class: "box" | "circle"; label: string; width?: number; height?: number; fill?: string };

const nodes: DiagramNode[] = [
  { id: "site-a", class: "box", x: 70, y: 80, width: 450, height: 510, label: "site-a（現 primary）", fill: "#eff6ff" },
  { id: "site-b", class: "box", x: 680, y: 100, width: 500, height: 610, label: "site-b（peer）", fill: "#f0fdf4" },
  { id: "operator", class: "box", x: 30, y: 20, label: "運用担当" },
  { id: "alert", class: "box", parent: "site-a", x: 100, y: 130, label: "site-a 障害を検知" },
  { id: "planned", class: "circle", parent: "site-a", x: 330, y: 130, label: "旧側へ接続可能？" },
  { id: "stop-vm", class: "box", parent: "site-a", x: 100, y: 230, label: "VM を停止" },
  { id: "detach", class: "box", parent: "site-a", x: 320, y: 230, label: "VM から RBD を切離す" },
  { id: "demote", class: "box", parent: "site-a", x: 180, y: 330, label: "rbd mirror image demote" },
  { id: "demoted", class: "circle", parent: "site-a", x: 360, y: 420, label: "demote を peer に伝達？" },
  { id: "forced", class: "box", parent: "site-b", x: 720, y: 160, label: "rbd mirror image promote --force" },
  { id: "normal", class: "box", parent: "site-b", x: 950, y: 260, label: "rbd mirror image promote" },
  { id: "resume", class: "box", parent: "site-b", x: 760, y: 350, label: "site-b の VM を起動" },
  { id: "write", class: "box", parent: "site-b", x: 970, y: 350, label: "site-b primary へ I/O" },
  { id: "status", class: "box", parent: "site-b", x: 760, y: 450, label: "rbd mirror image status" },
  { id: "split", class: "circle", parent: "site-b", x: 1000, y: 470, label: "split-brain 検出？" },
  { id: "owner", class: "box", parent: "site-b", x: 730, y: 560, label: "最新データを残す側を決定" },
  { id: "stale-demote", class: "box", parent: "site-a", x: 100, y: 490, label: "古い側を demote" },
  { id: "resync", class: "box", parent: "site-b", x: 900, y: 570, label: "rbd mirror image resync" },
  { id: "daemon", class: "box", parent: "site-b", x: 940, y: 640, label: "rbd-mirror が非同期再同期" },
  { id: "snapshot", class: "box", parent: "site-b", x: 710, y: 640, label: "snapshot mirror: 最新 snapshot 作成" },
  { id: "synced", class: "circle", parent: "site-b", x: 1110, y: 640, label: "同期回復？" },
  { id: "healthy", class: "box", parent: "site-b", x: 1040, y: 700, label: "status を確認・運用継続" },
  { id: "recheck", class: "box", parent: "site-b", x: 810, y: 700, label: "status を再確認" },
];

const edges = [
  { from: "operator", to: "alert" },
  { from: "alert", to: "planned" },
  { from: "planned", to: "stop-vm" },
  { from: "planned", to: "forced", label: "不可" },
  { from: "stop-vm", to: "detach" },
  { from: "detach", to: "demote" },
  { from: "demote", to: "demoted" },
  { from: "demoted", to: "normal" },
  { from: "demoted", to: "forced", label: "未伝達" },
  { from: "forced", to: "resume" },
  { from: "normal", to: "resume" },
  { from: "resume", to: "write" },
  { from: "write", to: "status" },
  { from: "status", to: "split" },
  { from: "split", to: "healthy", label: "なし" },
  { from: "split", to: "owner", label: "あり" },
  { from: "owner", to: "stale-demote" },
  { from: "stale-demote", to: "resync" },
  { from: "resync", to: "daemon" },
  { from: "daemon", to: "snapshot", label: "snapshot mode" },
  { from: "daemon", to: "synced", label: "journal mode" },
  { from: "snapshot", to: "synced" },
  { from: "synced", to: "healthy" },
  { from: "synced", to: "recheck", label: "未回復" },
  { from: "recheck", to: "synced" },
];

export default { nodes, edges, style: { box, circle }, rootAttrs: { "font-family": "sans-serif" } } satisfies Sketch<DiagramNode>;
