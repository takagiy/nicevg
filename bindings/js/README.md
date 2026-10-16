# nicevg

[![npm](https://img.shields.io/npm/v/nicevg)](https://www.npmjs.com/package/nicevg)
[![CI](https://github.com/takagiy/nicevg/actions/workflows/ci.yml/badge.svg)](https://github.com/takagiy/nicevg/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue)](https://github.com/takagiy/nicevg/blob/main/LICENSE)

An SVG diagram tool for AI agents, for TypeScript and JavaScript on Bun and Node. It fixes a diagram's problems mechanically on the agent's behalf, and optimizes the layout while preserving the original intent.

| Before | After |
|---|---|
| ![A service diagram with overlapping boxes, an overflowing label, a diagonal line, lines through a box, a box cut off at the edge and labels far from their lines](https://cdn.jsdelivr.net/gh/takagiy/nicevg@main/docs/images/before.svg) | ![The same diagram repaired by nicevg](https://cdn.jsdelivr.net/gh/takagiy/nicevg@main/docs/images/after.svg) |

## Install

```sh
npm install nicevg
# or
bun add nicevg
```

The package also has the `nicevg` command: install it with `npm install -g nicevg`, or run it once with `npx nicevg` or `bunx nicevg`.

## Example

### CLI

```sh
nicevg < diagram.svg > fixed.svg
```

It reads the SVG from stdin and writes the fixed one to stdout. The issues it could not fix go to stderr, and the exit status is `0` with none left, `1` with some, and `2` for invalid SVG.

### Library

```ts
import { render, svg, text } from "nicevg";
import { box } from "nicevg/style/simple";

const image = render({
  nodes: [
    { id: "order", class: "box", x: 20, y: 20, width: 120, height: 56, label: "Order" },
    { id: "orders", class: "pill", x: 300, y: 160, label: "Orders" },
  ],
  edges: [{ from: "order", to: "orders", label: "save" }],
  style: {
    box,
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

## API

### `render(input)`

Fixes the diagram, nudges its nodes into a cleaner layout and returns an `Svg`. `input` is a sketch (below), `{ svg: string }` with SVG text, or an `Svg` returned before. Bad input throws a `NicevgError` that says which field is wrong, such as `edges[0].to` or `nodes[1].class`.

| Option | |
|---|---|
| `nodes` | The boxes, circles and styled nodes (below). |
| `edges` | The lines between nodes (below). `connectors` is accepted as its old name. |
| `style` | For each `class` used by a node, a function drawing that node with `svg` and `text`. |
| `rootAttrs` | Attributes for the root `<svg>`, such as `{ "font-family": "sans-serif" }`. |

| Node field | |
|---|---|
| `id`, `x`, `y` | Its id and its top left corner. |
| `class` | The `style` that draws it; every node has one. Other fields you add are passed to that style. |
| `parent` | The id of the node it sits inside. |
| `allowOverlap` | `true` if it overlaps other nodes on purpose. |

| Edge field | |
|---|---|
| `from`, `to` | The ids of the nodes it joins. |
| `label`, `fontSize` | Its text, placed beside it. |
| `points` | Its route. Without one, rendering routes it from side to side. |

### `Svg`

| Member | |
|---|---|
| `write(path)` | Writes the SVG to a file. |
| `toString()` | The SVG text. |
| `report.issues` | Problems left after rendering; empty when there are none. |
| `report.diagram` | The diagram read back: `nodes` with their `bounds`, `connectors` with their `points`, and `labels`. |
| `changes` | What rendering fixed and moved, in order. |

### Drawing a style

| Function | |
|---|---|
| `` svg`...` `` | SVG markup. Embedded strings are escaped; `svg` fragments, texts and arrays of them go in as they are. |
| `text(content, size = 14)` | A text, lines split at `\n`, with the `width` and `height` nicevg measures it at: its widest line, and 1.2 font sizes a line. |
| `text(...).at({ left \| cx \| right, top \| cy \| bottom })` | Places the text so that point of its box sits at those coordinates. |
| `` svg`...`.at(x, y) `` | The fragment with its origin moved to `x`, `y`. |

`nicevg/style/simple` has `box` and `circle`, white shapes outlined in black with the label centred, growing to fit their label: `import { box, circle } from "nicevg/style/simple"`, then `style: { box, circle }`. Their nodes take `width`, `height`, `label` (lines split at `\n`) and `fontSize`; a circle sits in the middle of its `width` and `height`.
