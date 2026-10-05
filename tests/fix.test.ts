import { describe, expect } from "bun:test";
import { DOMParser } from "@xmldom/xmldom";
import { analyze, type Bounds } from "../src/index";
import {
  bendCount,
  connectorPoints,
  contains,
  distanceFromCircle,
  distanceToRoute,
  entersBox,
  entersPerpendicularly,
  gapBetween,
  inflate,
  isOrthogonal,
  issueCodes,
  labelBounds,
  leavesPerpendicularly,
  longestSegment,
  nodeBounds,
  overlapsRoute,
  qualityOf,
  runsAlong,
  sideOf,
} from "./support/geometry";
import { fix, test } from "./support/recording";

const parse = (svg: string) =>
  new DOMParser().parseFromString(svg, "image/svg+xml");

const viewBoxOf = (svg: string): Bounds => {
  const [x = 0, y = 0, width = 0, height = 0] = (
    parse(svg).documentElement.getAttribute("viewBox") ?? ""
  )
    .trim()
    .split(/[\s,]+/)
    .map(Number);
  return { x, y, width, height };
};

// Source, an unrelated block and a target in one row, joined by a connector
// that runs straight through the block.
const blockedRow = (extra = "") => `
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
    ${extra}
    <line id="flow" data-from="source" data-to="target"
      x1="120" y1="48" x2="260" y2="48" />
  </svg>
`;

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
    const viewBox = viewBoxOf(result.svg);

    expect(contains(viewBox, viewBoxOf(svg))).toBe(true);
    expect(
      result.report.drawingBounds !== null &&
        contains(viewBox, inflate(result.report.drawingBounds, 20)),
    ).toBe(true);
    expect(issueCodes(result.report)).not.toContain("viewport-clipping");
    expect(result.changes.map((change) => change.code)).toEqual([
      "expand-viewbox",
    ]);

    expect(qualityOf(result.report)).toMatchSnapshot();
    expect(result.svg).toMatchSnapshot();
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
    const text = parse(result.svg).getElementsByTagName("text").item(0);
    const box = nodeBounds(result.report, "confirm");
    const label = result.report.diagram.nodes[0]?.labelBounds[0];

    expect(contains(box, { x: 20, y: 20, width: 70, height: 40 })).toBe(true);
    expect(label !== undefined && contains(box, inflate(label, 12))).toBe(true);
    expect({ x: text?.getAttribute("x"), y: text?.getAttribute("y") }).toEqual({
      x: "55",
      y: "52",
    });
    expect(issueCodes(result.report)).not.toContain("text-overflow");

    expect(qualityOf(result.report)).toMatchSnapshot();
    expect(result.svg).toMatchSnapshot();
  });

  test("grows a circular node around its label without moving either", () => {
    /**
     * Given a circular node whose label reaches within 12px of its circle
     * When the diagram is fixed
     * Then the radius grows until the label fits, while the centre and the
     *   text coordinates stay put
     */
    const svg = `
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 240 200">
        <g data-node="confirm">
          <circle cx="120" cy="100" r="40" />
          <text x="120" y="105" text-anchor="middle" font-size="14">Confirmed</text>
        </g>
      </svg>
    `;

    const result = fix(svg);
    const document = parse(result.svg);
    const circle = document.getElementsByTagName("circle").item(0);
    const text = document.getElementsByTagName("text").item(0);

    expect({
      cx: circle?.getAttribute("cx"),
      cy: circle?.getAttribute("cy"),
    }).toEqual({ cx: "120", cy: "100" });
    expect(Number(circle?.getAttribute("r"))).toBeGreaterThan(40);
    expect({ x: text?.getAttribute("x"), y: text?.getAttribute("y") }).toEqual({
      x: "120",
      y: "105",
    });
    expect(issueCodes(result.report)).not.toContain("text-overflow");

    expect(qualityOf(result.report)).toMatchSnapshot();
    expect(result.svg).toMatchSnapshot();
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
    const first = nodeBounds(result.report, "first");
    const second = nodeBounds(result.report, "second");

    expect(first).toEqual({ x: 20, y: 20, width: 100, height: 56 });
    expect(second.x).toBeGreaterThan(first.x);
    expect(gapBetween(first, second)).toBeGreaterThanOrEqual(20);
    expect(issueCodes(result.report)).not.toContain("node-overlap");
    expect(issueCodes(result.report)).not.toContain("node-gap");

    expect(qualityOf(result.report)).toMatchSnapshot();
    expect(result.svg).toMatchSnapshot();
  });

  test("reroutes a connector around an unrelated node", () => {
    /**
     * Given a horizontal connector that crosses an unrelated node
     * When the diagram is fixed
     * Then it follows an orthogonal path with 8px obstacle clearance
     */
    const result = fix(blockedRow());
    const points = connectorPoints(result.report, "flow");

    expect(isOrthogonal(points)).toBe(true);
    expect(
      entersBox(points, inflate(nodeBounds(result.report, "obstacle"), 8)),
    ).toBe(false);
    expect(issueCodes(result.report)).not.toContain("connector-node-crossing");

    expect(qualityOf(result.report)).toMatchSnapshot();
    expect(result.svg).toMatchSnapshot();
  });

  test("keeps a rerouted line unfilled once it becomes a path", () => {
    /**
     * Given a line without a fill, which SVG never fills, that has to detour
     * When the diagram is fixed
     * Then the bent path that replaces it is not filled either, while
     *   its other attributes carry over
     */
    const result = fix(blockedRow());
    const path = parse(result.svg).getElementById("flow");

    expect(path?.tagName).toBe("path");
    expect(path?.getAttribute("fill")).toBe("none");
    expect(path?.getAttribute("data-from")).toBe("source");

    expect(qualityOf(result.report)).toMatchSnapshot();
    expect(result.svg).toMatchSnapshot();
  });

  test("leaves and enters node sides perpendicularly when detouring", () => {
    /**
     * Given a connector that must detour around a node between its ends
     * When the diagram is fixed
     * Then it leaves the source side and enters the target side at right
     *   angles, and no segment runs along a side of either node
     */
    const result = fix(blockedRow());
    const points = connectorPoints(result.report, "flow");
    const source = nodeBounds(result.report, "source");
    const target = nodeBounds(result.report, "target");

    expect(leavesPerpendicularly(points, source)).toBe(true);
    expect(entersPerpendicularly(points, target)).toBe(true);
    expect(runsAlong(points, source)).toBe(false);
    expect(runsAlong(points, target)).toBe(false);
    expect(result.report.issues).toEqual([]);

    expect(qualityOf(result.report)).toMatchSnapshot();
    expect(result.svg).toMatchSnapshot();
  });

  test("ends a rerouted connector on a circular node's circle", () => {
    /**
     * Given a connector from inside a circular node to a facing box whose
     *   middle sits so much lower than the circle's centre that the port on
     *   the circle's side moves down to meet it
     * When the diagram is fixed
     * Then the connector stays orthogonal and starts on the circle itself,
     *   not on its bounding box
     */
    const svg = `
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 400 200">
        <g data-node="start">
          <circle cx="70" cy="80" r="30" />
          <text x="70" y="85" text-anchor="middle" font-size="14">Go</text>
        </g>
        <g data-node="target">
          <rect x="240" y="73" width="100" height="56" />
          <text x="290" y="106" text-anchor="middle" font-size="14">Target</text>
        </g>
        <line id="flow" data-from="start" data-to="target"
          x1="70" y1="80" x2="290" y2="101" />
      </svg>
    `;

    const result = fix(svg);
    const points = connectorPoints(result.report, "flow");
    const start = points[0] ?? { x: 0, y: 0 };

    expect(isOrthogonal(points)).toBe(true);
    expect(
      distanceFromCircle(start, nodeBounds(result.report, "start")),
    ).toBeLessThanOrEqual(0.5);
    expect(
      entersPerpendicularly(points, nodeBounds(result.report, "target")),
    ).toBe(true);
    expect(result.report.issues).toEqual([]);

    expect(qualityOf(result.report)).toMatchSnapshot();
    expect(result.svg).toMatchSnapshot();
  });

  test("spreads connectors on one side of a circle, each ending on it", () => {
    /**
     * Given two connectors from inside a circular node to two boxes stacked
     *   on its right
     * When the diagram is fixed
     * Then both leave the circle's right half from separate points on the
     *   circle, without overlapping
     */
    const svg = `
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 400 240">
        <g data-node="hub">
          <circle cx="80" cy="120" r="40" />
          <text x="80" y="125" text-anchor="middle" font-size="14">Hub</text>
        </g>
        <g data-node="upper">
          <rect x="240" y="60" width="100" height="56" />
          <text x="290" y="93" text-anchor="middle" font-size="14">Upper</text>
        </g>
        <g data-node="lower">
          <rect x="240" y="136" width="100" height="56" />
          <text x="290" y="169" text-anchor="middle" font-size="14">Lower</text>
        </g>
        <line id="to-upper" data-from="hub" data-to="upper"
          x1="80" y1="120" x2="290" y2="88" />
        <line id="to-lower" data-from="hub" data-to="lower"
          x1="80" y1="120" x2="290" y2="164" />
      </svg>
    `;

    const result = fix(svg);
    const hub = nodeBounds(result.report, "hub");
    const upper = connectorPoints(result.report, "to-upper");
    const lower = connectorPoints(result.report, "to-lower");

    for (const route of [upper, lower]) {
      const start = route[0] ?? { x: 0, y: 0 };
      expect(isOrthogonal(route)).toBe(true);
      expect(distanceFromCircle(start, hub)).toBeLessThanOrEqual(0.5);
      expect(start.x).toBeGreaterThan(hub.x + hub.width / 2);
    }
    expect(upper[0]).not.toEqual(lower[0]);
    expect(overlapsRoute(upper, lower)).toBe(false);
    expect(result.report.issues).toEqual([]);

    expect(qualityOf(result.report)).toMatchSnapshot();
    expect(result.svg).toMatchSnapshot();
  });

  test("reroutes between offset ports orthogonally instead of diagonally", () => {
    /**
     * Given a connector whose ends sit inside two nodes too far apart in
     *   height to share a port coordinate
     * When the diagram is fixed
     * Then it becomes an orthogonal path meeting both nodes at right angles
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
    const points = connectorPoints(result.report, "flow");

    expect(isOrthogonal(points)).toBe(true);
    expect(
      leavesPerpendicularly(points, nodeBounds(result.report, "source")),
    ).toBe(true);
    expect(
      entersPerpendicularly(points, nodeBounds(result.report, "target")),
    ).toBe(true);
    expect(result.report.issues).toEqual([]);

    expect(qualityOf(result.report)).toMatchSnapshot();
    expect(result.svg).toMatchSnapshot();
  });

  test("aligns ports on facing sides so the connector stays straight", () => {
    /**
     * Given a tall source facing a target whose left side midpoint sits at a
     *   different height from the source's right side midpoint
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
    const points = connectorPoints(result.report, "to-upper");

    expect(points).toHaveLength(2);
    expect(isOrthogonal(points)).toBe(true);
    expect(
      sideOf(points[0] ?? { x: 0, y: 0 }, nodeBounds(result.report, "source")),
    ).toBe("right");
    expect(
      sideOf(points[1] ?? { x: 0, y: 0 }, nodeBounds(result.report, "upper")),
    ).toBe("left");
    expect(result.report.issues).toEqual([]);

    expect(qualityOf(result.report)).toMatchSnapshot();
    expect(result.svg).toMatchSnapshot();
  });

  test("keeps aligned ports at least 10px from other ports on the side", () => {
    /**
     * Given a target whose bottom is blocked by a node right below it, so
     *   its left side takes two connectors, and a facing source whose port
     *   would land 9px from the target's other port
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
        <g data-node="floor">
          <rect x="260" y="91" width="100" height="56" />
          <text x="310" y="124" text-anchor="middle" font-size="14">Floor</text>
        </g>
        <line id="facing" data-from="source" data-to="target"
          x1="100" y1="48" x2="280" y2="48" />
        <line id="climbing" data-from="other" data-to="target"
          x1="100" y1="168" x2="280" y2="60" />
      </svg>
    `;

    const result = fix(svg);
    const target = nodeBounds(result.report, "target");
    const facing = connectorPoints(result.report, "facing");
    const climbingEnd = connectorPoints(result.report, "climbing").at(-1);
    const facingEnd = facing.at(-1);

    expect(facing).toHaveLength(2);
    expect(isOrthogonal(facing)).toBe(true);
    expect(facingEnd && sideOf(facingEnd, target)).toBe("left");
    expect(climbingEnd && sideOf(climbingEnd, target)).toBe("left");
    expect(
      Math.abs((facingEnd?.y ?? 0) - (climbingEnd?.y ?? 0)),
    ).toBeGreaterThanOrEqual(10);
    expect(result.report.issues).toEqual([]);

    expect(qualityOf(result.report)).toMatchSnapshot();
    expect(result.svg).toMatchSnapshot();
  });

  test("switches to another side when it saves bends", () => {
    /**
     * Given a target below and to the right, where right-to-left ports need
     *   a two-bend jog
     * When the diagram is fixed
     * Then the connector reaches the target with a single bend
     */
    const svg = `
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="-100 -100 600 400">
        <g data-node="source">
          <rect x="20" y="20" width="100" height="56" />
          <text x="70" y="53" text-anchor="middle" font-size="14">Source</text>
        </g>
        <g data-node="target">
          <rect x="260" y="160" width="100" height="56" />
          <text x="310" y="193" text-anchor="middle" font-size="14">Target</text>
        </g>
        <line id="flow" data-from="source" data-to="target"
          x1="100" y1="48" x2="300" y2="180" />
      </svg>
    `;

    const result = fix(svg);
    const points = connectorPoints(result.report, "flow");

    expect(isOrthogonal(points)).toBe(true);
    expect(bendCount(points)).toBe(1);
    expect(
      leavesPerpendicularly(points, nodeBounds(result.report, "source")),
    ).toBe(true);
    expect(
      entersPerpendicularly(points, nodeBounds(result.report, "target")),
    ).toBe(true);
    expect(result.report.issues).toEqual([]);

    expect(qualityOf(result.report)).toMatchSnapshot();
    expect(result.svg).toMatchSnapshot();
  });

  test("keeps connectors off crowded sides so every label finds a slot", () => {
    /**
     * Given eight nodes from a data flow diagram where the fewest-bend routes
     *   would bunch several connectors onto the same node sides, leaving one
     *   label no slot clear of them
     * When the diagram is fixed
     * Then the connectors spread over less crowded sides and every label sits
     *   clear of every connector
     */
    const svg = `
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 2306 1060">
        <g data-node="aml">
          <rect x="1258" y="40" width="196" height="60" rx="14"/>
          <text x="1356" y="66" font-size="13" text-anchor="middle">9 AML Screening</text>
          <text x="1356" y="85" font-size="11" text-anchor="middle">fuzzy name match</text>
        </g>
        <g data-node="tfront">
          <rect x="40" y="270" width="196" height="60" rx="14"/>
          <text x="138" y="296" font-size="13" text-anchor="middle">3 Teller Front</text>
          <text x="138" y="315" font-size="11" text-anchor="middle">VB6 · 2006</text>
        </g>
        <g data-node="domadp">
          <rect x="2070" y="270" width="196" height="60" rx="14"/>
          <text x="2168" y="296" font-size="13" text-anchor="middle">17 Zengin Adapter</text>
          <text x="2168" y="315" font-size="11" text-anchor="middle">C · 2006</text>
        </g>
        <g data-node="partneradp">
          <rect x="1664" y="730" width="196" height="60" rx="14"/>
          <text x="1762" y="756" font-size="13" text-anchor="middle">24 Partner Adapter</text>
          <text x="1762" y="775" font-size="11" text-anchor="middle">REST + SFTP</text>
        </g>
        <g data-node="regrep">
          <rect x="2070" y="730" width="196" height="60" rx="14"/>
          <text x="2168" y="756" font-size="13" text-anchor="middle">25 Reg Reporting</text>
          <text x="2168" y="775" font-size="11" text-anchor="middle">STR / CTR export</text>
        </g>
        <g data-node="ledger">
          <rect x="852" y="960" width="196" height="60"/>
          <text x="964" y="986" font-size="13" text-anchor="middle">D4 Transfer Ledger</text>
          <text x="964" y="1005" font-size="11" text-anchor="middle">Oracle · 2006</text>
        </g>
        <g data-node="mq">
          <rect x="1664" y="960" width="196" height="60"/>
          <text x="1776" y="986" font-size="13" text-anchor="middle">D6 Message Queue</text>
          <text x="1776" y="1005" font-size="11" text-anchor="middle">IBM MQ + Kafka</text>
        </g>
        <g data-node="audit">
          <rect x="2070" y="960" width="196" height="60"/>
          <text x="2182" y="986" font-size="13" text-anchor="middle">D7 Audit Log</text>
          <text x="2182" y="1005" font-size="11" text-anchor="middle">WORM storage</text>
        </g>
        <path id="f57" data-from="partneradp" data-to="mq" d="M 1762 790 L 1762 960"/>
        <path id="f58" data-from="domadp" data-to="mq" d="M 2168 330 L 1762 960"/>
        <path id="f70" data-from="ledger" data-to="regrep" d="M 1048 990 L 2070 760"/>
        <path id="f71" data-from="aml" data-to="regrep" d="M 1454 70 L 2070 760"/>
        <path id="f75" data-from="aml" data-to="audit" d="M 1356 100 L 2168 960"/>
        <path id="f76" data-from="tfront" data-to="audit" d="M 236 300 L 2070 990"/>
        <text data-label="l57" data-label-for="f57" x="1762" y="869" font-size="12" text-anchor="middle">status event</text>
        <text data-label="l58" data-label-for="f58" x="1965" y="639" font-size="12" text-anchor="middle">status event</text>
        <text data-label="l70" data-label-for="f70" x="1559" y="869" font-size="12" text-anchor="middle">transactions</text>
        <text data-label="l71" data-label-for="f71" x="1762" y="409" font-size="12" text-anchor="middle">STR candidates</text>
        <text data-label="l75" data-label-for="f75" x="1762" y="524" font-size="12" text-anchor="middle">screening log</text>
        <text data-label="l76" data-label-for="f76" x="1153" y="639" font-size="12" text-anchor="middle">teller log</text>
      </svg>
    `;

    const result = fix(svg);

    for (const connector of result.report.diagram.connectors) {
      expect(isOrthogonal(connector.points)).toBe(true);
    }
    expect(issueCodes(result.report)).not.toContain(
      "connector-label-clearance",
    );
    expect(result.report.issues).toEqual([]);

    expect(qualityOf(result.report)).toMatchSnapshot();
    expect(result.svg).toMatchSnapshot();
  });

  test("fixes again while another pass leaves fewer issues", () => {
    /**
     * Given nine nodes from a data flow diagram where one pass routes some
     *   connectors before the label they later pass too close to is placed
     * When the diagram is fixed
     * Then a further pass routes them clear of that label, so fewer issues
     *   remain than after a single pass, here none
     */
    const svg = `
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1494 1060">
        <g data-node="aml">
          <rect x="40" y="40" width="196" height="60" rx="14"/>
          <text x="138" y="66" font-size="13" text-anchor="middle">9 AML Screening</text>
          <text x="138" y="85" font-size="11" text-anchor="middle">fuzzy name match</text>
        </g>
        <g data-node="fx">
          <rect x="446" y="40" width="196" height="60" rx="14"/>
          <text x="544" y="66" font-size="13" text-anchor="middle">10 FX Quote</text>
          <text x="544" y="85" font-size="11" text-anchor="middle">spread by segment</text>
        </g>
        <g data-node="fee">
          <rect x="40" y="270" width="196" height="60" rx="14"/>
          <text x="138" y="296" font-size="13" text-anchor="middle">11 Fee Calc</text>
          <text x="138" y="315" font-size="11" text-anchor="middle">legacy + new, diffed</text>
        </g>
        <g data-node="route">
          <rect x="446" y="270" width="196" height="60" rx="14"/>
          <text x="544" y="296" font-size="13" text-anchor="middle">16 Routing Engine</text>
          <text x="544" y="315" font-size="11" text-anchor="middle">rules in DB + code</text>
        </g>
        <g data-node="domadp">
          <rect x="852" y="270" width="196" height="60" rx="14"/>
          <text x="950" y="296" font-size="13" text-anchor="middle">17 Zengin Adapter</text>
          <text x="950" y="315" font-size="11" text-anchor="middle">C · 2006</text>
        </g>
        <g data-node="mxconv">
          <rect x="446" y="500" width="196" height="60" rx="14"/>
          <text x="544" y="526" font-size="13" text-anchor="middle">20 MT103 to MX</text>
          <text x="544" y="545" font-size="11" text-anchor="middle">ISO 20022 · 2023</text>
        </g>
        <g data-node="partneradp">
          <rect x="446" y="730" width="196" height="60" rx="14"/>
          <text x="544" y="756" font-size="13" text-anchor="middle">24 Partner Adapter</text>
          <text x="544" y="775" font-size="11" text-anchor="middle">REST + SFTP</text>
        </g>
        <g data-node="partner">
          <rect x="1258" y="730" width="196" height="60"/>
          <text x="1356" y="756" font-size="13" text-anchor="middle">Partner Remitter</text>
          <text x="1356" y="775" font-size="11" text-anchor="middle">12 corridors</text>
        </g>
        <g data-node="ratefee">
          <rect x="40" y="960" width="196" height="60"/>
          <text x="152" y="986" font-size="13" text-anchor="middle">D5 Rate / Fee Tables</text>
          <text x="152" y="1005" font-size="11" text-anchor="middle">Redis + Oracle</text>
        </g>
        <path id="f35" data-from="aml" data-to="route" d="M 236 70 L 446 300"/>
        <path id="f37" data-from="fx" data-to="ratefee" d="M 544 100 L 138 960"/>
        <path id="f42" data-from="fx" data-to="fee" d="M 446 70 L 236 300"/>
        <path id="f44" data-from="fee" data-to="route" d="M 236 300 L 446 300"/>
        <path id="f46" data-from="route" data-to="domadp" d="M 642 300 L 852 300"/>
        <path id="f47" data-from="route" data-to="mxconv" d="M 544 330 L 544 500"/>
        <path id="f48" data-from="route" data-to="partneradp" d="M 544 330 L 544 730"/>
        <path id="f56" data-from="partner" data-to="partneradp" d="M 1258 760 L 642 760"/>
        <text data-label="l35" data-label-for="f35" x="341" y="179" font-size="12" text-anchor="middle">cleared order</text>
        <text data-label="l37" data-label-for="f37" x="341" y="524" font-size="12" text-anchor="middle">cached rate</text>
        <text data-label="l42" data-label-for="f42" x="341" y="179" font-size="12" text-anchor="middle">quote</text>
        <text data-label="l44" data-label-for="f44" x="341" y="294" font-size="12" text-anchor="middle">priced order</text>
        <text data-label="l46" data-label-for="f46" x="747" y="294" font-size="12" text-anchor="middle">domestic</text>
        <text data-label="l47" data-label-for="f47" x="544" y="409" font-size="12" text-anchor="middle">MT103</text>
        <text data-label="l48" data-label-for="f48" x="544" y="524" font-size="12" text-anchor="middle">corridor</text>
        <text data-label="l56" data-label-for="f56" x="950" y="754" font-size="12" text-anchor="middle">payout confirm</text>
      </svg>
    `;

    const single = fix(svg, 1);
    const result = fix(svg);

    expect(single.report.issues.length).toBeGreaterThan(0);
    expect(result.report.issues.length).toBeLessThan(
      single.report.issues.length,
    );
    expect(result.report.issues).toEqual([]);

    expect(qualityOf(result.report)).toMatchSnapshot();
    expect(result.svg).toMatchSnapshot();
  });

  test("moves a connector's label beside its rerouted path", () => {
    /**
     * Given a label tied to a connector by data-label-for, left at the old
     *   midpoint after the connector has to detour
     * When the diagram is fixed
     * Then the label sits centred above the detour's longest segment
     */
    const result = fix(
      blockedRow(`<text data-label="retry" data-label-for="flow"
        x="190" y="42" text-anchor="middle" font-size="14">Retry</text>`),
    );
    const label = labelBounds(result.report, "retry");
    const [from, to] = longestSegment(connectorPoints(result.report, "flow"));

    expect(from.y).toBe(to.y);
    expect(label.x + label.width / 2).toBeCloseTo((from.x + to.x) / 2, 0);
    expect(label.y + label.height).toBeLessThanOrEqual(from.y - 8);
    expect(result.changes.map((change) => change.code)).toContain("move-label");
    expect(result.report.issues).toEqual([]);

    expect(qualityOf(result.report)).toMatchSnapshot();
    expect(result.svg).toMatchSnapshot();
  });

  test("brings a detached label back beside its connector", () => {
    /**
     * Given a straight connector whose tied label floats 45px above it
     * When the diagram is fixed
     * Then the label sits beside the connector and nothing is reported
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
          x1="120" y1="48" x2="260" y2="48" />
        <text data-label="retry" data-label-for="flow"
          x="190" y="0" text-anchor="middle" font-size="14">Retry</text>
      </svg>
    `;

    const result = fix(svg);

    expect(
      distanceToRoute(
        labelBounds(result.report, "retry"),
        connectorPoints(result.report, "flow"),
      ),
    ).toBeLessThanOrEqual(16);
    expect(result.report.issues).toEqual([]);

    expect(qualityOf(result.report)).toMatchSnapshot();
    expect(result.svg).toMatchSnapshot();
  });

  test("moves only a detached label, leaving its sound connector as drawn", () => {
    /**
     * Given a connector drawn as a sound two-bend path whose tied label
     *   floats far from it, with nothing else wrong
     * When the diagram is fixed
     * Then the connector keeps its path and only the label moves beside it
     */
    const svg = `
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="-100 -100 600 400">
        <g data-node="source">
          <rect x="20" y="20" width="100" height="56" />
          <text x="70" y="53" text-anchor="middle" font-size="14">Source</text>
        </g>
        <g data-node="target">
          <rect x="260" y="160" width="100" height="56" />
          <text x="310" y="193" text-anchor="middle" font-size="14">Target</text>
        </g>
        <path id="flow" data-from="source" data-to="target"
          d="M 120 48 L 190 48 L 190 188 L 260 188" />
        <text data-label="retry" data-label-for="flow"
          x="310" y="0" text-anchor="middle" font-size="14">Retry</text>
      </svg>
    `;

    const result = fix(svg);

    expect(connectorPoints(result.report, "flow")).toEqual(
      connectorPoints(analyze(svg), "flow"),
    );
    expect(
      distanceToRoute(
        labelBounds(result.report, "retry"),
        connectorPoints(result.report, "flow"),
      ),
    ).toBeLessThanOrEqual(16);
    expect(result.changes.map((change) => change.code)).toEqual(["move-label"]);
    expect(result.report.issues).toEqual([]);

    expect(qualityOf(result.report)).toMatchSnapshot();
    expect(result.svg).toMatchSnapshot();
  });

  test("reroutes a connector when its detached label has no room beside it", () => {
    /**
     * Given a connector drawn as a staircase of segments all shorter than its
     *   tied label, which floats far away
     * When the diagram is fixed
     * Then the connector is rerouted so the label can sit beside it, and
     *   nothing is reported
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
        <path id="flow" data-from="source" data-to="target"
          d="M 120 48 L 150 48 L 150 60 L 180 60 L 180 36 L 210 36 L 210 60 L 240 60 L 240 48 L 260 48" />
        <text data-label="retry" data-label-for="flow"
          x="190" y="-40" text-anchor="middle" font-size="14">Retry</text>
      </svg>
    `;

    const result = fix(svg);
    const points = connectorPoints(result.report, "flow");

    expect(isOrthogonal(points)).toBe(true);
    expect(
      distanceToRoute(labelBounds(result.report, "retry"), points),
    ).toBeLessThanOrEqual(16);
    expect(result.report.issues).toEqual([]);

    expect(qualityOf(result.report)).toMatchSnapshot();
    expect(result.svg).toMatchSnapshot();
  });

  test("keeps a moved label clear of other labels", () => {
    /**
     * Given a free note sitting where the moved label would go first
     * When the diagram is fixed
     * Then the label moves along the detour, at least 4px from the note,
     *   and no issue remains
     */
    const result = fix(
      blockedRow(`<text data-label="note" x="190" y="0" text-anchor="middle"
          font-size="14">Note</text>
        <text data-label="retry" data-label-for="flow"
          x="190" y="42" text-anchor="middle" font-size="14">Retry</text>`),
    );

    expect(labelBounds(result.report, "note")).toEqual({
      x: 173.5,
      y: -14,
      width: 33,
      height: 17,
    });
    expect(
      gapBetween(
        labelBounds(result.report, "retry"),
        labelBounds(result.report, "note"),
      ),
    ).toBeGreaterThanOrEqual(4);
    expect(result.report.issues).toEqual([]);

    expect(qualityOf(result.report)).toMatchSnapshot();
    expect(result.svg).toMatchSnapshot();
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
    const bus = connectorPoints(result.report, "bus");
    const flow = connectorPoints(result.report, "flow");

    expect(bus).toEqual([
      { x: 0, y: 12 },
      { x: 400, y: 12 },
    ]);
    expect(isOrthogonal(flow)).toBe(true);
    expect(overlapsRoute(flow, bus)).toBe(false);
    expect(result.report.issues).toEqual([]);

    expect(qualityOf(result.report)).toMatchSnapshot();
    expect(result.svg).toMatchSnapshot();
  });

  test("does not reroute onto a short segment between grid lines", () => {
    /**
     * Given a short connector segment on the detour's shortest leg, lying
     *   between two neighbouring grid lines
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
          d="M 0 -40 L 195 -40 L 195 12 L 200 12" />
        <line id="flow" data-from="source" data-to="target"
          x1="120" y1="48" x2="260" y2="48" />
      </svg>
    `;

    const result = fix(svg);
    const flow = connectorPoints(result.report, "flow");

    expect(isOrthogonal(flow)).toBe(true);
    expect(overlapsRoute(flow, connectorPoints(result.report, "tick"))).toBe(
      false,
    );
    expect(issueCodes(result.report)).not.toContain("connector-overlap");

    expect(qualityOf(result.report)).toMatchSnapshot();
    expect(result.svg).toMatchSnapshot();
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
    const flow = connectorPoints(result.report, "flow");

    expect(isOrthogonal(flow)).toBe(true);
    expect(entersBox(flow, nodeBounds(result.report, "obstacle"))).toBe(false);
    expect(result.report.issues).toEqual([]);

    expect(qualityOf(result.report)).toMatchSnapshot();
    expect(result.svg).toMatchSnapshot();
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
    const request = connectorPoints(result.report, "request");
    const reply = connectorPoints(result.report, "reply");
    const client = nodeBounds(result.report, "client");
    const server = nodeBounds(result.report, "server");

    expect(overlapsRoute(request, reply)).toBe(false);
    for (const [route, from, to] of [
      [request, client, server],
      [reply, server, client],
    ] as const) {
      expect(isOrthogonal(route)).toBe(true);
      expect(leavesPerpendicularly(route, from)).toBe(true);
      expect(entersPerpendicularly(route, to)).toBe(true);
    }
    expect(result.report.issues).toEqual([]);

    expect(qualityOf(result.report)).toMatchSnapshot();
    expect(result.svg).toMatchSnapshot();
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
    const points = connectorPoints(result.report, "flow");

    expect(isOrthogonal(points)).toBe(true);
    expect(
      leavesPerpendicularly(points, nodeBounds(result.report, "n20")),
    ).toBe(true);
    expect(
      entersPerpendicularly(points, nodeBounds(result.report, "n02")),
    ).toBe(true);
    expect(issueCodes(result.report)).not.toContain("connector-node-crossing");

    expect(qualityOf(result.report)).toMatchSnapshot();
    expect(result.svg).toMatchSnapshot();
  });
});
