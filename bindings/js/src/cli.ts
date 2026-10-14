#!/usr/bin/env node
/**
 * The nicevg command: an SVG diagram on stdin, fixed and arranged by the
 * native library, out on stdout. Issues left go to stderr, one per line;
 * the exit status is 0 with none left, 1 with some, and 2 for bad input.
 */

import { readFileSync } from "node:fs";

import { NicevgError, render } from "./index.ts";

const ABOUT =
  "Repair mechanical layout errors in an SVG diagram read from stdin, move nodes a little so connectors bend, cross and crowd less, and write the result to stdout";
const USAGE = "Usage: nicevg";

function main(args: string[]): number {
  for (const arg of args) {
    if (arg === "-h" || arg === "--help") {
      process.stdout.write(`${ABOUT}\n\n${USAGE}\n\nOptions:\n  -h, --help     Print help\n  -V, --version  Print version\n`);
      return 0;
    }
    if (arg === "-V" || arg === "--version") {
      const { version } = JSON.parse(readFileSync(new URL("../package.json", import.meta.url), "utf8"));
      process.stdout.write(`nicevg ${version}\n`);
      return 0;
    }
  }
  if (args.length > 0) {
    process.stderr.write(`error: unexpected argument '${args[0]}' found\n\n${USAGE}\n\nFor more information, try '--help'.\n`);
    return 2;
  }

  let svg;
  try {
    svg = render({ svg: readFileSync(0, "utf8") });
  } catch (error) {
    if (error instanceof NicevgError || (error instanceof Error && "code" in error)) {
      process.stderr.write(`Invalid SVG: ${error.message}\n`);
      return 2;
    }
    throw error;
  }
  process.stdout.write(`${svg}\n`);
  for (const issue of svg.report.issues) {
    process.stderr.write(`${issue.code}: ${issue.message}\n`);
  }
  return svg.report.valid ? 0 : 1;
}

// Set rather than exit, so stdout and stderr finish writing first.
process.exitCode = main(process.argv.slice(2));
