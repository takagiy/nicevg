import { DOMParser, XMLSerializer } from "@xmldom/xmldom";
import { SVGPathData } from "svg-pathdata";

export interface Bounds {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface DiagramNode {
  id: string;
  bounds: Bounds;
  labelBounds: Bounds[];
  parentId?: string;
  allowOverlap?: boolean;
}

export interface Point {
  x: number;
  y: number;
}

export interface DiagramConnector {
  id: string;
  from: string;
  to: string;
  points: Point[];
}

export interface DiagramLabel {
  id: string;
  bounds: Bounds;
  connector?: string;
}

export interface UnsupportedElement {
  id: string;
  tagName: string;
}

export interface Diagram {
  nodes: DiagramNode[];
  connectors: DiagramConnector[];
  labels: DiagramLabel[];
  unsupportedElements: UnsupportedElement[];
}

export interface DiagramIssue {
  code: string;
  message: string;
  elements: string[];
  details?: Record<string, unknown>;
}

export interface AnalysisReport {
  valid: boolean;
  issues: DiagramIssue[];
  drawingBounds: Bounds | null;
  diagram: Diagram;
}

export interface FixChange {
  code: string;
  message: string;
  elements: string[];
}

export interface FixResult {
  svg: string;
  changes: FixChange[];
  report: AnalysisReport;
}

export class SvgInputError extends Error {
  override name = "SvgInputError";
}

const parseSvg = (svg: string): Document => {
  const errors: string[] = [];
  const document = new DOMParser({
    errorHandler: {
      warning: (message: unknown) => errors.push(String(message)),
      error: (message: unknown) => errors.push(String(message)),
      fatalError: (message: unknown) => errors.push(String(message)),
    },
  }).parseFromString(svg, "image/svg+xml");
  if (
    errors.length > 0 ||
    document.documentElement.tagName.toLowerCase() !== "svg"
  ) {
    throw new SvgInputError(
      errors[0] ?? "Input root element must be an SVG element.",
    );
  }
  return document;
};

const numberAttribute = (element: Element, name: string): number => {
  const value = Number(element.getAttribute(name));
  return Number.isFinite(value) ? value : 0;
};

const elementTranslation = (element: Element): Point => {
  let x = 0;
  let y = 0;
  let current: Node | null = element;
  const numberPattern = String.raw`[-+]?(?:\d*\.?\d+)(?:[eE][-+]?\d+)?`;
  const translatePattern = new RegExp(
    String.raw`translate\(\s*(${numberPattern})(?:[\s,]+(${numberPattern}))?\s*\)`,
    "g",
  );
  while (current !== null) {
    if (current.nodeType === 1) {
      const transform = (current as Element).getAttribute("transform") ?? "";
      for (const match of transform.matchAll(translatePattern)) {
        x += Number(match[1]);
        y += Number(match[2] ?? 0);
      }
    }
    current = current.parentNode;
  }
  return { x, y };
};

const rectBounds = (element: Element): Bounds => {
  const translation = elementTranslation(element);
  return {
    x: numberAttribute(element, "x") + translation.x,
    y: numberAttribute(element, "y") + translation.y,
    width: numberAttribute(element, "width"),
    height: numberAttribute(element, "height"),
  };
};

const localRectBounds = (element: Element): Bounds => ({
  x: numberAttribute(element, "x"),
  y: numberAttribute(element, "y"),
  width: numberAttribute(element, "width"),
  height: numberAttribute(element, "height"),
});

// Estimates the box of a text run from its anchor point; glyph metrics are
// approximated by an average advance of 0.59em.
const measureText = (
  length: number,
  fontSize: number,
  anchorPoint: Point,
  anchor: string | null,
): Bounds => {
  const width = Math.round(length * fontSize * 0.59);
  const x =
    anchor === "middle"
      ? anchorPoint.x - width / 2
      : anchor === "end"
        ? anchorPoint.x - width
        : anchorPoint.x;

  return {
    x,
    y: anchorPoint.y - fontSize,
    width,
    height: Math.round(fontSize * 1.2),
  };
};

const textBounds = (element: Element): Bounds => {
  const translation = elementTranslation(element);
  return measureText(
    element.textContent.length,
    numberAttribute(element, "font-size") || 16,
    {
      x: numberAttribute(element, "x") + translation.x,
      y: numberAttribute(element, "y") + translation.y,
    },
    element.getAttribute("text-anchor"),
  );
};

const connectorPoints = (element: Element): Point[] => {
  const translation = elementTranslation(element);
  if (element.tagName === "line") {
    return [
      {
        x: numberAttribute(element, "x1") + translation.x,
        y: numberAttribute(element, "y1") + translation.y,
      },
      {
        x: numberAttribute(element, "x2") + translation.x,
        y: numberAttribute(element, "y2") + translation.y,
      },
    ];
  }
  if (element.tagName === "polyline") {
    const values =
      element
        .getAttribute("points")
        ?.trim()
        .split(/[\s,]+/)
        .map(Number) ?? [];
    const points: Point[] = [];
    for (let index = 0; index + 1 < values.length; index += 2) {
      const x = values[index];
      const y = values[index + 1];
      if (x !== undefined && y !== undefined) {
        points.push({ x: x + translation.x, y: y + translation.y });
      }
    }
    return points;
  }
  if (element.tagName === "path") {
    try {
      return new SVGPathData(element.getAttribute("d") ?? "")
        .toAbs()
        .normalizeHVZ()
        .commands.flatMap((command): Point[] =>
          "x" in command && "y" in command
            ? [
                {
                  x: command.x + translation.x,
                  y: command.y + translation.y,
                },
              ]
            : [],
        );
    } catch {
      return [];
    }
  }
  return [];
};

const enclosingBounds = (bounds: Bounds[]): Bounds | null => {
  if (bounds.length === 0) {
    return null;
  }

  const left = Math.min(...bounds.map((item) => item.x));
  const top = Math.min(...bounds.map((item) => item.y));
  const right = Math.max(...bounds.map((item) => item.x + item.width));
  const bottom = Math.max(...bounds.map((item) => item.y + item.height));
  return { x: left, y: top, width: right - left, height: bottom - top };
};

const pointsBounds = (points: Point[]): Bounds | null => {
  if (points.length === 0) return null;
  const xValues = points.map((point) => point.x);
  const yValues = points.map((point) => point.y);
  const left = Math.min(...xValues);
  const top = Math.min(...yValues);
  return {
    x: left,
    y: top,
    width: Math.max(...xValues) - left,
    height: Math.max(...yValues) - top,
  };
};

const parseViewBox = (root: Element): Bounds | null => {
  const values =
    root
      .getAttribute("viewBox")
      ?.trim()
      .split(/[\s,]+/)
      .map(Number) ?? [];
  if (values.length !== 4 || values.some((value) => !Number.isFinite(value))) {
    return null;
  }

  const [x, y, width, height] = values;
  if (
    x === undefined ||
    y === undefined ||
    width === undefined ||
    height === undefined
  ) {
    return null;
  }
  return { x, y, width, height };
};

const inspectViewport = (
  root: Element,
  drawingBounds: Bounds | null,
  padding = 20,
): DiagramIssue[] => {
  const viewBox = parseViewBox(root);
  if (viewBox === null || drawingBounds === null) {
    return [];
  }

  const requiredViewBox = {
    x: drawingBounds.x - padding,
    y: drawingBounds.y - padding,
    width: drawingBounds.width + padding * 2,
    height: drawingBounds.height + padding * 2,
  };
  const sides: string[] = [];
  if (requiredViewBox.x < viewBox.x) sides.push("left");
  if (requiredViewBox.y < viewBox.y) sides.push("top");
  if (requiredViewBox.x + requiredViewBox.width > viewBox.x + viewBox.width) {
    sides.push("right");
  }
  if (requiredViewBox.y + requiredViewBox.height > viewBox.y + viewBox.height) {
    sides.push("bottom");
  }
  if (sides.length === 0) {
    return [];
  }

  return [
    {
      code: "viewport-clipping",
      message: `Drawing exceeds the safe viewBox on the ${sides.join(", ")} side.`,
      elements: ["svg"],
      details: { sides, requiredViewBox },
    },
  ];
};

const inspectTextFit = (nodes: DiagramNode[], padding = 12): DiagramIssue[] =>
  nodes.flatMap((node): DiagramIssue[] => {
    const labelBounds = enclosingBounds(node.labelBounds);
    if (labelBounds === null) {
      return [];
    }

    const fits =
      labelBounds.x >= node.bounds.x + padding &&
      labelBounds.y >= node.bounds.y + padding &&
      labelBounds.x + labelBounds.width <=
        node.bounds.x + node.bounds.width - padding &&
      labelBounds.y + labelBounds.height <=
        node.bounds.y + node.bounds.height - padding;
    if (fits) {
      return [];
    }

    return [
      {
        code: "text-overflow",
        message: `Label does not fit inside node "${node.id}" with ${padding}px padding.`,
        elements: [node.id],
        details: {
          requiredWidth: labelBounds.width + padding * 2,
          requiredHeight: labelBounds.height + padding * 2,
        },
      },
    ];
  });

const intersection = (first: Bounds, second: Bounds): Bounds | null => {
  const left = Math.max(first.x, second.x);
  const top = Math.max(first.y, second.y);
  const right = Math.min(first.x + first.width, second.x + second.width);
  const bottom = Math.min(first.y + first.height, second.y + second.height);
  if (right <= left || bottom <= top) {
    return null;
  }
  return { x: left, y: top, width: right - left, height: bottom - top };
};

const inspectNodeOverlaps = (nodes: DiagramNode[]): DiagramIssue[] => {
  const issues: DiagramIssue[] = [];
  const nodesById = new Map(nodes.map((node) => [node.id, node]));
  const isAncestor = (ancestor: DiagramNode, descendant: DiagramNode) => {
    let parentId = descendant.parentId;
    while (parentId !== undefined) {
      if (parentId === ancestor.id) return true;
      parentId = nodesById.get(parentId)?.parentId;
    }
    return false;
  };
  for (let firstIndex = 0; firstIndex < nodes.length; firstIndex += 1) {
    const first = nodes[firstIndex];
    if (first === undefined) continue;
    for (
      let secondIndex = firstIndex + 1;
      secondIndex < nodes.length;
      secondIndex += 1
    ) {
      const second = nodes[secondIndex];
      if (second === undefined) continue;
      if (first.allowOverlap === true || second.allowOverlap === true) continue;
      if (isAncestor(first, second) || isAncestor(second, first)) continue;
      if (first.parentId !== second.parentId) continue;
      const overlap = intersection(first.bounds, second.bounds);
      if (overlap === null) continue;
      issues.push({
        code: "node-overlap",
        message: `Nodes "${first.id}" and "${second.id}" overlap.`,
        elements: [first.id, second.id],
        details: { intersection: overlap },
      });
    }
  }
  return issues;
};

const inspectNodeGaps = (
  nodes: DiagramNode[],
  requiredGap = 20,
): DiagramIssue[] => {
  const issues: DiagramIssue[] = [];
  const parentOf = new Map(nodes.map((node) => [node.id, node.parentId]));
  const isRelated = (first: DiagramNode, second: DiagramNode) => {
    let parentId = parentOf.get(first.id);
    while (parentId !== undefined) {
      if (parentId === second.id) return true;
      parentId = parentOf.get(parentId);
    }
    parentId = parentOf.get(second.id);
    while (parentId !== undefined) {
      if (parentId === first.id) return true;
      parentId = parentOf.get(parentId);
    }
    return false;
  };
  for (let firstIndex = 0; firstIndex < nodes.length; firstIndex += 1) {
    const first = nodes[firstIndex];
    if (first === undefined) continue;
    for (
      let secondIndex = firstIndex + 1;
      secondIndex < nodes.length;
      secondIndex += 1
    ) {
      const second = nodes[secondIndex];
      if (second === undefined) continue;
      if (first.allowOverlap === true || second.allowOverlap === true) continue;
      if (isRelated(first, second)) continue;
      if (first.parentId !== second.parentId) continue;
      const verticalOverlap =
        Math.min(
          first.bounds.y + first.bounds.height,
          second.bounds.y + second.bounds.height,
        ) - Math.max(first.bounds.y, second.bounds.y);
      if (verticalOverlap <= 0) continue;

      const [left, right] =
        first.bounds.x <= second.bounds.x ? [first, second] : [second, first];
      const actualGap = right.bounds.x - (left.bounds.x + left.bounds.width);
      if (actualGap < 0 || actualGap >= requiredGap) continue;
      issues.push({
        code: "node-gap",
        message: `Nodes "${left.id}" and "${right.id}" have a ${actualGap}px gap; ${requiredGap}px is required.`,
        elements: [left.id, right.id],
        details: {
          actualGap,
          requiredGap,
          shortage: requiredGap - actualGap,
        },
      });
    }
  }
  return issues;
};

const segmentIntersectsInterior = (
  start: Point,
  end: Point,
  bounds: Bounds,
): boolean => {
  const epsilon = 0.001;
  const left = bounds.x + epsilon;
  const right = bounds.x + bounds.width - epsilon;
  const top = bounds.y + epsilon;
  const bottom = bounds.y + bounds.height - epsilon;
  const dx = end.x - start.x;
  const dy = end.y - start.y;
  const checks: Array<[number, number]> = [
    [-dx, start.x - left],
    [dx, right - start.x],
    [-dy, start.y - top],
    [dy, bottom - start.y],
  ];
  let entering = 0;
  let leaving = 1;
  for (const [direction, distance] of checks) {
    if (direction === 0) {
      if (distance < 0) return false;
      continue;
    }
    const ratio = distance / direction;
    if (direction < 0) {
      entering = Math.max(entering, ratio);
    } else {
      leaving = Math.min(leaving, ratio);
    }
    if (entering > leaving) return false;
  }
  return entering <= 1 && leaving >= 0;
};

const inspectConnectorCrossings = (
  nodes: DiagramNode[],
  connectors: DiagramConnector[],
): DiagramIssue[] =>
  connectors.flatMap((connector): DiagramIssue[] =>
    nodes.flatMap((node): DiagramIssue[] => {
      if (node.id === connector.from || node.id === connector.to) {
        return [];
      }
      const crosses = connector.points.slice(0, -1).some((point, index) => {
        const next = connector.points[index + 1];
        return (
          next !== undefined &&
          segmentIntersectsInterior(point, next, node.bounds)
        );
      });
      if (!crosses) {
        return [];
      }
      return [
        {
          code: "connector-node-crossing",
          message: `Connector "${connector.id}" crosses unrelated node "${node.id}".`,
          elements: [connector.id, node.id],
        },
      ];
    }),
  );

const inspectConnectorLabelClearance = (
  connectors: DiagramConnector[],
  labels: DiagramLabel[],
  clearance = 8,
): DiagramIssue[] =>
  connectors.flatMap((connector): DiagramIssue[] =>
    labels.flatMap((label): DiagramIssue[] => {
      const expanded = {
        x: label.bounds.x - clearance,
        y: label.bounds.y - clearance,
        width: label.bounds.width + clearance * 2,
        height: label.bounds.height + clearance * 2,
      };
      const isTooClose = connector.points.slice(0, -1).some((point, index) => {
        const next = connector.points[index + 1];
        return (
          next !== undefined && segmentIntersectsInterior(point, next, expanded)
        );
      });
      if (!isTooClose) {
        return [];
      }
      return [
        {
          code: "connector-label-clearance",
          message: `Connector "${connector.id}" passes within ${clearance}px of label "${label.id}".`,
          elements: [connector.id, label.id],
          details: { requiredClearance: clearance },
        },
      ];
    }),
  );

const inspectLabelOverlaps = (labels: DiagramLabel[]): DiagramIssue[] => {
  const issues: DiagramIssue[] = [];
  for (let firstIndex = 0; firstIndex < labels.length; firstIndex += 1) {
    const first = labels[firstIndex];
    if (first === undefined) continue;
    for (
      let secondIndex = firstIndex + 1;
      secondIndex < labels.length;
      secondIndex += 1
    ) {
      const second = labels[secondIndex];
      if (
        second !== undefined &&
        intersection(first.bounds, second.bounds) !== null
      ) {
        issues.push({
          code: "label-overlap",
          message: `Labels "${first.id}" and "${second.id}" overlap.`,
          elements: [first.id, second.id],
        });
      }
    }
  }
  return issues;
};

const segmentsOverlap = (
  [firstStart, firstEnd]: [Point, Point],
  [secondStart, secondEnd]: [Point, Point],
): boolean => {
  const shared = (a1: number, a2: number, b1: number, b2: number) =>
    Math.min(Math.max(a1, a2), Math.max(b1, b2)) -
      Math.max(Math.min(a1, a2), Math.min(b1, b2)) >
    0;
  const horizontal = (start: Point, end: Point) => start.y === end.y;
  const vertical = (start: Point, end: Point) => start.x === end.x;
  if (
    horizontal(firstStart, firstEnd) &&
    horizontal(secondStart, secondEnd) &&
    firstStart.y === secondStart.y
  ) {
    return shared(firstStart.x, firstEnd.x, secondStart.x, secondEnd.x);
  }
  if (
    vertical(firstStart, firstEnd) &&
    vertical(secondStart, secondEnd) &&
    firstStart.x === secondStart.x
  ) {
    return shared(firstStart.y, firstEnd.y, secondStart.y, secondEnd.y);
  }
  return false;
};

const connectorSegments = (points: Point[]): Array<[Point, Point]> =>
  points.slice(0, -1).flatMap((point, index): Array<[Point, Point]> => {
    const next = points[index + 1];
    return next === undefined ? [] : [[point, next]];
  });

const routesOverlap = (first: Point[], second: Point[]): boolean =>
  connectorSegments(first).some((segment) =>
    connectorSegments(second).some((other) => segmentsOverlap(segment, other)),
  );

const inspectConnectorOverlaps = (
  connectors: DiagramConnector[],
): DiagramIssue[] =>
  connectors.flatMap((first, index): DiagramIssue[] =>
    connectors
      .slice(index + 1)
      .filter((second) => routesOverlap(first.points, second.points))
      .map((second) => ({
        code: "connector-overlap",
        message: `Connectors "${first.id}" and "${second.id}" overlap along a segment.`,
        elements: [first.id, second.id],
      })),
  );

const pointIsInside = (point: Point, bounds: Bounds): boolean =>
  point.x > bounds.x &&
  point.x < bounds.x + bounds.width &&
  point.y > bounds.y &&
  point.y < bounds.y + bounds.height;

const inspectConnectorEndpoints = (
  nodes: DiagramNode[],
  connectors: DiagramConnector[],
): DiagramIssue[] => {
  const nodesById = new Map(nodes.map((node) => [node.id, node]));
  return connectors.flatMap((connector): DiagramIssue[] => {
    const issues: DiagramIssue[] = [];
    const start = connector.points[0];
    const end = connector.points.at(-1);
    const source = nodesById.get(connector.from);
    const target = nodesById.get(connector.to);
    if (
      start !== undefined &&
      source !== undefined &&
      pointIsInside(start, source.bounds)
    ) {
      issues.push({
        code: "connector-endpoint-inside",
        message: `Connector "${connector.id}" starts inside node "${source.id}".`,
        elements: [connector.id, source.id],
        details: { endpoint: "start" },
      });
    }
    if (
      end !== undefined &&
      target !== undefined &&
      pointIsInside(end, target.bounds)
    ) {
      issues.push({
        code: "connector-endpoint-inside",
        message: `Connector "${connector.id}" ends inside node "${target.id}".`,
        elements: [connector.id, target.id],
        details: { endpoint: "end" },
      });
    }
    return issues;
  });
};

const connectorPorts = (
  source: Bounds,
  target: Bounds,
): { start: Point; end: Point; horizontal: boolean } => {
  const sourceCenter = {
    x: source.x + source.width / 2,
    y: source.y + source.height / 2,
  };
  const targetCenter = {
    x: target.x + target.width / 2,
    y: target.y + target.height / 2,
  };
  const horizontal =
    Math.abs(targetCenter.x - sourceCenter.x) >=
    Math.abs(targetCenter.y - sourceCenter.y);
  if (horizontal) {
    const movesRight = targetCenter.x >= sourceCenter.x;
    return {
      start: {
        x: movesRight ? source.x + source.width : source.x,
        y: sourceCenter.y,
      },
      end: {
        x: movesRight ? target.x : target.x + target.width,
        y: targetCenter.y,
      },
      horizontal,
    };
  }
  const movesDown = targetCenter.y >= sourceCenter.y;
  return {
    start: {
      x: sourceCenter.x,
      y: movesDown ? source.y + source.height : source.y,
    },
    end: {
      x: targetCenter.x,
      y: movesDown ? target.y : target.y + target.height,
    },
    horizontal,
  };
};

const inflateBounds = (bounds: Bounds, amount: number): Bounds => ({
  x: bounds.x - amount,
  y: bounds.y - amount,
  width: bounds.width + amount * 2,
  height: bounds.height + amount * 2,
});

const routeIsClear = (points: Point[], obstacles: Bounds[]): boolean =>
  points.slice(0, -1).every((point, index) => {
    const next = points[index + 1];
    return (
      next !== undefined &&
      obstacles.every(
        (obstacle) => !segmentIntersectsInterior(point, next, obstacle),
      )
    );
  });

// Finds a spot for a connector label beside one of the connector's segments,
// trying the longest segment first and fanning out from its midpoint. The
// label keeps the connector clearance from every route and stays at least
// 4px away from other labels and nodes.
const placeLabel = (
  length: number,
  fontSize: number,
  route: Point[],
  routes: Point[][],
  blocked: Bounds[],
  clearance = 8,
): { anchorPoint: Point; anchor: string; bounds: Bounds } | null => {
  const gap = clearance + 1;
  const height = Math.round(fontSize * 1.2);
  const segments = connectorSegments(route)
    .map((segment, index) => ({ segment, index }))
    .sort(
      (first, second) =>
        segmentLength(second.segment) - segmentLength(first.segment) ||
        first.index - second.index,
    );
  const fits = (bounds: Bounds) =>
    routes.every((other) =>
      connectorSegments(other).every(
        ([from, to]) =>
          !segmentIntersectsInterior(
            from,
            to,
            inflateBounds(bounds, clearance),
          ),
      ),
    ) &&
    blocked.every(
      (other) => intersection(bounds, inflateBounds(other, 4)) === null,
    );
  for (const { segment } of segments) {
    const [from, to] = segment;
    const horizontal = from.y === to.y;
    for (let step = 0; step <= 20; step += 1) {
      const fraction =
        0.5 + (step % 2 === 0 ? 1 : -1) * Math.ceil(step / 2) * 0.05;
      if (fraction < 0 || fraction > 1) continue;
      const along = {
        x: Math.round(from.x + (to.x - from.x) * fraction),
        y: Math.round(from.y + (to.y - from.y) * fraction),
      };
      const beside = Math.round(along.y + fontSize / 2);
      const candidates: Array<{ anchorPoint: Point; anchor: string }> =
        horizontal
          ? [
              {
                anchorPoint: {
                  x: along.x,
                  y: along.y - gap - height + fontSize,
                },
                anchor: "middle",
              },
              {
                anchorPoint: { x: along.x, y: along.y + gap + fontSize },
                anchor: "middle",
              },
            ]
          : [
              { anchorPoint: { x: along.x + gap, y: beside }, anchor: "start" },
              { anchorPoint: { x: along.x - gap, y: beside }, anchor: "end" },
            ];
      for (const candidate of candidates) {
        const bounds = measureText(
          length,
          fontSize,
          candidate.anchorPoint,
          candidate.anchor,
        );
        const withinSegment = horizontal
          ? bounds.x >= Math.min(from.x, to.x) &&
            bounds.x + bounds.width <= Math.max(from.x, to.x)
          : bounds.y >= Math.min(from.y, to.y) &&
            bounds.y + bounds.height <= Math.max(from.y, to.y);
        if (withinSegment && fits(bounds)) {
          return { ...candidate, bounds };
        }
      }
    }
  }
  return null;
};

const segmentLength = ([from, to]: [Point, Point]): number =>
  Math.abs(to.x - from.x) + Math.abs(to.y - from.y);

type ConnectorPorts = ReturnType<typeof connectorPorts>;

const sideOf = (point: Point, bounds: Bounds): string =>
  point.x === bounds.x
    ? "left"
    : point.x === bounds.x + bounds.width
      ? "right"
      : point.y === bounds.y
        ? "top"
        : "bottom";

// Spreads the endpoints that share a node side evenly along that side, ordered
// by the position of the node at the other end, so parallel connectors do not
// collapse onto one line.
const spreadPorts = (
  connectors: Array<{ id: string; source: Bounds; target: Bounds }>,
): Map<string, ConnectorPorts> => {
  const ports = new Map(
    connectors.map((connector) => [
      connector.id,
      connectorPorts(connector.source, connector.target),
    ]),
  );
  const sides = new Map<
    string,
    Array<{ id: string; end: "start" | "end"; bounds: Bounds; other: Bounds }>
  >();
  for (const connector of connectors) {
    const port = ports.get(connector.id);
    if (port === undefined) continue;
    for (const [end, bounds, other] of [
      ["start", connector.source, connector.target],
      ["end", connector.target, connector.source],
    ] as const) {
      const key = `${formatBounds(bounds)}:${sideOf(port[end], bounds)}`;
      sides.set(key, [
        ...(sides.get(key) ?? []),
        { id: connector.id, end, bounds, other },
      ]);
    }
  }
  for (const [key, users] of sides) {
    const alongX = key.endsWith(":top") || key.endsWith(":bottom");
    const centre = (bounds: Bounds) =>
      alongX ? bounds.x + bounds.width / 2 : bounds.y + bounds.height / 2;
    const ordered = users
      .map((user, index) => ({ user, index }))
      .sort(
        (first, second) =>
          centre(first.user.other) - centre(second.user.other) ||
          first.index - second.index,
      );
    ordered.forEach(({ user }, index) => {
      const port = ports.get(user.id);
      if (port === undefined) return;
      const fraction = (index + 1) / (ordered.length + 1);
      const point = port[user.end];
      ports.set(user.id, {
        ...port,
        [user.end]: alongX
          ? {
              x: Math.round(user.bounds.x + user.bounds.width * fraction),
              y: point.y,
            }
          : {
              x: point.x,
              y: Math.round(user.bounds.y + user.bounds.height * fraction),
            },
      });
    });
  }
  alignFacingPorts(connectors, ports);
  return ports;
};

// Straightens connectors between facing sides: when one end's coordinate also
// fits on the other end's side, away from its corners and at least a lane
// away from other ports there, both ends share it.
const alignFacingPorts = (
  connectors: Array<{ id: string; source: Bounds; target: Bounds }>,
  ports: Map<string, ConnectorPorts>,
  margin = 8,
): void => {
  const taken = (bounds: Bounds, side: string, value: number, id: string) =>
    connectors.some((other) => {
      if (other.id === id) return false;
      const port = ports.get(other.id);
      if (port === undefined) return false;
      return (["start", "end"] as const).some((end) => {
        const owner = end === "start" ? other.source : other.target;
        const point = port[end];
        return (
          owner === bounds &&
          sideOf(point, owner) === side &&
          Math.abs(
            (side === "left" || side === "right" ? point.y : point.x) - value,
          ) < laneSpacing
        );
      });
    });
  for (const connector of connectors) {
    const port = ports.get(connector.id);
    if (port === undefined) continue;
    const startSide = sideOf(port.start, connector.source);
    const endSide = sideOf(port.end, connector.target);
    const facing =
      (startSide === "right" && endSide === "left") ||
      (startSide === "left" && endSide === "right") ||
      (startSide === "bottom" && endSide === "top") ||
      (startSide === "top" && endSide === "bottom");
    if (!facing) continue;
    const alongY = startSide === "left" || startSide === "right";
    const coordinate = (point: Point) => (alongY ? point.y : point.x);
    if (coordinate(port.start) === coordinate(port.end)) continue;
    const fits = (bounds: Bounds, value: number) =>
      alongY
        ? value >= bounds.y + margin &&
          value <= bounds.y + bounds.height - margin
        : value >= bounds.x + margin &&
          value <= bounds.x + bounds.width - margin;
    const moved = (point: Point, value: number): Point =>
      alongY ? { x: point.x, y: value } : { x: value, y: point.y };
    const startValue = coordinate(port.start);
    const endValue = coordinate(port.end);
    if (
      fits(connector.target, startValue) &&
      !taken(connector.target, endSide, startValue, connector.id)
    ) {
      ports.set(connector.id, { ...port, end: moved(port.end, startValue) });
    } else if (
      fits(connector.source, endValue) &&
      !taken(connector.source, startSide, endValue, connector.id)
    ) {
      ports.set(connector.id, { ...port, start: moved(port.start, endValue) });
    }
  }
};

const routeConnector = (
  source: Bounds,
  target: Bounds,
  rawObstacles: Bounds[],
  ports = connectorPorts(source, target),
  occupied: Point[][] = [],
  clearance = 8,
): Point[] => {
  const { start, end } = ports;
  const obstacles = rawObstacles.map((bounds) =>
    inflateBounds(bounds, clearance),
  );
  const direct = [start, end];
  if (
    (start.x === end.x || start.y === end.y) &&
    routeIsClear(direct, obstacles) &&
    occupied.every((other) => !routesOverlap(direct, other))
  ) {
    return direct;
  }

  // Step straight out of each port before searching, and keep the search
  // clear of the end nodes too, so detours meet node sides at right angles
  // instead of running along them.
  const outward = (point: Point, bounds: Bounds): Point => {
    const side = sideOf(point, bounds);
    return {
      x:
        point.x +
        (side === "left" ? -clearance : side === "right" ? clearance : 0),
      y:
        point.y +
        (side === "top" ? -clearance : side === "bottom" ? clearance : 0),
    };
  };
  const detour = searchOrthogonalRoute(
    outward(start, source),
    outward(end, target),
    [
      ...obstacles,
      inflateBounds(source, clearance),
      inflateBounds(target, clearance),
    ],
    occupied,
    axisOf(start, source),
    axisOf(end, target),
  );
  return detour === null
    ? direct
    : withoutRedundantPoints([start, ...detour, end]);
};

const withoutRedundantPoints = (points: Point[]): Point[] =>
  points.filter((point, index) => {
    const before = points[index - 1];
    const after = points[index + 1];
    if (before === undefined || after === undefined) return true;
    return !(
      (before.x === point.x && point.x === after.x) ||
      (before.y === point.y && point.y === after.y)
    );
  });

const bendPenalty = 40;
const laneSpacing = 10;

// Minimal binary heap ordered by [cost, state] for deterministic ties.
class MinQueue {
  private items: Array<[number, number]> = [];

