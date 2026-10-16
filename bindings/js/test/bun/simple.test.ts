// The simple box and circle styles arrange like the native library's own rect and circle nodes.

import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

import { type AnalysisReport, type Bounds, type SketchConnector, render } from "nicevg";
import { box, circle } from "nicevg/style/simple";

import { call } from "../../src/native.ts";

/** A node the native library draws itself, as it takes from Rust: a rect, or a circle in its box. */
interface BuiltinNode {
  id: string;
  x: number;
  y: number;
  width: number;
  height: number;
  shape?: "circle";
  label: string;
  fontSize: number;
}

interface BuiltinSketch {
  nodes: BuiltinNode[];
  connectors: SketchConnector[];
}

/** What the native library makes of a sketch of its own nodes, which render() no longer takes. */
function builtin(sketch: BuiltinSketch): AnalysisReport {
  const reply = JSON.parse(call(new TextEncoder().encode(JSON.stringify(sketch))));
  return reply.ok.report;
}

const root = join(import.meta.dir, "..", "..", "..", "..");

const attr = (tag: string, name: string) => new RegExp(`\\s${name}="([^"]*)"`).exec(tag)?.[1];
const number = (tag: string, name: string) => Number(attr(tag, name) ?? 0);
const content = (text: string) => text.replace(/<[^>]+>/g, "").replace(/&amp;/g, "&").replace(/&lt;/g, "<").trim();

/** An example diagram's nodes, connectors and labels as a sketch: none of the examples nests nodes. */
function sketchOf(file: string): BuiltinSketch {
  const svg = readFileSync(join(root, file), "utf8");
  const nodes: BuiltinNode[] = [...svg.matchAll(/<g data-node="([^"]+)"[^>]*>([\s\S]*?)<\/g>/g)].map(([, id, body]) => {
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
function styled({ nodes, connectors }: BuiltinSketch) {
  return {
    nodes: nodes.map(({ shape, ...node }) => ({ ...node, class: shape === "circle" ? "circle" : "box" })),
    connectors,
    style: { box, circle },
  };
}

/** What arranging decided, with numbers to a millionth: drawn the two ways, they may differ in the last bits. */
function layout(report: AnalysisReport) {
  const round = (value: number) => Math.round(value * 1e6) / 1e6;
  const box = ({ x, y, width, height }: { x: number; y: number; width: number; height: number }) => [x, y, width, height].map(round);
  const { diagram, issues } = report;
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

      expect(layout(render(styled(sketch)).report)).toEqual(layout(builtin(sketch)));
    }, 120_000); // arranges the diagram twice: 40s on the macOS Intel runner
  }

  /**
   * Given a box and a circle with a label and no size
   * When they are rendered
   * Then each lies from its x and y, just big enough to keep its label 12px inside
   */
  test("fits a node given no size to its label", () => {
    const image = render({
      nodes: [
        { id: "order", class: "box", x: 20, y: 20, label: "Order\nservice" },
        { id: "user", class: "circle", x: 200, y: 20, label: "User" },
      ],
      style: { box, circle },
    });

    const node = (id: string) => image.report.diagram.nodes.find((node) => node.id === id)!;
    const insets = ({ bounds, labelBounds: [label] }: { bounds: Bounds; labelBounds: Bounds[] }) => ({
      across: Math.min(label!.x - bounds.x, bounds.x + bounds.width - label!.x - label!.width),
      down: Math.min(label!.y - bounds.y, bounds.y + bounds.height - label!.y - label!.height),
    });
    const order = node("order"), user = node("user");
    expect([order.bounds.x, order.bounds.y, user.bounds.x, user.bounds.y]).toEqual([20, 20, 200, 20]);
    expect(insets(order).across).toBeCloseTo(12);
    expect(insets(order).down).toBeCloseTo(12);
    const [label, r] = [user.labelBounds[0]!, user.bounds.width / 2];
    const corner = Math.hypot(label.width / 2, Math.max(user.bounds.y + r - label.y, label.y + label.height - user.bounds.y - r));
    expect(r - corner).toBeGreaterThanOrEqual(12);
    expect(r - corner).toBeLessThan(13);
  });

  /**
   * Given a box and a circle painted with a fill, a stroke, its width and dashes, and the same two unpainted
   * When they are drawn and rendered
   * Then the painted shapes carry the paint, the unpainted are white outlined in black, and both lie alike
   */
  test("paints a node with the fill and stroke given", () => {
    const paint = { fill: "#eef", stroke: "navy", strokeWidth: 2, strokeDasharray: "4 2" };
    const nodes = [
      { id: "order", class: "box" as const, x: 20, y: 20, label: "Order" },
      { id: "user", class: "circle" as const, x: 200, y: 20, label: "User" },
    ];
    const bounds = (image: ReturnType<typeof render>) => image.report.diagram.nodes.map((node) => node.bounds);

    for (const shape of [box({ label: "Order", ...paint }), circle({ label: "User", ...paint })]) {
      expect(String(shape)).toContain('fill="#eef" stroke="navy" stroke-width="2" stroke-dasharray="4 2"');
    }
    expect(String(box({ label: "Order" }))).toContain('fill="white" stroke="black"/>');
    expect(bounds(render({ nodes: nodes.map((node) => ({ ...node, ...paint })), style: { box, circle } }))).toEqual(
      bounds(render({ nodes, style: { box, circle } })),
    );
  });
});
