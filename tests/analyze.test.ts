import { describe, expect, test } from "bun:test";
import { analyze, SvgInputError } from "../src/index";

describe("analyze", () => {
  test("recognizes an annotated node with its box and label", () => {
    /**
     * Given an SVG group explicitly marked as a diagram node
     * When the SVG is analyzed
     * Then the public report exposes the node, its box, and its label bounds
     */
    const svg = `
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 240 120">
        <g data-node="checkout">
          <rect x="20" y="20" width="120" height="56" />
          <text x="80" y="53" text-anchor="middle" font-size="14">Checkout</text>
        </g>
      </svg>
    `;

    const report = analyze(svg);

    expect(report.diagram.nodes).toEqual([
      {
        id: "checkout",
        bounds: { x: 20, y: 20, width: 120, height: 56 },
        labelBounds: [{ x: 47, y: 39, width: 66, height: 17 }],
      },
    ]);
  });

  test("infers a node from a group containing a rectangle and text", () => {
    /**
     * Given an unannotated SVG group with a direct rectangle and text child
     * When the SVG is analyzed
     * Then the group is exposed as a node using its id
     */
    const svg = `
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 240 120">
        <g id="review">
          <rect x="30" y="24" width="100" height="48" />
          <text x="80" y="53" text-anchor="middle" font-size="14">Review</text>
        </g>
      </svg>
    `;

    const report = analyze(svg);

    expect(report.diagram.nodes.map((node) => node.id)).toEqual(["review"]);
  });

  test("recognizes an annotated circle as a circular node", () => {
    /**
     * Given an annotated group whose shape is a circle
     * When the SVG is analyzed
     * Then it is a circular node whose bounds are the circle's bounding box
     */
    const svg = `
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 240 160">
        <g data-node="start">
          <circle cx="80" cy="70" r="40" />
          <text x="80" y="75" text-anchor="middle" font-size="14">Start</text>
        </g>
      </svg>
    `;

    const [node] = analyze(svg).diagram.nodes;

    expect(node?.id).toBe("start");
    expect(node?.shape).toBe("circle");
    expect(node?.bounds).toEqual({ x: 40, y: 30, width: 80, height: 80 });
  });

  test("infers a circular node from a group containing a circle and text", () => {
    /**
     * Given an unannotated group with a direct circle and text child
     * When the SVG is analyzed
     * Then the group is exposed as a circular node using its id
     */
    const svg = `
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 240 160">
        <g id="done">
          <circle cx="80" cy="70" r="40" />
          <text x="80" y="75" text-anchor="middle" font-size="14">Done</text>
        </g>
      </svg>
    `;

    const nodes = analyze(svg).diagram.nodes;

    expect(nodes.map((node) => [node.id, node.shape])).toEqual([
      ["done", "circle"],
    ]);
  });

  test("recognizes an annotated connector between nodes", () => {
    /**
     * Given a line with explicit source and target node ids
     * When the SVG is analyzed
     * Then the public report exposes the connector and its endpoints
     */
    const svg = `
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 320 120">
        <g data-node="submit">
          <rect x="20" y="20" width="100" height="56" />
          <text x="70" y="53" text-anchor="middle">Submit</text>
        </g>
        <g data-node="review">
          <rect x="200" y="20" width="100" height="56" />
          <text x="250" y="53" text-anchor="middle">Review</text>
        </g>
        <line id="submit-review" data-from="submit" data-to="review"
          x1="120" y1="48" x2="200" y2="48" />
      </svg>
    `;

    const report = analyze(svg);

    expect(report.diagram.connectors).toEqual([
      {
        id: "submit-review",
        from: "submit",
        to: "review",
        points: [
          { x: 120, y: 48 },
          { x: 200, y: 48 },
        ],
      },
    ]);
  });

  test("treats a nested node as intentional containment", () => {
    /**
     * Given a child node nested inside a parent node group
     * When the SVG is analyzed
     * Then the child references its parent and their geometric overlap is allowed
     */
    const svg = `
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="-100 -100 600 400">
        <g data-node="system">
          <rect x="10" y="10" width="300" height="180" />
          <text x="30" y="35" font-size="14">System</text>
          <g data-node="service">
            <rect x="50" y="60" width="100" height="56" />
            <text x="100" y="93" text-anchor="middle" font-size="14">Service</text>
          </g>
        </g>
      </svg>
    `;

    const report = analyze(svg);
    const child = report.diagram.nodes.find((node) => node.id === "service");

    expect(child?.parentId).toBe("system");
    expect(report.issues.some((issue) => issue.code === "node-overlap")).toBe(
      false,
    );
  });

  test("rejects malformed XML as an SVG input error", () => {
    /**
     * Given malformed SVG XML
     * When the public API analyzes it
     * Then callers receive a typed input error instead of a partial report
     */
    expect(() => analyze("<svg><g></svg>")).toThrow(SvgInputError);
  });

  test("reports standalone freeform shapes as unsupported", () => {
    /**
     * Given a standalone shape that cannot be mapped to a diagram node
     * When the SVG is analyzed
     * Then it is exposed for diagnostics rather than assigned guessed semantics
     */
    const svg = `
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 120">
        <circle id="mechanism-wheel" cx="70" cy="60" r="30" />
      </svg>
    `;

    const report = analyze(svg);

    expect(report.diagram.unsupportedElements).toEqual([
      { id: "mechanism-wheel", tagName: "circle" },
    ]);
  });
});
