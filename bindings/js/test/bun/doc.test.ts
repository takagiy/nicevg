// `nicevg doc` on Bun: diagrams written as scripts in markdown comments.

import { describe, expect, test } from "bun:test";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";

const cli = join(import.meta.dir, "..", "..", "src", "cli.ts");

/** A git repository (a `.git` directory is all it takes) holding `files`. */
function repository(files: Record<string, string>): string {
  const root = mkdtempSync(join(tmpdir(), "nicevg-doc-"));
  mkdirSync(join(root, ".git"));
  for (const [path, content] of Object.entries(files)) {
    mkdirSync(dirname(join(root, path)), { recursive: true });
    writeFileSync(join(root, path), content);
  }
  return root;
}

/** `nicevg doc` run in `root` on `args`, its JSON result read. */
function doc(root: string, args: string[]) {
  const { exitCode, stdout, stderr } = Bun.spawnSync([process.execPath, cli, "doc", ...args], { cwd: root });
  const out = stdout.toString();
  return { exitCode, stderr: stderr.toString(), result: out ? JSON.parse(out) : undefined };
}

const read = (root: string, path: string) => readFileSync(join(root, path), "utf8");

const ORDER_FLOW = `import { render } from "nicevg";
import { box } from "nicevg/style/simple";

const nodes: { id: string; class: "box"; x: number; y: number; label: string }[] = [
  { id: "order", class: "box", x: 20, y: 20, label: "Order" },
  { id: "bill", class: "box", x: 220, y: 20, label: "Bill" },
];
render({ nodes, edges: [{ from: "order", to: "bill" }], style: { box } });
`;

