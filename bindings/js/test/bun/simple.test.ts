// The simple box and circle styles arrange like a sketch's own rect and circle nodes.

import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

import { type Sketch, type SketchConnector, type SketchNode, type Svg, render } from "nicevg";
import { box, circle } from "nicevg/style/simple";

const root = join(import.meta.dir, "..", "..", "..", "..");

const attr = (tag: string, name: string) => new RegExp(`\\s${name}="([^"]*)"`).exec(tag)?.[1];
const number = (tag: string, name: string) => Number(attr(tag, name) ?? 0);
const content = (text: string) => text.replace(/<[^>]+>/g, "").replace(/&amp;/g, "&").replace(/&lt;/g, "<").trim();

/** An example diagram's nodes, connectors and labels as a sketch: none of the examples nests nodes. */
function sketchOf(file: string): Sketch<SketchNode> {
  const svg = readFileSync(join(root, file), "utf8");
  const nodes: SketchNode[] = [...svg.matchAll(/<g data-node="([^"]+)"[^>]*>([\s\S]*?)<\/g>/g)].map(([, id, body]) => {
    const text = /<text[^>]*>[\s\S]*?<\/text>/.exec(body!)![0];
    const shape = /<(rect|circle)\b[^>]*>/.exec(body!)!;
    const label = { label: content(text), fontSize: number(text, "font-size") };
    if (shape[1] === "circle") {
      const r = number(shape[0], "r");
      return { id: id!, x: number(shape[0], "cx") - r, y: number(shape[0], "cy") - r, width: 2 * r, height: 2 * r, shape: "circle", ...label };
    }
    return { id: id!, x: number(shape[0], "x"), y: number(shape[0], "y"), width: number(shape[0], "width"), height: number(shape[0], "height"), ...label };
  });
  const labels = new Map(
    [...svg.matchAll(/<text[^>]*data-label-for="([^"]+)"[^>]*>[\s\S]*?<\/text>/g)].map(([text, id]) => [id!, text]),
  );
  const connectors: SketchConnector[] = [...svg.matchAll(/<(line|path)\b[^>]*data-from="[^"]*"[^>]*>/g)].map(([tag, kind]) => {
    const points =
      kind === "line"
        ? [
            { x: number(tag, "x1"), y: number(tag, "y1") },
            { x: number(tag, "x2"), y: number(tag, "y2") },
          ]
        : [...attr(tag, "d")!.matchAll(/[ML]\s*(-?[\d.]+)[\s,]+(-?[\d.]+)/g)].map(([, x, y]) => ({ x: Number(x), y: Number(y) }));
    const text = labels.get(attr(tag, "id")!);
    return {
      from: attr(tag, "data-from")!,
      to: attr(tag, "data-to")!,
      points,
      ...(text && { label: content(text), fontSize: number(text, "font-size") }),
    };
  });
  return { nodes, connectors };
}

/** The same sketch with its nodes drawn by the simple styles. */
function styled({ nodes, connectors }: Sketch<SketchNode>) {
  return {
    nodes: nodes.map(({ shape, ...node }) => ({ ...node, class: shape === "circle" ? "circle" : "box" })),
    connectors,
    style: { box, circle },
  };
}

/** What arranging decided, with numbers to a millionth: drawn the two ways, they may differ in the last bits. */
function layout(image: Svg) {
  const round = (value: number) => Math.round(value * 1e6) / 1e6;
  const box = ({ x, y, width, height }: { x: number; y: number; width: number; height: number }) => [x, y, width, height].map(round);
  const { diagram, issues } = image.report;
  return {
    nodes: diagram.nodes.map((node) => ({ id: node.id, shape: node.shape, bounds: box(node.bounds), labels: node.labelBounds.map(box) })),
    connectors: diagram.connectors.map((c) => ({ from: c.from, to: c.to, points: c.points.map((p) => [round(p.x), round(p.y)]) })),
    labels: diagram.labels.map((label) => ({ connector: label.connector, bounds: box(label.bounds) })),
    issues: issues.map((issue) => issue.code),
  };
}

describe("simple styles", () => {
  /**
   * Given an example diagram without nested nodes, as a sketch of rects and circles
   * When it is rendered with nicevg's own nodes, and again with its nodes drawn by box and circle
   * Then the two arrange alike: the same node boxes, shapes and label boxes, routes, connector labels and issues left
   */
  for (const file of ["tests/fixtures/loan-origination-dfd.svg", "tests/fixtures/order-checkout-dfd.svg", "docs/images/before.svg"]) {
    test(`arranges ${file} as nicevg's own nodes do`, () => {
      const sketch = sketchOf(file);

      expect(layout(render(styled(sketch)))).toEqual(layout(render(sketch)));
    });
  }
});
