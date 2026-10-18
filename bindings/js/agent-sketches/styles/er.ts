/**
 * Styles for entity-relationship diagrams: a table with a port on both
 * sides of each column's row, so a foreign key's edge can run from the
 * column that holds it to the key it refers to, and an area to group
 * tables in.
 *
 * ```ts
 * import type { Sketch } from "nicevg";
 * import { area, table, type ErNode } from "../styles/er.ts";
 *
 * const sketch = {
 *   nodes: [
 *     { id: "users", class: "table", x: 40, y: 40, columns: [["id", "uuid", "PK"], ["email", "text"]] },
 *     { id: "posts", class: "table", x: 320, y: 40, columns: [["id", "uuid", "PK"], ["author_id", "uuid", "FK"]] },
 *   ],
 *   // An edge end is "table.column": the port of that column's row.
 *   edges: [{ from: "posts.author_id", to: "users.id" }],
 *   style: { table, area },
 * } satisfies Sketch<ErNode>;
 * ```
 */

import type { SketchNode } from "nicevg";
import { svg, text } from "nicevg";

/** A column: its name, its type, and its key, if any (`PK`, `FK`, `PK FK`, `UQ` ...). */
export type Column = [name: string, type?: string, key?: string];

export interface ErTable extends SketchNode {
  class: "table";
  columns: Column[];
  /** The name in the head; the node's id when not given. */
  title?: string;
  /** The head's tint. */
  head?: string;
  fontSize?: number;
}

export interface ErArea extends SketchNode {
  class: "area";
  title: string;
  width: number;
  height: number;
  fill?: string;
}

export type ErNode = ErTable | ErArea;

const PAD = 8;

/**
 * A table: its name in a tinted head, then a row per column with its key,
 * name and type, each row a port named by the column on both sides.
 */
export function table({ id, title, columns, head = "#e2e8f0", fontSize = 12 }: ErTable) {
  const row = Math.round(fontSize * 1.7);
  const top = Math.round(fontSize * 2.2);
  const name = text(title ?? id, fontSize + 1);
  const keys = columns.map(([, , key]) => (key ? text(key, fontSize - 2) : undefined));
  const names = columns.map(([column]) => text(column, fontSize));
  const types = columns.map(([, type]) => (type ? text(type, fontSize) : undefined));
  const keyWidth = Math.max(0, ...keys.map((key) => (key ? key.width + PAD : 0)));
  const nameWidth = Math.max(0, ...names.map((line) => line.width));
  const typeWidth = Math.max(0, ...types.map((line) => (line ? line.width + 2 * PAD : 0)));
  const width = Math.max(name.width + 2 * PAD, PAD + keyWidth + nameWidth + typeWidth + PAD);
  const height = top + columns.length * row;
  const rowY = (index: number) => top + index * row + row / 2;
  return svg`
    <rect data-bounds="true" width="${width}" height="${height}" fill="white" stroke="#334155"/>
    <rect width="${width}" height="${top}" fill="${head}" stroke="#334155"/>
    ${name.at({ cx: width / 2, cy: top / 2 })}
    ${columns.map(([column], index) => [
      keys[index]?.at({ left: PAD, cy: rowY(index) }) ?? "",
      names[index]!.at({ left: PAD + keyWidth, cy: rowY(index) }),
      types[index]?.at({ right: width - PAD, cy: rowY(index) }) ?? "",
      [0, width].map((x) => svg`<circle data-port="${column}" cx="${x}" cy="${rowY(index)}" r="0"/>`),
    ])}`;
}

/** An area: a tinted frame with its title in the top left, for tables to sit in by `parent`. */
export function area({ title, width, height, fill = "#f8fafc" }: ErArea) {
  return svg`
    <rect data-bounds="true" width="${width}" height="${height}" fill="${fill}" stroke="#94a3b8"/>
    ${text(title, 13).at({ left: 12, top: 8 })}`;
}
