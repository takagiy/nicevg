// `nicevg run` on Bun: one DSL script drawn to one SVG.

import { describe, expect, test } from "bun:test";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";

const cli = join(import.meta.dir, "..", "..", "src", "cli.ts");

/** A git repository (a `.git` directory is all it takes) holding `files`. */
function repository(files: Record<string, string>): string {
  const root = mkdtempSync(join(tmpdir(), "nicevg-run-"));
  mkdirSync(join(root, ".git"));
  for (const [path, content] of Object.entries(files)) {
    mkdirSync(dirname(join(root, path)), { recursive: true });
    writeFileSync(join(root, path), content);
  }
  return root;
}

/** `nicevg run` in `cwd` on `args`, with `stdin`. */
function run(cwd: string, args: string[], stdin = "") {
  const { exitCode, stdout, stderr } = Bun.spawnSync([process.execPath, cli, "run", ...args], { cwd, stdin: Buffer.from(stdin) });
  return { exitCode, stdout: stdout.toString(), stderr: stderr.toString() };
}

/** `nicevg run` in `cwd` on `args`, its report read from stdout. */
function report(cwd: string, args: string[], stdin = "") {
  const result = run(cwd, args, stdin);
  return { ...result, report: result.stdout ? JSON.parse(result.stdout) : undefined };
}

const read = (root: string, path: string) => readFileSync(join(root, path), "utf8");

const FLOW = `import { render } from "nicevg";
import { box } from "nicevg/style/simple";

render({
  nodes: [
    { id: "order", class: "box", x: 20, y: 20, label: "Order" },
    { id: "bill", class: "box", x: 220, y: 20, label: "Bill" },
  ],
  edges: [{ from: "order", to: "bill" }],
  style: { box },
});
`;

