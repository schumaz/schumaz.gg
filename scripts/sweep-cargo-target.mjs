import { existsSync, readdirSync, statSync } from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const DEFAULT_ROOT_DIR = path.resolve(__dirname, "..");

export const DEFAULT_MAX_SIZE = "4GB";
export const DEFAULT_AUTO_THRESHOLD_GB = 6;
const BYTES_PER_GIB = 1024 ** 3;

function readOptionValue(argv, index, flagName) {
  const value = argv[index + 1];
  if (!value || value.startsWith("-")) {
    throw new Error(`Missing value for ${flagName}`);
  }
  return value;
}

export function parseSweepArgs(argv) {
  let maxSize = DEFAULT_MAX_SIZE;
  let ifOverGb = null;
  let dryRun = false;
  let allowMissing = false;
  let quiet = false;
  let help = false;

  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === "--maxsize") {
      maxSize = readOptionValue(argv, i, "--maxsize");
      i += 1;
    } else if (arg.startsWith("--maxsize=")) {
      maxSize = arg.slice("--maxsize=".length);
      if (!maxSize) {
        throw new Error("Missing value for --maxsize");
      }
    } else if (arg === "--if-over-gb") {
      const raw = readOptionValue(argv, i, "--if-over-gb");
      ifOverGb = parsePositiveNumber(raw, "--if-over-gb");
      i += 1;
    } else if (arg.startsWith("--if-over-gb=")) {
      const raw = arg.slice("--if-over-gb=".length);
      ifOverGb = parsePositiveNumber(raw, "--if-over-gb");
    } else if (arg === "--dry-run" || arg === "-d") {
      dryRun = true;
    } else if (arg === "--allow-missing") {
      allowMissing = true;
    } else if (arg === "--quiet" || arg === "-q") {
      quiet = true;
    } else if (arg === "--help" || arg === "-h") {
      help = true;
    } else {
      throw new Error(`Unknown option: ${arg}`);
    }
  }

  return { maxSize, ifOverGb, dryRun, allowMissing, quiet, help };
}

function parsePositiveNumber(raw, flagName) {
  const parsed = Number(raw);
  if (!Number.isFinite(parsed) || parsed <= 0) {
    throw new Error(`Invalid positive number for ${flagName}: ${raw}`);
  }
  return parsed;
}

export function measureDirBytes(dirPath) {
  if (!existsSync(dirPath)) {
    return 0;
  }

  let total = 0;
  const stack = [dirPath];

  while (stack.length > 0) {
    const current = stack.pop();
    let entries;
    try {
      entries = readdirSync(current, { withFileTypes: true });
    } catch {
      continue;
    }

    for (const entry of entries) {
      const fullPath = path.join(current, entry.name);
      if (entry.isDirectory()) {
        stack.push(fullPath);
      } else if (entry.isFile()) {
        try {
          total += statSync(fullPath).size;
        } catch {
          // Ignore transient files removed concurrently.
        }
      }
    }
  }

  return total;
}

export function formatGiB(bytes) {
  return `${(bytes / BYTES_PER_GIB).toFixed(2)} GiB`;
}

export function buildCargoSweepArgs({ maxSize = DEFAULT_MAX_SIZE, dryRun = false, projectDir = "src-tauri" }) {
  const args = ["sweep", "--maxsize", maxSize];
  if (dryRun) {
    args.push("--dry-run");
  }
  args.push(projectDir);
  return args;
}

export function isCargoSweepInstalled(run = spawnSync) {
  const probe = run("cargo", ["sweep", "--version"], {
    encoding: "utf8",
    stdio: "pipe",
  });
  return probe.status === 0;
}

export function runCargoSweep({
  rootDir = DEFAULT_ROOT_DIR,
  maxSize = DEFAULT_MAX_SIZE,
  ifOverGb = null,
  dryRun = false,
  allowMissing = false,
  quiet = false,
  run = spawnSync,
} = {}) {
  const targetDir = path.join(rootDir, "src-tauri", "target");
  if (!existsSync(targetDir)) {
    if (!quiet) {
      console.log("[cargo-sweep] src-tauri/target does not exist yet; skipping.");
    }
    return { status: "skipped-no-target", beforeBytes: 0, afterBytes: 0 };
  }

  const beforeBytes = measureDirBytes(targetDir);
  if (ifOverGb !== null) {
    const thresholdBytes = ifOverGb * BYTES_PER_GIB;
    if (beforeBytes < thresholdBytes) {
      if (!quiet) {
        console.log(
          `[cargo-sweep] src-tauri/target is ${formatGiB(beforeBytes)} (< ${ifOverGb} GiB threshold); skipping.`
        );
      }
      return { status: "skipped-under-threshold", beforeBytes, afterBytes: beforeBytes };
    }
  }

  if (!isCargoSweepInstalled(run)) {
    const message =
      "[cargo-sweep] cargo-sweep is not installed. Run 'cargo install cargo-sweep --locked' to enable target cleanup.";
    if (allowMissing) {
      console.warn(message);
      return { status: "skipped-missing-tool", beforeBytes, afterBytes: beforeBytes };
    }
    throw new Error(message);
  }

  const args = buildCargoSweepArgs({ maxSize, dryRun, projectDir: "src-tauri" });
  console.log(
    `[cargo-sweep] Cleaning src-tauri/target (${formatGiB(beforeBytes)} -> max ${maxSize}${dryRun ? ", dry-run" : ""})...`
  );

  const result = run("cargo", args, {
    cwd: rootDir,
    stdio: "inherit",
  });

  if (result.status !== 0) {
    throw new Error(`cargo sweep exited with code ${result.status ?? 1}`);
  }

  const afterBytes = dryRun ? beforeBytes : measureDirBytes(targetDir);
  if (!dryRun) {
    const freedBytes = Math.max(0, beforeBytes - afterBytes);
    console.log(
      `[cargo-sweep] Done: ${formatGiB(beforeBytes)} -> ${formatGiB(afterBytes)} (freed ${formatGiB(freedBytes)}).`
    );
  }

  return { status: "swept", beforeBytes, afterBytes };
}

function printHelp() {
  console.log(`Usage: node scripts/sweep-cargo-target.mjs [options]

Trims old Cargo build artifacts in src-tauri/target using cargo-sweep without wiping active dependencies.

Options:
  --maxsize <SIZE>      Target size limit passed to cargo-sweep (default: ${DEFAULT_MAX_SIZE})
  --if-over-gb <NUM>    Only run cargo-sweep if src-tauri/target exceeds NUM GiB
  -d, --dry-run         Preview deletions without removing files
  --allow-missing       Warn and exit 0 if cargo-sweep is not installed
  -q, --quiet           Suppress informational skip messages
  -h, --help            Show this help message
`);
}

function main() {
  const options = parseSweepArgs(process.argv.slice(2));
  if (options.help) {
    printHelp();
    return;
  }
  runCargoSweep(options);
}

const isMainModule =
  process.argv[1] && path.resolve(process.argv[1]) === path.resolve(__filename);
if (isMainModule) {
  try {
    main();
  } catch (error) {
    console.error(`[cargo-sweep] ${error.message}`);
    process.exit(1);
  }
}
