// Builds the native library and copies it beside the TypeScript sources.

import { copyFileSync, mkdirSync } from "node:fs";
import { join } from "node:path";
import { suffix } from "bun:ffi";

const root = join(import.meta.dir, "..", "..", "..");
const build = Bun.spawnSync(["cargo", "build", "--release", "-p", "nicevg-ffi"], { cwd: root, stdio: ["inherit", "inherit", "inherit"] });
if (build.exitCode !== 0) {
  process.exit(build.exitCode ?? 1);
}
const name = process.platform === "win32" ? `nicevg_ffi.${suffix}` : `libnicevg_ffi.${suffix}`;
const native = join(import.meta.dir, "..", "native");
mkdirSync(native, { recursive: true });
copyFileSync(join(root, "target", "release", name), join(native, name));
console.log(`copied ${name} to ${native}`);
