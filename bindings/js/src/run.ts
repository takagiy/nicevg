// `nicevg run`: draws the diagram one DSL script renders, in the sandbox as
// `nicevg doc` draws a block (see doc.ts), and writes its SVG.

import { basename, dirname, extname, join, resolve } from "node:path";

import { UsageError, cutAt, drawScript, gitRoot, outside, readInput, replace, shown } from "./doc.ts";

export interface RunOptions {
  /** The script's file, or its code with `inline`. */
  script: string;
  inline: boolean;
  /** Where the SVG goes: a file, `-` for the caller, or beside the script when not given. */
  output?: string;
  sandboxDir?: string;
  sandboxMemoryMB: number;
  timeoutSeconds: number;
}

/** The JSON report: paths relative to the current directory, and what the script wrote by `console.<method>`. */
export interface RunReport {
  output?: string;
  failures: { message: string; error?: string }[];
  remainingIssues: { code: string; message: string }[];
  [console: `console.${string}`]: string[];
}

/** What one script may write to the console. */
const CONSOLE_BYTES = 1024 * 1024;
/** The name an inline script runs under, in the current directory. */
const INLINE = "<script>";

/** Draws the script of `options`; the SVG comes back, not written, when the output is `-`. */
export async function runScript(options: RunOptions): Promise<{ report: RunReport; svg?: string }> {
  const cwd = process.cwd();
  const path = resolve(options.inline ? INLINE : options.script);
  const output = options.output ?? (options.inline ? "out.svg" : join(dirname(options.script), `${basename(options.script, extname(options.script))}.svg`));
  const target = output === "-" ? undefined : resolve(output);
  if (target === path) {
    throw new UsageError(`${output} is the script: give another output with -o`);
  }
  const root = options.sandboxDir === undefined ? gitRoot(path) : resolve(options.sandboxDir);
  if (root === undefined) {
    throw new UsageError(`${options.inline ? cwd : path} is in no git repository: give the sandbox directory with --sandbox-dir`);
  }
  if (outside(root, path)) {
    throw new UsageError(`${options.inline ? cwd : path} is outside the sandbox directory ${root}`);
  }

  // The sandbox names files relative to its directory; the report, relative to the current one.
  const here = (file: string) => shown(cwd, join(root, file));
  const frames = (text: string) => text.replace(/^(\s+at .*?\(?)([^\s()]+)(?=:\d+(?::\d+)?\)?$)/gm, (_, lead: string, file: string) => lead + here(file));
  const report: RunReport = { failures: [], remainingIssues: [] };
  const console: Record<string, string[]> = {};
  const say = cutAt(CONSOLE_BYTES, (text: string, call: { method: string; place?: string }) => {
    const place = call.place?.replace(/^(.*)(:\d+:\d+)$/, (_, file: string, at: string) => here(file) + at);
    (console[`console.${call.method}`] ??= []).push(place ? `${place}: ${text}` : text);
  });
  const code = options.inline ? options.script : readInput(path);
  const drawn = await drawScript(
    root,
    { file: shown(root, path), line: 1, code, modules: new Map() },
    { timeoutSeconds: options.timeoutSeconds, memoryMB: options.sandboxMemoryMB },
    say,
  );
  if ("failure" in drawn) {
    report.failures.push({ message: drawn.failure, ...(drawn.error && { error: frames(drawn.error) }) });
    return { report: { ...report, ...console } };
  }
  report.remainingIssues.push(...drawn.issues.map(({ code, message }) => ({ code, message })));
  if (target === undefined) {
    return { report: { ...report, ...console }, svg: drawn.svg };
  }
  replace(target, drawn.svg);
  return { report: { output: shown(cwd, target), ...report, ...console } };
}
