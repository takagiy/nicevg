// The package loaded with require() from CommonJS, on Node.

const assert: typeof import("node:assert/strict") = require("node:assert/strict");
const { test }: typeof import("node:test") = require("node:test");

/**
 * Given a CommonJS module
 * When it loads nicevg and its simple styles with require() and renders a sketch
 * Then it gets an Svg like an ES module does
 */
test("loads with require() from CommonJS", () => {
  const { render, Svg }: typeof import("nicevg") = require("nicevg");
  const { box }: typeof import("nicevg/style/simple") = require("nicevg/style/simple");

  const svg = render({ nodes: [{ id: "a", class: "box", x: 0, y: 0, width: 100, height: 50, label: "A" }], style: { box } });

  assert.ok(svg instanceof Svg);
  assert.match(svg.toString(), /^<svg/);
});
