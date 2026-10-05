import type { AnalysisReport, Bounds, Point } from "../../src/index";

// Geometry predicates for asserting what a fixed diagram means rather than
// which exact coordinates it ended up with.

export type Side = "top" | "right" | "bottom" | "left";

export const connectorPoints = (report: AnalysisReport, id: string): Point[] =>
  report.diagram.connectors.find((connector) => connector.id === id)?.points ??
  [];

export const nodeBounds = (report: AnalysisReport, id: string): Bounds => {
  const node = report.diagram.nodes.find((item) => item.id === id);
  if (node === undefined) throw new Error(`No node "${id}" in the report.`);
  return node.bounds;
};

export const labelBounds = (report: AnalysisReport, id: string): Bounds => {
  const label = report.diagram.labels.find((item) => item.id === id);
  if (label === undefined) throw new Error(`No label "${id}" in the report.`);
  return label.bounds;
};

export const issueCodes = (report: AnalysisReport): string[] =>
  report.issues.map((issue) => issue.code);

export const segments = (points: Point[]): Array<[Point, Point]> =>
  points.slice(1).flatMap((point, index): Array<[Point, Point]> => {
    const previous = points[index];
    return previous === undefined ? [] : [[previous, point]];
  });

export const isOrthogonal = (points: Point[]): boolean =>
  points.length >= 2 &&
  segments(points).every(([from, to]) => from.x === to.x || from.y === to.y);

export const bendCount = (points: Point[]): number =>
  Math.max(0, points.length - 2);

export const sideOf = (point: Point, box: Bounds): Side | undefined => {
  const withinX = point.x >= box.x && point.x <= box.x + box.width;
  const withinY = point.y >= box.y && point.y <= box.y + box.height;
  if (withinX && point.y === box.y) return "top";
  if (withinX && point.y === box.y + box.height) return "bottom";
  if (withinY && point.x === box.x) return "left";
  if (withinY && point.x === box.x + box.width) return "right";
  return undefined;
};

// The segment from `port` towards `next` heads straight out of the node side
// the port sits on.
const headsOutward = (
  port: Point | undefined,
  next: Point | undefined,
  box: Bounds,
): boolean => {
  if (port === undefined || next === undefined) return false;
  switch (sideOf(port, box)) {
    case "top":
      return next.x === port.x && next.y < port.y;
    case "bottom":
      return next.x === port.x && next.y > port.y;
    case "left":
      return next.y === port.y && next.x < port.x;
    case "right":
      return next.y === port.y && next.x > port.x;
    default:
      return false;
  }
};

export const leavesPerpendicularly = (points: Point[], box: Bounds): boolean =>
  headsOutward(points.at(0), points.at(1), box);

export const entersPerpendicularly = (points: Point[], box: Bounds): boolean =>
  headsOutward(points.at(-1), points.at(-2), box);

const sharedLength = (a1: number, a2: number, b1: number, b2: number) =>
  Math.min(Math.max(a1, a2), Math.max(b1, b2)) -
  Math.max(Math.min(a1, a2), Math.min(b1, b2));

const collinearOverlap = ([a, b]: [Point, Point], [c, d]: [Point, Point]) =>
  (a.y === b.y &&
    c.y === d.y &&
    a.y === c.y &&
    sharedLength(a.x, b.x, c.x, d.x) > 0) ||
  (a.x === b.x &&
    c.x === d.x &&
    a.x === c.x &&
    sharedLength(a.y, b.y, c.y, d.y) > 0);

const outline = (box: Bounds): Array<[Point, Point]> => {
  const right = box.x + box.width;
  const bottom = box.y + box.height;
  return [
    [
      { x: box.x, y: box.y },
      { x: right, y: box.y },
    ],
    [
      { x: right, y: box.y },
      { x: right, y: bottom },
    ],
    [
      { x: box.x, y: bottom },
      { x: right, y: bottom },
    ],
    [
      { x: box.x, y: box.y },
      { x: box.x, y: bottom },
    ],
  ];
};

