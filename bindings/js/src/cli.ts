#!/usr/bin/env node
/**
 * The nicevg command: an SVG diagram on stdin, fixed and arranged by the
 * native library, out on stdout. Issues left go to stderr, one per line;
 * the exit status is 0 with none left, 1 with some, and 2 for bad input.
 */

import { readFileSync, writeSync } from "node:fs";
import { parseArgs } from "node:util";

import { NicevgError, render } from "./index.ts";

const ABOUT =
  "Repair mechanical layout errors in an SVG diagram read from stdin, move nodes a little so connectors bend, cross and crowd less, and write the result to stdout";
const USAGE = "Usage: nicevg\n       nicevg doc [options] <files...>";
const DOC_USAGE = `Usage: nicevg doc [options] <files...>

Draws the diagrams written as diagram scripts in <!-- nicevg ... --> comments of
markdown files, and prints the files it changed, the diagrams that failed and
the issues left in the others as JSON. Diagram scripts run in a sandbox: they
read and write no files but those in the sandbox directory.

Options:
  --dry-run                    Write nothing; report the files that would change
  --sandbox-dir <dir>          The directory diagram scripts may import from and
                               write SVGs to (default: the files' git repository)
  -h, --help                   Print help

Limits apply to each diagram:
  --sandbox-memory <MB>        Memory for its script in the sandbox (default: 256)
  --timeout-per-diagram <sec>  Time for running its script and laying it out
                               (default: 300)
  --max-svg-size <MB>          Size of its SVG (default: 20)
`;

/** Writes to stdout, as nothing once the reader has gone, as `head` does. */
function print(text: string): void {
  try {
    writeSync(1, text);
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code !== "EPIPE") {
      throw error;
    }
  }
}

/** `nicevg doc`: exits 0 when every diagram was drawn, 2 when one failed or the arguments are wrong. */
async function docMain(args: string[]): Promise<number> {
// Loaded here, so that drawing from stdin does not load the sandbox and the markdown parser.
const { UsageError, doc } = await import("./doc.ts");
  let parsed;
  try {
    parsed = parseArgs({
      args,
      allowPositionals: true,
      options: {
        "dry-run": { type: "boolean", default: false },
        "sandbox-dir": { type: "string" },
        "sandbox-memory": { type: "string", default: "256" },
        "timeout-per-diagram": { type: "string", default: "300" },
        "max-svg-size": { type: "string", default: "20" },
        help: { type: "boolean", short: "h", default: false },
      },
    });
  } catch (error) {
    process.stderr.write(`error: ${(error as Error).message}\n\n${DOC_USAGE}`);
    return 2;
  }
  const { values, positionals } = parsed;
  if (values.help) {
    print(DOC_USAGE);
    return 0;
  }
  const limits = ["sandbox-memory", "timeout-per-diagram", "max-svg-size"] as const;
  const bad = limits.find((name) => !(Number(values[name]) > 0));
  if (bad || positionals.length === 0) {
    process.stderr.write(`error: ${bad ? `--${bad} takes a positive number` : "give the markdown files to draw"}\n\n${DOC_USAGE}`);
    return 2;
  }
  try {
    const result = await doc(
      positionals,
      {
        dryRun: values["dry-run"],
        sandboxDir: values["sandbox-dir"],
        sandboxMemoryMB: Number(values["sandbox-memory"]),
        timeoutPerDiagramSeconds: Number(values["timeout-per-diagram"]),
        maxSvgSizeMB: Number(values["max-svg-size"]),
      },
      (text) => process.stderr.write(text),
    );
    print(`${JSON.stringify(result, null, 2)}\n`);
    return result.failures.length > 0 ? 2 : 0;
  } catch (error) {
    if (error instanceof UsageError) {
      process.stderr.write(`error: ${error.message}\n`);
      return 2;
    }
    throw error;
  }
}

function main(args: string[]): number | Promise<number> {
  if (args[0] === "doc") {
    return docMain(args.slice(1));
  }
  for (const arg of args) {
    if (arg === "-h" || arg === "--help") {
      print(`${ABOUT}\n\n${USAGE}\n\nOptions:\n  -h, --help     Print help\n  -V, --version  Print version\n`);
      return 0;
    }
    if (arg === "-V" || arg === "--version") {
      const { version } = JSON.parse(readFileSync(new URL("../package.json", import.meta.url), "utf8"));
      print(`nicevg ${version}\n`);
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
  print(`${svg}\n`);
  for (const issue of svg.report.issues) {
    process.stderr.write(`${issue.code}: ${issue.message}\n`);
  }
  return svg.report.valid ? 0 : 1;
}

// Set rather than exit, so stderr finishes writing first.
process.exitCode = await main(process.argv.slice(2));
