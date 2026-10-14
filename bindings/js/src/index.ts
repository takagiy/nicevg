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

import type { SvgFragment } from "./draw.ts";
import { call as native } from "./native.ts";

export { SvgFragment, Text, type TextPlace, svg, text } from "./draw.ts";

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

/**
 * A node drawn by the style its `class` names, with its origin at `x`, `y`.
 * It lies around what the style draws, and rendering moves it but never
 * resizes it. Its other fields, such as `label`, are for the style to use.
 */
export interface StyledNode {
  id: string;
  x: number;
  y: number;
  class: string;
  /** The node this one sits inside. */
  parent?: string;
  /** Whether it may overlap other nodes on purpose. */
  allowOverlap?: boolean;
  [field: string]: unknown;
}

/** Draws a styled node's picture, from its own origin, with `svg` and `text`. */
export type Style<Node extends StyledNode = StyledNode> = (node: Node) => SvgFragment;

/** A style for each class the nodes name, each given the nodes of its class. */
export type Styles<Node extends SketchNode | StyledNode> = {
  [Class in Extract<Node, StyledNode>["class"]]: Style<Extract<Node, { class: Class }>>;
};

/** A diagram as nodes and the edges (connectors) between them. */
export interface Sketch<Node extends SketchNode | StyledNode = SketchNode | StyledNode> {
  nodes: readonly Node[];
  edges?: SketchConnector[];
  /** The same as `edges`, its name before; pass one or the other. */
  connectors?: SketchConnector[];
  /** How to draw the nodes of each class. */
  style?: Styles<NoInfer<Node>>;
  /**
   * Attributes for the root `<svg>`, such as `font-family`, which its texts
   * inherit. nicevg writes `xmlns`, `viewBox`, `width` and `height` itself.
   */
  rootAttrs?: Record<string, string | number>;
}

/** A diagram to render: a sketch, SVG text following nicevg's SVG contract, or an Svg rendered before. */
export type Input<Node extends SketchNode | StyledNode = SketchNode | StyledNode> = Sketch<Node> | { svg: string } | Svg;

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
export function render<const Node extends SketchNode | StyledNode>(input: Input<Node>): Svg {
  const request = new TextEncoder().encode(
    JSON.stringify(input instanceof Svg ? { svg: input.toString() } : "svg" in input ? input : sketchRequest(input)),
  );
  const reply = JSON.parse(native(request)) as
    | { ok: { svg: string; changes: Change[]; report: AnalysisReport } }
    | { error: string };
  if ("error" in reply) {
    throw new NicevgError(reply.error);
  }
  return new Svg(reply.ok.svg, reply.ok.changes, reply.ok.report);
}

/** The sketch as the native library reads it: styled nodes drawn, edges as connectors. */
function sketchRequest<Node extends SketchNode | StyledNode>({ nodes, edges, connectors, style, rootAttrs }: Sketch<Node>) {
  // Each style is called only with nodes of its own class.
  const styles = (style ?? {}) as Record<string, Style>;
  if (edges && connectors) {
    throw new NicevgError("edges and connectors are the same; pass one of them");
  }
  return {
    nodes: nodes.map((node, index) => {
      if (!("class" in node)) {
        return node;
      }
      const draw = Object.hasOwn(styles, node.class) ? styles[node.class] : undefined;
      if (!draw) {
        throw new NicevgError(`nodes[${index}].class: no style named "${node.class}"`);
      }
      const { id, x, y, parent, allowOverlap } = node;
      return { id, x, y, parent, allowOverlap, svg: String(draw(node)) };
    }),
    connectors: edges ?? connectors,
    rootAttrs: rootAttrs && Object.fromEntries(Object.entries(rootAttrs).map(([name, value]) => [name, String(value)])),
  };
}
