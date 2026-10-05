#!/usr/bin/env bun

import { Command } from "commander";
import { fix, SvgInputError } from "./index";

const program = new Command()
  .name("nicevg")
  .description(
    "Repair mechanical layout errors in an SVG diagram read from stdin and write the result to stdout.",
  )
  .allowExcessArguments(false)
  .action(async () => {
    const result = fix(await Bun.stdin.text());
    process.stdout.write(`${result.svg}\n`);
    for (const issue of result.report.issues) {
      console.error(`${issue.code}: ${issue.message}`);
    }
    if (!result.report.valid) process.exitCode = 1;
  });

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
