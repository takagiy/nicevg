// Running a diagram script for `nicevg doc` in QuickJS compiled to
// WebAssembly: the script reaches no file, network or process of the host,
// only the modules this file hands it, and stops at a deadline and a memory
// limit.

import { existsSync, readFileSync, realpathSync } from "node:fs";
import { dirname, join, posix, sep } from "node:path";
import { fileURLToPath } from "node:url";

import amaro from "amaro";
import { type QuickJSContext, type QuickJSHandle, getQuickJS, shouldInterruptAfterDeadline } from "quickjs-emscripten";

/** A diagram script: its code, the markdown line it starts on, and the module blocks it may import. */
export interface Script {
  /** The markdown file, relative to the sandbox directory, `/`-separated. */
  file: string;
  /** The markdown line the code's first line is on. */
  line: number;
  code: string;
  /** Module blocks of the same file by name, each with the line its code starts on. */
  modules: Map<string, { line: number; code: string }>;
}

export interface Limits {
  /** When the script must have finished, in `Date.now()` milliseconds. */
  deadline: number;
  memoryBytes: number;
}

/** What the script asked `render` to draw and where it called it, or why it drew nothing. */
export type Outcome = { request: unknown; stack: string } | { failure: string; error?: string };

/** The sandbox's own modules: `nicevg` as a diagram script sees it, and the files it imports. */
// Named without their extension: the package runs from src/*.ts on Bun and from dist/*.js on Node.
const PACKAGE: Record<string, string> = { nicevg: "nicevg:guest", "nicevg/style/simple": "nicevg:style/simple" };
const SOURCE = dirname(fileURLToPath(import.meta.url));
/** What the guest `nicevg` leaves for the host (see guest.ts). */
const RECORD = "__nicevgRender";
/** A module name the normalizer gives an import it refuses, carrying why: the loader reports it. */
const REFUSED = "refused:";

/** A syntax error in a diagram script's TypeScript, printed as JavaScript prints one. */
class StripError extends Error {}

/** `code` with its types stripped, its lines and columns kept; `file` and `line` place an error. */
function strip(code: string, file: string, line = 1): string {
  try {
    return amaro.transformSync(code, { mode: "strip-only" }).code;
  } catch (error) {
    const { message, startLine, startColumn } = error as { message: string; startLine?: number; startColumn?: number };
    const place = startLine === undefined ? file : `${file}:${line + startLine - 1}:${(startColumn ?? 0) + 1}`;
    throw new StripError(`SyntaxError: ${message}\n    at ${place}`);
  }
}

/** `code` moved down to `line`, so that the lines QuickJS reports are the markdown's. */
const at = (line: number, code: string) => "\n".repeat(line - 1) + code;

/** Where an import of `requested` from the module `base` leads, or why it may not. */
function normalize(root: string, script: Script, base: string, requested: string): string {
  const refuse = (why: string) => `${REFUSED}${base} cannot import "${requested}": ${why}`;
  if (requested in PACKAGE) {
    return PACKAGE[requested]!;
  }
  if (base.startsWith("nicevg:")) {
    return `nicevg:${posix.join(posix.dirname(base.slice("nicevg:".length)), requested).replace(/\.[jt]s$/, "")}`;
  }
  const file = base.split("#")[0]!;
  if (requested.startsWith("#")) {
    return file === script.file && script.modules.has(requested.slice(1))
      ? `${file}${requested}`
      : refuse(`${file} has no <!-- nicevg-module ${requested.slice(1)} --> block`);
  }
  if (!requested.startsWith("./") && !requested.startsWith("../")) {
    return refuse("a diagram script imports nicevg, nicevg/style/simple, #name or a relative path");
  }
  const path = posix.normalize(posix.join(posix.dirname(file), requested));
  if (path.startsWith("../")) {
    return refuse("it lies outside the sandbox directory");
  }
  if (!/\.[jt]s$/.test(path)) {
    return refuse("only .ts and .js files can be imported");
  }
  // Compared ignoring case, as Windows and macOS find .GIT where .git is.
  if (path.toLowerCase().split("/").some((part) => part === ".git" || part === "node_modules")) {
    return refuse("files in .git and node_modules cannot be imported");
  }
  const full = join(root, path);
  if (!existsSync(full)) {
    return refuse("there is no such file");
  }
  if (!(realpathSync(full) + sep).startsWith(realpathSync(root) + sep)) {
    return refuse("it leads outside the sandbox directory");
  }
  return path;
}

