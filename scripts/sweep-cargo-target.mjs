import { existsSync, readdirSync, rmSync, statSync } from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const DEFAULT_ROOT_DIR = path.resolve(__dirname, "..");

export const DEFAULT_KEEP_INCREMENTAL = 3;
export const DEFAULT_SWEEP_DAYS = 30;
const BYTES_PER_GIB = 1024 ** 3;
const SKIP_SCAN_DIRS = new Set(["deps", "build", ".fingerprint", "tmp"]);

function readOptionValue(argv, index, flagName) {
  const value = argv[index + 1];
  if (!value || value.startsWith("-")) {
    throw new Error(`Missing value for ${flagName}`);
  }
  return value;
}

function parsePositiveInt(raw, flagName) {
  const parsed = Number(raw);
  if (!Number.isInteger(parsed) || parsed <= 0) {
    throw new Error(`Invalid positive integer for ${flagName}: ${raw}`);
  }
  return parsed;
}

export function parseSweepArgs(argv) {
  let pruneIncrementalFlag = false;
  let keepIncrementalSet = false;
  let keepIncremental = DEFAULT_KEEP_INCREMENTAL;
  let timeDays = null;
  let toolchains = false;
  let dryRun = false;
  let allowMissing = false;
  let quiet = false;
  let help = false;

  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === "--maxsize" || arg.startsWith("--maxsize=")) {
      throw new Error(
        "--maxsize was removed because it deletes active deps/ artifacts by oldest mtime; use --prune-incremental, --time <days>, or --toolchains instead."
      );
    } else if (arg === "--prune-incremental") {
      pruneIncrementalFlag = true;
    } else if (arg === "--keep-incremental") {
      keepIncremental = parsePositiveInt(
        readOptionValue(argv, i, "--keep-incremental"),
        "--keep-incremental"
      );
      keepIncrementalSet = true;
      i += 1;
    } else if (arg.startsWith("--keep-incremental=")) {
      const raw = arg.slice("--keep-incremental=".length);
      if (!raw) {
        throw new Error("Missing value for --keep-incremental");
      }
      keepIncremental = parsePositiveInt(raw, "--keep-incremental");
      keepIncrementalSet = true;
    } else if (arg === "--time") {
      timeDays = parsePositiveInt(readOptionValue(argv, i, "--time"), "--time");
      i += 1;
    } else if (arg.startsWith("--time=")) {
      const raw = arg.slice("--time=".length);
      if (!raw) {
        throw new Error("Missing value for --time");
      }
      timeDays = parsePositiveInt(raw, "--time");
    } else if (arg === "--toolchains") {
      toolchains = true;
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

  const pruneIncremental =
    pruneIncrementalFlag || keepIncrementalSet || (timeDays === null && !toolchains);

  return {
    pruneIncremental,
    keepIncremental,
    timeDays,
    toolchains,
    dryRun,
    allowMissing,
    quiet,
    help,
  };
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

export function extractIncrementalCratePrefix(dirName) {
  const match = /^(.+)-([a-z0-9]{8,})$/.exec(dirName);
  return match ? match[1] : null;
}

export function findIncrementalRoots(targetDir) {
  if (!existsSync(targetDir)) {
    return [];
  }

  const roots = [];
  const stack = [{ dir: targetDir, depth: 0 }];

  while (stack.length > 0) {
    const { dir, depth } = stack.pop();
    let entries;
    try {
      entries = readdirSync(dir, { withFileTypes: true });
    } catch {
      continue;
    }

    for (const entry of entries) {
      if (!entry.isDirectory()) {
        continue;
      }
      const name = entry.name;
      const fullPath = path.join(dir, name);
      if (name === "incremental") {
        roots.push(fullPath);
        continue;
      }
      if (depth >= 3 || SKIP_SCAN_DIRS.has(name) || name.startsWith("flycheck")) {
        continue;
      }
      stack.push({ dir: fullPath, depth: depth + 1 });
    }
  }

  roots.sort((a, b) => a.localeCompare(b));
  return roots;
}

export function inspectIncrementalSessionDir(dirPath) {
  let bytes = 0;
  let newestFileMtimeMs = 0;
  let newestSessionName = "";

  const stack = [{ dir: dirPath, isRoot: true }];

  while (stack.length > 0) {
    const { dir, isRoot } = stack.pop();
    let entries;
    try {
      entries = readdirSync(dir, { withFileTypes: true });
    } catch {
      continue;
    }

    for (const entry of entries) {
      const fullPath = path.join(dir, entry.name);
      if (entry.isDirectory()) {
        if (
          isRoot &&
          /^s-[a-z0-9]+-/.test(entry.name) &&
          entry.name.localeCompare(newestSessionName) > 0
        ) {
          newestSessionName = entry.name;
        }
        stack.push({ dir: fullPath, isRoot: false });
      } else if (entry.isFile()) {
        try {
          const st = statSync(fullPath);
          bytes += st.size;
          if (st.mtimeMs > newestFileMtimeMs) {
            newestFileMtimeMs = st.mtimeMs;
          }
        } catch {
          // Ignore transient files removed concurrently.
        }
      }
    }
  }

  if (newestFileMtimeMs === 0) {
    try {
      newestFileMtimeMs = statSync(dirPath).mtimeMs;
    } catch {
      newestFileMtimeMs = 0;
    }
  }

  return { bytes, newestFileMtimeMs, newestSessionName };
}

export function pruneIncrementalOrphans({
  rootDir = DEFAULT_ROOT_DIR,
  keepPerCrate = DEFAULT_KEEP_INCREMENTAL,
  dryRun = false,
  quiet = false,
} = {}) {
  const targetDir = path.join(rootDir, "src-tauri", "target");
  if (!existsSync(targetDir)) {
    if (!quiet) {
      console.log("[cargo-prune] src-tauri/target does not exist yet; skipping.");
    }
    return {
      status: "skipped-no-target",
      keptCount: 0,
      removedCount: 0,
      freedBytes: 0,
      removedPaths: [],
    };
  }

  const roots = findIncrementalRoots(targetDir);
  let keptCount = 0;
  let removedCount = 0;
  let freedBytes = 0;
  const removedPaths = [];

  for (const incRoot of roots) {
    let entries;
    try {
      entries = readdirSync(incRoot, { withFileTypes: true });
    } catch {
      continue;
    }

    const groups = new Map();
    for (const entry of entries) {
      if (!entry.isDirectory()) {
        continue;
      }
      const cratePrefix = extractIncrementalCratePrefix(entry.name);
      if (!cratePrefix) {
        continue;
      }
      const fullPath = path.join(incRoot, entry.name);
      const stats = inspectIncrementalSessionDir(fullPath);
      const list = groups.get(cratePrefix) ?? [];
      list.push({
        dirName: entry.name,
        fullPath,
        ...stats,
      });
      groups.set(cratePrefix, list);
    }

    for (const list of groups.values()) {
      list.sort(
        (a, b) =>
          b.newestFileMtimeMs - a.newestFileMtimeMs ||
          b.newestSessionName.localeCompare(a.newestSessionName) ||
          b.dirName.localeCompare(a.dirName)
      );

      keptCount += Math.min(keepPerCrate, list.length);
      for (const orphan of list.slice(keepPerCrate)) {
        if (dryRun) {
          removedCount += 1;
          freedBytes += orphan.bytes;
          removedPaths.push(orphan.fullPath);
          continue;
        }

        try {
          rmSync(orphan.fullPath, { recursive: true, force: true });
          removedCount += 1;
          freedBytes += orphan.bytes;
          removedPaths.push(orphan.fullPath);
        } catch (error) {
          console.warn(
            `[cargo-prune] Skipped locked directory ${orphan.fullPath}: ${error.message}`
          );
        }
      }
    }
  }

  if (removedCount === 0) {
    if (!quiet) {
      console.log(
        `[cargo-prune] incremental/ is clean (kept ${keptCount} active session dir(s), 0 orphans).`
      );
    }
    return { status: "clean", keptCount, removedCount: 0, freedBytes: 0, removedPaths: [] };
  }

  if (!quiet) {
    console.log(
      `[cargo-prune] ${dryRun ? "Would remove" : "Removed"} ${removedCount} orphan incremental dir(s) ` +
        `(kept ${keptCount}, ${dryRun ? "potential savings" : "freed"} ${formatGiB(freedBytes)}; deps/ untouched).`
    );
  }

  return { status: "pruned", keptCount, removedCount, freedBytes, removedPaths };
}

export function buildCargoSweepArgs({
  timeDays = null,
  toolchains = false,
  dryRun = false,
  projectDir = "src-tauri",
}) {
  const args = ["sweep"];
  if (toolchains) {
    args.push("--toolchains");
  }
  if (timeDays !== null) {
    args.push("--time", String(timeDays));
  }
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

export function runCargoTargetMaintenance({
  rootDir = DEFAULT_ROOT_DIR,
  pruneIncremental = true,
  keepIncremental = DEFAULT_KEEP_INCREMENTAL,
  timeDays = null,
  toolchains = false,
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
    return {
      status: "skipped-no-target",
      pruneResult: {
        status: "skipped-no-target",
        keptCount: 0,
        removedCount: 0,
        freedBytes: 0,
        removedPaths: [],
      },
      sweepStatus: "skipped-no-target",
    };
  }

  const pruneResult = pruneIncremental
    ? pruneIncrementalOrphans({
        rootDir,
        keepPerCrate: keepIncremental,
        dryRun,
        quiet,
      })
    : null;

  if (timeDays === null && !toolchains) {
    return {
      status: pruneResult?.status ?? "clean",
      pruneResult,
      sweepStatus: "not-requested",
    };
  }

  if (!isCargoSweepInstalled(run)) {
    const message =
      "[cargo-sweep] cargo-sweep is not installed. Run 'cargo install cargo-sweep --locked' to enable time/toolchain cleanup.";
    if (allowMissing) {
      console.warn(message);
      return {
        status: "skipped-missing-tool",
        pruneResult,
        sweepStatus: "skipped-missing-tool",
      };
    }
    throw new Error(message);
  }

  const beforeBytes = dryRun ? 0 : measureDirBytes(targetDir);
  const args = buildCargoSweepArgs({
    timeDays,
    toolchains,
    dryRun,
    projectDir: "src-tauri",
  });
  if (!quiet) {
    console.log(`[cargo-sweep] Running cargo ${args.join(" ")}...`);
  }

  const result = run("cargo", args, {
    cwd: rootDir,
    stdio: "inherit",
  });

  if (result.status !== 0) {
    throw new Error(`cargo sweep exited with code ${result.status ?? 1}`);
  }

  if (!dryRun && !quiet) {
    const afterBytes = measureDirBytes(targetDir);
    const freedBytes = Math.max(0, beforeBytes - afterBytes);
    console.log(
      `[cargo-sweep] Done: ${formatGiB(beforeBytes)} -> ${formatGiB(afterBytes)} (freed ${formatGiB(freedBytes)}).`
    );
  }

  return {
    status: "swept",
    pruneResult,
    sweepStatus: "swept",
  };
}

export const runCargoSweep = runCargoTargetMaintenance;

function printHelp() {
  console.log(`Usage: node scripts/sweep-cargo-target.mjs [options]

Maintains src-tauri/target by pruning orphan incremental/ session folders while keeping deps/ intact.

Options:
  --prune-incremental        Prune orphan incremental/ session dirs (default when no sweep flag is set)
  --keep-incremental <N>     Keep newest N incremental session dirs per crate (default: ${DEFAULT_KEEP_INCREMENTAL})
  --time <DAYS>              Also run 'cargo sweep --time <DAYS>' on stale artifacts
  --toolchains               Also run 'cargo sweep --toolchains' on uninstalled rustc toolchains
  -d, --dry-run              Preview deletions without removing files
  --allow-missing            Warn and exit 0 if cargo-sweep is not installed
  -q, --quiet                Suppress informational messages
  -h, --help                 Show this help message
`);
}

function main() {
  const options = parseSweepArgs(process.argv.slice(2));
  if (options.help) {
    printHelp();
    return;
  }
  runCargoTargetMaintenance(options);
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