describe("nicevg doc", () => {
  /**
   * Given a markdown file with a block whose first line names diagrams/order.svg,
   *   written like the README: imports, a typed array and one render call
   * When nicevg doc runs on it
   * Then the SVG is written next to the markdown at that path, and an image
   *   link to it between output markers follows the block
   */
  test("writes a block's diagram to the SVG its first line names and links it", () => {
    const markdown = `# Orders\n\n<!-- nicevg diagrams/order.svg\n${ORDER_FLOW}-->\n\nAfter.\n`;
    const root = repository({ "docs/guide.md": markdown });

    const { exitCode, result } = doc(root, ["docs/guide.md"]);

    expect(exitCode).toBe(0);
    expect(result.failures).toEqual([]);
    expect(read(root, "docs/diagrams/order.svg")).toContain('data-node="order"');
    expect(read(root, "docs/guide.md")).toBe(
      `# Orders\n\n<!-- nicevg diagrams/order.svg\n${ORDER_FLOW}-->\n<!-- nicevg:output -->\n![order](diagrams/order.svg)\n<!-- /nicevg:output -->\n\nAfter.\n`,
    );
    expect(result.changedFiles.sort()).toEqual(["docs/diagrams/order.svg", "docs/guide.md"]);
  });

  /**
   * Given a block whose first line names no SVG but a description
   * When nicevg doc runs
   * Then the SVG goes between output markers after the block, with no blank
   *   line (which would end the markdown's HTML block) and the description as
   *   its title
   */
  test("embeds the SVG of a block naming no file, titled by its first line", () => {
    const root = repository({ "guide.md": `<!-- nicevg Order flow\n${ORDER_FLOW}-->\n` });

    const { exitCode } = doc(root, ["guide.md"]);

    const markdown = read(root, "guide.md");
    const embedded = markdown.slice(markdown.indexOf("<!-- nicevg:output -->"));
    expect(exitCode).toBe(0);
    expect(embedded).toMatch(/^<!-- nicevg:output -->\n<svg[^>]*><title>Order flow<\/title>/);
    expect(embedded).toEndWith("</svg>\n<!-- /nicevg:output -->\n");
    expect(embedded).not.toMatch(/\n\s*\n/);
  });

  /**
   * Given blocks naming their SVG, one with a description after the path and one
   *   with a quoted path holding a space
   * When nicevg doc runs
   * Then the links take the description, or the file's name without one, and
   *   the path with a space goes in angle brackets
   */
  test("links an SVG with the description after its path, in angle brackets when the path has a space", () => {
    const root = repository({
      "guide.md": `<!-- nicevg a.svg The order flow\n${ORDER_FLOW}-->\n\n<!-- nicevg "with space/caf\\u00e9.svg"\n${ORDER_FLOW}-->\n`,
    });

    const { exitCode } = doc(root, ["guide.md"]);

    const markdown = read(root, "guide.md");
    expect(exitCode).toBe(0);
    expect(markdown).toContain("![The order flow](a.svg)");
    expect(markdown).toContain("![café](<with space/café.svg>)");
    expect(read(root, "with space/café.svg")).toContain("<svg");
  });

  /**
   * Given a markdown file drawn once, with text on either side of its block
   * When nicevg doc runs on it again
   * Then nothing outside the output markers changes, and since the diagram is
   *   the same, no file is written and none is reported changed
   */
  test("changes nothing outside the markers, and nothing at all when the diagram is the same", () => {
    const before = "Intro *text*  \n\n<!-- nicevg a.svg\n";
    const after = "-->\n<!-- nicevg:output -->\nstale link\n<!-- /nicevg:output -->\n\n> quote\n";
    const root = repository({ "guide.md": `${before}${ORDER_FLOW}${after}` });

    doc(root, ["guide.md"]);
    const drawn = read(root, "guide.md");
    const again = doc(root, ["guide.md"]);

    expect(drawn).toBe(`${before}${ORDER_FLOW}-->\n<!-- nicevg:output -->\n![a](a.svg)\n<!-- /nicevg:output -->\n\n> quote\n`);
    expect(again.result.changedFiles).toEqual([]);
    expect(read(root, "guide.md")).toBe(drawn);
  });

  /**
   * Given a block shown in a fenced code block and in inline code, as a
   *   document about nicevg doc would show it
   * When nicevg doc runs
   * Then neither is drawn and the markdown stays as it is
   */
  test("leaves blocks in code alone", () => {
    const markdown = `\`\`\`markdown\n<!-- nicevg a.svg\n${ORDER_FLOW}-->\n\`\`\`\n\nInline: \`<!-- nicevg b.svg render() -->\`\n`;
    const root = repository({ "guide.md": markdown });

    const { exitCode, result } = doc(root, ["guide.md"]);

    expect(exitCode).toBe(0);
    expect(result).toEqual({ changedFiles: [], failures: [], remainingIssues: [] });
    expect(read(root, "guide.md")).toBe(markdown);
  });

  /**
   * Given a block inside a list item and one inside a quote
   * When nicevg doc runs
   * Then each fails, asking for the block at the top level
   */
  test("fails a block in a list or a quote", () => {
    const markdown = `- item\n\n  <!-- nicevg a.svg\n  render()\n  -->\n\n> <!-- nicevg b.svg\n> render()\n> -->\n`;
    const root = repository({ "guide.md": markdown });

    const { exitCode, result } = doc(root, ["guide.md"]);

    expect(exitCode).toBe(2);
    expect(result.failures.map((failure: { line: number }) => failure.line)).toEqual([3, 7]);
    expect(result.failures[0].message).toMatch(/top level/);
    expect(read(root, "guide.md")).toBe(markdown);
  });

  /**
   * Given a block followed by an opening output marker with no closing one
   * When nicevg doc runs
   * Then the block fails and the markdown is left as it is
   */
  test("fails a block whose output markers are broken and leaves the markdown", () => {
    const markdown = `<!-- nicevg a.svg\n${ORDER_FLOW}-->\n<!-- nicevg:output -->\n![a](a.svg)\n`;
    const root = repository({ "guide.md": markdown });

    const { exitCode, result } = doc(root, ["guide.md"]);

    expect(exitCode).toBe(2);
    expect(result.failures).toHaveLength(1);
    expect(result.failures[0].message).toMatch(/output markers/);
    expect(read(root, "guide.md")).toBe(markdown);
  });
});

