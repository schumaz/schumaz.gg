import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import {
  DEFAULT_KEEP_INCREMENTAL,
  pruneIncrementalOrphans,
} from "./sweep-cargo-target.mjs";
import { withMainBinaryArg } from "./tauri-args.mjs";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const ROOT_DIR = path.resolve(__dirname, "..");

const tauriArgs = process.argv.slice(2);
const isDevCommand = tauriArgs[0] === "dev";

if (isDevCommand && process.env.SCHUMAZ_SKIP_CARGO_SWEEP !== "1") {
  try {
    pruneIncrementalOrphans({
      rootDir: ROOT_DIR,
      keepPerCrate: DEFAULT_KEEP_INCREMENTAL,
      quiet: true,
    });
  } catch (error) {
    // Never block local Tauri startup if incremental cleanup fails unexpectedly.
    console.warn(`[cargo-prune] Auto-cleanup warning: ${error.message}`);
  }
}

const tauriCliPath = path.join(ROOT_DIR, "node_modules", "@tauri-apps", "cli", "tauri.js");
const result = spawnSync(process.execPath, [tauriCliPath, ...withMainBinaryArg(tauriArgs)], {
  cwd: ROOT_DIR,
  stdio: "inherit",
});

if (result.error) {
  console.error(result.error);
  process.exit(1);
}

process.exit(result.status ?? 0);
