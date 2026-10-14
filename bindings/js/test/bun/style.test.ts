// Styled nodes and the svg/text DSL on Bun.

import { describe, expect, test } from "bun:test";

import { NicevgError, type Sketch, type Style, render, svg, text } from "nicevg";

/** A data store open on its right: its id, a dividing line, then its name. */
const store: Style = ({ no, label }) => {
  const id = text(String(no)), name = text(String(label));
  const sep = id.width + 24, w = sep + name.width + 24, h = Math.max(id.height, name.height) + 20;
  return svg`
    <g fill="none" stroke="#333">
      <path d="M ${w} 0 H 0 V ${h} H ${w}"/>
      <line x1="${sep}" y1="0" x2="${sep}" y2="${h}"/>
    </g>
    ${id.at({ cx: sep / 2, cy: h / 2 })}
    ${name.at({ left: sep + 12, cy: h / 2 })}`;
};

const dfd: Sketch = {
  nodes: [
    { id: "order", x: 20, y: 20, width: 120, height: 56, label: "Order" },
    { id: "orders", class: "store", x: 300, y: 160, no: "D1", label: "注文" },
  ],
  edges: [{ from: "order", to: "orders", label: "order" }],
  style: { store },
};

describe("svg and text", () => {
  /**
   * Given strings, a fragment and a placed text embedded in svg
   * When the fragment is written out
   * Then strings are escaped, the fragment and the text's <text> element go in as they are
   */
  test("escapes strings and embeds fragments and texts as SVG", () => {
    const inner = svg`<rect width="10" height="10"/>`;
    const out = String(svg`<g>${"a & <b>"}${inner}${text("x").at({ left: 4, top: 0 })}</g>`);

    expect(out).toBe(
      '<g>a &amp; &lt;b&gt;<rect width="10" height="10"/><text x="4" y="8.5" font-size="14" text-anchor="start" dominant-baseline="central">x</text></g>',
    );
  });

  /**
   * Given a text placed by its centre and one by its right and bottom
   * When they are written out
   * Then the anchor follows the point named across, and y is the line's centre
   */
  test("places a text by the point of its box named across and down", () => {
    expect(String(text("注文").at({ cx: 50, cy: 20 }))).toContain('x="50" y="20" font-size="14" text-anchor="middle"');
    expect(String(text("a").at({ right: 50, bottom: 34 }))).toContain('x="50" y="25.5" font-size="14" text-anchor="end"');
    expect(text("注文").width).toBe(28);
    expect(text("ab").width).toBe(17);
  });

  /**
   * Given a place naming two points across, or none down
   * When the text is placed
   * Then it throws, naming the text
   */
  test("rejects a place that does not name one point across and one down", () => {
    expect(() => text("a").at({ left: 0, cx: 1, top: 0 } as never)).toThrow('text "a"');
    expect(() => text("a").at({ left: 0 } as never)).toThrow(TypeError);
  });
});

describe("render with style", () => {
  /**
   * Given a node of a class drawn by its style, joined by an edge to a box
   * When it is rendered
   * Then the node lies around its drawing, which comes out as drawn, and the edge meets it orthogonally
   */
  test("draws styled nodes with their style and routes edges to them", () => {
    const image = render(dfd);

    const node = image.report.diagram.nodes.find((node) => node.id === "orders")!;
    const sep = text("D1").width + 24;
    expect([node.bounds.width, node.bounds.height]).toEqual([sep + text("注文").width + 24, text("注文").height + 20]);
    expect(String(image)).toContain(`<line x1="${sep}" y1="0" x2="${sep}"`);
    const edge = image.report.diagram.connectors.find((c) => c.from === "order" && c.to === "orders")!;
    expect(edge.points.slice(1).every((p, i) => p.x === edge.points[i]!.x || p.y === edge.points[i]!.y)).toBe(true);
    expect(image.report.issues).toEqual([]);
  });

  /**
   * Given a node whose class has no style, and a sketch with both edges and connectors
   * When it is rendered
   * Then each throws a NicevgError saying what is wrong
   */
  test("rejects a class with no style and edges given twice", () => {
    expect(() => render({ ...dfd, style: {} })).toThrow('nodes[1].class: no style named "store"');
    expect(() => render({ ...dfd, connectors: dfd.edges })).toThrow(NicevgError);
  });

  /**
   * Given root attributes with a font family and a numeric font size
   * When the sketch is rendered
   * Then the root of the SVG carries both
   */
  test("writes root attributes on the svg", () => {
    const image = render({ ...dfd, rootAttrs: { "font-family": "Yu Gothic, sans-serif", "font-size": 13 } });
    const root = String(image).split("\n")[0];

    expect(root).toContain('font-family="Yu Gothic, sans-serif"');
    expect(root).toContain('font-size="13"');
  });

  /**
   * Given a root attribute nicevg writes itself
   * When the sketch is rendered
   * Then it throws, naming the attribute
   */
  test("rejects a root attribute nicevg writes", () => {
    expect(() => render({ ...dfd, rootAttrs: { viewBox: "0 0 10 10" } })).toThrow("rootAttrs.viewBox");
  });
});