/** A markdown file with one diagram block drawn to a.svg, its code starting on line 2. */
const oneBlock = (code: string) => `<!-- nicevg a.svg\n${code}\n-->\n`;

/** The failures of drawing `code` alone in a block, from a file of `others`. */
function failuresOf(code: string, others: Record<string, string> = {}) {
  const root = repository({ "docs/guide.md": oneBlock(code), ...others });
  return doc(root, ["docs/guide.md"]).result.failures as { file: string; line: number; message: string; error?: string }[];
}

describe("nicevg doc diagram scripts", () => {
  /**
   * Given a block that never calls render, and one that calls it twice
   * When nicevg doc runs
   * Then both fail, the second saying render was called a second time where
   */
  test("fails a block calling render no time or twice", () => {
    const twice = `import { render } from "nicevg";\nrender({ nodes: [] });\nrender({ nodes: [] });`;

    expect(failuresOf(`const nothing = 1;`)).toEqual([
      { file: "docs/guide.md", line: 1, message: "The diagram script did not call render" },
    ]);
    const [failure] = failuresOf(twice);
    expect(failure!.message).toBe("The diagram script threw an exception");
    expect(failure!.error).toMatch(/^NicevgError: render was called a second time[^\n]*\n\s+at .*docs\/guide\.md:4:/);
  });

  /**
   * Given blocks using what render gives back as the library's Svg: write(),
   *   report and toString()
   * When nicevg doc runs
   * Then each fails, saying it is not available in a diagram block
   */
  test("fails a block using write, report or toString of render's result", () => {
    for (const use of ["await image.write('x.svg')", "image.report.issues", "String(image)"]) {
      const [failure] = failuresOf(`import { render } from "nicevg";\nconst image = render({ nodes: [] });\n${use};`);
      expect(failure!.error).toMatch(/^NicevgError: .* is not available in a diagram block/);
    }
  });

  /**
   * Given a block calling a helper in a .ts file it imports, which throws
   * When nicevg doc runs
   * Then the error holds the exception and a stack through the helper's file
   *   and the block's own markdown line and column
   */
  test("reports an exception with a stack in markdown lines and columns", () => {
    const helper = `export function service(id: string): never {\n  throw new TypeError("no " + id);\n}\n`;
    const code = `import { service } from "./helpers.ts";\n\n  service("orders");`;

    const [failure] = failuresOf(code, { "docs/helpers.ts": helper });

    expect(failure!.error).toMatch(/^TypeError: no orders\n\s+at service \(docs\/helpers\.ts:2:\d+\)\n\s+at .*docs\/guide\.md:4:\d+/);
  });

  /**
   * Given a block with a syntax error on its third line, and one using enum,
   *   which stripping types does not turn into JavaScript
   * When nicevg doc runs
   * Then both fail as syntax errors, the first saying where
   */
  test("fails a syntax error, and TypeScript that needs compiling, as syntax errors", () => {
    const [broken] = failuresOf(`const a = 1;\nconst b = 2;\nlet = = 3;`);
    const [typed] = failuresOf(`enum Side { Left }`);
    const [imported] = failuresOf(`import { x } from "./broken.ts";`, { "docs/broken.ts": "export const x: number = ;\n" });

    expect(broken!.message).toBe("The diagram script has a syntax error");
    expect(broken!.error).toMatch(/^SyntaxError: .*\n\s+at docs\/guide\.md:4:7$/);
    expect(imported!.message).toBe("The diagram script has a syntax error");
    expect(imported!.error).toMatch(/^SyntaxError: .*\n\s+at docs\/broken\.ts:1:26$/);
    expect(typed!.message).toBe("The diagram script has a syntax error");
    expect(typed!.error).toMatch(/enum/);
  });

  /**
   * Given a module block named shapes and a diagram block importing #shapes
   * When nicevg doc runs
   * Then the diagram is drawn with what the module block exports
   */
  /**
   * Given two module blocks both named shapes, and a diagram block importing #shapes
   * When nicevg doc runs
   * Then both module blocks fail, each naming the other, rather than one hiding the other
   */
  test("fails module blocks sharing a name", () => {
    const shapes = (n: number) => `<!-- nicevg-module shapes\nexport const n = ${n};\n-->\n`;
    const diagram = `<!-- nicevg a.svg\nimport { render } from "nicevg";\nimport { n } from "#shapes";\nrender({ nodes: [] });\n-->\n`;
    const root = repository({ "guide.md": `${shapes(1)}\n${shapes(2)}\n${diagram}` });

    const { result } = doc(root, ["guide.md"]);

    expect(result.failures.slice(0, 2)).toEqual([
      { file: "guide.md", line: 1, message: "guide.md:5 is a module block named shapes too" },
      { file: "guide.md", line: 5, message: "guide.md:1 is a module block named shapes too" },
    ]);
  });
  test("imports a module block by #name", () => {
    const shapes = `<!-- nicevg-module shapes\nimport { box } from "nicevg/style/simple";\nexport const style = { box };\nexport const service = (id: string, x: number) => ({ id, class: "box" as const, x, y: 20, label: id });\n-->\n`;
    const diagram = `<!-- nicevg a.svg\nimport { render } from "nicevg";\nimport { service, style } from "#shapes";\nrender({ nodes: [service("orders", 20), service("billing", 220)], style });\n-->\n`;
    const root = repository({ "guide.md": `${shapes}\n${diagram}` });

    const { exitCode } = doc(root, ["guide.md"]);

    expect(exitCode).toBe(0);
    expect(read(root, "a.svg")).toContain('data-node="billing"');
  });

  /**
   * Given blocks importing node:fs, a URL, a package, a file outside the
   *   repository, a file in .git and node_modules, a JSON file, and a module
   *   block that does not exist
   * When nicevg doc runs
   * Then each fails, saying it cannot import what it names
   */
  test("refuses imports of anything but nicevg, module blocks and files in the sandbox directory", () => {
    const others = { "docs/data.json": "{}", ".git/hooks.ts": "export {}", "node_modules/x/index.ts": "export {}" };
    for (const specifier of ["node:fs", "https://example.com/x.js", "lodash", "../../outside.ts", "../.git/hooks.ts", "../node_modules/x/index.ts", "./data.json", "#missing"]) {
      const [failure] = failuresOf(`import "${specifier}";`, others);
      expect(failure?.message).toBe("The diagram script imports what it may not");
      expect(failure?.error ?? "").toContain(`cannot import "${specifier}"`);
    }
  });

  /**
   * Given a block reading process, require and fetch
   * When nicevg doc runs
   * Then none exists in the sandbox
   */
  test("gives scripts no process, require or fetch", () => {
    const [failure] = failuresOf(`throw new Error([typeof process, typeof require, typeof fetch].join(","));`);

    expect(failure!.error).toMatch(/^Error: undefined,undefined,undefined/);
  });

  /**
   * Given a block logging with console.log, Date and Math.random
   * When nicevg doc runs
   * Then what it logs goes to stderr after the block's place, and stdout stays JSON
   */
  test("writes the console to stderr after the block's place, with Date and Math.random at hand", () => {
    const code = `import { render } from "nicevg";\nconsole.log("now", typeof Date.now(), typeof Math.random());\nrender({ nodes: [] });`;
    const root = repository({ "docs/guide.md": oneBlock(code) });

    const { stderr, result } = doc(root, ["docs/guide.md"]);

    expect(stderr).toBe("docs/guide.md:1: now number number\n");
    expect(result.failures).toEqual([]);
  });
});

