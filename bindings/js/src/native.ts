// Finding and calling the native library, through Bun's FFI on Bun and
// through koffi elsewhere.

import { existsSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { join } from "node:path";

import { open as openOnBun } from "./bun-ffi.ts";
import { open as openOnNode } from "./node-ffi.ts";

/** Fixes and arranges the diagram in a JSON request, returning the JSON reply. */
export type Render = (request: Uint8Array) => string;

const extension = process.platform === "win32" ? "dll" : process.platform === "darwin" ? "dylib" : "so";
const libraryName = process.platform === "win32" ? `nicevg_ffi.${extension}` : `libnicevg_ffi.${extension}`;
/** The package's root: this file sits in `src/`, or compiled in `dist/`. */
const root = fileURLToPath(new URL("..", import.meta.url));

/** Where the native library may be: as set, packaged in the package, or built in this repository. */
function libraryPath(): string {
  const candidates = [
    process.env.NICEVG_LIBRARY,
    join(root, "native", libraryName),
    join(root, "..", "..", "target", "release", libraryName),
  ].filter((candidate): candidate is string => candidate !== undefined);
  const found = candidates.find((candidate) => existsSync(candidate));
  if (!found) {
    throw new Error(`nicevg: no ${libraryName} found; looked at ${candidates.join(", ")}. Build it with \`bun run build\`.`);
  }
  return found;
}

let render: Render | undefined;

/** Fixes and arranges the diagram in `request`, opening the library on first use. */
export function call(request: Uint8Array): string {
  render ??= (typeof Bun === "undefined" ? openOnNode : openOnBun)(libraryPath());
  return render(request);
}
