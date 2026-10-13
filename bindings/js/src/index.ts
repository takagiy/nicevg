/**
 * nicevg for TypeScript on Bun and Node: renders diagrams given as shapes
 * (or as SVG text) into SVG, repairing their layout and arranging their
 * nodes, by calling the native library through Bun's FFI or through koffi.
 *
 * ```ts
 * import { render } from "nicevg";
 *
 * const svg = render({
 *   nodes: [
 *     { id: "order", x: 20, y: 20, width: 120, height: 56, label: "Order" },
 *     { id: "bill", x: 300, y: 160, width: 120, height: 56, label: "Bill" },
 *   ],
 *   connectors: [{ from: "order", to: "bill", label: "invoice" }],
 * });
 * await svg.write("diagram.svg");
 * svg.report.diagram.connectors; // where the routes ended up, in the order given
 * ```
 *
 * Rendering runs on the calling thread and blocks it until done; arranging
 * a large diagram takes a second or two, using every core.
 */

import { writeFile } from "node:fs/promises";

import { call as native } from "./native.ts";

export interface Point {
  x: number;
  y: number;
}

export interface Bounds {
  x: number;
  y: number;
  width: number;
  height: number;
}

/** A box, or a circle whose bounds are the square around it. */
export interface SketchNode {
  id: string;
  x: number;
  y: number;
  width: number;
  height: number;
  shape?: "rect" | "circle";
  /** Its text, lines split at `\n`: at the top left of a node with children, centred in any other. */
  label?: string;
  fontSize?: number;
  /** The node this one sits inside. */
  parent?: string;
  /** Whether it may overlap other nodes on purpose. */
  allowOverlap?: boolean;
}

/**
 * A connector from one node to another, along `points` when given. Without
 * them it runs between the nodes' centres, which rendering reroutes from
 * side to side. Connectors have no id: in the rendered diagram, tell them
 * apart by their `from` and `to`.
 */
export interface SketchConnector {
  from: string;
  to: string;
  points?: Point[];
  /** Its text, lines split at `\n`, placed beside it. */
  label?: string;
  fontSize?: number;
}

/** A diagram as nodes and the connectors between them. */
export interface Sketch {
  nodes: SketchNode[];
  connectors?: SketchConnector[];
}

/** A diagram to render: a sketch, SVG text following nicevg's SVG contract, or an Svg rendered before. */
export type Input = Sketch | { svg: string } | Svg;

export interface DiagramNode {
  id: string;
  shape?: "circle";
  bounds: Bounds;
  labelBounds: Bounds[];
  parentId?: string;
  allowOverlap?: boolean;
}

export interface DiagramConnector {
  /** Its id in an SVG; a sketch's connectors get one for their labels to refer to. */
  id: string;
  from: string;
  to: string;
  points: Point[];
}

export interface DiagramLabel {
  id: string;
  bounds: Bounds;
  /** The id of the connector it describes. */
  connector?: string;
}

export interface Diagram {
  nodes: DiagramNode[];
  connectors: DiagramConnector[];
  labels: DiagramLabel[];
  unsupportedElements: { id: string; tagName: string }[];
}

export interface Issue {
  code: string;
  message: string;
  elements: string[];
  details?: Record<string, unknown>;
}

export interface AnalysisReport {
  valid: boolean;
  issues: Issue[];
  drawingBounds: Bounds | null;
  diagram: Diagram;
}

export interface Change {
  code: string;
  message: string;
  elements: string[];
}

/** A rendered diagram as SVG, with what rendering changed and the diagram read back from it. */
export class Svg {
  readonly #text: string;

  constructor(
    text: string,
    /** What fixing and arranging changed, in order. */
    readonly changes: Change[],
    /** The diagram read back: where its nodes, routes and labels ended up, and any issue left. */
    readonly report: AnalysisReport,
  ) {
    this.#text = text;
  }

  /** The SVG text. */
  toString(): string {
    return this.#text;
  }

  /** Writes the SVG text to a file. */
  write(path: string | URL): Promise<void> {
    return writeFile(path, this.#text);
  }
}

/** An input nicevg cannot read, saying what is wrong and where. */
export class NicevgError extends Error {
  override name = "NicevgError";
}

/**
 * Renders a diagram into SVG: repairs its layout errors (overlapping nodes,
 * routes through nodes, stray labels and more), routes the connectors given
 * no points, and moves nodes a little where that reads better.
 */
export function render(input: Input): Svg {
  const request = new TextEncoder().encode(JSON.stringify(input instanceof Svg ? { svg: input.toString() } : input));
  const reply = JSON.parse(native(request)) as
    | { ok: { svg: string; changes: Change[]; report: AnalysisReport } }
    | { error: string };
  if ("error" in reply) {
    throw new NicevgError(reply.error);
  }
  return new Svg(reply.ok.svg, reply.ok.changes, reply.ok.report);
}