describe("nicevg doc limits", () => {
  /**
   * Given a block looping forever, then a block that draws
   * When nicevg doc runs with a 1 s limit per diagram
   * Then the first fails for the time limit and the second is still drawn
   */
  test("fails a script past the time limit and draws the next diagram", () => {
    const root = repository({ "guide.md": `${oneBlock("while (true) {}")}\n<!-- nicevg b.svg\n${ORDER_FLOW}-->\n` });

    const { result } = doc(root, ["--timeout-per-diagram", "1", "guide.md"]);

    expect(result.failures).toEqual([{ file: "guide.md", line: 1, message: "The diagram script ran past the time limit" }]);
    expect(read(root, "b.svg")).toContain("<svg");
  });

  /**
   * Given a block filling memory
   * When nicevg doc runs with 16 MB of sandbox memory
   * Then it fails for running out of sandbox memory
   */
  test("fails a script past the memory limit", () => {
    const root = repository({ "guide.md": oneBlock("const kept = [];\nfor (;;) kept.push(new Array(100000).fill(1));") });

    const { result } = doc(root, ["--sandbox-memory", "16", "guide.md"]);

    expect(result.failures).toEqual([{ file: "guide.md", line: 1, message: "The diagram script ran out of sandbox memory" }]);
  });

  /**
   * Given a block rendering a grid of 64 boxes joined to their neighbours,
   *   which takes the layout well over 2 s
   * When nicevg doc runs with a 2 s limit per diagram
   * Then it fails for the time limit soon after 2 s, the layout process stopped
   */
  test("stops a layout past the time limit", () => {
    const grid = `import { render } from "nicevg";
import { box } from "nicevg/style/simple";
const nodes = [], edges = [];
for (let i = 0; i < 64; i++) {
  nodes.push({ id: "n" + i, class: "box", x: (i % 8) * 90 + (i * 7) % 13, y: Math.floor(i / 8) * 70, label: "N" + i });
  if (i % 8 < 7) edges.push({ from: "n" + i, to: "n" + (i + 1), label: "e" + i });
  if (i < 56) edges.push({ from: "n" + i, to: "n" + (i + 9 < 64 ? i + 9 : i + 8) });
}
render({ nodes, edges, style: { box } });`;
    const root = repository({ "guide.md": oneBlock(grid) });

    const started = Date.now();
    const { result } = doc(root, ["--timeout-per-diagram", "2", "guide.md"]);

    expect(result.failures).toEqual([{ file: "guide.md", line: 1, message: "Gave up drawing the diagram after 2 s" }]);
    expect(Date.now() - started).toBeLessThan(10_000);
  });

  /**
   * Given a block that draws
   * When nicevg doc runs allowing SVGs of 0.0001 MB (about 100 bytes)
   * Then it fails for the size and writes nothing
   */
  test("fails a diagram whose SVG is past the size limit and writes nothing", () => {
    const root = repository({ "guide.md": oneBlock(ORDER_FLOW) });

    const { result } = doc(root, ["--max-svg-size", "0.0001", "guide.md"]);

    expect(result.failures).toEqual([{ file: "guide.md", line: 1, message: "The diagram's SVG is larger than 0.0001 MB" }]);
    expect(result.changedFiles).toEqual([]);
  });

  /**
   * Given two blocks that draw, and NICEVG_LIBRARY pointing at a file that is no native library
   * When nicevg doc runs
   * Then the layout process stops for each, each diagram fails saying so,
   *   and the JSON is still printed
   */
  test("fails each diagram whose layout process stops, and carries on", () => {
    const markdown = `${oneBlock(ORDER_FLOW)}\n<!-- nicevg b.svg\n${ORDER_FLOW}-->\n`;
    const second = markdown.slice(0, markdown.indexOf("<!-- nicevg b.svg")).split("\n").length;
    const root = repository({ "guide.md": markdown });

    const { exitCode, stdout } = Bun.spawnSync([process.execPath, cli, "doc", "guide.md"], {
      cwd: root,
      env: { ...process.env, NICEVG_LIBRARY: join(root, "guide.md") },
    });

    const { failures } = JSON.parse(stdout.toString());
    expect(exitCode).toBe(2);
    expect(failures.map((failure: { line: number }) => failure.line)).toEqual([1, second]);
    expect(failures[0].message).toMatch(/^The layout process stopped/);
  });

  /**
   * Given a block writing 2 MB to the console
   * When nicevg doc runs
   * Then stderr stops at 1 MB with a line saying the output was cut
   */
  test("cuts a script's console output at 1 MB, saying so", () => {
    const code = `import { render } from "nicevg";\nfor (let i = 0; i < 2048; i++) console.log("x".repeat(1023));\nrender({ nodes: [] });`;
    const root = repository({ "guide.md": oneBlock(code) });

    const { stderr } = doc(root, ["guide.md"]);

    expect(stderr.length).toBeLessThan(1.1 * 1024 * 1024);
    expect(stderr).toEndWith("guide.md:1: (console output cut at 1 MB)\n");
  });
});

