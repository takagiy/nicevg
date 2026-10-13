// The native library through koffi, for Node.

import { createRequire } from "node:module";

import type { Render } from "./native.ts";

export function open(path: string): Render {
  // Loaded here rather than imported, so that Bun never loads it.
  const koffi: typeof import("koffi") = createRequire(import.meta.url)("koffi");
  const library = koffi.load(path);
  const render = library.func(
    "void *nicevg_render(const uint8_t *request, size_t length, _Out_ size_t *reply_length)",
  );
  const free = library.func("void nicevg_free(void *reply, size_t length)");
  return (request) => {
    const replyLength = [0];
    const reply = render(request, request.length, replyLength);
    const length = Number(replyLength[0]);
    try {
      // Copied out rather than viewed in place, which Electron forbids.
      const bytes: Uint8Array = koffi.decode(reply, koffi.array("uint8_t", length, "Typed"));
      return new TextDecoder().decode(bytes);
    } finally {
      free(reply, length);
    }
  };
}
