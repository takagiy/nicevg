// `nicevg doc`: draws the diagrams written as diagram scripts in
// `<!-- nicevg … -->` comments of markdown files. Each script runs in the
// sandbox (sandbox.ts); the diagram it renders is laid out in a child process
// (render-child.ts) and written to the SVG its first line names, or into the
// markdown, between output markers after the block.

import { spawn } from "node:child_process";
import { existsSync, lstatSync, mkdirSync, readFileSync, realpathSync, renameSync, writeFileSync } from "node:fs";
import { basename, dirname, extname, isAbsolute, join, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

import type { Html, Nodes } from "mdast";
import { fromMarkdown } from "mdast-util-from-markdown";

import { type Script, run } from "./sandbox.ts";

export interface DocOptions {
  dryRun: boolean;
  sandboxDir?: string;
  sandboxMemoryMB: number;
  timeoutPerDiagramSeconds: number;
  maxSvgSizeMB: number;
}

export interface DocResult {
  changedFiles: string[];
  failures: { file: string; line: number; message: string; error?: string }[];
  remainingIssues: { file: string; line: number; code: string; message: string }[];
}

/** Arguments `nicevg doc` cannot work from, such as files outside any git repository. */
export class UsageError extends Error {}

const BLOCK = /^<!--[ \t]*nicevg(-module)?(?=[ \t\r\n])/;
const OUTPUT_START = "<!-- nicevg:output -->";
const OUTPUT_END = "<!-- /nicevg:output -->";
const CHILD = fileURLToPath(new URL(`./render-child${extname(fileURLToPath(import.meta.url))}`, import.meta.url));
/** What one diagram block's console may write to stderr. */
const CONSOLE_BYTES = 1024 * 1024;

/** A `<!-- nicevg` comment at the top level of a markdown file. */
interface Block {
  module: boolean;
  /** Its first line after `nicevg`. */
  header: string;
  code: string;
  /** The markdown line of `<!--`, and the line its code starts on. */
  line: number;
  codeLine: number;
  /** Where the comment ends in the markdown text. */
  end: number;
}

/** The `.git`-holding directory `path` lies in, if any. */
function gitRoot(path: string): string | undefined {
  for (let dir = dirname(path); ; dir = dirname(dir)) {
    if (existsSync(join(dir, ".git"))) {
      return dir;
    }
    if (dirname(dir) === dir) {
      return undefined;
    }
  }
}

/** `path` relative to `root`, `/`-separated. */
const shown = (root: string, path: string) => relative(root, path).split(sep).join("/");

/** Whether `path` lies outside `root`: above it, or on another drive. */
function outside(root: string, path: string): boolean {
  const inside = shown(root, path);
  return inside.startsWith("../") || isAbsolute(inside);
}

/** The nicevg comments in `markdown`, and those not at its top level. */
function blocksOf(markdown: string): { blocks: Block[]; nested: number[] } {
  const tree = fromMarkdown(markdown);
  const blocks: Block[] = [];
  const nested: number[] = [];
  const visit = (node: Nodes, top: boolean) => {
    if (node.type === "html" && BLOCK.test(node.value)) {
      if (top) {
        blocks.push(block(markdown, node));
      } else {
        nested.push(node.position!.start.line);
      }
    }
    if ("children" in node) {
      for (const child of node.children) {
        visit(child, node.type === "root");
      }
    }
  };
  visit(tree, false);
  return { blocks, nested };
}

function block(markdown: string, node: Html): Block {
  const start = node.position!.start.offset!;
  const close = markdown.indexOf("-->", start);
  const text = markdown.slice(start, close);
  const newline = text.indexOf("\n");
  const first = newline < 0 ? text : text.slice(0, newline);
  const [, module] = BLOCK.exec(first)!;
  return {
    module: module !== undefined,
    header: first.replace(BLOCK, "").trim(),
    code: newline < 0 ? "" : text.slice(newline + 1),
    line: node.position!.start.line,
    codeLine: node.position!.start.line + 1,
    end: close + "-->".length,
  };
}

/** A block's first line read: the SVG it is drawn to, if any, and the text describing it. */
function header(text: string): { output?: string; description: string } {
  const quoted = /^"(?:[^"\\]|\\.)*"/.exec(text);
  const first = quoted ? (JSON.parse(quoted[0]) as string) : text.split(/\s+/)[0]!;
  const rest = text.slice(quoted ? quoted[0].length : first.length).trim();
  if (first.endsWith(".svg")) {
    return { output: first, description: rest || basename(first, ".svg") };
  }
  return { description: text };
}

