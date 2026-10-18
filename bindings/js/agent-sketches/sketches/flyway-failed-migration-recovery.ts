// Flyway schema history and migration recovery, based on:
// https://documentation.red-gate.com/flyway/flyway-concepts/migrations/flyway-schema-history-table
// https://documentation.red-gate.com/flyway/reference/commands/validate
// https://documentation.red-gate.com/flyway/reference/commands/repair
// https://documentation.red-gate.com/flyway/reference/commands/migrate
// This sketch follows a versioned migration that fails, is diagnosed against its
// schema-history row, and is repaired before a later migrate records success.
import type { Sketch, SketchNode } from "nicevg";
import { svg, text } from "nicevg";

interface Card extends SketchNode {
  class: "file" | "step" | "state" | "record";
  label: string;
}

const card = ({ label, class: kind }: Card) => {
  const lines = text(label, 13);
  const w = Math.max(134, lines.width + 24);
  const h = lines.height + 20;
  if (kind === "state") {
    return svg`<ellipse data-bounds="true" cx="${w / 2}" cy="${h / 2}" rx="${w / 2}" ry="${h / 2}" fill="#fef3c7" stroke="#92400e"/>${lines.at({ cx: w / 2, cy: h / 2 })}`;
  }
  if (kind === "record") {
    return svg`<path data-bounds="true" d="M 0 0 H ${w - 12} L ${w} 12 V ${h} H 0 Z" fill="#ecfdf5" stroke="#047857"/><path d="M ${w - 12} 0 V 12 H ${w}" fill="none" stroke="#047857"/>${lines.at({ cx: w / 2, cy: h / 2 })}`;
  }
  if (kind === "file") {
    return svg`<path data-bounds="true" d="M 0 0 H ${w - 18} L ${w} 18 V ${h} H 0 Z M ${w - 18} 0 V 18 H ${w}" fill="#eff6ff" stroke="#1d4ed8"/>${lines.at({ cx: w / 2, cy: h / 2 })}`;
  }
  return svg`<rect data-bounds="true" width="${w}" height="${h}" rx="5" fill="#f1f5f9" stroke="#475569"/>${lines.at({ cx: w / 2, cy: h / 2 })}`;
};

const sketch = {
  nodes: [
    { id: "migration", class: "file", x: 30, y: 110, label: "V042__add_index.sql" },
    { id: "migrate-fail", class: "step", x: 230, y: 110, label: "migrate\nexecute SQL" },
    { id: "failed", class: "state", x: 430, y: 110, label: "Failed" },
    { id: "history", class: "record", x: 635, y: 95, label: "flyway_schema_history\ninstalled_rank · version\ndescription · type · script\nchecksum · installed_by\ninstalled_on · execution_time\nsuccess" },
    { id: "validate", class: "step", x: 865, y: 110, label: "validate\ncompare resolved files" },
    { id: "repair", class: "step", x: 1080, y: 110, label: "repair" },
    { id: "cleanup", class: "step", x: 1080, y: 285, label: "remove incomplete\nDB changes manually" },
    { id: "edited", class: "file", x: 865, y: 285, label: "corrected migration\nchecksum" },
    { id: "pending", class: "state", x: 635, y: 310, label: "Pending" },
    { id: "success", class: "state", x: 230, y: 310, label: "Success" },
    { id: "retry", class: "step", x: 30, y: 310, label: "migrate\nretry" },
  ] satisfies Card[],
  edges: [
    { from: "migration", to: "migrate-fail", label: "SQL script" },
    { from: "migrate-fail", to: "failed", label: "error" },
    { from: "failed", to: "history", label: "insert failed row" },
    { from: "validate", to: "history", label: "read records" },
    { from: "validate", to: "migration", label: "resolve migration files" },
    { from: "validate", to: "repair", label: "validation error" },
    { from: "repair", to: "cleanup", label: "after incomplete changes removed" },
    { from: "repair", to: "history", label: "remove failed row" },
    { from: "repair", to: "edited", label: "realign checksum" },
    { from: "edited", to: "pending", label: "resolved version" },
    { from: "pending", to: "retry", label: "eligible to apply" },
    { from: "retry", to: "edited", label: "execute corrected SQL" },
    { from: "retry", to: "success", label: "successful execution" },
    { from: "success", to: "history", label: "insert success row + checksum" },
  ],
  style: { file: card, step: card, state: card, record: card },
  rootAttrs: { "font-family": "sans-serif" },
} satisfies Sketch<Card>;

export default sketch;
