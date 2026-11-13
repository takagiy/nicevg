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
  const parentOf = parentsOf(nodes);
  const parents = new Set(parentOf.values());
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
      const { id, x, y, allowOverlap } = node;
      const parent = parentOf.get(id);
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
 * Each child's parent, from the `children` each node lists: each naming a
 * node of the sketch, none in two lists, and none holding its own parent.
 */
function parentsOf(nodes: readonly SketchNode[]): Map<string, string> {
const ids = new Set<string>();
// As the native library says it, before children naming the id could tell a wrong story.
nodes.forEach(({ id }, index) => {
  if (ids.has(id)) {
    throw new NicevgError(`nodes[${index}].id is "${id}", the id of an earlier node too`);
  }
  ids.add(id);
});
  const parentOf = new Map<string, string>();
  nodes.forEach((node, index) => {
    if (node.parent !== undefined) {
      throw new NicevgError(
        `nodes[${index}].parent: list a node in its parent's children instead: children: ["${node.id}"] on "${node.parent}"`,
      );
    }
    (node.children ?? []).forEach((child, at) => {
      const earlier = parentOf.get(child);
      if (!ids.has(child)) {
        throw new NicevgError(`nodes[${index}].children[${at}]: no node "${child}"`);
      }
      if (earlier !== undefined) {
        throw new NicevgError(`nodes[${index}].children[${at}]: "${child}" is a child of "${earlier}" already`);
      }
      parentOf.set(child, node.id);
    });
  });
  nodes.forEach((node, index) => {
    (node.children ?? []).forEach((child, at) => {
      // Followed up from the node, its parents reach the child only round a loop.
      for (let above = parentOf.get(node.id), steps = 0; above !== undefined && steps < nodes.length; steps += 1) {
        if (above === child) {
          throw new NicevgError(`nodes[${index}].children[${at}]: "${child}" holds "${node.id}": parents cannot loop`);
        }
        above = parentOf.get(above);
      }
    });
  });
  return parentOf;
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
