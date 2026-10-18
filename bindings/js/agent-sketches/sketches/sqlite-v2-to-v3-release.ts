import type { Sketch, SketchNode } from "nicevg";
import { svg, text } from "nicevg";

// SQLite 2.8 → 3.0 file format and release path.
// Sources: https://www.sqlite.org/version3.html, https://www.sqlite.org/copyright.html,
// https://www.sqlite.org/src, https://www.sqlite.org/fileformat.html
// The sketch follows a proposed change through SQLite's contribution constraints and Fossil
// check-ins to release, then compares the old and new on-disk structures and their dump/load path.

type Node = SketchNode & { class: "frame" | "record" | "page" | "step"; label: string; w?: number; h?: number };

function shape(node: Node, form: "rect" | "round" | "diamond" | "frame") {
  const lines = node.label.split("\n").map((line) => text(line, 12));
  const width = Math.max(node.w ?? 0, ...lines.map((line) => line.width + 22));
  const height = Math.max(node.h ?? 0, lines.reduce((sum, line) => sum + line.height, 0) + 18);
  const fill = node.class === "frame" ? "#f1f5f9" : node.class === "step" ? "#fef3c7" : "white";
  const outline = form === "frame" ? "#64748b" : "#334155";
  const body = form === "diamond"
    ? svg`<path data-bounds="true" d="M ${width / 2} 0 L ${width} ${height / 2} L ${width / 2} ${height} L 0 ${height / 2} Z" fill="${fill}" stroke="${outline}"/>`
    : form === "round"
      ? svg`<rect data-bounds="true" width="${width}" height="${height}" rx="14" fill="${fill}" stroke="${outline}"/>`
      : svg`<rect data-bounds="true" width="${width}" height="${height}" rx="${form === "frame" ? 4 : 0}" fill="${fill}" stroke="${outline}" stroke-dasharray="${form === "frame" ? "5 3" : ""}"/>`;
  const label = lines.map((line, i) => line.at({ cx: width / 2, top: (height - lines.reduce((sum, item) => sum + item.height, 0)) / 2 + lines.slice(0, i).reduce((sum, item) => sum + item.height, 0) }));
  return svg`${body}${label}`;
}

const styles = {
  frame: (node: Node) => shape(node, "frame"),
  record: (node: Node) => shape(node, "rect"),
  page: (node: Node) => shape(node, "round"),
  step: (node: Node) => shape(node, "diamond"),
};

const nodes: Node[] = [
  { id: "v2", class: "frame", x: 40, y: 420, w: 350, h: 300, label: "SQLite 2.8 database file" },
  { id: "v2-tree", class: "page", parent: "v2", x: 70, y: 490, label: "Table B-tree\n32-bit rowid" },
  { id: "v2-row", class: "record", parent: "v2", x: 240, y: 490, label: "Row data\nvalues serialized as ASCII" },
  { id: "v2-index", class: "page", parent: "v2", x: 70, y: 590, label: "Index B-tree" },
  { id: "v2-key", class: "record", parent: "v2", x: 240, y: 590, label: "Index key\nASCII representation" },
  { id: "v2-text", class: "record", parent: "v2", x: 70, y: 665, label: "API and disk data\nconverted to ASCII text" },
  { id: "v2-overflow", class: "record", parent: "v2", x: 240, y: 665, label: "Large fields\nmay use overflow pages" },
  { id: "v3", class: "frame", x: 520, y: 420, w: 440, h: 300, label: "SQLite 3.0 database file — incompatible format" },
  { id: "v3-header", class: "record", parent: "v3", x: 550, y: 490, label: "100-byte header\npage size · encoding · schema format" },
  { id: "v3-table", class: "page", parent: "v3", x: 760, y: 490, label: "Table B+tree\nrow data only in leaves" },
  { id: "v3-index", class: "page", parent: "v3", x: 550, y: 585, label: "Index B-tree\nkey records" },
  { id: "v3-record", class: "record", parent: "v3", x: 760, y: 585, label: "Record payload\nheader size + serial types + values" },
  { id: "v3-varint", class: "record", parent: "v3", x: 550, y: 665, label: "Variable-length integers\nlengths and 64-bit rowid" },
  { id: "v3-values", class: "record", parent: "v3", x: 760, y: 665, label: "Manifest values\nintegers / floats binary; BLOB; UTF-8/16" },
  { id: "contributor", class: "step", x: 30, y: 70, label: "Contributor\nproposes change" },
  { id: "affidavit", class: "step", x: 180, y: 50, label: "Public-domain\naffidavit on file" },
  { id: "proof", class: "step", x: 330, y: 70, label: "Proof-of-concept\npatch / suggestion" },
  { id: "maintainer", class: "step", x: 500, y: 50, label: "Maintainer may\nrewrite patch" },
  { id: "fossil", class: "step", x: 670, y: 70, label: "Canonical Fossil\nsource check-in" },
  { id: "tests", class: "step", x: 820, y: 50, label: "Test scripts and\nsource validation" },
  { id: "release", class: "step", x: 970, y: 70, label: "Version-3.0\nrelease check-in" },
  { id: "archive", class: "step", x: 1120, y: 50, label: "Release source\ntarball / ZIP" },
  { id: "migrate", class: "step", x: 1120, y: 420, label: "sqlite OLD.DB .dump\n| sqlite3 NEW.DB" },
];

const edges = [
  { from: "contributor", to: "proof", label: "suggestion; no PR" },
  { from: "affidavit", to: "maintainer", label: "required before accepting code" },
  { from: "proof", to: "maintainer" },
  { from: "maintainer", to: "fossil", label: "accepted change" },
  { from: "fossil", to: "tests" },
  { from: "tests", to: "release", label: "release check-in" },
  { from: "release", to: "archive" },
  { from: "v2", to: "migrate", label: "old shell emits SQL" },
  { from: "migrate", to: "v3", label: "new shell rebuilds file" },
  { from: "v2-tree", to: "v2-row" },
  { from: "v2-index", to: "v2-key" },
  { from: "v3-header", to: "v3-table", label: "root page map" },
  { from: "v3-table", to: "v3-record", label: "leaf payload" },
  { from: "v3-index", to: "v3-record", label: "indexed columns + row key" },
  { from: "v3-record", to: "v3-varint" },
  { from: "v3-record", to: "v3-values" },
];

export default { nodes, edges, style: styles, rootAttrs: { "font-family": "sans-serif" } } satisfies Sketch<Node>;
