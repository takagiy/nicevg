import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { test as bunTest } from "bun:test";
import {
  analyze,
  fix as fixSvg,
  type AnalysisReport,
  type FixResult,
} from "../../src/index";

// When NICEVG_RECORD_DIR is set, each test's outcome and every fix() call made
// inside it are saved with input, output and reports for the fix gallery.
const recordDir = process.env.NICEVG_RECORD_DIR;

export interface FixCall {
  input: string;
  output: string;
  changes: FixResult["changes"];
  before: AnalysisReport;
  after: AnalysisReport;
}

export interface TestRecord {
  test: string;
  outcome: { status: "passed" | "failed"; message?: string };
  calls: FixCall[];
}

let current: TestRecord | undefined;

const fileNameFor = (title: string): string =>
  `${title.replace(/[^a-z0-9]+/gi, "-").replace(/^-|-$/g, "")}.json`;

const save = (record: TestRecord): void => {
  if (recordDir === undefined) return;
  mkdirSync(recordDir, { recursive: true });
  writeFileSync(
    join(recordDir, fileNameFor(record.test)),
    JSON.stringify(record, null, 2),
  );
};

export const test = (title: string, body: () => void | Promise<void>): void => {
  bunTest(title, async () => {
    const record: TestRecord = {
      test: title,
      outcome: { status: "passed" },
      calls: [],
    };
    current = record;
    try {
      await body();
    } catch (error) {
      record.outcome = {
        status: "failed",
        message: Bun.stripANSI(
          error instanceof Error ? error.message : String(error),
        ),
      };
      throw error;
    } finally {
      current = undefined;
      save(record);
    }
  });
};

export const fix = (svg: string): FixResult => {
  const result = fixSvg(svg);
  if (recordDir !== undefined && current !== undefined) {
    current.calls.push({
      input: svg,
      output: result.svg,
      changes: result.changes,
      before: analyze(svg),
      after: result.report,
    });
  }
  return result;
};