/** Why `path` may not be written, if it may not: it must be an SVG inside `root`, outside `.git`. */
function unwritable(root: string, path: string): string | undefined {
if (outside(root, path)) {
  return "it lies outside the sandbox directory";
}
// Compared ignoring case, as Windows and macOS find .GIT where .git is.
if (shown(root, path).toLowerCase().split("/").includes(".git")) {
  return "files in .git cannot be written";
}
  let existing = path;
  while (!existsSync(existing)) {
    existing = dirname(existing);
  }
  if (!(realpathSync(existing) + sep).startsWith(realpathSync(root) + sep)) {
    return "it leads outside the sandbox directory";
  }
  if (existsSync(path) && lstatSync(path).isSymbolicLink()) {
    return "it is a symbolic link";
  }
  return undefined;
}

/** Lays out `request` in a child process, or gives up at `deadline`. */
/** The native library's reply for a diagram. */
type Reply = { ok: { svg: string; report: { issues: { code: string; message: string }[] } } } | { error: string };

/**
 * Lays out `request` in a child process: its reply, `"timeout"` when stopped
 * at `deadline`, or how the process stopped on its own.
 */
function layOut(request: unknown, deadline: number): Promise<Reply | "timeout" | { stopped: string }> {
  return new Promise((done) => {
    const child = spawn(process.execPath, [CHILD], { stdio: ["pipe", "pipe", "inherit"] });
    const chunks: Buffer[] = [];
    let timedOut = false;
    const timer = setTimeout(
      () => {
        timedOut = true;
        child.kill("SIGKILL");
      },
      Math.max(0, deadline - Date.now()),
    );
    child.stdout.on("data", (chunk: Buffer) => chunks.push(chunk));
    // The process may end before reading its request.
    child.stdin.on("error", () => {});
    child.on("error", (error) => {
      clearTimeout(timer);
      done({ stopped: `as it could not start: ${error.message}` });
    });
    child.on("close", (code, signal) => {
      clearTimeout(timer);
      if (timedOut) {
        done("timeout");
      } else if (code !== 0) {
        done({ stopped: signal ? `on ${signal}` : `with exit code ${code}` });
      } else {
        try {
          done(JSON.parse(Buffer.concat(chunks).toString("utf8")) as Reply);
        } catch {
          done({ stopped: "with a reply that is no JSON" });
        }
      }
    });
    child.stdin.end(JSON.stringify(request));
  });
}

/** Writes `text` to `path` through a file beside it, so a stop midway leaves the old file whole. */
function replace(path: string, text: string): void {
  const temporary = `${path}.nicevg-${process.pid}`;
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(temporary, text);
  renameSync(temporary, path);
}

/** Text in the markdown between output markers, in place of `[from, to)`. */
interface Edit {
  from: number;
  to: number;
  text: string;
}

