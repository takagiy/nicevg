// The native library through Bun's FFI.

import { createRequire } from "node:module";

import type { Pointer } from "bun:ffi";

import type { Render } from "./native.ts";

export function open(path: string): Render {
  // Loaded here rather than imported, so that the package loads on Node.
  const { dlopen, FFIType, toArrayBuffer }: typeof import("bun:ffi") = createRequire(import.meta.url)("bun:ffi");
  // Bun runs on 64-bit platforms only, where a `usize` is a `u64`.
  const { symbols } = dlopen(path, {
    nicevg_render: { args: [FFIType.ptr, FFIType.u64, FFIType.ptr], returns: FFIType.ptr },
    nicevg_free: { args: [FFIType.ptr, FFIType.u64], returns: FFIType.void },
  });
  return (request) => {
    const replyLength = new BigUint64Array(1);
    const reply = symbols.nicevg_render(request, request.length, replyLength) as Pointer;
    const length = Number(replyLength[0]);
    try {
      return new TextDecoder().decode(new Uint8Array(toArrayBuffer(reply, 0, length)));
    } finally {
      symbols.nicevg_free(reply, length);
    }
  };
}
