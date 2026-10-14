// render() on Node, through koffi, from the compiled package in dist/.

import assert from "node:assert/strict";
import { mkdtempSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, test } from "node:test";
import { fileURLToPath } from "node:url";

import { NicevgError, type Sketch, Svg, render, svg, text } from "nicevg";

const fixtures = fileURLToPath(new URL("../../../../tests/fixtures/", import.meta.url));

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

describe("render on Node", () => {
  /**
   * Given a sketch of two boxes and a labelled connector with no route
   * When it is rendered
   * Then an Svg comes back, from which both nodes, the connector's orthogonal route (found by its ends) and its label's box read, with no issue left
   */
  test("turns a sketch into an Svg with nodes, routes and label boxes", () => {
    const svg = render(twoBoxes);

    assert.ok(svg instanceof Svg);
    assert.deepEqual(svg.report.diagram.nodes.map((node) => node.id).sort(), ["bill", "order"]);
    const flow = svg.report.diagram.connectors.find((c) => c.from === "order" && c.to === "bill")!;
    assert.ok(isOrthogonal(flow.points));
    assert.ok(svg.report.diagram.labels.some((label) => label.connector === flow.id));
    assert.deepEqual(svg.report.issues, []);
  });

  /**
   * Given a connector that bends where moving a node would straighten it
   * When it is rendered
   * Then one of its nodes moves and the connector runs straight
   */
  test("moves a node to straighten a bent connector", () => {
    const svg = render(bent);

    const email = svg.report.diagram.connectors.find((c) => c.from === "notify" && c.to === "mail")!;
    assert.equal(email.points.length, 2);
    const given = Object.fromEntries(bent.nodes.map((node) => [node.id, { x: node.x, y: node.y }]));
    const moved = svg.report.diagram.nodes.filter((node) => node.bounds.x !== given[node.id]!.x || node.bounds.y !== given[node.id]!.y);
    assert.ok(moved.length > 0);
  });

  /**
   * Given a rendered sketch
   * When its Svg is written to a file, and rendered again
   * Then the file holds its text, and it is read as the SVG it holds
   */
  test("writes the Svg to a file and renders it again", async () => {
    const svg = render(bent);
    const path = join(mkdtempSync(join(tmpdir(), "nicevg-")), "rendered.svg");

    await svg.write(path);

    assert.equal(readFileSync(path, "utf8"), svg.toString());
    assert.equal(render(svg).toString(), render({ svg: `${svg}` }).toString());
  });

  /**
   * Given the SVG text of an example diagram
   * When it is rendered on Node
   * Then the result is the same SVG as on any other runtime: arranged, with no issue left
   */
  test("takes SVG text too", () => {
    const svg = render({ svg: readFileSync(join(fixtures, "order-checkout-dfd.svg"), "utf8") });

    assert.ok(svg.report.diagram.nodes.length > 5);
    assert.deepEqual(svg.report.issues, []);
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

    assert.throws(() => render(missingWidth), NicevgError);
    assert.throws(() => render(missingWidth), /nodes\[0\].*width/);
    assert.throws(() => render(dangling), /connectors\[0\]\.to.*"ghost"/);
  });

  /**
   * Given a node of a class whose style draws a box with svg and text, joined by an edge
   * When it is rendered from the built package
   * Then the node lies around what the style drew, with no issue left
   */
  test("draws a styled node with its style", () => {
    const image = render({
      nodes: [
        { id: "order", x: 20, y: 20, width: 120, height: 56, label: "Order" },
        { id: "bill", class: "box", x: 300, y: 160, label: "Bill" },
      ],
      edges: [{ from: "order", to: "bill" }],
      style: {
        box: ({ label }) => {
          const name = text(String(label));
          return svg`<rect width="${name.width + 32}" height="40" fill="none" stroke="black"/>${name.at({ left: 16, cy: 20 })}`;
        },
      },
    });

    const bill = image.report.diagram.nodes.find((node) => node.id === "bill")!;
    assert.deepEqual([bill.bounds.width, bill.bounds.height], [text("Bill").width + 32, 40]);
    assert.deepEqual(image.report.issues, []);
  });
});
