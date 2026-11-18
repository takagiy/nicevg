// The nicevg command on Node (the built dist/cli.js) and on Bun (src/cli.ts).

import assert from "node:assert/strict";
import { spawn, spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, test } from "node:test";
import { fileURLToPath } from "node:url";

const runtimes = [
  ["node", fileURLToPath(new URL("../../dist/cli.js", import.meta.url))],
  ["bun", fileURLToPath(new URL("../../src/cli.ts", import.meta.url))],
] as const;

const page = readFileSync(new URL("../../doc/nicevg.txt", import.meta.url), "utf8");

/** The help page's section under `*tag*`, up to the next rule. */
const section = (tag: string) => page.slice(page.indexOf(`*${tag}*`)).split(/\n[-=]{78}\n/)[0]!;

/** The help page's code block after the line ending with `lead` and `>`, its tab indent taken off. */
const codeAfter = (lead: string) => {
  const start = page.indexOf(`${lead} >\n`) + lead.length + 3;
  return page
    .slice(start, page.indexOf("\n<\n", start))
    .split("\n")
    .map((line) => line.replace(/^\t/, ""))
    .join("\n")
    .concat("\n");
};

const bends = (svg: string, id: string) => {
  const d = new RegExp(`<path[^>]*id="${id}"[^>]*\\sd="([^"]*)"`).exec(svg)![1]!;
  return d.match(/[ML]/g)!.length - 2;
};

