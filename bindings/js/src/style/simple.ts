/**
 * Boxes and circles drawn the way nicevg draws a sketch's rect and circle
 * nodes: white, outlined in black unless painted, the label centred in lines. A label
 * too big for its node grows it as fixing grows those, so a diagram
 * drawn with these styles arranges as the same diagram without them.
 *
 * ```ts
 * import { render } from "nicevg";
 * import { box, circle } from "nicevg/style/simple";
 *
 * render({ nodes: [{ id: "a", class: "box", x: 20, y: 20, label: "Order" }], style: { box, circle } });
 * ```
 */

import { type SvgFragment, WIDE, svg, text } from "../draw.ts";

/** A node a simple style draws: the box it takes, just big enough for its label when not given, and its text. */
export interface SimpleNode {
  width?: number;
  height?: number;
  /** Its text, lines split at `\n`, centred unless `box` is told otherwise. */
  label?: string;
  fontSize?: number;
  /** Where `box` writes its label: centred, or `padding` in from its top left, as a parent's title. */
  labelPosition?: "center" | "top-left";
  /** How far `box` keeps its label, and a parent its children, inside its border: 12px unless given. */
  padding?: number;
  /** The shape's paint, as the SVG attributes of the same names: white, outlined 1px in black, unless given. */
  fill?: string;
  stroke?: string;
  strokeWidth?: number;
  strokeDasharray?: string;
}

/** The shape's paint attributes. */
function paint({ fill = "white", stroke = "black", strokeWidth, strokeDasharray }: SimpleNode): SvgFragment {
  return svg`fill="${fill}" stroke="${stroke}"${strokeWidth === undefined ? "" : svg` stroke-width="${strokeWidth}"`}${strokeDasharray === undefined ? "" : svg` stroke-dasharray="${strokeDasharray}"`}`;
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

/** How far a label centred on a point reaches from it across and down. */
function reach(content: string | undefined, size: number): [number, number] {
  if (content === undefined) {
    return [0, 0];
  }
  const { x, y, width, height } = label(content, 0, 0, size).bounds;
  return [Math.max(-x, x + width), Math.max(-y, y + height)];
}

/** A box, grown to keep its label `padding` (12px) inside it, marking that padding for a parent's children. */
export function box({
  label: content,
  fontSize = 14,
  labelPosition = "center",
  padding,
  ...node
}: SimpleNode): SvgFragment {
  const inset = padding ?? PADDING;
  const frame = (x: number, y: number, width: number, height: number) =>
    svg`<rect data-bounds="true"${padding === undefined ? "" : svg` data-padding="${padding}"`} x="${x}" y="${y}" width="${width}" height="${height}" ${paint(node)}/>`;
  if (labelPosition === "top-left") {
    const title = content === undefined ? undefined : text(content, fontSize);
    // Grown to its title, as a centred label grows its box.
    const width = Math.max(node.width ?? 0, (title?.width ?? 0) + 2 * inset);
    const height = Math.max(node.height ?? 0, (title?.height ?? 0) + 2 * inset);
    return svg`${frame(0, 0, width, height)}${title?.at({ left: inset, top: inset }) ?? ""}`;
  }
  const [dx, dy] = reach(content, fontSize);
  const { width = 2 * (dx + inset), height = 2 * (dy + inset) } = node;
  if (content === undefined) {
    return frame(0, 0, width, height);
  }
  const centred = label(content, width / 2, height / 2, fontSize);
  const left = Math.min(0, centred.bounds.x - inset);
  const top = Math.min(0, centred.bounds.y - inset);
  const right = Math.max(width, centred.bounds.x + centred.bounds.width + inset);
  const bottom = Math.max(height, centred.bounds.y + centred.bounds.height + inset);
  return svg`${frame(left, top, right - left, bottom - top)}${centred.markup}`;
}

/** A circle in the middle of the box it is given, grown to keep its label 12px inside it. */
export function circle({ label: content, fontSize = 14, ...node }: SimpleNode): SvgFragment {
  const diameter = 2 * Math.ceil(Math.hypot(...reach(content, fontSize)) + PADDING);
  const { width = diameter, height = diameter } = node;
  const [cx, cy] = [width / 2, height / 2];
  let radius = Math.min(width, height) / 2;
  if (content === undefined) {
    return svg`<circle data-bounds="true" cx="${cx}" cy="${cy}" r="${radius}" ${paint(node)}/>`;
  }
  const text = label(content, cx, cy, fontSize);
  const dx = Math.max(Math.abs(text.bounds.x - cx), Math.abs(text.bounds.x + text.bounds.width - cx));
  const dy = Math.max(Math.abs(text.bounds.y - cy), Math.abs(text.bounds.y + text.bounds.height - cy));
  const needed = Math.hypot(dx, dy) + PADDING;
  if (needed > radius) {
    radius = Math.ceil(needed);
  }
  return svg`<circle data-bounds="true" cx="${cx}" cy="${cy}" r="${radius}" ${paint(node)}/>${text.markup}`;
}
