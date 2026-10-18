// Bugzilla's default bug workflow, based on https://bugzilla.readthedocs.io/en/latest/using/editing.html
// and https://bugzilla.readthedocs.io/en/5.2/administering/workflow.html. Shows a report moving
// from triage through development and QA, including a bad fix being reopened and sent back to work.

import { svg, text, type Sketch } from "nicevg";

const state = ({ label }: { label: string }) => {
  const t = text(label, 14);
  const w = Math.max(130, t.width + 30), h = t.height + 24;
  return svg`<rect width="${w}" height="${h}" rx="3" fill="#fff7d6" stroke="#765d22" stroke-width="2"/>${t.at({ cx: w / 2, cy: h / 2 })}`;
};

const person = ({ label }: { label: string }) => {
  const t = text(label, 13);
  const w = Math.max(120, t.width + 28), h = t.height + 30;
  return svg`<path d="M 14 0 H ${w - 14} L ${w} ${h / 2} L ${w - 14} ${h} H 14 L 0 ${h / 2} Z" fill="#e1efff" stroke="#315d87" stroke-width="1.5"/>${t.at({ cx: w / 2, cy: h / 2 })}`;
};

const work = ({ label }: { label: string }) => {
  const t = text(label, 13);
  const w = Math.max(135, t.width + 28), h = t.height + 24;
  return svg`<path d="M 0 0 H ${w - 16} L ${w} 16 V ${h} H 0 Z" fill="#e8f4e8" stroke="#3e7042" stroke-width="1.5"/>${t.at({ cx: w / 2, cy: h / 2 })}`;
};

const nodes = [
  { id: "reporter", class: "person", x: 24, y: 30, label: "報告者\n再現手順・環境" },
  { id: "unconfirmed", class: "state", x: 210, y: 56, label: "UNCONFIRMED\n未確認" },
  { id: "triage", class: "work", x: 420, y: 18, label: "トリアージ\n再現・妥当性を確認" },
  { id: "new", class: "state", x: 620, y: 80, label: "NEW\n確認済み" },
  { id: "assigned", class: "state", x: 825, y: 30, label: "ASSIGNED\n担当中" },
  { id: "developer", class: "person", x: 805, y: 200, label: "開発者\n原因調査・修正" },
  { id: "resolved", class: "state", x: 1090, y: 72, label: "RESOLVED\nFIXED" },
  { id: "qa", class: "person", x: 1300, y: 20, label: "QA\n修正を検証" },
  { id: "verified", class: "state", x: 1500, y: 90, label: "VERIFIED\n検証済み" },
  { id: "reopened", class: "state", x: 1240, y: 270, label: "REOPENED\n不具合が残る" },
  { id: "closed", class: "state", x: 1720, y: 40, label: "CLOSED\n完了" },
] as const;

const sketch = {
  nodes,
  edges: [
    { from: "reporter", to: "unconfirmed", label: "新規 Bug（説明・再現手順）" },
    { from: "unconfirmed", to: "triage", label: "確認依頼" },
    { from: "triage", to: "new", label: "有効と確認" },
    { from: "triage", to: "resolved", label: "無効・重複等と判断" },
    { from: "new", to: "assigned", label: "担当を引き受ける" },
    { from: "assigned", to: "developer", label: "担当 Bug・調査" },
    { from: "developer", to: "resolved", label: "修正をチェックイン" },
    { from: "resolved", to: "qa", label: "修正版・検証依頼" },
    { from: "qa", to: "verified", label: "修正が機能" },
    { from: "qa", to: "reopened", label: "修正が不十分（失敗）" },
    { from: "reopened", to: "developer", label: "再調査・修正へ戻す" },
    { from: "verified", to: "closed", label: "製品リリース" },
  ],
  style: { state, person, work },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<(typeof nodes)[number]>;

export default sketch;
