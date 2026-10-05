import { describe, expect, test } from "bun:test";
import { join } from "node:path";
import { analyze } from "../src/index";

const runCli = async (input: string, ...arguments_: string[]) => {
  const process = Bun.spawn(
    ["bun", join(import.meta.dir, "../src/cli.ts"), ...arguments_],
    { stdin: "pipe", stdout: "pipe", stderr: "pipe" },
  );
  await process.stdin.write(input);
  await process.stdin.end();
  const [exitCode, stdout, stderr] = await Promise.all([
    process.exited,
    new Response(process.stdout).text(),
    new Response(process.stderr).text(),
  ]);
  return { exitCode, stdout, stderr };
};

describe("CLI", () => {
  test("help uses nicevg as the command name", async () => {
    /**
     * Given the installed command
     * When its help is displayed
     * Then the public tool name is nicevg
     */
    const result = await runCli("", "--help");

    expect(result.exitCode).toBe(0);
    expect(result.stdout).toStartWith("Usage: nicevg");
  });

  test("fixes SVG from stdin and writes it to stdout", async () => {
    /**
     * Given a clipped SVG diagram on standard input
     * When the CLI runs
     * Then the fixed SVG, now free of issues, is written to standard output
     *   and the command succeeds
     */
    const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 120">
      <g data-node="checkout">
        <rect x="20" y="20" width="100" height="56"/>
        <text x="70" y="53" text-anchor="middle" font-size="14">Checkout</text>
      </g>
    </svg>`;

    const result = await runCli(svg);

    expect(result.exitCode).toBe(0);
    expect(result.stderr).toBe("");
    expect(analyze(svg).issues).not.toEqual([]);
    expect(analyze(result.stdout).issues).toEqual([]);
  });

  test("reports issues fix cannot resolve and exits with status 1", async () => {
    /**
     * Given two free labels that overlap, which fix does not move
     * When the CLI runs
     * Then it still writes the SVG to standard output, lists the remaining
     *   issue on standard error and exits with status 1
     */
    const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 300 120">
      <text data-label="first" x="40" y="60" font-size="14">Overlap</text>
      <text data-label="second" x="50" y="62" font-size="14">Overlap</text>
    </svg>`;

    const result = await runCli(svg);

    expect(result.exitCode).toBe(1);
    expect(analyze(result.stdout).issues.map((issue) => issue.code)).toEqual([
      "label-overlap",
    ]);
    expect(result.stderr).toContain("label-overlap");
  });

  test("invalid SVG exits with status 2 and a concise input error", async () => {
    /**
     * Given malformed SVG on standard input
     * When the CLI runs
     * Then it reports an input error without a stack trace and exits with
     *   status 2
     */
    const result = await runCli("<svg><g></svg>");

    expect(result.exitCode).toBe(2);
    expect(result.stdout).toBe("");
    expect(result.stderr).toStartWith("Invalid SVG:");
    expect(result.stderr).not.toContain("at ");
  });

  test("rejects file and subcommand arguments because input is stdin only", async () => {
    /**
     * Given a valid diagram on standard input
     * When the CLI is also given an argument, as the old fix and check
     *   commands took
     * Then it fails without writing anything to standard output
     */
    const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"/>`;

    for (const argument of ["fix", "diagram.svg"]) {
      const result = await runCli(svg, argument);

      expect(result.exitCode).not.toBe(0);
      expect(result.stdout).toBe("");
      expect(result.stderr).not.toBe("");
    }
  });
});
