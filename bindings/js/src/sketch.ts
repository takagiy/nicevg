// Turning a sketch into the request the native library reads. Kept apart
// from index.ts, which loads the native library, so that a diagram script in
// a sandbox (doc.ts) builds its request the same way.

import type { Sketch, SketchNode, Style } from "./index.ts";

/** An input nicevg cannot read, saying what is wrong and where. */
export class NicevgError extends Error {
  override name = "NicevgError";
}

/** The sketch as the native library reads it: nodes drawn by their styles, edges as connectors. */
export function sketchRequest<Node extends SketchNode>({ nodes, edges, connectors, style, rootAttrs }: Sketch<Node>) {
  // Each style is called only with nodes of its own class.
  const styles = (style ?? {}) as Record<string, Style>;
  if (edges && connectors) {
    throw new NicevgError("edges and connectors are the same; pass one of them");
  }
  const ids = new Set(nodes.map((node) => node.id));
  // A node's id as it is, else split at the last dot into a node and its port.
  const end = (name: string) => {
    const dot = name.lastIndexOf(".");
    return ids.has(name) || dot < 0 ? { node: name } : { node: name.slice(0, dot), port: name.slice(dot + 1) };
  };
  return {
    nodes: nodes.map((node, index) => {
      if (typeof node.class !== "string") {
        throw new NicevgError(`nodes[${index}].class: every node needs the class of the style that draws it`);
      }
      const draw = Object.hasOwn(styles, node.class) ? styles[node.class] : undefined;
      if (!draw) {
        throw new NicevgError(`nodes[${index}].class: no style named "${node.class}"`);
      }
      const { id, x, y, parent, allowOverlap } = node;
      return { id, x, y, parent, allowOverlap, svg: String(draw(node)) };
    }),
    connectors: (edges ?? connectors)?.map(({ from, to, ...connector }) => {
      const [start, finish] = [end(from), end(to)];
      return { ...connector, from: start.node, fromPort: start.port, to: finish.node, toPort: finish.port };
    }),
    rootAttrs: rootAttrs && Object.fromEntries(Object.entries(rootAttrs).map(([name, value]) => [name, String(value)])),
  };
}
