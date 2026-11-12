// Turning a sketch into the requests the native library reads. Kept apart
// from index.ts, which loads the native library, so that a diagram script in
// a sandbox (doc.ts) builds its requests the same way.

import type { Sketch, SketchNode, Style } from "./index.ts";

/** An input nicevg cannot read, saying what is wrong and where. */
export class NicevgError extends Error {
  override name = "NicevgError";
}

/** A picture the native library asks a parent's style to draw: the parent's own, or the one to lay it out by. */
export interface Asked {
  id: string;
  width: number;
  height: number;
  last: boolean;
}

/** What the native library replies to a step. */
export type StepReply<Result> = { ok: Result } | { draw: Asked[]; state: unknown } | { error: string };

/**
 * A sketch as the native library reads it, nodes drawn by their styles and
 * edges as connectors, and how to draw its parents' pictures when asked: a
 * parent goes without one, for the native library to ask its style for at
 * the sizes laying out needs.
 */
export function sketchRequest<Node extends SketchNode>({ nodes, edges, connectors, style, rootAttrs }: Sketch<Node>) {
  // Each style is called only with nodes of its own class.
  const styles = (style ?? {}) as Record<string, Style>;
  if (edges && connectors) {
    throw new NicevgError("edges and connectors are the same; pass one of them");
  }
  const ids = new Set(nodes.map((node) => node.id));
  const parents = new Set(nodes.flatMap((node) => (node.parent === undefined ? [] : [node.parent])));
  // A node's id as it is, else split at the last dot into a node and its port.
  const end = (name: string) => {
    const dot = name.lastIndexOf(".");
    return ids.has(name) || dot < 0 ? { node: name } : { node: name.slice(0, dot), port: name.slice(dot + 1) };
  };
  const styleOf = (node: SketchNode, index: number) => {
    if (typeof node.class !== "string") {
      throw new NicevgError(`nodes[${index}].class: every node needs the class of the style that draws it`);
    }
    const draw = Object.hasOwn(styles, node.class) ? styles[node.class] : undefined;
    if (!draw) {
      throw new NicevgError(`nodes[${index}].class: no style named "${node.class}"`);
    }
    return draw;
  };
  /** A parent's picture, an error from its style naming the parent. */
  const pictured = (node: SketchNode, index: number, styled: SketchNode) => {
    try {
      return String(styleOf(node, index)(styled));
    } catch (error) {
      throw new NicevgError(`nodes[${index}]: the style "${node.class}" threw ${error}`, { cause: error });
    }
  };
  const sketch = {
    nodes: nodes.map((node, index) => {
      const { id, x, y, parent, allowOverlap } = node;
      if (parents.has(id)) {
        styleOf(node, index);
        return { id, x, y, parent, allowOverlap, redrawn: true };
      }
      return { id, x, y, parent, allowOverlap, svg: String(styleOf(node, index)(node)) };
    }),
    connectors: (edges ?? connectors)?.map(({ from, to, ...connector }) => {
      const [start, finish] = [end(from), end(to)];
      return { ...connector, from: start.node, fromPort: start.port, to: finish.node, toPort: finish.port };
    }),
    rootAttrs: rootAttrs && Object.fromEntries(Object.entries(rootAttrs).map(([name, value]) => [name, String(value)])),
  };
  const indexOf = new Map(nodes.map((node, index) => [node.id, index]));
  /** The pictures asked for: the parent's own `width` and `height` win for one to lay it out by, laying out's for its last. */
  const draw = (asked: Asked[]) =>
    asked.map(({ id, width, height, last }) => {
      const index = indexOf.get(id)!;
      const node = nodes[index]!;
      const size = last ? { width, height } : { width: node.width ?? width, height: node.height ?? height };
      return pictured(node, index, { ...node, ...size });
    });
  return { sketch, draw };
}

/**
 * The steps of laying out a sketch whose parents styles draw: yields each
 * request for the native library, to be given its reply back, and each set
 * of pictures the native library asks for, to be given the pictures drawn,
 * until the native library replies with the diagram.
 */
export function* layOutSketch<Result>(
  sketch: unknown,
): Generator<{ native: unknown } | { draw: Asked[] }, Result, StepReply<Result> | string[]> {
  const pictures: Record<string, string[]> = {};
  let state: unknown = null;
  for (;;) {
    const reply = (yield { native: { sketch, pictures, state } }) as StepReply<Result>;
    if ("error" in reply) {
      throw new NicevgError(reply.error);
    }
    if ("ok" in reply) {
      return reply.ok;
    }
    const drawn = (yield { draw: reply.draw }) as string[];
    reply.draw.forEach(({ id }, index) => (pictures[id] ??= []).push(drawn[index]!));
    state = reply.state;
  }
}
