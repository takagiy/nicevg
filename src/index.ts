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

const textBounds = (element: Element): Bounds => {
  const translation = elementTranslation(element);
  const fontSize = numberAttribute(element, "font-size") || 16;
  const width = Math.round(element.textContent.length * fontSize * 0.59);
  const anchor = element.getAttribute("text-anchor");
  const anchorX = numberAttribute(element, "x") + translation.x;
  const x =
    anchor === "middle"
      ? anchorX - width / 2
      : anchor === "end"
        ? anchorX - width
        : anchorX;

  return {
    x,
    y: numberAttribute(element, "y") + translation.y - fontSize,
    width,
    height: Math.round(fontSize * 1.2),
  };
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

const routeConnector = (
  source: Bounds,
  target: Bounds,
  rawObstacles: Bounds[],
  clearance = 8,
): Point[] => {
  const { start, end, horizontal } = connectorPorts(source, target);
  const obstacles = rawObstacles.map((bounds) =>
    inflateBounds(bounds, clearance),
  );
  const direct = [start, end];
  if (routeIsClear(direct, obstacles)) {
    return direct;
  }

  const candidates = horizontal
    ? rawObstacles.flatMap((bounds) => [
        bounds.y - clearance,
        bounds.y + bounds.height + clearance,
      ])
    : rawObstacles.flatMap((bounds) => [
        bounds.x - clearance,
        bounds.x + bounds.width + clearance,
      ]);
  const routes = candidates
    .map((coordinate) =>
      horizontal
        ? [
            start,
            { x: start.x, y: coordinate },
            { x: end.x, y: coordinate },
            end,
          ]
        : [
            start,
            { x: coordinate, y: start.y },
            { x: coordinate, y: end.y },
            end,
          ],
    )
    .filter((route) => routeIsClear(route, obstacles))
    .sort((first, second) => {
      const routeLength = (route: Point[]) =>
        route.slice(0, -1).reduce((total, point, index) => {
          const next = route[index + 1];
          return next === undefined
            ? total
            : total + Math.abs(next.x - point.x) + Math.abs(next.y - point.y);
        }, 0);
      return (
        routeLength(first) - routeLength(second) ||
        (horizontal
          ? (first[1]?.y ?? 0) - (second[1]?.y ?? 0)
          : (first[1]?.x ?? 0) - (second[1]?.x ?? 0))
      );
    });
  return routes[0] ?? direct;
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
        ].includes(issue.code),
      )
      .flatMap((issue) => issue.elements.slice(0, 1)),
  );
  const nodesById = new Map(
    report.diagram.nodes.map((node) => [node.id, node]),
  );
  for (const connector of report.diagram.connectors) {
    const followsMovedNode =
      movedNodeIds.has(connector.from) || movedNodeIds.has(connector.to);
    if (!connectorsWithIssues.has(connector.id) && !followsMovedNode) continue;
    const source = nodesById.get(connector.from);
    const target = nodesById.get(connector.to);
    if (source === undefined || target === undefined) continue;
    const obstacles = [
      ...report.diagram.nodes
        .filter(
          (node) => node.id !== connector.from && node.id !== connector.to,
        )
        .map((node) => node.bounds),
      ...report.diagram.labels.map((label) => label.bounds),
    ];
    const route = routeConnector(source.bounds, target.bounds, obstacles);
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