describe("nicevg doc output paths", () => {
  /**
   * Given blocks naming SVGs outside the repository, one in .git, one through
   *   a link to a directory outside, and an absolute path
   * When nicevg doc runs
   * Then each fails, saying it cannot write there, and nothing is written
   */
  test("fails a block naming an SVG it may not write", () => {
    const outside = mkdtempSync(join(tmpdir(), "nicevg-outside-"));
    const root = repository({});
    symlinkSync(outside, join(root, "linked"), "junction");
    const paths = ["../outside.svg", ".git/a.svg", ".GIT/b.svg", "linked/a.svg", "/tmp/a.svg"];
    writeFileSync(join(root, "guide.md"), paths.map((path) => `<!-- nicevg ${path}\n${ORDER_FLOW}-->\n`).join("\n"));

    const { result } = doc(root, ["guide.md"]);

    expect(result.failures.map((failure: { message: string }) => failure.message.replace(/:.*/, ""))).toEqual(
      paths.map((path) => `Cannot write the diagram to "${path}"`),
    );
    expect(readdirSync(outside)).toEqual([]);
    expect(readdirSync(join(root, ".git"))).toEqual([]);
  });

  /**
   * Given two blocks in two markdown files naming the same SVG
   * When nicevg doc runs on both
   * Then both fail, each naming the other, and the SVG is not written
   */
  test("fails both blocks naming the same SVG", () => {
    const root = repository({
      "a.md": `<!-- nicevg out/x.svg\n${ORDER_FLOW}-->\n`,
      "docs/b.md": `Text\n\n<!-- nicevg ../out/x.svg\n${ORDER_FLOW}-->\n`,
    });

    const { result } = doc(root, ["a.md", "docs/b.md"]);

    expect(result.failures).toEqual([
      { file: "a.md", line: 1, message: "docs/b.md:3 writes its diagram to out/x.svg too" },
      { file: "docs/b.md", line: 3, message: "a.md:1 writes its diagram to out/x.svg too" },
    ]);
    expect(existsSync(join(root, "out", "x.svg"))).toBe(false);
  });
});

