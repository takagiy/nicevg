/**
 * Boxes and circles drawn the way nicevg draws a sketch's rect and circle
 * nodes: white, outlined in black, the label centred in lines. A label
 * too big for its node grows it as fixing grows those, so a diagram
 * drawn with these styles arranges as the same diagram without them.
 *
 * ```ts
 * import { render } from "nicevg";
 * import { box, circle } from "nicevg/style/simple";
 *
 * render({ nodes: [{ id: "a", class: "box", x: 20, y: 20, width: 120, height: 56, label: "Order" }], style: { box, circle } });
 * ```
 */

import { type SvgFragment, WIDE, svg } from "../draw.ts";

/** A node a simple style draws: the box it takes, and its text. */
export interface SimpleNode {
  width: number;
  height: number;
  /** Its text, lines split at `\n`, centred. */
  label?: string;
  fontSize?: number;
}

const LINE_HEIGHT = 1.2;
/** How far a label keeps inside its node. */
const PADDING = 12;

/** nicevg's rounding: half up. */
const round = (value: number) => (value - Math.floor(value) >= 0.5 ? Math.floor(value) + 1 : Math.floor(value));

/** A line's width as nicevg estimates it: wide characters one font size, others 0.59, rounded. */
function lineWidth(line: string, size: number): number {
  let wide = 0;
  let narrow = 0;
  for (const character of line) {
    if (WIDE.test(character)) {
      wide += 1;
    } else {
      narrow += character.length;
    }
  }
  return round(narrow * size * 0.59 + wide * size);
}

interface Box {
  x: number;
  y: number;
  width: number;
  height: number;
}

/** A label centred on `cx`, `cy` as nicevg writes one, and the box nicevg measures around it. */
function label(content: string, cx: number, cy: number, size: number): { markup: SvgFragment; bounds: Box } {
  const lines = content.split("\n");
  const first = cy - ((lines.length - 1) * size * LINE_HEIGHT) / 2 + size * 0.35;
  const ys = lines.map((_, index) => first + index * size * LINE_HEIGHT);
  const boxes = lines.map((line, index) => {
    const width = lineWidth(line, size);
    return { x: cx - width / 2, y: ys[index]! - size, width, height: round(size * LINE_HEIGHT) };
  });
  const left = Math.min(...boxes.map((b) => b.x));
  const top = Math.min(...boxes.map((b) => b.y));
  const right = Math.max(...boxes.map((b) => b.x + b.width));
  const bottom = Math.max(...boxes.map((b) => b.y + b.height));
  return {
    markup: svg`<text text-anchor="middle" font-size="${size}">${lines.map((line, index) => svg`<tspan x="${cx}" y="${ys[index]!}">${line}</tspan>`)}</text>`,
    bounds: { x: left, y: top, width: right - left, height: bottom - top },
  };
}

/** A box, grown to keep its label 12px inside it. */
export function box({ width, height, label: content, fontSize = 14 }: SimpleNode): SvgFragment {
  if (content === undefined) {
    return svg`<rect data-bounds="true" x="0" y="0" width="${width}" height="${height}" fill="white" stroke="black"/>`;
  }
  const text = label(content, width / 2, height / 2, fontSize);
  const left = Math.min(0, text.bounds.x - PADDING);
  const top = Math.min(0, text.bounds.y - PADDING);
  const right = Math.max(width, text.bounds.x + text.bounds.width + PADDING);
  const bottom = Math.max(height, text.bounds.y + text.bounds.height + PADDING);
  return svg`<rect data-bounds="true" x="${left}" y="${top}" width="${right - left}" height="${bottom - top}" fill="white" stroke="black"/>${text.markup}`;
}

/** A circle in the middle of the box it is given, grown to keep its label 12px inside it. */
export function circle({ width, height, label: content, fontSize = 14 }: SimpleNode): SvgFragment {
  const [cx, cy] = [width / 2, height / 2];
  let radius = Math.min(width, height) / 2;
  if (content === undefined) {
    return svg`<circle data-bounds="true" cx="${cx}" cy="${cy}" r="${radius}" fill="white" stroke="black"/>`;
  }
  const text = label(content, cx, cy, fontSize);
  const dx = Math.max(Math.abs(text.bounds.x - cx), Math.abs(text.bounds.x + text.bounds.width - cx));
  const dy = Math.max(Math.abs(text.bounds.y - cy), Math.abs(text.bounds.y + text.bounds.height - cy));
  const needed = Math.hypot(dx, dy) + PADDING;
  if (needed > radius) {
    radius = Math.ceil(needed);
  }
  return svg`<circle data-bounds="true" cx="${cx}" cy="${cy}" r="${radius}" fill="white" stroke="black"/>${text.markup}`;
}
