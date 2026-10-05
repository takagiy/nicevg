import { describe, expect, test } from "bun:test";
import { DOMParser } from "@xmldom/xmldom";
import { analyze, fix } from "../src/index";

describe("fix", () => {
  test("expands a clipping viewBox without shrinking its existing extent", () => {
    /**
     * Given a diagram clipped on the right side
     * When the diagram is fixed
     * Then the viewBox expands to include safe padding and no clipping remains
     */
    const svg = `
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 120">
        <g data-node="checkout">
          <rect x="20" y="20" width="100" height="56" />
          <text x="70" y="53" text-anchor="middle" font-size="14">Checkout</text>
        </g>
      </svg>
    `;

    const result = fix(svg);
    const document = new DOMParser().parseFromString(
      result.svg,
      "image/svg+xml",
    );

    expect(document.documentElement.getAttribute("viewBox")).toBe(
      "0 0 140 120",
    );
    expect(
      analyze(result.svg).issues.some(
        (issue) => issue.code === "viewport-clipping",
      ),
    ).toBe(false);
    expect(result.changes).toEqual([
      {
        code: "expand-viewbox",
        message: "Expanded viewBox from 0 0 100 120 to 0 0 140 120.",
        elements: ["svg"],
      },
    ]);
  });

  test("expands a node box around its label without moving the label", () => {
    /**
     * Given a label that violates its node's 12px inner padding
     * When the diagram is fixed
     * Then only the box expands and the text coordinates remain unchanged
     */
    const svg = `
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="-100 -100 400 300">
        <g data-node="confirm">
          <rect x="20" y="20" width="70" height="40" />
          <text x="55" y="52" text-anchor="middle" font-size="14">Confirm payment</text>
        </g>
      </svg>
    `;

    const result = fix(svg);
    const document = new DOMParser().parseFromString(
      result.svg,
      "image/svg+xml",
    );
    const rect = document.getElementsByTagName("rect").item(0);
    const text = document.getElementsByTagName("text").item(0);

    expect({
      x: rect?.getAttribute("x"),
      y: rect?.getAttribute("y"),
      width: rect?.getAttribute("width"),
      height: rect?.getAttribute("height"),
    }).toEqual({ x: "-19", y: "20", width: "148", height: "47" });
    expect({ x: text?.getAttribute("x"), y: text?.getAttribute("y") }).toEqual({
      x: "55",
      y: "52",
    });
    expect(
      result.report.issues.some((issue) => issue.code === "text-overflow"),
    ).toBe(false);
  });

  test("pushes overlapping nodes apart while preserving their order", () => {
    /**
     * Given two same-row nodes that overlap by 30px
     * When the diagram is fixed
     * Then the right node is shifted to leave a 20px gap
     */
    const svg = `
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="-100 -100 600 300">
        <g data-node="first">
          <rect x="20" y="20" width="100" height="56" />
          <text x="70" y="53" text-anchor="middle" font-size="14">First</text>
        </g>
        <g data-node="second">
          <rect x="90" y="20" width="100" height="56" />
          <text x="140" y="53" text-anchor="middle" font-size="14">Second</text>
        </g>
      </svg>
    `;

    const result = fix(svg);
    const [first, second] = result.report.diagram.nodes;

    expect(first?.bounds.x).toBe(20);
    expect(second?.bounds.x).toBe(140);
    expect(
      result.report.issues.some(
        (issue) => issue.code === "node-overlap" || issue.code === "node-gap",
      ),
    ).toBe(false);
  });

  test("reroutes a connector around an unrelated node", () => {
    /**
     * Given a horizontal connector that crosses an unrelated node
     * When the diagram is fixed
     * Then it follows an orthogonal path with 8px obstacle clearance
     */
    const svg = `
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="-100 -100 600 300">
        <g data-node="source">
          <rect x="20" y="20" width="100" height="56" />
          <text x="70" y="53" text-anchor="middle" font-size="14">Source</text>
        </g>
        <g data-node="obstacle">
          <rect x="150" y="20" width="80" height="56" />
          <text x="190" y="53" text-anchor="middle" font-size="14">Block</text>
        </g>
        <g data-node="target">
          <rect x="260" y="20" width="100" height="56" />
          <text x="310" y="53" text-anchor="middle" font-size="14">Target</text>
        </g>
        <line id="flow" data-from="source" data-to="target"
          marker-end="url(#arrow)" x1="120" y1="48" x2="260" y2="48" />
      </svg>
    `;

    const result = fix(svg);
    const connector = result.report.diagram.connectors.find(
      (item) => item.id === "flow",
    );

    expect(connector?.points).toEqual([
      { x: 120, y: 48 },
      { x: 120, y: 12 },
      { x: 260, y: 12 },
      { x: 260, y: 48 },
    ]);
    expect(
      result.report.issues.some(
        (issue) => issue.code === "connector-node-crossing",
      ),
    ).toBe(false);
  });
});