describe("nicevg doc results", () => {
  /**
   * Given a group 200px wide holding two boxes joined by a connector whose
   *   label is wider than the room beside it in the group
   * When nicevg doc runs
   * Then the diagram is drawn, the stdout JSON holds only changedFiles,
   *   failures and remainingIssues, and the label left across the group's
   *   border is a remaining issue with its code and message
   */
  test("reports the issues left in a drawn diagram", () => {
    const code = `import { render, svg } from "nicevg";
import { box } from "nicevg/style/simple";
const group = ({ width, height }: { width: number; height: number }) =>
  svg\`<rect data-bounds="true" width="\${width}" height="\${height}" fill="none" stroke="gray"/>\`;
render({
  nodes: [
    { id: "g", class: "group", x: 0, y: 0, width: 200, height: 300 },
    { id: "a", class: "box", label: "A", x: 60, y: 20, parent: "g" },
    { id: "b", class: "box", label: "B", x: 60, y: 220, parent: "g" },
  ],
  edges: [{ from: "a", to: "b", label: "a long label on the edge" }],
  style: { box, group },
});`;
    const root = repository({ "guide.md": oneBlock(code) });

    const { exitCode, result } = doc(root, ["guide.md"]);

    expect(exitCode).toBe(0);
    expect(Object.keys(result)).toEqual(["changedFiles", "failures", "remainingIssues"]);
    expect(result.remainingIssues).toEqual([
      { file: "guide.md", line: 1, code: "label-node-overlap", message: 'Label "connector-0" overlaps node "g".' },
    ]);
  });

  /**
   * Given a block rendering a connector to a node the sketch does not have,
   *   which only laying it out finds
   * When nicevg doc runs
   * Then it fails with what is wrong and where render was called
   */
  test("fails a diagram the layout finds invalid, saying where render was called", () => {
    const code = `import { render } from "nicevg";
import { box } from "nicevg/style/simple";
const nodes = [{ id: "a", class: "box", x: 0, y: 0, label: "A" }];
  render({ nodes, edges: [{ from: "a", to: "ghost" }], style: { box } });`;
    const root = repository({ "guide.md": oneBlock(code) });

    const { exitCode, result } = doc(root, ["guide.md"]);

    expect(exitCode).toBe(2);
    expect(result.failures).toHaveLength(1);
    expect(result.failures[0].message).toMatch(/^The diagram passed to render is invalid: connectors\[0\]\.to refers to node "ghost".*\(render was called at guide\.md:5:\d+\)$/);
  });

  /**
   * Given a markdown file with a block not drawn yet
   * When nicevg doc runs with --dry-run
   * Then nothing is written, and the files it would write are reported
   */
  test("writes nothing on a dry run but reports the files it would change", () => {
    const markdown = oneBlock(ORDER_FLOW);
    const root = repository({ "guide.md": markdown });

    const { exitCode, result } = doc(root, ["--dry-run", "guide.md"]);

    expect(exitCode).toBe(0);
    expect(result.changedFiles.sort()).toEqual(["a.svg", "guide.md"]);
    expect(read(root, "guide.md")).toBe(markdown);
    expect(existsSync(join(root, "a.svg"))).toBe(false);
  });

  /**
   * Given a markdown file in a subdirectory of a repository, run from that subdirectory
   * When nicevg doc runs without --sandbox-dir
   * Then the sandbox directory is the repository's root: a file in a sibling
   *   directory can be imported, and paths are reported from the root
   */
  test("takes the git repository's root as the sandbox directory", () => {
    const code = `import { render } from "nicevg";\nimport { style, nodes } from "../shared/shapes.ts";\nrender({ nodes, style });`;
    const shapes = `import { box } from "nicevg/style/simple";\nexport const style = { box };\nexport const nodes = [{ id: "a", class: "box" as const, x: 0, y: 0, label: "A" }];\n`;
    const root = repository({ "docs/guide.md": oneBlock(code), "shared/shapes.ts": shapes });

    const { exitCode, stdout } = Bun.spawnSync([process.execPath, cli, "doc", "guide.md"], { cwd: join(root, "docs") });

    expect(exitCode).toBe(0);
    expect(JSON.parse(stdout.toString()).changedFiles.sort()).toEqual(["docs/a.svg", "docs/guide.md"]);
  });

  /**
   * Given files of two git repositories, a file in no repository, and a file that is no markdown
   * When nicevg doc runs on each case
   * Then it draws nothing, prints no JSON and exits with 2, saying why on
   *   stderr; with --sandbox-dir the file in no repository is drawn
   */
  test("refuses files of two repositories, outside any without --sandbox-dir, or not markdown", () => {
    const first = repository({ "a.md": oneBlock(ORDER_FLOW) });
    const second = repository({ "b.md": oneBlock(ORDER_FLOW) });
    const loose = mkdtempSync(join(tmpdir(), "nicevg-loose-"));
    writeFileSync(join(loose, "c.md"), oneBlock(ORDER_FLOW));

    for (const args of [[join(first, "a.md"), join(second, "b.md")], [join(loose, "c.md")], [join(first, "notes.txt")]]) {
      const { exitCode, stdout, stderr } = Bun.spawnSync([process.execPath, cli, "doc", ...args]);
      expect(exitCode).toBe(2);
      expect(stdout.toString()).toBe("");
      expect(stderr.toString()).toMatch(/^error: /);
    }
    expect(existsSync(join(first, "a.svg"))).toBe(false);
    expect(doc(loose, ["--sandbox-dir", ".", "c.md"]).exitCode).toBe(0);
  });
});
