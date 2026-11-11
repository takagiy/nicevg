// A child process of `nicevg doc`: lays out the diagram whose request comes
// on stdin with the native library, and writes the library's reply to stdout.
// The native layout cannot be stopped from within; `nicevg doc` stops it at
// its deadline by ending this process.

import { readFileSync } from "node:fs";

import { call as native } from "./native.ts";

process.stdout.write(native(readFileSync(0)));
