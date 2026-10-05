// Runs the fix tests with recording on, then writes a self-contained HTML
// gallery of every recorded fix() call to test-results/fix-gallery.html.
import {
  existsSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { join, resolve } from "node:path";
import type { TestRecord } from "../../tests/support/recording";

const root = resolve(import.meta.dir, "../..");
const resultsDir = join(root, "test-results");
const recordDir = join(resultsDir, "fix-records");
const testFile = join(root, "tests/fix.test.ts");
const outputFile = join(resultsDir, "fix-gallery.html");

rmSync(recordDir, { recursive: true, force: true });
mkdirSync(resultsDir, { recursive: true });

const run = Bun.spawnSync([process.execPath, "test", "tests/fix.test.ts"], {
  cwd: root,
  env: { ...process.env, NICEVG_RECORD_DIR: recordDir },
  stdout: "inherit",
  stderr: "inherit",
});

// Pull each test's Given/When/Then comment out of the test source, in order.
const source = readFileSync(testFile, "utf8");
const specs: Array<{ title: string; scenario: string }> = [];
for (const match of source.matchAll(
  /test\(\s*"((?:[^"\\]|\\.)*)",\s*(?:async\s*)?\(\)\s*=>\s*\{\s*\/\*\*([\s\S]*?)\*\//g,
)) {
  specs.push({
    title: JSON.parse(`"${match[1] ?? ""}"`) as string,
    scenario: (match[2] ?? "")
      .split("\n")
      .map((line) => line.replace(/^\s*\*\s?/, "").trimEnd())
      .filter((line) => line !== "")
      .join("\n"),
  });
}

const records = new Map(
  (existsSync(recordDir) ? readdirSync(recordDir) : []).map((file) => {
    const record = JSON.parse(
      readFileSync(join(recordDir, file), "utf8"),
    ) as TestRecord;
    return [record.test, record];
  }),
);

// Tests that never ran (skipped, or a filter excluded them) have no record.
const cases = specs.map((spec) => ({
  ...spec,
  outcome: records.get(spec.title)?.outcome ?? { status: "skipped" },
  calls: records.get(spec.title)?.calls ?? [],
}));

const data = JSON.stringify({
  generatedAt: new Date().toISOString(),
  exitCode: run.exitCode,
  cases,
}).replace(/</g, "\\u003c");
const template = readFileSync(join(import.meta.dir, "template.html"), "utf8");
writeFileSync(
  outputFile,
  template.replace("__GALLERY_DATA__", () => data),
);

const failed = cases.filter((item) => item.outcome.status === "failed").length;
console.log(
  `\nfix gallery: ${cases.length} cases, ${failed} failed -> ${outputFile}`,
);
process.exitCode = run.exitCode;
