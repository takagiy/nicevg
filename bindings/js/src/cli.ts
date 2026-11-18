#!/usr/bin/env node
/**
  * The nicevg command. `nicevg fix-raw` takes an SVG diagram on stdin, fixed
  * and arranged by the native library, out on stdout. Issues left go to
  * stderr, one per line; the exit status is 0 with none left, 1 with some,
  * and 2 for bad input.
  */

import { readFileSync, writeSync } from "node:fs";
import { parseArgs } from "node:util";

import { NicevgError, render } from "./index.ts";
import { man } from "./man.ts";

const ABOUT = "A diagram tool for AI agents. It adjusts the layout and fixes visual issues, so the agent does not have to.";
const USAGE = "Usage: nicevg doc [options] <files...>\n       nicevg run <script> [options]\n       nicevg fix-raw < <in.svg>\n       nicevg man [<tag> | <text>]...";
const HELP = `${ABOUT}\n\n${USAGE}\n\nOptions:\n  -h, --help     Print help\n  -V, --version  Print version\n`;
const FIX_RAW_USAGE = `Usage: nicevg fix-raw < <in.svg>

Reads an SVG diagram from stdin, fixes it and writes it to stdout. The issues
left go to stderr, one per line. The exit status is 0 with none left, 1 with
some, and 2 for invalid SVG or wrong arguments.

Options:
  -h, --help  Print help
`;
const RUN_USAGE = `Usage: nicevg run <script> [options]
       nicevg run -s <code> [options]

Draws the diagram a DSL script renders, writes its SVG, and prints a JSON report:
the file written, the failure if the script drew nothing, the issues left and
what the script wrote to the console. The script runs in a sandbox: it reads no
files but those in the sandbox directory.

Options:
  -s, --script <code>    The DSL itself, or - to read it from stdin
  -o, --output <file>    Where to write the SVG, or - to write it to stdout and the
                         report to stderr (default: the script's path with .svg
                         for its extension, or out.svg with --script)
  --sandbox-dir <dir>    The directory the script may import from (default: its
                         git repository)
  --sandbox-memory <MB>  Memory for the script (default: 256)
  --timeout <sec>        Time for running the script and laying it out
                         (default: 300)
  -h, --help             Print help
`;
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

/** `nicevg run`: exits 0 when the script drew, 2 when it failed or the arguments are wrong. */
async function runMain(args: string[]): Promise<number> {
  // Loaded here, as for `nicevg doc`.
  const { runScript } = await import("./run.ts");
  const { UsageError } = await import("./doc.ts");
  const usage = (message: string) => {
    process.stderr.write(`error: ${message}\n\n${RUN_USAGE}`);
    return 2;
  };
  let parsed;
  try {
    parsed = parseArgs({
      args,
      allowPositionals: true,
      options: {
        script: { type: "string", short: "s" },
        output: { type: "string", short: "o" },
        "sandbox-dir": { type: "string" },
        "sandbox-memory": { type: "string", default: "256" },
        timeout: { type: "string", default: "300" },
        help: { type: "boolean", short: "h", default: false },
      },
    });
  } catch (error) {
    return usage((error as Error).message);
  }
  const { values, positionals } = parsed;
  if (values.help) {
    print(RUN_USAGE);
    return 0;
  }
  const bad = (["sandbox-memory", "timeout"] as const).find((name) => !(Number(values[name]) > 0));
  if (bad) {
    return usage(`--${bad} takes a positive number`);
  }
  const scripts = positionals.length + (values.script === undefined ? 0 : 1);
  if (scripts !== 1) {
    return usage(scripts === 0 ? "no script given: give a file, or its code with -s" : "more than one script given: give one");
  }
  const inline = values.script !== undefined;
  try {
    const { report, svg } = await runScript({
      script: !inline ? positionals[0]! : values.script === "-" ? readFileSync(0, "utf8") : values.script!,
      inline,
      output: values.output,
      sandboxDir: values["sandbox-dir"],
      sandboxMemoryMB: Number(values["sandbox-memory"]),
      timeoutSeconds: Number(values.timeout),
    });
    const json = `${JSON.stringify(report, null, 2)}\n`;
    if (values.output === "-") {
      print(svg ?? "");
      process.stderr.write(json);
    } else {
      print(json);
    }
    return report.failures.length > 0 ? 2 : 0;
  } catch (error) {
    if (error instanceof UsageError) {
      process.stderr.write(`error: ${error.message}\n`);
      return 2;
    }
    throw error;
  }
}

/** `nicevg man`: reads the help page by tags and text (see man.ts); exits 1 if a query matched nothing. */
function manMain(queries: string[]): number {
  const page = readFileSync(new URL("../doc/nicevg.txt", import.meta.url), "utf8");
  // Emphasis as grep does: on a terminal, or when FORCE_COLOR asks, unless NO_COLOR is set.
  const color = !process.env.NO_COLOR && ((!!process.env.FORCE_COLOR && process.env.FORCE_COLOR !== "0") || process.stdout.isTTY);
  const { text, missing } = man(page, queries, (match) => (color ? `\x1b[1;31m${match}\x1b[0m` : match));
  print(text);
  for (const query of missing) {
    process.stderr.write(`error: no section matches '${query}'\n`);
  }
  return missing.length > 0 ? 1 : 0;
}

/** `nicevg fix-raw`: exits 0 with no issues left, 1 with some, 2 for invalid SVG or wrong arguments. */
function fixRawMain(args: string[]): number {
  if (args.length === 1 && (args[0] === "-h" || args[0] === "--help")) {
    print(FIX_RAW_USAGE);
    return 0;
  }
  if (args.length > 0) {
    process.stderr.write(`error: unexpected argument '${args[0]}' found\n\n${FIX_RAW_USAGE}`);
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

function main(args: string[]): number | Promise<number> {
  const [command, ...rest] = args;
  if (command === "doc") {
    return docMain(rest);
  }
  if (command === "run") {
    return runMain(rest);
  }
  if (command === "fix-raw") {
    return fixRawMain(rest);
  }
  if (command === "man") {
    return manMain(rest);
  }
  if (command === "-h" || command === "--help") {
    print(HELP);
    return 0;
  }
  if (command === "-V" || command === "--version") {
    const { version } = JSON.parse(readFileSync(new URL("../package.json", import.meta.url), "utf8"));
    print(`nicevg ${version}\n`);
    return 0;
  }
  // With no command, help rather than waiting on stdin.
  process.stderr.write(command === undefined ? HELP : `error: unexpected argument '${command}' found\n\n${USAGE}\n\nFor more information, try '--help'.\n`);
  return 2;
}

// Set rather than exit, so stderr finishes writing first.
process.exitCode = await main(process.argv.slice(2));
