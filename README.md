# nicevg

[![CI](https://github.com/takagiy/nicevg/actions/workflows/ci.yml/badge.svg)](https://github.com/takagiy/nicevg/actions/workflows/ci.yml)
[![npm](https://img.shields.io/npm/v/nicevg)](https://www.npmjs.com/package/nicevg)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue)](LICENSE)

An SVG diagram tool for AI agents. It fixes a diagram's problems mechanically on the agent's behalf, and optimizes the layout while preserving the original intent.

| Before | After |
|---|---|
| ![A service diagram with overlapping boxes, an overflowing label, a diagonal line, lines through a box, a box cut off at the edge and labels far from their lines](docs/images/before.svg) | ![The same diagram repaired by nicevg](docs/images/after.svg) |

## Install

### CLI (Node 20+ or Bun)

```sh
npm install -g nicevg
# or
brew install takagiy/tap/nicevg
```

To run it once without installing: `npx nicevg < diagram.svg > fixed.svg`, or `bunx nicevg`.

### TypeScript library (Bun, Node)

```sh
npm install nicevg
```

## Examples

### CLI

```sh
nicevg < diagram.svg > fixed.svg
```

### TypeScript library

```ts
import { render } from "nicevg";
import { box, circle } from "nicevg/style/simple";

const image = render({
  nodes: [
    { id: "user", class: "circle", x: 20, y: 20, label: "User", fill: "#fef3c7" },
    { id: "order", class: "box", x: 200, y: 30, label: "Order", fill: "#dbeafe" },
  ],
  edges: [{ from: "user", to: "order", label: "place" }],
  style: { box, circle },
});
await image.write("diagram.svg");
```

#### Styling

A style is a function that draws its nodes as SVG.

```ts
import { render, svg, text } from "nicevg";

const image = render({
  nodes: [{ id: "orders", class: "pill", x: 20, y: 20, label: "Orders" }],
  style: {
    pill: ({ label }) => {
      const name = text(label);
      const w = name.width + 32, h = name.height + 20;
      return svg`
        <rect width="${w}" height="${h}" rx="${h / 2}" fill="white" stroke="black"/>
        ${name.at({ cx: w / 2, cy: h / 2 })}`;
    },
  },
  rootAttrs: { "font-family": "sans-serif" },
});
await image.write("diagram.svg");
```

## Docs

- [SVG contract](docs/reference/svg-contract.md): the attributes nicevg reads (`data-node`, `data-from`, `data-label-for`, ...)
- [Test list](docs/test-list/diagram.md): what nicevg checks, fixes and arranges, one behaviour per line
- [TypeScript library](docs/reference/typescript.md): `render()`, the sketch's fields, `style`, the result and errors
- [Development](docs/guide/development.md)
