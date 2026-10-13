// The native library through Bun's FFI.

import { dlopen, FFIType, toArrayBuffer, type Pointer } from "bun:ffi";

import type { Run } from "./native.ts";

export function open(path: string): Run {
  // Bun runs on 64-bit platforms only, where a `usize` is a `u64`.
  const { symbols } = dlopen(path, {
    nicevg_run: { args: [FFIType.u32, FFIType.ptr, FFIType.u64, FFIType.ptr], returns: FFIType.ptr },
    nicevg_free: { args: [FFIType.ptr, FFIType.u64], returns: FFIType.void },
  });
  return (operation, request) => {
    const replyLength = new BigUint64Array(1);
    const reply = symbols.nicevg_run(operation, request, request.length, replyLength) as Pointer;
    const length = Number(replyLength[0]);
    try {
      return new TextDecoder().decode(new Uint8Array(toArrayBuffer(reply, 0, length)));
    } finally {
      symbols.nicevg_free(reply, length);
    }
  };
}