for (const [runtime, cli] of runtimes) {
  const run = (input: string, args: string[] = []) => {
    const { status, stdout, stderr } = spawnSync(runtime, [cli, ...args], { input, encoding: "utf8" });
    return { status, stdout, stderr };
  };

  describe(`nicevg on ${runtime}`, () => {
    /**
     * Given the installed command
     * When its help is displayed
     * Then the public tool name is nicevg
     */
    test("help uses nicevg as the command name", () => {
      const result = run("", ["--help"]);

      assert.equal(result.status, 0);
      assert.match(result.stdout, /Usage: nicevg/);
    });

    /**
     * Given a clipped SVG diagram on standard input
     * When the command runs
     * Then the fixed SVG, now free of issues, is written to standard output and the command succeeds
     */
    test("fixes SVG from stdin and writes it to stdout", () => {
      const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 120">
        <g data-node="checkout">
          <rect x="20" y="20" width="100" height="56"/>
          <text x="70" y="53" text-anchor="middle" font-size="14">Checkout</text>
        </g>
      </svg>`;

      const result = run(svg);

      assert.equal(result.status, 0);
      assert.equal(result.stderr, "");
      assert.match(result.stdout, /viewBox="0 0 140 120"/);
    });

    /**
     * Given two free labels that overlap, which fixing does not move
     * When the command runs
     * Then it still writes the SVG to standard output, lists the remaining issue on standard error and exits with status 1
     */
    test("reports issues fixing cannot resolve and exits with status 1", () => {
      const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 300 120">
        <text data-label="first" x="40" y="60" font-size="14">Overlap</text>
        <text data-label="second" x="50" y="62" font-size="14">Overlap</text>
      </svg>`;

      const result = run(svg);

      assert.equal(result.status, 1);
      assert.match(result.stdout, /^<svg/);
      assert.match(result.stderr, /^label-overlap: /);
    });

    /**
     * Given malformed SVG on standard input
     * When the command runs
     * Then it reports an input error without a stack trace and exits with status 2
     */
    test("exits with status 2 and a concise input error on invalid SVG", () => {
      const result = run("<svg><g></svg>");

      assert.equal(result.status, 2);
      assert.equal(result.stdout, "");
      assert.match(result.stderr, /^Invalid SVG: /);
      assert.doesNotMatch(result.stderr, /\n\s+at /);
    });

    /**
     * Given a valid diagram on standard input
     * When the command is also given an argument, as the old fix and check commands and the old --arrange option took
     * Then it fails without writing anything to standard output
     */
    test("rejects file and subcommand arguments because input is stdin only", () => {
      const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"/>`;

      for (const argument of ["fix", "diagram.svg", "--arrange"]) {
        const result = run(svg, [argument]);

        assert.notEqual(result.status, 0);
        assert.equal(result.stdout, "");
        assert.notEqual(result.stderr, "");
      }
    });

    /**
     * Given a diagram whose connector bends only because one node sits 24px off, on standard input
     * When the command runs
     * Then it writes the arranged SVG, with the connector straight, and succeeds
     */
    test("arranges nodes by default", () => {
      const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 420 200">
        <g data-node="notify"><circle cx="100" cy="110" r="54"/><text x="100" y="115" text-anchor="middle" font-size="13">7 Notify</text></g>
        <g data-node="mail"><rect x="280" y="60" width="130" height="52"/><text x="345" y="91" text-anchor="middle" font-size="13">Email service</text></g>
        <path id="email" data-from="notify" data-to="mail" d="M 154 110 L 220 110 L 220 86 L 280 86"/>
      </svg>`;

      const result = run(svg);

      assert.equal(result.status, 0);
      assert.equal(bends(svg, "email"), 2);
      assert.equal(bends(result.stdout, "email"), 0);
    });

    /**
     * Given a reader that stops reading after the first bytes, as `head` does
     * When the command writes the arranged SVG to it
     * Then it still succeeds, without a broken pipe error
     */
    test("succeeds when the reader of stdout goes away early", async () => {
      const svg = readFileSync(new URL("../../../../docs/images/before.svg", import.meta.url), "utf8");
      const child = spawn(runtime, [cli], { stdio: ["pipe", "pipe", "pipe"] });
      let stderr = "";
      child.stderr.on("data", (chunk) => (stderr += chunk));
      child.stdout.once("data", () => child.stdout.destroy());
      child.stdin.end(svg);

      const status = await new Promise((resolve) => child.on("exit", (code, signal) => resolve(signal ?? code)));

      assert.equal(status, 0);
      assert.doesNotMatch(stderr, /EPIPE/);
    });

    /**
     * Given a diagram whose arranging frees its old candidates on other threads, run by many processes at once
     * When each process exits right after writing the SVG
     * Then none of them crashes: the library is done with its threads before the runtime unloads it
     */
    test("exits cleanly after arranging, even under load", async () => {
      const svg = readFileSync(new URL("../../../../docs/images/before.svg", import.meta.url), "utf8");

      const statuses = await Promise.all(
        Array.from({ length: 16 }, () => {
          const child = spawn(runtime, [cli], { stdio: ["pipe", "ignore", "ignore"] });
          child.stdin.end(svg);
          return new Promise((resolve) => child.on("exit", (code, signal) => resolve(signal ?? code)));
        }),
      );

      assert.deepEqual(statuses, Array(16).fill(0));
    });

    /**
     * Given a git repository whose markdown file has a diagram block naming a.svg
     * When nicevg doc runs on it
     * Then it writes a.svg, links it after the block and reports both files changed as JSON
     */
    test("draws the diagram block of a markdown file", () => {
      const root = mkdtempSync(join(tmpdir(), "nicevg-doc-"));
      mkdirSync(join(root, ".git"));
      const block = `<!-- nicevg a.svg
import { render } from "nicevg";
import { box } from "nicevg/style/simple";
const label: string = "A";
render({ nodes: [{ id: "a", class: "box", x: 0, y: 0, label }], style: { box } });
-->
`;
      writeFileSync(join(root, "guide.md"), block);

      const result = spawnSync(runtime, [cli, "doc", "guide.md"], { cwd: root, encoding: "utf8" });

      assert.equal(result.status, 0, result.stderr);
      assert.deepEqual(JSON.parse(result.stdout).changedFiles.sort(), ["a.svg", "guide.md"]);
      assert.match(readFileSync(join(root, "a.svg"), "utf8"), /data-node="a"/);
      assert.match(readFileSync(join(root, "guide.md"), "utf8"), /<!-- nicevg:output -->\n!\[a\]\(a\.svg\)\n<!-- \/nicevg:output -->/);
    });

    /**
     * [x] When `nicevg man` runs, it can print the help page to stdout and exit 0.
     * Given the installed command
     * When `nicevg man` runs
     * Then the help page is written to stdout as it is, and the command succeeds
     */
    test("man prints the help page", () => {
      const result = run("", ["man"]);

      assert.equal(result.status, 0, result.stderr);
      assert.equal(result.stdout, page);
    });

    /**
     * [x] When `nicevg man --toc` runs, it can print the table of contents alone.
     * Given the installed command
     * When `nicevg man --toc` runs
     * Then only the entries of the table of contents are written
     */
    test("man --toc prints the table of contents alone", () => {
      const result = run("", ["man", "--toc"]);

      assert.equal(result.status, 0, result.stderr);
      const lines = result.stdout.trimEnd().split("\n");
      assert.match(lines[0]!, /^1\. Introduction \.+ \|nicevg-intro\||nicevg-engine-issue|$/);
      assert.match(lines.at(-1)!, /\|nicevg-engine-issue\|$/);
      assert.ok(lines.every((line) => /\|[\w-]+\||nicevg-engine-issue|$/.test(line)), result.stdout);
    });

    /**
     * [x] When `nicevg --help` runs, it can name `nicevg man`.
     * Given the installed command
     * When its help is displayed
     * Then it names `nicevg man`
     */
    test("help names nicevg man", () => {
      assert.match(run("", ["--help"]).stdout, /nicevg man/);
    });

    /**
     * [x] When the style example in `nicevg-dsl-render-style` is drawn with `nicevg doc`, it can draw with no failures.
     * Given a git repository whose README.md has the help page's style example as a diagram block
     * When nicevg doc runs on it
     * Then the diagram is drawn without failures
     */
    test("draws the help page's style example", () => {
      const root = mkdtempSync(join(tmpdir(), "nicevg-man-"));
      mkdirSync(join(root, ".git"));
      writeFileSync(join(root, "README.md"), `<!-- nicevg pill.svg\n${codeAfter("as the origin:")}\n-->\n`);

      const result = spawnSync(runtime, [cli, "doc", "README.md"], { cwd: root, encoding: "utf8" });

      assert.equal(result.status, 0, result.stderr);
      assert.deepEqual(JSON.parse(result.stdout).failures, []);
      assert.match(readFileSync(join(root, "pill.svg"), "utf8"), /data-node="orders"/);
    });
  });
}

describe("nicevg help page", () => {
  /**
   * [x] When the help page is read, every issue code but the port ones can be found as a `*nicevg-engine-issue-<code>*` tag.
   * Given the issue codes the native library reports
   * When the help page is read
   * Then each, but those of ports, which it does not document yet, has an entry
   */
  test("documents every issue code but the port ones", () => {
    const inspect = readFileSync(new URL("../../../../src/inspect.rs", import.meta.url), "utf8");
    const codes = [...inspect.matchAll(/issue\(\s*"([a-z][a-z-]*)"/g)].map(([, code]) => code!);
    const ports = ["connector-end-off-port", "connector-unknown-port"];

    assert.ok(codes.length > 0);
    for (const code of codes.filter((code) => !ports.includes(code))) {
      assert.ok(page.includes(`*nicevg-engine-issue-${code}*`), code);
    }
  });

  /**
   * [x] When the help page is read, every option of `nicevg doc` can be found in `nicevg-cli-doc-input`.
   * Given the options `nicevg doc --help` lists
   * When the help page's input section is read
   * Then it names each
   */
  test("documents every option of nicevg doc in its input section", () => {
    const usage = spawnSync("node", [runtimes[0][1], "doc", "--help"], { encoding: "utf8" }).stdout;
    const options = usage.match(/--[a-z-]+/g)!;
    const input = section("nicevg-cli-doc-input");

    assert.ok(options.length > 0);
    for (const option of options) {
      assert.ok(input.includes(option), option);
    }
  });

  /**
   * [x] When the help page is read, every `|link|` can be found as a `*tag*`.
   * Given the help page
   * When its links are read
   * Then each names a tag the page has
   */
  test("links only to tags it has", () => {
    for (const [, tag] of page.matchAll(/\|([\w-]+)\|/g)) {
      assert.ok(page.includes(`*${tag}*`), tag);
    }
  });
});
