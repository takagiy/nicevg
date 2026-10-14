// The nicevg command on Node (the built dist/cli.js) and on Bun (src/cli.ts).

import assert from "node:assert/strict";
import { spawn, spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { describe, test } from "node:test";
import { fileURLToPath } from "node:url";

const runtimes = [
  ["node", fileURLToPath(new URL("../../dist/cli.js", import.meta.url))],
  ["bun", fileURLToPath(new URL("../../src/cli.ts", import.meta.url))],
] as const;

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
  });
}
