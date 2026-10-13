// render() on Bun, through its FFI.

import { describe, expect, test } from "bun:test";
import { mkdtempSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { NicevgError, type Bounds, type Sketch, Svg, render } from "nicevg";

const fixtures = join(import.meta.dir, "..", "..", "..", "..", "tests", "fixtures");

const twoBoxes: Sketch = {
  nodes: [
    { id: "order", x: 20, y: 20, width: 120, height: 56, label: "Order" },
    { id: "bill", x: 300, y: 160, width: 120, height: 56, label: "Bill" },
  ],
  connectors: [{ from: "order", to: "bill", label: "invoice" }],
};

/** A connector that bends where moving `mail` up would straighten it. */
const bent: Sketch = {
  nodes: [
    { id: "notify", x: 30, y: 100, width: 120, height: 56, label: "Notify" },
    { id: "mail", x: 280, y: 52, width: 120, height: 56, label: "Mail" },
  ],
  connectors: [
    {
      from: "notify",
      to: "mail",
      points: [
        { x: 150, y: 128 },
        { x: 220, y: 128 },
        { x: 220, y: 80 },
        { x: 280, y: 80 },
      ],
    },
  ],
};

const isOrthogonal = (points: { x: number; y: number }[]) =>
  points.length >= 2 && points.slice(1).every((p, i) => p.x === points[i]!.x || p.y === points[i]!.y);

const boundsOf = (svg: Svg, id: string): Bounds => svg.report.diagram.nodes.find((node) => node.id === id)!.bounds;

describe("render", () => {
  /**
   * Given a sketch of two boxes and a labelled connector with no route
   * When it is rendered
   * Then an Svg comes back, from which both nodes, the connector's orthogonal route (found by its ends) and its label's box read, with no issue left
   */
  test("turns a sketch into an Svg with nodes, routes and label boxes", () => {
    const svg = render(twoBoxes);

    expect(svg).toBeInstanceOf(Svg);
    expect(svg.report.diagram.nodes.map((node) => node.id).sort()).toEqual(["bill", "order"]);
    const flow = svg.report.diagram.connectors.find((c) => c.from === "order" && c.to === "bill")!;
    expect(isOrthogonal(flow.points)).toBe(true);
    const label = svg.report.diagram.labels.find((label) => label.connector === flow.id)!;
    expect(label.bounds.width).toBeGreaterThan(0);
    expect(svg.report.issues).toEqual([]);
  });

  /**
   * Given a sketch of two boxes overlapping each other
   * When it is rendered
   * Then they no longer overlap and no issue is left
   */
  test("moves overlapping nodes apart", () => {
    const svg = render({
      nodes: [
        { id: "a", x: 0, y: 0, width: 100, height: 50 },
        { id: "b", x: 50, y: 20, width: 100, height: 50 },
      ],
    });

    const [a, b] = [boundsOf(svg, "a"), boundsOf(svg, "b")];
    const apart = a.x + a.width <= b.x || b.x + b.width <= a.x || a.y + a.height <= b.y || b.y + b.height <= a.y;
    expect(apart).toBe(true);
    expect(svg.report.issues).toEqual([]);
  });

  /**
   * Given a connector that bends where moving a node would straighten it
   * When it is rendered
   * Then one of its nodes moves and the connector runs straight
   */
  test("moves a node to straighten a bent connector", () => {
    const svg = render(bent);

    const email = svg.report.diagram.connectors.find((c) => c.from === "notify" && c.to === "mail")!;
    expect(email.points.length).toBe(2);
    const given = Object.fromEntries(bent.nodes.map((node) => [node.id, { x: node.x, y: node.y }]));
    const moved = svg.report.diagram.nodes.filter((node) => node.bounds.x !== given[node.id]!.x || node.bounds.y !== given[node.id]!.y);
    expect(moved.length).toBeGreaterThan(0);
  });

  /**
   * Given the SVG text of an example diagram
   * When it is rendered
   * Then it is arranged like a sketch, with no issue left
   */
  test("takes SVG text too", () => {
    const text = readFileSync(join(fixtures, "order-checkout-dfd.svg"), "utf8");

    const svg = render({ svg: text });

    expect(svg.report.diagram.nodes.length).toBeGreaterThan(5);
    expect(svg.report.issues).toEqual([]);
  });

  /**
   * Given a node without its width, and a connector to a node the sketch lacks
   * When each is rendered
   * Then each throws a NicevgError saying what is wrong and where
   */
  test("throws an error saying what is wrong", () => {
    const missingWidth = { nodes: [{ id: "a", x: 0, y: 0, height: 50 }] } as unknown as Sketch;
    const dangling: Sketch = {
      nodes: [{ id: "a", x: 0, y: 0, width: 100, height: 50 }],
      connectors: [{ from: "a", to: "ghost" }],
    };

    expect(() => render(missingWidth)).toThrow(NicevgError);
    expect(() => render(missingWidth)).toThrow(/nodes\[0\].*width/);
    expect(() => render(dangling)).toThrow(/connectors\[0\]\.to.*"ghost"/);
  });
});

describe("the Svg rendered", () => {
  /**
   * Given a rendered sketch
   * When its Svg is turned into a string and written to a file
   * Then both hold the same SVG text
   */
  test("turns into text and writes to a file", async () => {
    const svg = render(twoBoxes);
    const path = join(mkdtempSync(join(tmpdir(), "nicevg-")), "rendered.svg");

    await svg.write(path);

    expect(svg.toString()).toStartWith("<svg");
    expect(`${svg}`).toBe(svg.toString());
    expect(readFileSync(path, "utf8")).toBe(svg.toString());
  });

  /**
   * Given a rendered Svg
   * When it is rendered again
   * Then it is read as the SVG it holds
   */
  test("can be rendered again", () => {
    const svg = render(bent);

    expect(render(svg).toString()).toBe(render({ svg: svg.toString() }).toString());
  });
});
