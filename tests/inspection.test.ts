import { describe, expect, test } from "bun:test";
import { analyze } from "../src/index";

describe("diagram inspection", () => {
  test("accepts a diagram whose drawing fits inside the safe viewport", () => {
    /**
     * Given a diagram whose marks fit inside the viewBox with 20px padding
     * When the diagram is analyzed
     * Then the report is valid and contains the measured drawing bounds
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

    expect(report.valid).toBe(true);
    expect(report.issues).toEqual([]);
    expect(report.drawingBounds).toEqual({
      x: 20,
      y: 20,
      width: 120,
      height: 56,
    });
  });

  test("reports the side and required bounds when the viewBox clips content", () => {
    /**
     * Given a node whose right edge plus safety padding exceeds the viewBox
     * When the diagram is analyzed
     * Then a viewport clipping issue identifies the right side and required viewBox
     */
    const svg = `
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 120">
        <g data-node="checkout">
          <rect x="20" y="20" width="100" height="56" />
          <text x="70" y="53" text-anchor="middle" font-size="14">Checkout</text>
        </g>
      </svg>
    `;

    const report = analyze(svg);

    expect(report.valid).toBe(false);
    expect(report.issues).toEqual([
      {
        code: "viewport-clipping",
        message: "Drawing exceeds the safe viewBox on the right side.",
        elements: ["svg"],
        details: {
          sides: ["right"],
          requiredViewBox: { x: 0, y: 0, width: 140, height: 96 },
        },
      },
    ]);
  });

  test("reports the required box size when a label violates node padding", () => {
    /**
     * Given a label wider than its node's inner area
     * When the diagram is analyzed
     * Then a text overflow issue reports the minimum box dimensions
     */
    const svg = `
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="-100 -100 400 300">
        <g data-node="confirm">
          <rect x="20" y="20" width="70" height="40" />
          <text x="55" y="52" text-anchor="middle" font-size="14">Confirm payment</text>
        </g>
      </svg>
    `;

    const report = analyze(svg);

    expect(report.issues).toEqual([
      {
        code: "text-overflow",
        message: 'Label does not fit inside node "confirm" with 12px padding.',
        elements: ["confirm"],
        details: { requiredWidth: 148, requiredHeight: 41 },
      },
    ]);
  });

  test("reports overlap between unrelated nodes", () => {
    /**
     * Given two nodes with intersecting rectangles and no containment relation
     * When the diagram is analyzed
     * Then the overlapping pair and intersection bounds are reported
     */
    const svg = `
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="-100 -100 500 300">
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

    const report = analyze(svg);

    expect(report.issues).toContainEqual({
      code: "node-overlap",
      message: 'Nodes "first" and "second" overlap.',
      elements: ["first", "second"],
      details: {
        intersection: { x: 90, y: 20, width: 30, height: 56 },
      },
    });
  });

  test("reports insufficient spacing between nodes in the same row", () => {
    /**
     * Given two horizontally adjacent nodes with only a 10px gap
     * When the diagram is analyzed
     * Then the report identifies the missing distance to the required 20px gap
     */
    const svg = `
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="-100 -100 500 300">
        <g data-node="first">
          <rect x="20" y="20" width="100" height="56" />
          <text x="70" y="53" text-anchor="middle" font-size="14">First</text>
        </g>
        <g data-node="second">
          <rect x="130" y="20" width="100" height="56" />
          <text x="180" y="53" text-anchor="middle" font-size="14">Second</text>
        </g>
      </svg>
    `;

    const report = analyze(svg);

    expect(report.issues).toContainEqual({
      code: "node-gap",
      message: 'Nodes "first" and "second" have a 10px gap; 20px is required.',
      elements: ["first", "second"],
      details: { actualGap: 10, requiredGap: 20, shortage: 10 },
    });
  });

  test("reports a connector crossing an unrelated node", () => {
    /**
     * Given a connector whose segment passes through a third node
     * When the diagram is analyzed
     * Then the connector and obstructing node are reported
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
          x1="120" y1="48" x2="260" y2="48" />
      </svg>
    `;

    const report = analyze(svg);

    expect(report.issues).toContainEqual({
      code: "connector-node-crossing",
      message: 'Connector "flow" crosses unrelated node "obstacle".',
      elements: ["flow", "obstacle"],
    });
  });

  test("reports a connector passing within 8px of a free label", () => {
    /**
     * Given a connector that passes 5px below an edge label
     * When the diagram is analyzed
     * Then the connector-label clearance violation is reported
     */
    const svg = `
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="-100 -100 600 300">
        <g data-node="source">
          <rect x="20" y="20" width="100" height="56" />
          <text x="70" y="53" text-anchor="middle" font-size="14">Source</text>
        </g>
        <g data-node="target">
          <rect x="260" y="20" width="100" height="56" />
          <text x="310" y="53" text-anchor="middle" font-size="14">Target</text>
        </g>
        <text id="retry-label" data-label="retry"
          x="150" y="38" font-size="14">Retry</text>
        <line id="flow" data-from="source" data-to="target"
          x1="120" y1="46" x2="260" y2="46" />
      </svg>
    `;

    const report = analyze(svg);

    expect(report.issues).toContainEqual({
      code: "connector-label-clearance",
      message: 'Connector "flow" passes within 8px of label "retry".',
      elements: ["flow", "retry"],
      details: { requiredClearance: 8 },
    });
  });

  test("reports connector endpoints placed inside their nodes", () => {
    /**
     * Given a connector whose endpoints stop inside the source and target
     * When the diagram is analyzed
     * Then each endpoint is reported instead of being accepted as edge-to-edge
     */
    const svg = `
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="-100 -100 600 300">
        <g data-node="source">
          <rect x="20" y="20" width="100" height="56" />
          <text x="70" y="53" text-anchor="middle" font-size="14">Source</text>
        </g>
        <g data-node="target">
          <rect x="260" y="20" width="100" height="56" />
          <text x="310" y="53" text-anchor="middle" font-size="14">Target</text>
        </g>
        <line id="flow" data-from="source" data-to="target"
          x1="110" y1="48" x2="270" y2="48" />
      </svg>
    `;

    const report = analyze(svg);

    expect(
      report.issues.filter(
        (issue) => issue.code === "connector-endpoint-inside",
      ),
    ).toEqual([
      {
        code: "connector-endpoint-inside",
        message: 'Connector "flow" starts inside node "source".',
        elements: ["flow", "source"],
        details: { endpoint: "start" },
      },
      {
        code: "connector-endpoint-inside",
        message: 'Connector "flow" ends inside node "target".',
        elements: ["flow", "target"],
        details: { endpoint: "end" },
      },
    ]);
  });

  test("allows overlap explicitly marked as intentional", () => {
    /**
     * Given overlapping peer nodes where one declares intentional overlap
     * When the diagram is analyzed
     * Then no overlap issue is reported for that pair
     */
    const svg = `
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="-100 -100 500 300">
        <g data-node="back">
          <rect x="20" y="20" width="100" height="56" />
          <text x="70" y="53" text-anchor="middle" font-size="14">Back</text>
        </g>
        <g data-node="front" data-allow-overlap="true">
          <rect x="90" y="20" width="100" height="56" />
          <text x="140" y="53" text-anchor="middle" font-size="14">Front</text>
        </g>
      </svg>
    `;

    const report = analyze(svg);

    expect(report.issues.some((issue) => issue.code === "node-overlap")).toBe(
      false,
    );
  });

  test("reports overlapping free labels", () => {
    /**
     * Given two free labels whose estimated bounding boxes intersect
     * When the diagram is analyzed
     * Then the overlapping label pair is reported
     */
    const svg = `
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="-100 -100 500 300">
        <text data-label="approved" x="40" y="60" font-size="14">Approved</text>
        <text data-label="pending" x="80" y="60" font-size="14">Pending</text>
      </svg>
    `;

    const report = analyze(svg);

    expect(report.issues).toContainEqual({
      code: "label-overlap",
      message: 'Labels "approved" and "pending" overlap.',
      elements: ["approved", "pending"],
    });
  });

  test("includes free labels when checking the safe viewBox", () => {
    /**
     * Given a free label that extends beyond the viewBox safety area
     * When the diagram is analyzed
     * Then viewport clipping is reported even when no nodes exist
     */
    const svg = `
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 120 80">
        <text data-label="note" x="90" y="40" font-size="14">Note</text>
      </svg>
    `;

    const report = analyze(svg);

    expect(report.issues).toContainEqual({
      code: "viewport-clipping",
      message: "Drawing exceeds the safe viewBox on the right side.",
      elements: ["svg"],
      details: {
        sides: ["right"],
        requiredViewBox: { x: 70, y: 6, width: 73, height: 57 },
      },
    });
  });

  test("reports connectors that run along the same segment", () => {
    /**
     * Given a request and a reply drawn on the same horizontal line
     * When the diagram is analyzed
     * Then the pair of overlapping connectors is reported
     */
    const svg = `
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="-100 -100 600 300">
        <g data-node="client">
          <rect x="20" y="20" width="100" height="56" />
          <text x="70" y="53" text-anchor="middle" font-size="14">Client</text>
        </g>
        <g data-node="server">
          <rect x="260" y="20" width="100" height="56" />
          <text x="310" y="53" text-anchor="middle" font-size="14">Server</text>
        </g>
        <line id="request" data-from="client" data-to="server"
          x1="120" y1="48" x2="260" y2="48" />
        <line id="reply" data-from="server" data-to="client"
          x1="260" y1="48" x2="120" y2="48" />
      </svg>
    `;

    const report = analyze(svg);

    expect(report.issues).toContainEqual({
      code: "connector-overlap",
      message: 'Connectors "request" and "reply" overlap along a segment.',
      elements: ["request", "reply"],
    });
  });

  test("does not report connectors that only cross each other", () => {
    /**
     * Given a horizontal and a vertical connector that cross at one point
     * When the diagram is analyzed
     * Then no connector overlap is reported
     */
    const svg = `
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="-100 -100 600 400">
        <g data-node="west">
          <rect x="0" y="100" width="80" height="40" />
          <text x="40" y="125" text-anchor="middle" font-size="14">West</text>
        </g>
        <g data-node="east">
          <rect x="300" y="100" width="80" height="40" />
          <text x="340" y="125" text-anchor="middle" font-size="14">East</text>
        </g>
        <g data-node="north">
          <rect x="150" y="0" width="80" height="40" />
          <text x="190" y="25" text-anchor="middle" font-size="14">North</text>
        </g>
        <g data-node="south">
          <rect x="150" y="200" width="80" height="40" />
          <text x="190" y="225" text-anchor="middle" font-size="14">South</text>
        </g>
        <line id="across" data-from="west" data-to="east"
          x1="80" y1="120" x2="300" y2="120" />
        <line id="down" data-from="north" data-to="south"
          x1="190" y1="40" x2="190" y2="200" />
      </svg>
    `;

    const report = analyze(svg);

    expect(
      report.issues.some((issue) => issue.code === "connector-overlap"),
    ).toBe(false);
  });
  const tiedLabel = (labelY: number) => `
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="-100 -100 600 300">
      <g data-node="source">
        <rect x="20" y="20" width="100" height="56" />
        <text x="70" y="53" text-anchor="middle" font-size="14">Source</text>
      </g>
      <g data-node="target">
        <rect x="260" y="20" width="100" height="56" />
        <text x="310" y="53" text-anchor="middle" font-size="14">Target</text>
      </g>
      <line id="flow" data-from="source" data-to="target"
        x1="120" y1="48" x2="260" y2="48" />
      <text data-label="retry" data-label-for="flow"
        x="190" y="${labelY}" text-anchor="middle" font-size="14">Retry</text>
    </svg>
  `;

  test("reports a tied label that sits away from its connector", () => {
    /**
     * Given a label tied to a connector by data-label-for, 45px above it
     * When the diagram is analyzed
     * Then the label is reported as detached from that connector
     */
    const report = analyze(tiedLabel(0));

    expect(
      report.issues
        .filter((issue) => issue.code === "label-detached")
        .map((issue) => issue.elements),
    ).toEqual([["retry", "flow"]]);
  });

  test("accepts a tied label right beside its connector", () => {
    /**
     * Given a label tied to a connector, keeping 9px above it
     * When the diagram is analyzed
     * Then no detached label is reported
     */
    const report = analyze(tiedLabel(36));

    expect(report.issues.map((issue) => issue.code)).not.toContain(
      "label-detached",
    );
  });
});