export const runsAlong = (points: Point[], box: Bounds): boolean =>
  segments(points).some((segment) =>
    outline(box).some((side) => collinearOverlap(segment, side)),
  );

export const overlapsRoute = (first: Point[], second: Point[]): boolean =>
  segments(first).some((segment) =>
    segments(second).some((other) => collinearOverlap(segment, other)),
  );

export const inflate = (box: Bounds, amount: number): Bounds => ({
  x: box.x - amount,
  y: box.y - amount,
  width: box.width + amount * 2,
  height: box.height + amount * 2,
});

// Whether any segment passes through the open interior of the box.
export const entersBox = (points: Point[], box: Bounds): boolean =>
  segments(points).some(([from, to]) => {
    const left = Math.min(from.x, to.x);
    const right = Math.max(from.x, to.x);
    const top = Math.min(from.y, to.y);
    const bottom = Math.max(from.y, to.y);
    return (
      right > box.x &&
      left < box.x + box.width &&
      bottom > box.y &&
      top < box.y + box.height
    );
  });

export const contains = (outer: Bounds, inner: Bounds): boolean =>
  inner.x >= outer.x &&
  inner.y >= outer.y &&
  inner.x + inner.width <= outer.x + outer.width &&
  inner.y + inner.height <= outer.y + outer.height;

// Empty space between two boxes along the axis that separates them.
export const gapBetween = (first: Bounds, second: Bounds): number =>
  Math.max(
    second.x - (first.x + first.width),
    first.x - (second.x + second.width),
    second.y - (first.y + first.height),
    first.y - (second.y + second.height),
  );

export const segmentLength = ([from, to]: [Point, Point]): number =>
  Math.abs(to.x - from.x) + Math.abs(to.y - from.y);

export const longestSegment = (points: Point[]): [Point, Point] => {
  const [first, ...rest] = segments(points);
  if (first === undefined) throw new Error("A route needs two points.");
  return rest.reduce(
    (longest, segment) =>
      segmentLength(segment) > segmentLength(longest) ? segment : longest,
    first,
  );
};

// Shortest distance between a box and any segment of an orthogonal route.
export const distanceToRoute = (box: Bounds, points: Point[]): number =>
  Math.min(
    ...segments(points).map(([from, to]) => {
      const dx = Math.max(
        0,
        box.x - Math.max(from.x, to.x),
        Math.min(from.x, to.x) - (box.x + box.width),
      );
      const dy = Math.max(
        0,
        box.y - Math.max(from.y, to.y),
        Math.min(from.y, to.y) - (box.y + box.height),
      );
      return Math.hypot(dx, dy);
    }),
  );

// A label tied to a connector reads as belonging to it only when it sits
// right beside it: placed labels keep 9px, so beyond 16px it is detached.
const detachedDistance = 16;

// Quality of a fixed diagram, for snapshot assertions that catch regressions
// the per-test expectations do not pin down.
export const qualityOf = (report: AnalysisReport) => ({
  remainingIssues: report.issues.length,
  bends: report.diagram.connectors.reduce(
    (total, connector) => total + bendCount(connector.points),
    0,
  ),
  diagonalSegments: report.diagram.connectors.reduce(
    (total, connector) =>
      total +
      segments(connector.points).filter(
        ([from, to]) => from.x !== to.x && from.y !== to.y,
      ).length,
    0,
  ),
  detachedLabels: report.diagram.labels.filter((label) => {
    const connector = report.diagram.connectors.find(
      (item) => item.id === label.connector,
    );
    return (
      connector !== undefined &&
      distanceToRoute(label.bounds, connector.points) > detachedDistance
    );
  }).length,
});

// Distance of a point from the circle inscribed in a circular node's bounds.
export const distanceFromCircle = (point: Point, box: Bounds): number =>
  Math.abs(
    Math.hypot(
      point.x - (box.x + box.width / 2),
      point.y - (box.y + box.height / 2),
    ) -
      box.width / 2,
  );
