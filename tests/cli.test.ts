import { afterEach, describe, expect, test } from "bun:test";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

const temporaryDirectories: string[] = [];

afterEach(async () => {
  await Promise.all(
    temporaryDirectories
      .splice(0)
      .map((directory) => rm(directory, { recursive: true, force: true })),
  );
});

const runCli = async (...arguments_: string[]) => {
  const process = Bun.spawn(
    ["bun", join(import.meta.dir, "../src/cli.ts"), ...arguments_],
    { stdout: "pipe", stderr: "pipe" },
  );
  const [exitCode, stdout, stderr] = await Promise.all([
    process.exited,
    new Response(process.stdout).text(),
    new Response(process.stderr).text(),
  ]);
  return { exitCode, stdout, stderr };
};

const runCliWithStdin = async (input: string, ...arguments_: string[]) => {
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
    const result = await runCli("--help");

    expect(result.exitCode).toBe(0);
    expect(result.stdout).toStartWith("Usage: nicevg ");
  });

  test("check succeeds for a diagram without issues", async () => {
    /**
     * Given a valid SVG diagram file
     * When check is invoked through the executable CLI
     * Then it exits successfully and confirms that no issues were found
     */
    const directory = await mkdtemp(join(tmpdir(), "nicevg-"));
    temporaryDirectories.push(directory);
    const file = join(directory, "valid.svg");
    await writeFile(
      file,
      `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 240 120">
        <g data-node="checkout">
          <rect x="20" y="20" width="120" height="56"/>
          <text x="80" y="53" text-anchor="middle" font-size="14">Checkout</text>
        </g>
      </svg>`,
    );

    const result = await runCli("check", file);

    expect(result).toEqual({
      exitCode: 0,
      stdout: "No diagram issues found.\n",
      stderr: "",
    });
  });

  test("check reads SVG from stdin when the file is omitted", async () => {
    /**
     * Given a valid SVG diagram on standard input
     * When check is invoked without a file argument
     * Then the piped SVG is checked successfully
     */
    const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 240 120">
      <g data-node="checkout">
        <rect x="20" y="20" width="120" height="56"/>
        <text x="80" y="53" text-anchor="middle" font-size="14">Checkout</text>
      </g>
    </svg>`;

    const result = await runCliWithStdin(svg, "check");

    expect(result).toEqual({
      exitCode: 0,
      stdout: "No diagram issues found.\n",
      stderr: "",
    });
  });

  test("check reads SVG from stdin when the file is a hyphen", async () => {
    /**
     * Given a valid SVG diagram on standard input
     * When check receives the conventional "-" input path
     * Then the piped SVG is checked successfully
     */
    const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 240 120">
      <g data-node="checkout">
        <rect x="20" y="20" width="120" height="56"/>
        <text x="80" y="53" text-anchor="middle" font-size="14">Checkout</text>
      </g>
    </svg>`;

    const result = await runCliWithStdin(svg, "check", "-");

    expect(result.exitCode).toBe(0);
    expect(result.stderr).toBe("");
  });

  test("check --json emits the structured report and preserves failure status", async () => {
    /**
     * Given an SVG file with a clipping issue
     * When check is invoked with JSON output
     * Then stdout contains the API report and the command exits with status 1
     */
    const directory = await mkdtemp(join(tmpdir(), "nicevg-"));
    temporaryDirectories.push(directory);
    const file = join(directory, "clipped.svg");
    await writeFile(
      file,
      `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 120">
        <g data-node="checkout">
          <rect x="20" y="20" width="100" height="56"/>
          <text x="70" y="53" text-anchor="middle" font-size="14">Checkout</text>
        </g>
      </svg>`,
    );

    const result = await runCli("check", "--json", file);
    const report: unknown = JSON.parse(result.stdout);

    expect(result.exitCode).toBe(1);
    expect(result.stderr).toBe("");
    expect(report).toMatchObject({
      valid: false,
      issues: [{ code: "viewport-clipping" }],
    });
  });

  test("fix --output writes a repaired copy without changing the input", async () => {
    /**
     * Given an SVG file with a mechanically repairable issue
     * When fix is invoked with a separate output path
     * Then the repaired SVG is written there and the source remains unchanged
     */
    const directory = await mkdtemp(join(tmpdir(), "nicevg-"));
    temporaryDirectories.push(directory);
    const input = join(directory, "clipped.svg");
    const output = join(directory, "fixed.svg");
    const source = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 120">
      <g data-node="checkout">
        <rect x="20" y="20" width="100" height="56"/>
        <text x="70" y="53" text-anchor="middle" font-size="14">Checkout</text>
      </g>
    </svg>`;
    await writeFile(input, source);

    const result = await runCli("fix", "--output", output, input);

    expect(result.exitCode).toBe(0);
    expect(await readFile(input, "utf8")).toBe(source);
    expect(await readFile(output, "utf8")).toContain('viewBox="0 0 140 120"');
  });

  test("invalid SVG exits with status 2 and a concise input error", async () => {
    /**
     * Given a malformed SVG file
     * When the CLI checks it
     * Then it reports an input error without a stack trace and exits with status 2
     */
    const directory = await mkdtemp(join(tmpdir(), "nicevg-"));
    temporaryDirectories.push(directory);
    const file = join(directory, "invalid.svg");
    await writeFile(file, "<svg><g></svg>");

    const result = await runCli("check", file);

    expect(result.exitCode).toBe(2);
    expect(result.stdout).toBe("");
    expect(result.stderr).toStartWith("Invalid SVG:");
    expect(result.stderr).not.toContain("at ");
  });

  test("fix --write replaces the input with the repaired SVG", async () => {
    /**
     * Given a repairable SVG file
     * When fix is invoked with explicit in-place writing
     * Then the source file is replaced by the repaired SVG
     */
    const directory = await mkdtemp(join(tmpdir(), "nicevg-"));
    temporaryDirectories.push(directory);
    const file = join(directory, "clipped.svg");
    await writeFile(
      file,
      `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 120">
        <g data-node="checkout">
          <rect x="20" y="20" width="100" height="56"/>
          <text x="70" y="53" text-anchor="middle" font-size="14">Checkout</text>
        </g>
      </svg>`,
    );

    const result = await runCli("fix", "--write", file);

    expect(result.exitCode).toBe(0);
    expect(await readFile(file, "utf8")).toContain('viewBox="0 0 140 120"');
  });

  test("fix reads stdin and writes the repaired SVG to stdout", async () => {
    /**
     * Given a clipped SVG diagram on standard input
     * When fix is invoked without a file or output option
     * Then the repaired SVG is emitted on standard output
     */
    const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 120">
      <g data-node="checkout">
        <rect x="20" y="20" width="100" height="56"/>
        <text x="70" y="53" text-anchor="middle" font-size="14">Checkout</text>
      </g>
    </svg>`;

    const result = await runCliWithStdin(svg, "fix");

    expect(result.exitCode).toBe(0);
    expect(result.stderr).toBe("");
    expect(result.stdout).toContain('viewBox="0 0 140 120"');
  });

  test("fix rejects --write when stdin is the input", async () => {
    /**
     * Given SVG input from stdin
     * When in-place writing is requested
     * Then the command fails because stdin has no path to replace
     */
    const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"/>`;

    const result = await runCliWithStdin(svg, "fix", "--write");

    expect(result).toEqual({
      exitCode: 2,
      stdout: "",
      stderr: "--write requires an input file.\n",
    });
  });
});
