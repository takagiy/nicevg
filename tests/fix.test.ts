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
      { x: 135, y: 48 },
      { x: 135, y: 12 },
      { x: 245, y: 12 },
      { x: 245, y: 48 },
      { x: 260, y: 48 },
    ]);
    expect(
      result.report.issues.some(
        (issue) => issue.code === "connector-node-crossing",
      ),
    ).toBe(false);
  });

  test("leaves and enters node sides perpendicularly when detouring", () => {
    /**
     * Given a connector that must detour around a node between its ends
     * When the diagram is fixed
     * Then it leaves the source side and enters the target side at right
     *   angles, and no segment runs along a side of either node
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

    const result = fix(svg);
    const points =
      result.report.diagram.connectors.find((item) => item.id === "flow")
        ?.points ?? [];
    const sides = [
      // source and target outlines as [from, to] edges
      [
        { x: 20, y: 20 },
        { x: 120, y: 20 },
      ],
      [
        { x: 120, y: 20 },
        { x: 120, y: 76 },
      ],
      [
        { x: 20, y: 76 },
        { x: 120, y: 76 },
      ],
      [
        { x: 260, y: 20 },
        { x: 360, y: 20 },
      ],
      [
        { x: 260, y: 20 },
        { x: 260, y: 76 },
      ],
      [
        { x: 260, y: 76 },
        { x: 360, y: 76 },
      ],
    ];
    const runsAlong = (
      a: { x: number; y: number },
      b: { x: number; y: number },
    ) =>
      sides.some(([from, to]) => {
        if (from === undefined || to === undefined) return false;
        const shared = (p1: number, p2: number, q1: number, q2: number) =>
          Math.min(Math.max(p1, p2), Math.max(q1, q2)) -
            Math.max(Math.min(p1, p2), Math.min(q1, q2)) >
          0;
        return from.y === to.y
          ? a.y === b.y && a.y === from.y && shared(a.x, b.x, from.x, to.x)
          : a.x === b.x && a.x === from.x && shared(a.y, b.y, from.y, to.y);
      });

    expect(points.at(0)).toEqual({ x: 120, y: 48 });
    expect(points.at(1)?.y).toBe(48);
    expect(points.at(1)?.x).toBeGreaterThan(120);
    expect(points.at(-1)).toEqual({ x: 260, y: 48 });
    expect(points.at(-2)?.y).toBe(48);
    expect(points.at(-2)?.x).toBeLessThan(260);
    expect(
      points.slice(1).some((point, index) => {
        const previous = points[index];
        return previous !== undefined && runsAlong(previous, point);
      }),
    ).toBe(false);
    expect(result.report.issues).toEqual([]);
  });

  test("reroutes between offset ports with a jog instead of a diagonal", () => {
    /**
     * Given a connector whose ends sit inside two nodes too far apart in
     *   height to share a port coordinate
     * When the diagram is fixed
     * Then it becomes an orthogonal path that jogs in the channel between them
     */
    const svg = `
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="-100 -100 600 300">
        <g data-node="source">
          <rect x="20" y="20" width="100" height="56" />
          <text x="70" y="53" text-anchor="middle" font-size="14">Source</text>
        </g>
        <g data-node="target">
          <rect x="260" y="60" width="100" height="56" />
          <text x="310" y="93" text-anchor="middle" font-size="14">Target</text>
        </g>
        <line id="flow" data-from="source" data-to="target"
          x1="100" y1="48" x2="280" y2="88" />
      </svg>
    `;

    const result = fix(svg);

    expect(
      result.report.diagram.connectors.find((item) => item.id === "flow")
        ?.points,
    ).toEqual([
      { x: 120, y: 48 },
      { x: 190, y: 48 },
      { x: 190, y: 88 },
      { x: 260, y: 88 },
    ]);
    expect(result.report.issues).toEqual([]);
  });

  test("aligns ports on facing sides so the connector stays straight", () => {
    /**
     * Given a tall source with two outgoing connectors on its right side and
     *   a target whose left side takes only one of them
     * When the diagram is fixed
     * Then the connector to the facing target is a single straight segment
     */
    const svg = `
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="-100 -100 600 400">
        <g data-node="source">
          <rect x="20" y="20" width="100" height="90" />
          <text x="70" y="70" text-anchor="middle" font-size="14">Source</text>
        </g>
        <g data-node="upper">
          <rect x="260" y="20" width="100" height="56" />
          <text x="310" y="53" text-anchor="middle" font-size="14">Upper</text>
        </g>
        <g data-node="lower">
          <rect x="260" y="140" width="100" height="56" />
          <text x="310" y="173" text-anchor="middle" font-size="14">Lower</text>
        </g>
        <line id="to-upper" data-from="source" data-to="upper"
          x1="100" y1="50" x2="280" y2="48" />
        <line id="to-lower" data-from="source" data-to="lower"
          x1="100" y1="80" x2="280" y2="168" />
      </svg>
    `;

    const result = fix(svg);

    expect(
      result.report.diagram.connectors.find((item) => item.id === "to-upper")
        ?.points,
    ).toEqual([
      { x: 120, y: 50 },
      { x: 260, y: 50 },
    ]);
    expect(result.report.issues).toEqual([]);
  });

  test("keeps aligned ports at least 10px from other ports on the side", () => {
    /**
     * Given a target whose left side takes two connectors, and a facing
     *   source whose port would land 9px from the target's other port
     * When the diagram is fixed
     * Then the source port moves instead, and the connector stays straight
     */
    const svg = `
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="-100 -100 600 400">
        <g data-node="source">
          <rect x="20" y="20" width="100" height="56" />
          <text x="70" y="53" text-anchor="middle" font-size="14">Source</text>
        </g>
        <g data-node="other">
          <rect x="20" y="140" width="100" height="56" />
          <text x="70" y="173" text-anchor="middle" font-size="14">Other</text>
        </g>
        <g data-node="target">
          <rect x="260" y="20" width="100" height="56" />
          <text x="310" y="53" text-anchor="middle" font-size="14">Target</text>
        </g>
        <line id="facing" data-from="source" data-to="target"
          x1="100" y1="48" x2="280" y2="48" />
        <line id="climbing" data-from="other" data-to="target"
          x1="100" y1="168" x2="280" y2="60" />
      </svg>
    `;

    const result = fix(svg);

    expect(
      result.report.diagram.connectors.find((item) => item.id === "facing")
        ?.points,
    ).toEqual([
      { x: 120, y: 39 },
      { x: 260, y: 39 },
    ]);
    expect(result.report.issues).toEqual([]);
  });

  test("moves a connector's label beside its rerouted path", () => {
    /**
     * Given a label tied to a connector by data-label-for, left at the old
     *   midpoint after the connector has to detour
     * When the diagram is fixed
     * Then the label sits centred above the detour's longest segment
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
        <text data-label="retry" data-label-for="flow"
          x="190" y="42" text-anchor="middle" font-size="14">Retry</text>
      </svg>
    `;

    const result = fix(svg);
    const label = new DOMParser()
      .parseFromString(result.svg, "image/svg+xml")
      .getElementsByTagName("text")
      .item(3);

    expect({
      x: label?.getAttribute("x"),
      y: label?.getAttribute("y"),
      anchor: label?.getAttribute("text-anchor"),
    }).toEqual({ x: "190", y: "0", anchor: "middle" });
    expect(result.changes).toContainEqual({
      code: "move-label",
      message: 'Moved label "retry" beside connector "flow".',
      elements: ["retry", "flow"],
    });
    expect(result.report.issues).toEqual([]);
  });

  test("keeps a moved label clear of other labels", () => {
    /**
     * Given a free note sitting where the moved label would go first
     * When the diagram is fixed
     * Then the label moves along the detour, at least 4px from the note,
     *   and no issue remains
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
        <text data-label="note" x="190" y="0" text-anchor="middle"
          font-size="14">Note</text>
        <line id="flow" data-from="source" data-to="target"
          x1="120" y1="48" x2="260" y2="48" />
        <text data-label="retry" data-label-for="flow"
          x="190" y="42" text-anchor="middle" font-size="14">Retry</text>
      </svg>
    `;

    const result = fix(svg);
    const labelBounds = (id: string) =>
      result.report.diagram.labels.find((label) => label.id === id)?.bounds;
    const retry = labelBounds("retry");
    const note = labelBounds("note");

    expect(retry).toBeDefined();
    expect(note).toEqual({ x: 173.5, y: -14, width: 33, height: 17 });
    expect(
      retry !== undefined &&
        note !== undefined &&
        (retry.x + retry.width + 4 <= note.x ||
          note.x + note.width + 4 <= retry.x ||
          retry.y + retry.height + 4 <= note.y ||
          note.y + note.height + 4 <= retry.y),
    ).toBe(true);
    expect(result.report.issues).toEqual([]);
  });

  test("does not reroute a connector onto another connector", () => {
    /**
     * Given a crossing connector whose shortest detour is taken by a bus line
     * When the diagram is fixed
     * Then the connector detours on the free side instead of overlapping
     */
    const svg = `
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="-100 -100 600 300">
        <g data-node="west">
          <rect x="-60" y="-10" width="60" height="40" />
          <text x="-30" y="15" text-anchor="middle" font-size="12">W</text>
        </g>
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
        <g data-node="east">
          <rect x="400" y="-10" width="60" height="40" />
          <text x="430" y="15" text-anchor="middle" font-size="12">E</text>
        </g>
        <line id="bus" data-from="west" data-to="east"
          x1="0" y1="12" x2="400" y2="12" />
        <line id="flow" data-from="source" data-to="target"
          x1="120" y1="48" x2="260" y2="48" />
      </svg>
    `;

    const result = fix(svg);
    const pointsOf = (id: string) =>
      result.report.diagram.connectors.find((item) => item.id === id)?.points;

    expect(pointsOf("bus")).toEqual([
      { x: 0, y: 12 },
      { x: 400, y: 12 },
    ]);
    expect(pointsOf("flow")).toEqual([
      { x: 120, y: 48 },
      { x: 135, y: 48 },
      { x: 135, y: 84 },
      { x: 245, y: 84 },
      { x: 245, y: 48 },
      { x: 260, y: 48 },
    ]);
    expect(result.report.issues).toEqual([]);
  });

  test("does not reroute onto a short segment between grid lines", () => {
    /**
     * Given a short connector segment on the detour's shortest vertical leg
     * When the diagram is fixed
     * Then the detour takes the other side instead of overlapping it
     */
    const svg = `
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="-100 -100 600 300">
        <g data-node="west">
          <rect x="-60" y="-60" width="60" height="40" />
          <text x="-30" y="-35" text-anchor="middle" font-size="12">W</text>
        </g>
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
        <path id="tick" data-from="west" data-to="target"
          d="M 0 -40 L 142 -40 L 142 34 L 135 34 L 135 40" />
        <line id="flow" data-from="source" data-to="target"
          x1="120" y1="48" x2="260" y2="48" />
      </svg>
    `;

    const result = fix(svg);
    const pointsOf = (id: string) =>
      result.report.diagram.connectors.find((item) => item.id === id)?.points;

    expect(pointsOf("flow")).toEqual([
      { x: 120, y: 48 },
      { x: 135, y: 48 },
      { x: 135, y: 84 },
      { x: 245, y: 84 },
      { x: 245, y: 48 },
      { x: 260, y: 48 },
    ]);
    expect(
      result.report.issues.some((issue) => issue.code === "connector-overlap"),
    ).toBe(false);
  });

  test("detours in a parallel lane when both detour lanes are taken", () => {
    /**
     * Given a row of equally tall nodes, so the only detour lines run 8px
     *   above and below them, and two buses already occupying both lines
     * When the diagram is fixed
     * Then the crossing connector detours in a lane beside a bus instead of
     *   staying a straight line through the obstacle
     */
    const node = (id: string, x: number, width: number) => `
      <g data-node="${id}">
        <rect x="${x}" y="20" width="${width}" height="56" />
        <text x="${x + width / 2}" y="53" text-anchor="middle"
          font-size="12">${id.slice(0, 1).toUpperCase()}</text>
      </g>`;
    const svg = `
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="-100 -100 600 300">
        ${node("west", -60, 60)}
        ${node("source", 20, 100)}
        ${node("obstacle", 150, 80)}
        ${node("target", 260, 100)}
        ${node("east", 400, 60)}
        <line id="upper" data-from="west" data-to="east"
          x1="0" y1="12" x2="400" y2="12" />
        <line id="lower" data-from="west" data-to="east"
          x1="0" y1="84" x2="400" y2="84" />
        <line id="flow" data-from="source" data-to="target"
          x1="120" y1="48" x2="260" y2="48" />
      </svg>
    `;

    const result = fix(svg);
    const points =
      result.report.diagram.connectors.find((item) => item.id === "flow")
        ?.points ?? [];

    expect(points.length).toBeGreaterThan(2);
    expect(
      points
        .slice(1)
        .every(
          (point, index) =>
            point.x === points[index]?.x || point.y === points[index]?.y,
        ),
    ).toBe(true);
    expect(result.report.issues).toEqual([]);
  });

  test("separates a request and reply drawn on the same line", () => {
    /**
     * Given a request and a reply that overlap between the same two nodes
     * When the diagram is fixed
     * Then their endpoints are spread along the node edges and no overlap remains
     */
    const svg = `
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="-100 -100 600 300">
        <g data-node="client">
          <rect x="20" y="20" width="100" height="60" />
          <text x="70" y="55" text-anchor="middle" font-size="14">Client</text>
        </g>
        <g data-node="server">
          <rect x="260" y="20" width="100" height="60" />
          <text x="310" y="55" text-anchor="middle" font-size="14">Server</text>
        </g>
        <line id="request" data-from="client" data-to="server"
          x1="120" y1="50" x2="260" y2="50" />
        <line id="reply" data-from="server" data-to="client"
          x1="260" y1="50" x2="120" y2="50" />
      </svg>
    `;

    const result = fix(svg);
    const pointsOf = (id: string) =>
      result.report.diagram.connectors.find((item) => item.id === id)?.points;

    expect(pointsOf("request")).toEqual([
      { x: 120, y: 40 },
      { x: 260, y: 40 },
    ]);
    expect(pointsOf("reply")).toEqual([
      { x: 260, y: 60 },
      { x: 120, y: 60 },
    ]);
    expect(result.report.issues).toEqual([]);
  });

  test("reroutes with several bends when one detour cannot clear the obstacles", () => {
    /**
     * Given a 3x3 grid of nodes and a connector between opposite corners
     * When the diagram is fixed
     * Then the connector becomes an orthogonal path that crosses no node
     */
    const cell = (id: string, x: number, y: number) => `
      <g data-node="${id}">
        <rect x="${x}" y="${y}" width="100" height="50" />
        <text x="${x + 50}" y="${y + 30}" text-anchor="middle" font-size="14">${id}</text>
      </g>`;
    const svg = `
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="-100 -100 600 500">
        ${[0, 1, 2]
          .flatMap((row) =>
            [0, 1, 2].map((col) => cell(`n${row}${col}`, col * 150, row * 100)),
          )
          .join("")}
        <line id="flow" data-from="n20" data-to="n02"
          x1="100" y1="225" x2="300" y2="25" />
      </svg>
    `;

    const result = fix(svg);
    const points =
      result.report.diagram.connectors.find((item) => item.id === "flow")
        ?.points ?? [];

    expect(points.at(0)).toEqual({ x: 100, y: 225 });
    expect(points.at(-1)).toEqual({ x: 300, y: 25 });
    expect(
      points
        .slice(1)
        .every(
          (point, index) =>
            point.x === points[index]?.x || point.y === points[index]?.y,
        ),
    ).toBe(true);
    expect(
      result.report.issues.some(
        (issue) => issue.code === "connector-node-crossing",
      ),
    ).toBe(false);
  });
});