  get size(): number {
    return this.items.length;
  }

  private less(a: number, b: number): boolean {
    const first = this.items[a];
    const second = this.items[b];
    if (first === undefined || second === undefined) return false;
    return (
      first[0] < second[0] || (first[0] === second[0] && first[1] < second[1])
    );
  }

  private swap(a: number, b: number): void {
    const first = this.items[a];
    const second = this.items[b];
    if (first === undefined || second === undefined) return;
    this.items[a] = second;
    this.items[b] = first;
  }

  push(item: [number, number]): void {
    this.items.push(item);
    let index = this.items.length - 1;
    while (index > 0) {
      const parent = Math.floor((index - 1) / 2);
      if (!this.less(index, parent)) break;
      this.swap(index, parent);
      index = parent;
    }
  }

  pop(): [number, number] | undefined {
    const top = this.items[0];
    const last = this.items.pop();
    if (top === undefined || last === undefined || this.items.length === 0) {
      return top;
    }
    this.items[0] = last;
    let index = 0;
    for (;;) {
      const left = index * 2 + 1;
      const right = left + 1;
      let smallest = index;
      if (left < this.items.length && this.less(left, smallest))
        smallest = left;
      if (right < this.items.length && this.less(right, smallest))
        smallest = right;
      if (smallest === index) break;
      this.swap(index, smallest);
      index = smallest;
    }
    return top;
  }
}

// 0: horizontal, 1: vertical
type Axis = 0 | 1;

const axisOf = (point: Point, bounds: Bounds): Axis => {
  const side = sideOf(point, bounds);
  return side === "left" || side === "right" ? 0 : 1;
};

// Dijkstra over a sparse orthogonal grid built from obstacle edges, the
// channels between them and the two ports. Returns null when no path exists.
const searchOrthogonalRoute = (
  start: Point,
  end: Point,
  obstacles: Bounds[],
  occupied: Point[][],
  startAxis: Axis,
  endAxis: Axis,
): Point[] | null => {
  const axis = (pick: (point: Point) => number, low: "x" | "y") => {
    const size = low === "x" ? "width" : "height";
    // Lanes beside connectors already drawn let new routes run alongside
    // them when every other line through a channel is taken.
    const lanes = occupied
      .flatMap(connectorSegments)
      .flatMap(([from, to]) =>
        pick(from) === pick(to)
          ? [pick(from) - laneSpacing, pick(from) + laneSpacing]
          : [],
      );
    const edges = [
      pick(start),
      pick(end),
      ...obstacles.flatMap((bounds) => [
        bounds[low],
        bounds[low] + bounds[size],
      ]),
    ];
    const sorted = [...new Set(edges)].sort((a, b) => a - b);
    // Midlines of the gaps between edges, rounded to whole units
    const channels = sorted
      .slice(1)
      .map((value, index) =>
        Math.round(((sorted[index] ?? value) + value) / 2),
      );
    return [...new Set([...sorted, ...channels, ...lanes])].sort(
      (a, b) => a - b,
    );
  };
  const xs = axis((point) => point.x, "x");
  const ys = axis((point) => point.y, "y");
  // Grid lines include every obstacle edge, so an edge between neighbouring
  // grid points enters an obstacle exactly when its midpoint is inside it.
  const blockedCell = new Uint8Array(xs.length * ys.length);
  const blockedHorizontal = new Uint8Array(xs.length * ys.length);
  const blockedVertical = new Uint8Array(xs.length * ys.length);
  // Edges on an obstacle's clearance outline carry a tiny extra cost that only
  // breaks ties, so equal detours run mid-channel instead of skirting a node.
  const outlineHorizontal = new Uint8Array(xs.length * ys.length);
  const outlineVertical = new Uint8Array(xs.length * ys.length);
  for (const bounds of obstacles) {
    const left = xs.indexOf(bounds.x);
    const right = xs.indexOf(bounds.x + bounds.width);
    const top = ys.indexOf(bounds.y);
    const bottom = ys.indexOf(bounds.y + bounds.height);
    for (let yi = top; yi <= bottom; yi += 1) {
      for (let xi = left; xi <= right; xi += 1) {
        const index = yi * xs.length + xi;
        const insideX = xi > left && xi < right;
        const insideY = yi > top && yi < bottom;
        if (insideX && insideY) blockedCell[index] = 1;
        if (insideY && xi < right) blockedHorizontal[index] = 1;
        if (insideX && yi < bottom) blockedVertical[index] = 1;
        if ((yi === top || yi === bottom) && xi < right) {
          outlineHorizontal[index] = 1;
        }
        if ((xi === left || xi === right) && yi < bottom) {
          outlineVertical[index] = 1;
        }
      }
    }
  }
  // Grid edges that share any length with another connector's segment are
  // taken, including segments shorter than one grid step.
  for (const [from, to] of occupied.flatMap(connectorSegments)) {
    if (from.y === to.y) {
      const yi = ys.indexOf(from.y);
      if (yi === -1) continue;
      for (let xi = 0; xi < xs.length - 1; xi += 1) {
        const low = xs[xi] ?? 0;
        const high = xs[xi + 1] ?? 0;
        if (high > Math.min(from.x, to.x) && low < Math.max(from.x, to.x)) {
          blockedHorizontal[yi * xs.length + xi] = 1;
        }
      }
    } else if (from.x === to.x) {
      const xi = xs.indexOf(from.x);
      if (xi === -1) continue;
      for (let yi = 0; yi < ys.length - 1; yi += 1) {
        const low = ys[yi] ?? 0;
        const high = ys[yi + 1] ?? 0;
        if (high > Math.min(from.y, to.y) && low < Math.max(from.y, to.y)) {
          blockedVertical[yi * xs.length + xi] = 1;
        }
      }
    }
  }
  const key = (xi: number, yi: number) => yi * xs.length + xi;
  const startKey = key(xs.indexOf(start.x), ys.indexOf(start.y));
  const endKey = key(xs.indexOf(end.x), ys.indexOf(end.y));
  const pointOf = (cell: number): Point => ({
    x: xs[cell % xs.length] ?? 0,
    y: ys[Math.floor(cell / xs.length)] ?? 0,
  });
  // state = cell * 2 + (0: arrived horizontally, 1: arrived vertically)
  const distance = new Map<number, number>();
  const previous = new Map<number, number>();
  const queue = new MinQueue();
  queue.push([0, startKey * 2 + startAxis]);
  distance.set(startKey * 2 + startAxis, 0);
  let reached: number | null = null;
  while (queue.size > 0) {
    const [cost, state] = queue.pop() ?? [0, 0];
    if (cost > (distance.get(state) ?? Infinity)) continue;
    const cell = Math.floor(state / 2);
    if (cell === endKey) {
      reached = state;
      break;
    }
    const point = pointOf(cell);
    const xi = cell % xs.length;
    const yi = Math.floor(cell / xs.length);
    const neighbours: Array<[number, number]> = [
      [xi - 1, yi],
      [xi + 1, yi],
      [xi, yi - 1],
      [xi, yi + 1],
    ];
    for (const [nxi, nyi] of neighbours) {
      if (nxi < 0 || nyi < 0 || nxi >= xs.length || nyi >= ys.length) continue;
      const nextCell = key(nxi, nyi);
      const next = pointOf(nextCell);
      if (nextCell !== endKey && blockedCell[nextCell] === 1) continue;
      const vertical = nxi === xi ? 1 : 0;
      const edge = key(Math.min(xi, nxi), Math.min(yi, nyi));
      const blocked = vertical === 1 ? blockedVertical : blockedHorizontal;
      if (blocked[edge] === 1) continue;
      // The path continues straight out of the start stub and straight into
      // the end stub, so turning onto or off them counts as a bend.
      const bend =
        (state % 2 !== vertical ? bendPenalty : 0) +
        (nextCell === endKey && vertical !== endAxis ? bendPenalty : 0);
      const nextState = nextCell * 2 + vertical;
      const length = Math.abs(next.x - point.x) + Math.abs(next.y - point.y);
      const outline = vertical === 1 ? outlineVertical : outlineHorizontal;
      const nextCost = cost + length * (outline[edge] === 1 ? 1.01 : 1) + bend;
      if (nextCost >= (distance.get(nextState) ?? Infinity)) continue;
      distance.set(nextState, nextCost);
      previous.set(nextState, state);
      queue.push([nextCost, nextState]);
    }
  }
  if (reached === null) return null;
  const path: Point[] = [];
  for (let state: number | undefined = reached; state !== undefined;) {
    path.unshift(pointOf(Math.floor(state / 2)));
    state = previous.get(state);
  }
  return withoutRedundantPoints(path);
};

export const analyze = (svg: string): AnalysisReport => {
  const document = parseSvg(svg);
  const groups = Array.from(document.getElementsByTagName("g"));
  const recognizedNodes = groups.flatMap((group, index) => {
    const directElements = Array.from(group.childNodes).filter(
      (child): child is Element => child.nodeType === 1,
    );
    const annotation = group.getAttribute("data-node");
    const isAnnotated = annotation !== "";
    const directBox = directElements.find(
      (element) => element.tagName === "rect",
    );
    const directLabels = directElements.filter(
      (element) => element.tagName === "text",
    );
    if (
      !isAnnotated &&
      (directBox === undefined || directLabels.length === 0)
    ) {
      return [];
    }

    const box = isAnnotated
      ? group.getElementsByTagName("rect").item(0)
      : (directBox ?? null);
    if (box === null) {
      return [];
    }

    return [
      {
        element: group,
        id:
          annotation || group.getAttribute("id") || `node-${String(index + 1)}`,
        box,
        labels: directLabels,
        allowOverlap: group.getAttribute("data-allow-overlap") === "true",
      },
    ];
  });
  const recognizedByElement = new Map<
    Element,
    (typeof recognizedNodes)[number]
  >(recognizedNodes.map((node) => [node.element, node]));
  const nodes = recognizedNodes.map((recognized): DiagramNode => {
    let ancestor = recognized.element.parentNode;
    let parentId: string | undefined;
    while (ancestor !== null) {
      if (ancestor.nodeType === 1) {
        const parent = recognizedByElement.get(ancestor as Element);
        if (parent !== undefined) {
          parentId = parent.id;
          break;
        }
      }
      ancestor = ancestor.parentNode;
    }

    return {
      id: recognized.id,
      bounds: rectBounds(recognized.box),
      labelBounds: recognized.labels.map(textBounds),
      ...(parentId === undefined ? {} : { parentId }),
      ...(recognized.allowOverlap ? { allowOverlap: true } : {}),
    };
  });
  const connectorElements = ["line", "polyline", "path"].flatMap((tagName) =>
    Array.from(document.getElementsByTagName(tagName)),
  );
  const connectors = connectorElements.flatMap(
    (element, index): DiagramConnector[] => {
      const from = element.getAttribute("data-from");
      const to = element.getAttribute("data-to");
      if (from === null || from === "" || to === null || to === "") {
        return [];
      }

      return [
        {
          id: element.getAttribute("id") || `connector-${String(index + 1)}`,
          from,
          to,
          points: connectorPoints(element),
        },
      ];
    },
  );
  const labels = Array.from(document.getElementsByTagName("text")).flatMap(
    (label, index): DiagramLabel[] => {
      const annotation = label.getAttribute("data-label");
      if (annotation === "") {
        return [];
      }
      return [
        {
          id:
            annotation ||
            label.getAttribute("id") ||
            `label-${String(index + 1)}`,
          bounds: textBounds(label),
          ...(label.getAttribute("data-label-for")
            ? { connector: label.getAttribute("data-label-for") ?? "" }
            : {}),
        },
      ];
    },
  );
  const unsupportedElements = Array.from(
    document.documentElement.childNodes,
  ).flatMap((child, index): UnsupportedElement[] => {
    if (child.nodeType !== 1) return [];
    const element = child as Element;
    if (
      !["rect", "circle", "ellipse", "polygon", "path"].includes(
        element.tagName,
      ) ||
      element.getAttribute("data-from") !== ""
    ) {
      return [];
    }
    return [
      {
        id: element.getAttribute("id") || `unsupported-${String(index + 1)}`,
        tagName: element.tagName,
      },
    ];
  });

  const drawingBounds = enclosingBounds([
    ...nodes.map((node) => node.bounds),
    ...labels.map((label) => label.bounds),
    ...connectors.flatMap((connector) => {
      const bounds = pointsBounds(connector.points);
      return bounds === null ? [] : [bounds];
    }),
  ]);
  const issues = [
    ...inspectViewport(document.documentElement, drawingBounds),
    ...inspectTextFit(nodes),
    ...inspectNodeOverlaps(nodes),
    ...inspectNodeGaps(nodes),
    ...inspectConnectorCrossings(nodes, connectors),
    ...inspectLabelOverlaps(labels),
    ...inspectConnectorLabelClearance(connectors, labels),
    ...inspectConnectorEndpoints(nodes, connectors),
    ...inspectConnectorOverlaps(connectors),
  ];

  return {
    valid: issues.length === 0,
    issues,
    drawingBounds,
    diagram: { nodes, connectors, labels, unsupportedElements },
  };
};

const formatBounds = (bounds: Bounds): string =>
  [bounds.x, bounds.y, bounds.width, bounds.height].join(" ");

export const fix = (svg: string): FixResult => {
  const document = parseSvg(svg);
  const root = document.documentElement;
  const changes: FixChange[] = [];
  const movedNodeIds = new Set<string>();
  const serializer = new XMLSerializer();
  let report = analyze(svg);

  for (const issue of report.issues) {
    if (issue.code !== "text-overflow") continue;
    const nodeId = issue.elements[0];
    if (nodeId === undefined) continue;
    const group = Array.from(document.getElementsByTagName("g")).find(
      (candidate) =>
        (candidate.getAttribute("data-node") ||
          candidate.getAttribute("id")) === nodeId,
    );
    if (group === undefined) continue;
    const directElements = Array.from(group.childNodes).filter(
      (child): child is Element => child.nodeType === 1,
    );
    const box = directElements.find((element) => element.tagName === "rect");
    const labels = directElements.filter(
      (element) => element.tagName === "text",
    );
    if (box === undefined || labels.length === 0) continue;
    const current = rectBounds(box);
    const local = localRectBounds(box);
    const translation = {
      x: current.x - local.x,
      y: current.y - local.y,
    };
    const measuredLabels = enclosingBounds(labels.map(textBounds));
    if (measuredLabels === null) continue;
    const padding = 12;
    const left = Math.min(current.x, measuredLabels.x - padding);
    const top = Math.min(current.y, measuredLabels.y - padding);
    const right = Math.max(
      current.x + current.width,
      measuredLabels.x + measuredLabels.width + padding,
    );
    const bottom = Math.max(
      current.y + current.height,
      measuredLabels.y + measuredLabels.height + padding,
    );
    const expanded = {
      x: left,
      y: top,
      width: right - left,
      height: bottom - top,
    };
    box.setAttribute("x", String(expanded.x - translation.x));
    box.setAttribute("y", String(expanded.y - translation.y));
    box.setAttribute("width", String(expanded.width));
    box.setAttribute("height", String(expanded.height));
    changes.push({
      code: "expand-node",
      message: `Expanded node "${nodeId}" from ${formatBounds(current)} to ${formatBounds(expanded)}.`,
      elements: [nodeId],
    });
  }

  report = analyze(serializer.serializeToString(document));
  for (let attempt = 0; attempt < 100; attempt += 1) {
    const spacingIssue = report.issues.find(
      (issue) => issue.code === "node-overlap" || issue.code === "node-gap",
    );
    if (spacingIssue === undefined) break;
    const [firstId, secondId] = spacingIssue.elements;
    const first = report.diagram.nodes.find((node) => node.id === firstId);
    const second = report.diagram.nodes.find((node) => node.id === secondId);
    if (first === undefined || second === undefined) break;
    const [leftNode, rightNode] =
      first.bounds.x <= second.bounds.x ? [first, second] : [second, first];
    const desiredX = leftNode.bounds.x + leftNode.bounds.width + 20;
    const deltaX = desiredX - rightNode.bounds.x;
    if (deltaX <= 0) break;
    const group = Array.from(document.getElementsByTagName("g")).find(
      (candidate) =>
        (candidate.getAttribute("data-node") ||
          candidate.getAttribute("id")) === rightNode.id,
    );
    if (group === undefined) break;
    const existingTransform = group.getAttribute("transform")?.trim() ?? "";
    group.setAttribute(
      "transform",
      `${existingTransform}${existingTransform === "" ? "" : " "}translate(${deltaX} 0)`,
    );
    changes.push({
      code: "move-node",
      message: `Moved node "${rightNode.id}" ${deltaX}px right to preserve a 20px gap.`,
      elements: [rightNode.id],
    });
    movedNodeIds.add(rightNode.id);
    report = analyze(serializer.serializeToString(document));
  }

  const connectorsWithIssues = new Set(
    report.issues
      .filter((issue) =>
        [
          "connector-node-crossing",
          "connector-label-clearance",
          "connector-endpoint-inside",
          "connector-overlap",
        ].includes(issue.code),
      )
      .flatMap((issue) =>
        issue.code === "connector-overlap"
          ? issue.elements
          : issue.elements.slice(0, 1),
      ),
  );
  const nodesById = new Map(
    report.diagram.nodes.map((node) => [node.id, node]),
  );
  const reroutes = report.diagram.connectors.flatMap((connector) => {
    const followsMovedNode =
      movedNodeIds.has(connector.from) || movedNodeIds.has(connector.to);
    if (!connectorsWithIssues.has(connector.id) && !followsMovedNode) return [];
    const source = nodesById.get(connector.from);
    const target = nodesById.get(connector.to);
    if (source === undefined || target === undefined) return [];
    return [{ connector, source, target }];
  });
  const ports = spreadPorts(
    reroutes.map(({ connector, source, target }) => ({
      id: connector.id,
      source: source.bounds,
      target: target.bounds,
    })),
  );
  const reroutedIds = new Set(reroutes.map(({ connector }) => connector.id));
  const settledRoutes = report.diagram.connectors
    .filter((connector) => !reroutedIds.has(connector.id))
    .map((connector) => connector.points);
  const finalRoutes = new Map<string, Point[]>(
    report.diagram.connectors.map((connector) => [
      connector.id,
      connector.points,
    ]),
  );
  for (const { connector, source, target } of reroutes) {
    const obstacles = [
      ...report.diagram.nodes
        .filter(
          (node) => node.id !== connector.from && node.id !== connector.to,
        )
        .map((node) => node.bounds),
      ...report.diagram.labels
        .filter(
          (label) =>
            label.connector === undefined || !reroutedIds.has(label.connector),
        )
        .map((label) => label.bounds),
    ];
    const route = routeConnector(
      source.bounds,
      target.bounds,
      obstacles,
      ports.get(connector.id),
      settledRoutes,
    );
    settledRoutes.push(route);
    finalRoutes.set(connector.id, route);
    const connectorElement = ["line", "polyline", "path"]
      .flatMap((tagName) => Array.from(document.getElementsByTagName(tagName)))
      .find(
        (element) =>
          element.getAttribute("id") === connector.id ||
          (element.getAttribute("data-from") === connector.from &&
            element.getAttribute("data-to") === connector.to),
      );
    if (connectorElement === undefined) continue;
    const path = document.createElementNS("http://www.w3.org/2000/svg", "path");
    for (
      let attributeIndex = 0;
      attributeIndex < connectorElement.attributes.length;
      attributeIndex += 1
    ) {
      const attribute = connectorElement.attributes.item(attributeIndex);
      if (
        attribute === null ||
        ["x1", "y1", "x2", "y2", "points", "d"].includes(attribute.name)
      ) {
        continue;
      }
      path.setAttribute(attribute.name, attribute.value);
    }
    path.setAttribute(
      "d",
      route
        .map(
          (point, index) => `${index === 0 ? "M" : "L"} ${point.x} ${point.y}`,
        )
        .join(" "),
    );
    connectorElement.parentNode?.replaceChild(path, connectorElement);
    changes.push({
      code: "route-connector",
      message: `Rerouted connector "${connector.id}" around diagram obstacles.`,
      elements: [connector.id],
    });
  }

  const movingLabels = report.diagram.labels.filter(
    (label) =>
      label.connector !== undefined && reroutedIds.has(label.connector),
  );
  const labelObstacles = report.diagram.labels
    .filter((label) => !movingLabels.includes(label))
    .map((label) => label.bounds);
  const labelElements = Array.from(document.getElementsByTagName("text"));
  for (const label of movingLabels) {
    const connectorId = label.connector ?? "";
    const route = finalRoutes.get(connectorId);
    const element = labelElements.find(
      (candidate) =>
        (candidate.getAttribute("data-label") ||
          candidate.getAttribute("id")) === label.id,
    );
    if (route === undefined || element === undefined) continue;
    const placement = placeLabel(
      element.textContent.length,
      numberAttribute(element, "font-size") || 16,
      route,
      [...finalRoutes.values()],
      [...labelObstacles, ...report.diagram.nodes.map((node) => node.bounds)],
    );
    if (placement === null) continue;
    const translation = elementTranslation(element);
    element.setAttribute("x", String(placement.anchorPoint.x - translation.x));
    element.setAttribute("y", String(placement.anchorPoint.y - translation.y));
    element.setAttribute("text-anchor", placement.anchor);
    labelObstacles.push(placement.bounds);
    changes.push({
      code: "move-label",
      message: `Moved label "${label.id}" beside connector "${connectorId}".`,
      elements: [label.id, connectorId],
    });
  }
  report = analyze(serializer.serializeToString(document));

  const viewBox = parseViewBox(root);
  if (
    viewBox !== null &&
    report.drawingBounds !== null &&
    report.issues.some((issue) => issue.code === "viewport-clipping")
  ) {
    const padding = 20;
    const required = {
      x: report.drawingBounds.x - padding,
      y: report.drawingBounds.y - padding,
      width: report.drawingBounds.width + padding * 2,
      height: report.drawingBounds.height + padding * 2,
    };
    const left = Math.min(viewBox.x, required.x);
    const top = Math.min(viewBox.y, required.y);
    const right = Math.max(
      viewBox.x + viewBox.width,
      required.x + required.width,
    );
    const bottom = Math.max(
      viewBox.y + viewBox.height,
      required.y + required.height,
    );
    const expanded = {
      x: left,
      y: top,
      width: right - left,
      height: bottom - top,
    };
    root.setAttribute("viewBox", formatBounds(expanded));
    changes.push({
      code: "expand-viewbox",
      message: `Expanded viewBox from ${formatBounds(viewBox)} to ${formatBounds(expanded)}.`,
      elements: ["svg"],
    });
  }

  const fixedSvg = serializer.serializeToString(document);
  return { svg: fixedSvg, changes, report: analyze(fixedSvg) };
};