describe("nicevg run", () => {
  /**
   * [x] When `nicevg run <script>` runs, it can write the SVG beside the script, its extension replaced by `.svg`, and print the report to stdout.
   * Given docs/flow.ts in a git repository
   * When nicevg run runs on it at the root
   * Then docs/flow.svg holds the diagram, and the report names it with nothing failed or left
   */
  test("writes the SVG beside the script and prints the report", () => {
    const root = repository({ "docs/flow.ts": FLOW });

    const { exitCode, report: result } = report(root, ["docs/flow.ts"]);

    expect(exitCode).toBe(0);
    expect(result).toEqual({ output: "docs/flow.svg", failures: [], remainingIssues: [] });
    expect(read(root, "docs/flow.svg")).toContain('data-node="order"');
  });

  /**
   * [x] When `-o <file>` is given, it can write the SVG there, overwriting what was there.
   * Given flow.ts and an old build/order.svg
   * When nicevg run runs with -o build/order.svg
   * Then build/order.svg holds the diagram, and the report names it
   */
  test("writes the SVG where -o says, overwriting it", () => {
    const root = repository({ "flow.ts": FLOW, "build/order.svg": "old" });

    const { exitCode, report: result } = report(root, ["flow.ts", "-o", "build/order.svg"]);

    expect(exitCode).toBe(0);
    expect(result.output).toBe("build/order.svg");
    expect(read(root, "build/order.svg")).toContain('data-node="order"');
    expect(existsSync(join(root, "flow.svg"))).toBe(false);
  });

  /**
   * [x] When `-o -` is given, it can write the SVG to stdout and the report to stderr.
   * Given flow.ts
   * When nicevg run runs with -o -
   * Then stdout is the SVG, stderr the report, and no file is written
   */
  test("writes the SVG to stdout and the report to stderr with -o -", () => {
    const root = repository({ "flow.ts": FLOW });

    const { exitCode, stdout, stderr } = run(root, ["flow.ts", "-o", "-"]);

    expect(exitCode).toBe(0);
    expect(stdout).toStartWith("<svg");
    expect(stdout).toContain('data-node="order"');
    expect(JSON.parse(stderr)).toEqual({ failures: [], remainingIssues: [] });
    expect(existsSync(join(root, "flow.svg"))).toBe(false);
  });

  /**
   * [x] When `-s <code>` is given, it can draw that DSL to `out.svg` in the current directory.
   * Given a git repository with no script file
   * When nicevg run runs with the DSL as -s
   * Then out.svg holds the diagram
   */
  test("draws the DSL given as -s to out.svg", () => {
    const root = repository({});

    const { exitCode, report: result } = report(root, ["-s", FLOW]);

    expect(exitCode).toBe(0);
    expect(result.output).toBe("out.svg");
    expect(read(root, "out.svg")).toContain('data-node="order"');
  });

  /**
   * [x] When `-s -` is given, it can read the DSL from stdin.
   * Given the DSL on stdin
   * When nicevg run runs with -s - and -o -
   * Then stdout is its SVG
   */
  test("reads the DSL from stdin with -s -", () => {
    const root = repository({});

    const { exitCode, stdout } = run(root, ["-s", "-", "-o", "-"], FLOW);

    expect(exitCode).toBe(0);
    expect(stdout).toContain('data-node="order"');
  });

  /**
   * [x] When both or neither of `<script>` and `-s` are given, it can print the usage and exit 2.
   * Given flow.ts
   * When nicevg run runs with flow.ts and -s, and with neither
   * Then each prints the usage to stderr and exits 2, writing nothing
   */
  test("asks for one script, from a file or -s", () => {
    const root = repository({ "flow.ts": FLOW });

    for (const args of [["flow.ts", "-s", FLOW], []]) {
      const { exitCode, stdout, stderr } = run(root, args);

      expect(exitCode).toBe(2);
      expect(stdout).toBe("");
      expect(stderr).toContain("Usage: nicevg run");
    }
    expect(existsSync(join(root, "flow.svg"))).toBe(false);
  });

  /**
   * [x] When the SVG would overwrite the script, it can refuse and exit 2.
   * Given a script named flow.svg
   * When nicevg run runs on it
   * Then it exits 2 and the script is unchanged
   */
  test("refuses to write the SVG over the script", () => {
    const root = repository({ "flow.svg": FLOW });

    const { exitCode, stderr } = run(root, ["flow.svg"]);

    expect(exitCode).toBe(2);
    expect(stderr).toContain("flow.svg");
    expect(read(root, "flow.svg")).toBe(FLOW);
  });

  /**
   * [x] When the script file does not exist, it can say so and exit 2.
   * Given no file named missing.ts
   * When nicevg run runs on it
   * Then it exits 2 with no report, naming the file on stderr
   */
  test("refuses a script that does not exist", () => {
    const root = repository({});

    const { exitCode, stdout, stderr } = run(root, ["missing.ts"]);

    expect(exitCode).toBe(2);
    expect(stdout).toBe("");
    expect(stderr).toMatch(/^error: .*missing\.ts/);
  });

  /**
   * [x] When run in a subdirectory of the sandbox, the report's paths can be relative to the current directory.
   * Given docs/flow.ts logging a line, run from docs
   * When nicevg run runs on flow.ts there
   * Then the output and the console line's place are relative to docs
   */
  test("reports paths relative to the current directory", () => {
    const root = repository({ "docs/flow.ts": `console.log("hi");\n${FLOW}` });

    const { exitCode, report: result } = report(join(root, "docs"), ["flow.ts"]);

    expect(exitCode).toBe(0);
    expect(result.output).toBe("flow.svg");
    expect(result["console.log"]).toEqual(["flow.ts:1:12: hi"]);
    expect(read(root, "docs/flow.svg")).toContain('data-node="order"');
  });

  /**
   * [x] When the script writes to the console, the report can hold each line as `<file>:<line>:<column>: <message>` under `console.<method>`, the place being where it was called.
   * Given a script that logs a string and an object, and warns from a style module it imports
   * When nicevg run runs on it
   * Then console.log holds both lines placed at their calls, and console.warn the module's line
   */
  test("reports the console by method, each line placed where it was called", () => {
    const style = `export function warned() {\n  console.warn("from style");\n}\n`;
    const code = `import { warned } from "./style.ts";\nconsole.log("width", 120);\n  console.log({ a: 1 });\nwarned();\n${FLOW}`;
    const root = repository({ "flow.ts": code, "style.ts": style });

    const { exitCode, report: result } = report(root, ["flow.ts"]);

    expect(exitCode).toBe(0);
    // QuickJS places a call at its opening parenthesis.
    expect(result["console.log"]).toEqual(["flow.ts:2:12: width 120", 'flow.ts:3:14: {"a":1}']);
    expect(result["console.warn"]).toEqual(["style.ts:2:15: from style"]);
  });

  /**
   * [x] When the script throws, it can report the failure with the stack, write nothing, keep the old SVG and exit 2.
   * Given a script that throws on line 2 and an old flow.svg
   * When nicevg run runs on it
   * Then the failure holds the stack at flow.ts:2, flow.svg is unchanged, and no output is reported
   */
  test("reports a script that throws and keeps the old SVG", () => {
    const root = repository({ "flow.ts": `import { render } from "nicevg";\nthrow new Error("no nodes");\n`, "flow.svg": "old" });

    const { exitCode, report: result } = report(root, ["flow.ts"]);

    expect(exitCode).toBe(2);
    expect(result.output).toBeUndefined();
    expect(result.failures).toHaveLength(1);
    expect(result.failures[0].message).toBe("The diagram script threw an exception");
    expect(result.failures[0].error).toContain("Error: no nodes");
    expect(result.failures[0].error).toContain("flow.ts:2");
    expect(read(root, "flow.svg")).toBe("old");
  });

  /**
   * [x] When the diagram has issues left, the report can list them and the command exit 0.
   * Given a group too narrow for the label of the edge inside it
   * When nicevg run runs on it
   * Then the report lists the label left over the group, and the command succeeds
   */
  test("lists the issues left and succeeds", () => {
    const code = `import { render, svg } from "nicevg";
import { box } from "nicevg/style/simple";
const group = ({ width, height }: { width: number; height: number }) =>
  svg\`<rect data-bounds="true" width="\${width}" height="\${height}" fill="none" stroke="gray"/>\`;
render({
  nodes: [
    { id: "g", children: ["a", "b"], class: "group", x: 0, y: 0, width: 200, height: 300 },
    { id: "a", class: "box", label: "A", x: 60, y: 20 },
    { id: "b", class: "box", label: "B", x: 60, y: 220 },
  ],
  edges: [{ from: "a", to: "b", label: "a long label on the edge, wider than its group" }],
  style: { box, group },
});`;
    const root = repository({ "flow.ts": code });

    const { exitCode, report: result } = report(root, ["flow.ts"]);

    expect(exitCode).toBe(0);
    expect(result.remainingIssues).toEqual([{ code: "label-overlaps-node", message: 'Label "edge-0" overlaps node "g".' }]);
    expect(result.output).toBe("flow.svg");
  });

  /**
   * [x] When the script imports a file in the sandbox directory, it can draw; outside it, it can fail.
   * Given docs/flow.ts importing ../shared/nodes.ts
   * When nicevg run runs on it with the repository as the sandbox, and with docs
   * Then the first draws and the second fails on the import
   */
  test("imports files in the sandbox directory only", () => {
    const code = `import { render } from "nicevg";\nimport { box } from "nicevg/style/simple";\nimport { nodes } from "../shared/nodes.ts";\nrender({ nodes, style: { box } });\n`;
    const nodes = `export const nodes = [{ id: "a", class: "box" as const, x: 0, y: 0, label: "A" }];\n`;
    const root = repository({ "docs/flow.ts": code, "shared/nodes.ts": nodes });

    expect(report(root, ["docs/flow.ts"]).exitCode).toBe(0);
    const { exitCode, report: result } = report(root, ["docs/flow.ts", "--sandbox-dir", "docs"]);
    expect(exitCode).toBe(2);
    expect(result.failures[0].message).toBe("The diagram script imports what it may not");
  });

  /**
   * [x] When the script runs past `--timeout`, it can fail.
   * Given a script looping forever
   * When nicevg run runs on it with --timeout 1
   * Then it fails on the time limit
   */
  test("fails a script running past --timeout", () => {
    const root = repository({ "flow.ts": `import { render } from "nicevg";\nwhile (true) {}\n` });

    const { exitCode, report: result } = report(root, ["flow.ts", "--timeout", "1"]);

    expect(exitCode).toBe(2);
    expect(result.failures[0].message).toBe("The diagram script ran past the time limit");
  });
});