/** Draws the diagram blocks of the markdown `files`, writing the results unless `options.dryRun`. */
export async function doc(files: string[], options: DocOptions, print: (text: string) => void): Promise<DocResult> {
  const paths = files.map((file) => resolve(file));
  for (const path of paths) {
    if (extname(path) !== ".md") {
      throw new UsageError(`${path}: nicevg doc reads markdown (.md) files`);
    }
  }
  const root = sandboxDir(paths, options.sandboxDir);
  const result: DocResult = { changedFiles: [], failures: [], remainingIssues: [] };
  const changed = (path: string) => result.changedFiles.push(shown(root, path));

  const documents = paths.map((path) => {
    const markdown = readFileSync(path, "utf8");
    return { path, file: shown(root, path), markdown, ...blocksOf(markdown) };
  });
  // Every block writing each SVG, as `file:line`, to fail those that share one.
  const writers = new Map<string, string[]>();
  for (const { path, file, blocks } of documents) {
    for (const block of blocks.filter((block) => !block.module)) {
      const { output } = header(block.header);
      if (output !== undefined && !isAbsolute(output)) {
        const target = resolve(dirname(path), output);
        writers.set(target, [...(writers.get(target) ?? []), `${file}:${block.line}`]);
      }
    }
  }

  for (const { path, file, markdown, blocks, nested } of documents) {
    for (const line of nested) {
      result.failures.push({ file, line, message: "A diagram block must be at the top level of the markdown, not in a list or quote" });
    }
    // Module blocks by name; two of one name fail rather than one hiding the other.
    const named = new Map<string, Block[]>();
    for (const block of blocks.filter((block) => block.module)) {
      const name = block.header.split(/\s+/)[0]!;
      named.set(name, [...(named.get(name) ?? []), block]);
    }
    const modules = new Map<string, { line: number; code: string }>();
    for (const [name, all] of named) {
      if (all.length === 1) {
        modules.set(name, { line: all[0]!.codeLine, code: all[0]!.code });
        continue;
      }
      for (const block of all) {
        const others = all.filter((other) => other !== block).map((other) => `${file}:${other.line}`);
        result.failures.push({ file, line: block.line, message: `${others.join(", ")} is a module block named ${name} too` });
      }
    }
    const edits: Edit[] = [];
    for (const block of blocks.filter((block) => !block.module)) {
      const fail = (message: string, error?: string) => result.failures.push({ file, line: block.line, message, ...(error && { error }) });
      const { output, description } = header(block.header);
      const target = output === undefined ? undefined : resolve(dirname(path), output);
      const refused = target && (isAbsolute(output!) ? "the path must be relative to the markdown file" : unwritable(root, target));
      if (refused) {
        fail(`Cannot write the diagram to "${output}": ${refused}`);
        continue;
      }
      const others = target ? writers.get(target)!.filter((writer) => writer !== `${file}:${block.line}`) : [];
      if (others.length > 0) {
        fail(`${others.join(", ")} writes its diagram to ${shown(root, target!)} too`);
        continue;
      }
      const markers = outputMarkers(markdown, block.end);
      if (markers === undefined) {
        fail(`The output markers after this block are broken: each ${OUTPUT_START} needs a ${OUTPUT_END} after it`);
        continue;
      }

      const deadline = Date.now() + options.timeoutPerDiagramSeconds * 1000;
      let written = 0;
      const script: Script = { file, line: block.codeLine, code: block.code, modules };
      const outcome = await run(root, script, { deadline, memoryBytes: options.sandboxMemoryMB * 1024 * 1024 }, (text) => {
        const before = written;
        written += text.length;
        if (written <= CONSOLE_BYTES) {
          print(`${file}:${block.line}: ${text}\n`);
        } else if (before <= CONSOLE_BYTES) {
          print(`${file}:${block.line}: (console output cut at 1 MB)\n`);
        }
      });
      if ("failure" in outcome) {
        fail(outcome.failure, outcome.error);
        continue;
      }
      const reply = await layOut(outcome.request, deadline);
      if (reply === "timeout") {
        fail(`Gave up drawing the diagram after ${options.timeoutPerDiagramSeconds} s`);
        continue;
      }
      if ("stopped" in reply) {
        fail(`The layout process stopped ${reply.stopped}`);
        continue;
      }
      if ("error" in reply) {
        fail(`The diagram passed to render is invalid: ${reply.error}${calledAt(outcome.stack)}`);
        continue;
      }
      const svg = reply.ok.svg;
      if (Buffer.byteLength(svg) > options.maxSvgSizeMB * 1024 * 1024) {
        fail(`The diagram's SVG is larger than ${options.maxSvgSizeMB} MB`);
        continue;
      }
      for (const issue of reply.ok.report.issues) {
        result.remainingIssues.push({ file, line: block.line, code: issue.code, message: issue.message });
      }
      if (target !== undefined && readIfExists(target) !== svg) {
        changed(target);
        if (!options.dryRun) {
          replace(target, svg);
        }
      }
      const between = output === undefined ? embedded(svg, description) : link(output, description);
      // Without markers yet, the output starts on a line of its own after the block.
      const lead = markers.from === markers.to ? "\n" : "";
      edits.push({ ...markers, text: `${lead}${OUTPUT_START}\n${between}\n${OUTPUT_END}` });
    }
    const rewritten = edits.reduceRight((text, edit) => text.slice(0, edit.from) + edit.text + text.slice(edit.to), markdown);
    if (rewritten !== markdown) {
      changed(path);
      if (!options.dryRun) {
        replace(path, rewritten);
      }
    }
  }
  return result;
}

