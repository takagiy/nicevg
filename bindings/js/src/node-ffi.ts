// The native library through koffi, for Node.

import koffi from "koffi";

import type { Run } from "./native.ts";

export function open(path: string): Run {
  const library = koffi.load(path);
  const run = library.func(
    "void *nicevg_run(uint32_t operation, const uint8_t *request, size_t length, _Out_ size_t *reply_length)",
  );
  const free = library.func("void nicevg_free(void *reply, size_t length)");
  return (operation, request) => {
    const replyLength = [0];
    const reply = run(operation, request, request.length, replyLength);
    const length = Number(replyLength[0]);
    try {
      return new TextDecoder().decode(new Uint8Array(koffi.view(reply, length)));
    } finally {
      free(reply, length);
    }
  };
}