/** The source of the module named `name`. */
function load(root: string, script: Script, name: string): string {
  if (name.startsWith(REFUSED)) {
    throw new Error(name.slice(REFUSED.length));
  }
  if (name.startsWith("nicevg:")) {
    // The package's own code, trusted, may use TypeScript that only compiles to JavaScript.
    const path = join(SOURCE, name.slice("nicevg:".length));
    const source = existsSync(`${path}.ts`) ? `${path}.ts` : `${path}.js`;
    return amaro.transformSync(readFileSync(source, "utf8"), { mode: "transform" }).code;
  }
  const [file, block] = name.split("#");
  if (block !== undefined) {
    const module = script.modules.get(block)!;
    return at(module.line, strip(module.code, file!, module.line));
  }
  const code = readFileSync(join(root, file!), "utf8");
  return file!.endsWith(".ts") ? strip(code, file!) : code;
}

/** An error as JavaScript prints it, without the frames of the sandbox's own modules. */
function printed(context: QuickJSContext, handle: QuickJSHandle): { name: string; message: string; text: string } {
  const error = context.dump(handle) as { name?: string; message?: string; stack?: string } | string;
  if (typeof error !== "object" || error === null) {
    return { name: "Error", message: String(error), text: String(error) };
  }
  const frames = (error.stack ?? "").split("\n").filter((frame) => frame.trim() && !frame.includes("nicevg:"));
  const name = error.name ?? "Error";
  const message = error.message ?? "";
  return { name, message, text: [`${name}: ${message}`, ...frames].join("\n") };
}

/** Why a script stopped with `error`. */
function failed(error: { name: string; message: string; text: string }): { failure: string; error?: string } {
  if (error.name === "InternalError" && error.message === "interrupted") {
    return { failure: "The diagram script ran past the time limit" };
  }
  if (error.name === "InternalError" && /out of memory/.test(error.message)) {
    return { failure: "The diagram script ran out of sandbox memory" };
  }
  if (error.name === "SyntaxError") {
    return { failure: "The diagram script has a syntax error", error: error.text };
  }
  // What the module loader refuses or cannot strip comes back as the message of a plain Error (see load).
  if (error.message.startsWith("SyntaxError: ")) {
    return { failure: "The diagram script has a syntax error", error: error.message };
  }
  if (/ cannot import "/.test(error.message)) {
    return { failure: "The diagram script imports what it may not", error: error.text };
  }
  return { failure: "The diagram script threw an exception", error: error.text };
}

/** Runs `script` in a fresh sandbox, `print` taking what it writes to the console. */
export async function run(root: string, script: Script, limits: Limits, print: (text: string) => void): Promise<Outcome> {
  let code: string;
  try {
    code = at(script.line, strip(script.code, script.file, script.line));
  } catch (error) {
    return { failure: "The diagram script has a syntax error", error: (error as StripError).message };
  }
  const runtime = (await getQuickJS()).newRuntime();
  runtime.setMemoryLimit(limits.memoryBytes);
  runtime.setInterruptHandler(shouldInterruptAfterDeadline(limits.deadline));
  runtime.setModuleLoader(
    (name) => {
      try {
        return load(root, script, name);
      } catch (error) {
        return { error: error as Error };
      }
    },
    (base, requested) => normalize(root, script, base, requested),
  );
  const context = runtime.newContext();
  try {
    const console = context.newObject();
    for (const level of ["log", "info", "warn", "error", "debug"]) {
      const write = context.newFunction(level, (...args) =>
        print(args.map((arg) => (context.typeof(arg) === "string" ? context.getString(arg) : JSON.stringify(context.dump(arg)))).join(" ")),
      );
      context.setProp(console, level, write);
      write.dispose();
    }
    context.setProp(context.global, "console", console);
    console.dispose();

    const result = context.evalCode(code, script.file, { type: "module" });
    if (result.error) {
      const outcome = failed(printed(context, result.error));
      result.error.dispose();
      return outcome;
    }
    runtime.executePendingJobs();
    const state = context.getPromiseState(result.value);
    const rejected = state.type === "rejected" ? failed(printed(context, state.error)) : undefined;
    if (!("notAPromise" in state && state.notAPromise)) {
      (state.type === "rejected" ? state.error : state.type === "fulfilled" ? state.value : undefined)?.dispose();
    }
    result.value.dispose();
    if (rejected) {
      return rejected;
    }
    const record = context.getProp(context.global, RECORD);
    const recorded = context.typeof(record) === "string" ? context.getString(record) : undefined;
    record.dispose();
    if (recorded === undefined) {
      return { failure: "The diagram script did not call render" };
    }
    const { request, stack } = JSON.parse(recorded) as { request: unknown; stack: string };
    return { request, stack };
  } finally {
    context.dispose();
    runtime.dispose();
  }
}