/** The directory diagram scripts may import from and write to. */
function sandboxDir(paths: string[], given: string | undefined): string {
  if (given !== undefined) {
    const root = resolve(given);
    const outsider = paths.find((path) => outside(root, path));
    if (outsider) {
      throw new UsageError(`${outsider} is outside the sandbox directory ${root}`);
    }
    return root;
  }
  const roots = new Set(
    paths.map((path) => {
      const root = gitRoot(path);
      if (root === undefined) {
        throw new UsageError(`${path} is in no git repository: give the sandbox directory with --sandbox-dir`);
      }
      return root;
    }),
  );
  if (roots.size > 1) {
    throw new UsageError(`the files lie in different git repositories (${[...roots].join(", ")}): run nicevg doc once for each`);
  }
  return [...roots][0]!;
}

/**
 * Where the output after a block ending at `end` goes: between its markers,
 * or, without any, right after the block; `undefined` when broken.
 */
function outputMarkers(markdown: string, end: number): { from: number; to: number } | undefined {
  const after = markdown.slice(end);
  const lead = /^[ \t]*\r?\n/.exec(after);
  const start = lead ? end + lead[0].length : end;
  if (!markdown.startsWith(OUTPUT_START, start)) {
    const next = markdown.indexOf(OUTPUT_START, start);
    const close = markdown.indexOf(OUTPUT_END, start);
    const nextBlock = markdown.slice(start).search(/<!--[ \t]*nicevg[ \t\r\n]/);
    const before = (at: number) => at >= 0 && (nextBlock < 0 || at < start + nextBlock);
    return before(close) && !before(next) ? undefined : { from: end, to: end };
  }
  const close = markdown.indexOf(OUTPUT_END, start);
  return close < 0 ? undefined : { from: start, to: close + OUTPUT_END.length };
}

function readIfExists(path: string): string | undefined {
  return existsSync(path) ? readFileSync(path, "utf8") : undefined;
}

/** An image link to `path`, in angle brackets when it holds a space. */
function link(path: string, description: string): string {
  return `![${description}](${/\s/.test(path) ? `<${path}>` : path})`;
}

/** `svg` for the markdown: no blank line, which would end the HTML block, and `description` as its title. */
function embedded(svg: string, description: string): string {
  const escaped = description.replace(/&/g, "&amp;").replace(/</g, "&lt;");
  const titled = description ? svg.replace(/<svg\b[^>]*>/, (open) => `${open}<title>${escaped}</title>`) : svg;
  return titled
    .split("\n")
    .filter((line) => line.trim())
    .join("\n");
}

/** Where render was called, from the stack it recorded, for a message. */
function calledAt(stack: string): string {
  const frame = stack.split("\n").find((line) => line.trim().startsWith("at") && !line.includes("nicevg:"));
  const place = frame && /\(?([^()\s]+:\d+(?::\d+)?)\)?\s*$/.exec(frame);
  return place ? ` (render was called at ${place[1]})` : "";
}
