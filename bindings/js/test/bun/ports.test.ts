// Edges written to a port of a node as "node.port".

import { describe, expect, test } from "bun:test";

import { render, svg } from "nicevg";

/** A table 120 wide and 96 high, with its port on both sides 62px down. */
const table = ({ port }: { port: string }) => svg`
  <rect data-bounds="true" width="120" height="96" fill="white" stroke="black"/>
  <circle data-port="${port}" cx="0" cy="62" r="0"/>
  <circle data-port="${port}" cx="120" cy="62" r="0"/>`;

const connector = (image: ReturnType<typeof render>) => image.report.diagram.connectors[0]!;

describe("edges to ports", () => {
  /**
   * Given an edge from "article.author" to "user.id"
   * When it is rendered
   * Then it joins article to user, from the port "author" to the port "id"
   */
  test("takes the part before the dot as the node and after it as the port", () => {
    const image = render({
      nodes: [
        { id: "article", class: "table", x: 20, y: 20, port: "author" },
        { id: "user", class: "table", x: 300, y: 20, port: "id" },
      ],
      edges: [{ from: "article.author", to: "user.id" }],
      style: { table },
    });

    const { from, fromPort, to, toPort } = connector(image);
    expect({ from, fromPort, to, toPort }).toEqual({ from: "article", fromPort: "author", to: "user", toPort: "id" });
  });

  /**
   * Given an edge from "blog.article.author", whose node id has a dot of its own
   * When it is rendered
   * Then the last dot splits it: the node "blog.article" and its port "author"
   */
  test("splits an end with several dots at the last one", () => {
    const image = render({
      nodes: [
        { id: "blog.article", class: "table", x: 20, y: 20, port: "author" },
        { id: "user", class: "table", x: 300, y: 20, port: "id" },
      ],
      edges: [{ from: "blog.article.author", to: "user.id" }],
      style: { table },
    });

    const { from, fromPort } = connector(image);
    expect({ from, fromPort }).toEqual({ from: "blog.article", fromPort: "author" });
  });

  /**
   * Given an edge from "blog.article", the id of a node with a dot in it
   * When it is rendered
   * Then it leaves that node, at no port
   */
  test("joins a node whose id has a dot to the edge naming it whole", () => {
    const image = render({
      nodes: [
        { id: "blog.article", class: "table", x: 20, y: 20, port: "author" },
        { id: "user", class: "table", x: 300, y: 20, port: "id" },
      ],
      edges: [{ from: "blog.article", to: "user.id" }],
      style: { table },
    });

    const { from, fromPort } = connector(image);
    expect({ from, fromPort }).toEqual({ from: "blog.article", fromPort: undefined });
  });
});
