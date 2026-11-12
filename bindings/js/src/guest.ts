// The module `nicevg` as a diagram script in `nicevg doc` sees it, run in the
// sandbox: drawing as the library draws, and a `render` that records the one
// diagram the script describes for the host to lay out, rather than laying it
// out itself.

import { NicevgError, sketchRequest } from "./sketch.ts";

export { SvgFragment, Text, svg, text } from "./draw.ts";
export { NicevgError } from "./sketch.ts";

/** Where `render` leaves its request and the stack it was called from, for the host to read. */
const RECORD = "__nicevgRender";
/** Where `render` leaves, for the host to call while it lays the sketch out, how to draw the pictures of its parents. */
const DRAW = "__nicevgDraw";

const unavailable = (what: string) => () => {
  throw new NicevgError(
    `${what} is not available in a diagram block: nicevg doc draws the diagram after the script has run, to the path in the block's first line`,
  );
};

/** What `render` gives back in a diagram block: nothing it would read has been drawn yet. */
const NOT_DRAWN = Object.freeze({
  get report() {
    return unavailable("report")();
  },
  get changes() {
    return unavailable("changes")();
  },
  write: unavailable("write()"),
  toString: unavailable("toString()"),
});

export function render(input: Parameters<typeof sketchRequest>[0] | { svg: string }): typeof NOT_DRAWN {
  const global = globalThis as Record<string, unknown>;
  if (global[RECORD] !== undefined) {
    throw new NicevgError("render was called a second time: a diagram block draws one diagram");
  }
  if ("svg" in input) {
    global[RECORD] = JSON.stringify({ request: { svg: String(input.svg) }, stack: new Error().stack });
    return NOT_DRAWN;
  }
  const { sketch, draw } = sketchRequest(input);
  global[RECORD] = JSON.stringify({ request: { sketch }, stack: new Error().stack });
  global[DRAW] = (asked: string) => JSON.stringify(draw(JSON.parse(asked)));
  return NOT_DRAWN;
}
