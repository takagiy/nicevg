#!/usr/bin/env bun

import { readFile, writeFile } from "node:fs/promises";
import { Command } from "commander";
import { analyze, fix, SvgInputError } from "./index";

const program = new Command()
  .name("nicevg")
  .description("Check and repair mechanical layout errors in SVG diagrams.");

const readInput = async (file: string | undefined): Promise<string> =>
  file === undefined || file === "-"
    ? Bun.stdin.text()
    : readFile(file, "utf8");

program
  .command("check")
  .description("Check an SVG diagram without changing it.")
  .argument("[file]", "SVG file to check; omit or use - to read stdin")
  .option("--json", "Print the structured report as JSON.")
  .action(async (file: string | undefined, options: { json?: boolean }) => {
    const report = analyze(await readInput(file));
    if (options.json === true) {
      console.log(JSON.stringify(report, null, 2));
      if (!report.valid) process.exitCode = 1;
      return;
    }
    if (report.valid) {
      console.log("No diagram issues found.");
      return;
    }

    for (const issue of report.issues) {
      console.error(`${issue.code}: ${issue.message}`);
    }
    process.exitCode = 1;
  });

program
  .command("fix")
  .description("Repair deterministic layout errors in an SVG diagram.")
  .argument("[file]", "SVG file to repair; omit or use - to read stdin")
  .option("-o, --output <file>", "Write the repaired SVG to another file.")
  .option("--write", "Replace the input file with the repaired SVG.")
  .action(
    async (
      file: string | undefined,
      options: { output?: string; write?: boolean },
    ) => {
      const readsStdin = file === undefined || file === "-";
      if (options.write === true && readsStdin) {
        console.error("--write requires an input file.");
        process.exitCode = 2;
        return;
      }
      const result = fix(await readInput(file));
      if (options.output !== undefined) {
        await writeFile(options.output, result.svg);
        return;
      }
      if (options.write === true) {
        if (file === undefined) return;
        await writeFile(file, result.svg);
        return;
      }
      process.stdout.write(`${result.svg}\n`);
    },
  );

try {
  await program.parseAsync();
} catch (error) {
  if (error instanceof SvgInputError) {
    console.error(`Invalid SVG: ${error.message}`);
    process.exitCode = 2;
  } else {
    throw error;
  }
}
