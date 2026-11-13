// Parents a style draws, sized round their children (docs/adr/parents-fit-their-children.md).

import { describe, expect, test } from "bun:test";

import { NicevgError, type SketchNode, render, svg, text } from "nicevg";
import { box } from "nicevg/style/simple";

/** A node whose style draws it at the width and height it is given. */
interface Framed extends SketchNode {
  width?: number;
  height?: number;
}

/** A frame drawn at the size given, each call's size kept in `calls`. */
function framed(calls: { width?: number; height?: number }[]) {
  return ({ width, height }: Framed) => {
    calls.push({ width, height });
    return svg`<rect data-bounds="true" width="${width}" height="${height}" fill="none" stroke="black"/>`;
  };
}

describe("parents a style draws", () => {
  /**
   * Given a parent with no place or size round one box, drawn by a style
   *   that draws a frame at the size it is given
   * When the sketch is rendered
   * Then the style is called first at the size round the box with 12px
   *   padding, then at the size laying out gave the parent, and the parent
   *   comes out at that size round the box
   */
  test("draws a parent at the size round its children, then at the size it was laid out at", () => {
    const calls: { width?: number; height?: number }[] = [];
    const image = render({
      nodes: [
        { id: "group", children: ["a"], class: "frame" },
        { id: "a", class: "box", x: 100, y: 100, width: 80, height: 40 },
      ],
      style: { frame: framed(calls), box },
    });

    const group = image.report.diagram.nodes.find((node) => node.id === "group")!;
    expect(calls).toEqual([
      { width: 104, height: 64 },
      { width: group.bounds.width, height: group.bounds.height },
    ]);
    expect(group.bounds).toEqual({ x: 88, y: 88, width: 104, height: 64 });
    expect(image.report.issues).toEqual([]);
  });

  /**
   * Given a parent given a width of 300 and a height of 200, round one box
   *   that needs far less
   * When the sketch is rendered
   * Then its style is first called with the size the parent was given, and
   *   the parent comes out at that size
   */
  test("draws a parent first at the size it was given", () => {
    const calls: { width?: number; height?: number }[] = [];
    const image = render({
      nodes: [
        { id: "group", children: ["a"], class: "frame", width: 300, height: 200 },
        { id: "a", class: "box", x: 100, y: 100, width: 80, height: 40 },
      ],
      style: { frame: framed(calls), box },
    });

    const group = image.report.diagram.nodes.find((node) => node.id === "group")!;
    expect(calls[0]).toEqual({ width: 300, height: 200 });
    expect([group.bounds.width, group.bounds.height]).toEqual([300, 200]);
  });

  /**
   * Given a parent inside a parent, round one box
   * When the sketch is rendered
   * Then the inner parent is drawn before the outer one, and the outer one
   *   lies 12px out from the inner one's frame
   */
  test("draws nested parents from the inside out, each round the one inside it", () => {
    const order: string[] = [];
    const frame = ({ id, width, height }: Framed) => {
      order.push(id);
      return svg`<rect data-bounds="true" width="${width}" height="${height}" fill="none" stroke="black"/>`;
    };
    const image = render({
      nodes: [
        { id: "outer", children: ["inner"], class: "frame" },
        { id: "inner", children: ["a"], class: "frame" },
        { id: "a", class: "box", x: 100, y: 100, width: 80, height: 40 },
      ],
      style: { frame, box },
    });

    const bounds = (id: string) => image.report.diagram.nodes.find((node) => node.id === id)!.bounds;
    expect(order.slice(0, 2)).toEqual(["inner", "outer"]);
    expect(bounds("inner")).toEqual({ x: 88, y: 88, width: 104, height: 64 });
    expect(bounds("outer")).toEqual({ x: 76, y: 76, width: 128, height: 88 });
  });

  /**
   * Given a parent whose style writes a title 12px in from its top left,
   *   round one box
   * When the sketch is rendered
   * Then the box lies at least 12px below the title, which stays inside the
   *   parent
   */
  test("keeps a parent's children clear of the texts its style draws", () => {
    const titled = ({ width, height }: Framed) =>
      svg`<rect data-bounds="true" width="${width}" height="${height}" fill="none" stroke="black"/>${text("Orders").at({ left: 12, top: 12 })}`;
    const image = render({
      nodes: [
        { id: "group", children: ["a"], class: "titled" },
        { id: "a", class: "box", x: 100, y: 100, width: 80, height: 40 },
      ],
      style: { titled, box },
    });

    const node = (id: string) => image.report.diagram.nodes.find((node) => node.id === id)!;
    const title = node("group").labelBounds[0]!;
    expect(node("a").bounds.y - (title.y + title.height)).toBeGreaterThanOrEqual(12);
    expect(title.y).toBeGreaterThanOrEqual(node("group").bounds.y);
    expect(image.report.issues).toEqual([]);
  });

  /**
   * Given a parent whose style draws its frame at least 200px wide, round a
   *   box that needs a frame only 104px wide
   * When the sketch is rendered
   * Then the parent is laid out and drawn 200px wide, with no issue
   */
  test("takes the size a style draws a parent at as the least it takes", () => {
    const calls: { width?: number; height?: number }[] = [];
    const wide = ({ width = 0, height }: Framed) => {
      calls.push({ width, height });
      return svg`<rect data-bounds="true" width="${Math.max(width, 200)}" height="${height}" fill="none" stroke="black"/>`;
    };
    const image = render({
      nodes: [
        { id: "group", children: ["a"], class: "wide" },
        { id: "a", class: "box", x: 100, y: 100, width: 80, height: 40 },
      ],
      style: { wide, box },
    });

    const group = image.report.diagram.nodes.find((node) => node.id === "group")!;
    expect(calls.map((call) => call.width)).toEqual([104, 200]);
    expect(group.bounds.width).toBe(200);
    expect(image.report.issues).toEqual([]);
  });

  /**
   * Given a parent whose style marks a port "in" on its left side, 20px
   *   down, and an edge to that port from a box to its left
   * When the sketch is rendered
   * Then the edge ends at the port, with no issue
   */
  test("routes an edge to a port a parent's style marks", () => {
    const ported = ({ width, height }: Framed) =>
      svg`<rect data-bounds="true" width="${width}" height="${height}" fill="none" stroke="black"/><circle data-port="in" cx="0" cy="20" r="0"/>`;
    const image = render({
      nodes: [
        { id: "group", children: ["a"], class: "ported" },
        { id: "a", class: "box", x: 100, y: 100, width: 80, height: 40 },
        { id: "source", class: "box", x: -100, y: 88, width: 80, height: 40 },
      ],
      edges: [{ from: "source", to: "group.in" }],
      style: { ported, box },
    });

    const port = image.report.diagram.nodes.find((node) => node.id === "group")!.ports![0]!;
    const edge = image.report.diagram.connectors[0]!;
    expect(edge.points.at(-1)).toEqual(port.point);
    expect(image.report.issues).toEqual([]);
  });

  /** A parent whose style writes a title `top(call)` px down on its `call`th call, 12px in from its left. */
  function titledAt(top: (call: number) => number) {
    let calls = 0;
    const style = ({ width, height }: Framed) => {
      calls += 1;
      return svg`<rect data-bounds="true" width="${width}" height="${height}" fill="none" stroke="black"/>${text("Orders").at({ left: 12, top: top(calls) })}`;
    };
    const image = render({
      nodes: [
        { id: "group", children: ["a"], class: "titled" },
        { id: "a", class: "box", x: 100, y: 100, width: 80, height: 40 },
      ],
      style: { titled: style, box },
    });
    return { image, calls };
  }

  /**
   * Given a parent whose style writes its title 12px down when first drawn,
   *   to lay it out by, and 40px down when drawn again
   * When the sketch is rendered
   * Then it is laid out once more by the title 40px down: the style is
   *   called a third time, the box lies at least 12px below that title, and
   *   no issue is left
   */
  test("lays a sketch out again when a parent's texts move once it is drawn at its size", () => {
    const { image, calls } = titledAt((call) => (call === 1 ? 12 : 40));

    const node = (id: string) => image.report.diagram.nodes.find((node) => node.id === id)!;
    const title = node("group").labelBounds[0]!;
    expect(calls).toBe(3);
    expect(title.y - node("group").bounds.y).toBeGreaterThan(30);
    expect(node("a").bounds.y - (title.y + title.height)).toBeGreaterThanOrEqual(12);
    expect(image.report.issues).toEqual([]);
  });

  /**
   * Given a parent whose style writes its title further down each time it
   *   is drawn, 12px, then 40px, then 68px
   * When the sketch is rendered
   * Then after laying it out once more, the parent is reported as drawn
   *   with its texts elsewhere than it was laid out by
   */
  test("reports a parent still drawn otherwise than laid out after laying out again", () => {
    const { image, calls } = titledAt((call) => 12 + 28 * (call - 1));

    expect(calls).toBe(3);
    expect(image.report.issues.filter((issue) => issue.code === "parent-drawn-differently")).toEqual([
      {
        code: "parent-drawn-differently",
        message: 'The style of parent "group" drew it with its texts elsewhere than it was laid out by.',
        elements: ["group"],
      },
    ]);
  });

  /**
   * Given a parent whose style throws on its first call, and one whose style
   *   throws on its second, once the sketch is laid out
   * When each sketch is rendered
   * Then each throws a NicevgError naming the parent and its style
   */
  test("names the parent whose style throws, before laying out or after", () => {
    const throwingOn = (failing: number) => {
      let calls = 0;
      return ({ width, height }: Framed) => {
        calls += 1;
        if (calls === failing) {
          throw new Error("no frame today");
        }
        return svg`<rect data-bounds="true" width="${width}" height="${height}" fill="none" stroke="black"/>`;
      };
    };
    const sketch = (frame: ReturnType<typeof throwingOn>) => ({
      nodes: [
        { id: "a", class: "box", x: 100, y: 100, width: 80, height: 40 },
        { id: "group", children: ["a"], class: "frame" },
      ],
      style: { frame, box },
    });

    for (const failing of [1, 2]) {
      expect(() => render(sketch(throwingOn(failing)))).toThrow(NicevgError);
      expect(() => render(sketch(throwingOn(failing)))).toThrow('nodes[1]: the style "frame" threw Error: no frame today');
    }
  });

  /**
   * Given a parent drawn by box with labelPosition top-left and padding 20,
   *   and one with labelPosition top-left and no padding, each round a box
   * When each sketch is rendered
   * Then each title sits its padding in from the parent's top left, 20px and
   *   12px, and the box keeps that padding from the parent's left
   */
  test("draws a box's label at its top left with its padding", () => {
    for (const [padding, expected] of [[20, 20], [undefined, 12]] as const) {
      const image = render({
        nodes: [
          { id: "group", children: ["a"], class: "box", label: "Orders", labelPosition: "top-left", padding },
          { id: "a", class: "box", x: 100, y: 100, width: 80, height: 40 },
        ],
        style: { box },
      });

      const node = (id: string) => image.report.diagram.nodes.find((node) => node.id === id)!;
      const [group, title] = [node("group").bounds, node("group").labelBounds[0]!];
      expect(title.x - group.x).toBe(expected);
      // `text` rounds its height up to whole pixels, nicevg measures it 1.2 font sizes: 0.1px lower here.
      expect(title.y - group.y).toBeCloseTo(expected, 0);
      expect(node("a").bounds.x - group.x).toBe(expected);
      expect(image.report.issues).toEqual([]);
    }
  });

  /**
   * Given a parent drawn by box with its label in the middle, placed and sized
   *   as x 0 to 300, y 0 to 200, round a box clear of the label
   * When the sketch is rendered
   * Then the parent keeps the place and size it was given, with no issue
   */
  test("leaves a parent its frame for a text its style writes in the middle", () => {
    const image = render({
      nodes: [
        { id: "host", children: ["a"], class: "box", label: "Host", x: 0, y: 0, width: 300, height: 200 },
        { id: "a", class: "box", x: 20, y: 140, width: 80, height: 40 },
      ],
      style: { box },
    });

    expect(image.report.diagram.nodes.find((node) => node.id === "host")!.bounds).toEqual({ x: 0, y: 0, width: 300, height: 200 });
    expect(image.report.issues).toEqual([]);
  });

  /**
   * Given a parent drawn by box with labelPosition top-left and a title far
   *   wider than the one box it holds
   * When the sketch is rendered
   * Then the parent is wide enough to keep the title 12px inside its right side
   */
  test("widens a box titled at its top left to its title", () => {
    const image = render({
      nodes: [
        { id: "group", children: ["a"], class: "box", label: "A title much wider than the box", labelPosition: "top-left" },
        { id: "a", class: "box", x: 100, y: 100, width: 40, height: 40 },
      ],
      style: { box },
    });

    const group = image.report.diagram.nodes.find((node) => node.id === "group")!;
    const title = group.labelBounds[0]!;
    expect(group.bounds.x + group.bounds.width - (title.x + title.width)).toBeGreaterThanOrEqual(12);
    expect(image.report.issues).toEqual([]);
  });
});
