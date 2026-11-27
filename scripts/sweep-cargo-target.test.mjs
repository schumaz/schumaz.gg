import { existsSync, mkdirSync, mkdtempSync, rmSync, utimesSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  buildCargoSweepArgs,
  DEFAULT_KEEP_INCREMENTAL,
  extractIncrementalCratePrefix,
  findIncrementalRoots,
  measureDirBytes,
  parseSweepArgs,
  pruneIncrementalOrphans,
  runCargoTargetMaintenance,
} from "./sweep-cargo-target.mjs";

const tempDirs = [];

function createTempProject() {
  const dir = mkdtempSync(path.join(tmpdir(), "schumazgg-sweep-test-"));
  tempDirs.push(dir);
  return dir;
}

function createIncrementalCrateDir(
  incRoot,
  dirName,
  { mtimeSec, sizeBytes = 256, sessionName = null }
) {
  const crateDir = path.join(incRoot, dirName);
  const targetDir = sessionName ? path.join(crateDir, sessionName) : crateDir;
  mkdirSync(targetDir, { recursive: true });
  const filePath = path.join(targetDir, "dep-graph.bin");
  writeFileSync(filePath, Buffer.alloc(sizeBytes));
  const stamp = new Date(mtimeSec * 1000);
  utimesSync(filePath, stamp, stamp);
  return crateDir;
}

afterEach(() => {
  while (tempDirs.length > 0) {
    const dir = tempDirs.pop();
    rmSync(dir, { recursive: true, force: true });
  }
});

describe("sweep-cargo-target CLI and incremental pruner", () => {
  it("parses default and explicit CLI options and rejects --maxsize", () => {
    expect(parseSweepArgs([])).toEqual({
      pruneIncremental: true,
      keepIncremental: DEFAULT_KEEP_INCREMENTAL,
      timeDays: null,
      toolchains: false,
      dryRun: false,
      allowMissing: false,
      quiet: false,
      help: false,
    });

    expect(
      parseSweepArgs([
        "--prune-incremental",
        "--keep-incremental",
        "2",
        "--time=14",
        "--toolchains",
        "--dry-run",
        "--allow-missing",
        "-q",
      ])
    ).toEqual({
      pruneIncremental: true,
      keepIncremental: 2,
      timeDays: 14,
      toolchains: true,
      dryRun: true,
      allowMissing: true,
      quiet: true,
      help: false,
    });

    expect(parseSweepArgs(["--time", "30"]).pruneIncremental).toBe(false);
    expect(() => parseSweepArgs(["--maxsize", "4GB"])).toThrow("--maxsize was removed");
    expect(() => parseSweepArgs(["--maxsize=4GB"])).toThrow("--maxsize was removed");
    expect(() => parseSweepArgs(["--keep-incremental", "0"])).toThrow("Invalid positive integer");
    expect(() => parseSweepArgs(["--time"])).toThrow("Missing value for --time");
    expect(() => parseSweepArgs(["--bogus"])).toThrow("Unknown option: --bogus");
  });

  it("extracts crate prefix from incremental directory names", () => {
    expect(extractIncrementalCratePrefix("schumazgg_lib-39dwbitvk6o51")).toBe("schumazgg_lib");
    expect(extractIncrementalCratePrefix("my-crate-name-0k80o2h0h0z7d")).toBe("my-crate-name");
    expect(extractIncrementalCratePrefix("not_a_hash_dir")).toBeNull();
    expect(extractIncrementalCratePrefix("short-abc12")).toBeNull();
  });

  it("prunes only oldest incremental sessions per crate while preserving deps/", () => {
    const rootDir = createTempProject();
    const targetDir = path.join(rootDir, "src-tauri", "target");
    const debugDeps = path.join(targetDir, "debug", "deps");
    const debugInc = path.join(targetDir, "debug", "incremental");
    const raInc = path.join(targetDir, "rust-analyzer", "debug", "incremental");

    mkdirSync(debugDeps, { recursive: true });
    const activeDepFile = path.join(debugDeps, "libtokio-1234.rlib");
    writeFileSync(activeDepFile, Buffer.alloc(1024));
    utimesSync(activeDepFile, new Date(1_700_000_000_000), new Date(1_700_000_000_000));

    // Same parent mtime ties resolved by inner file mtime and session folder name
    const baseSec = 1_760_000_000;
    const oldA = createIncrementalCrateDir(debugInc, "schumazgg_lib-aaaaaaaa0001", {
      mtimeSec: baseSec + 10,
      sizeBytes: 300,
      sessionName: "s-hoi100-01-aaa",
    });
    const oldB = createIncrementalCrateDir(debugInc, "schumazgg_lib-aaaaaaaa0002", {
      mtimeSec: baseSec + 20,
      sizeBytes: 400,
      sessionName: "s-hoi200-01-bbb",
    });
    // Tie on mtimeSec, distinct sessionName
    const tieOlder = createIncrementalCrateDir(debugInc, "schumazgg_lib-aaaaaaaa0003", {
      mtimeSec: baseSec + 30,
      sizeBytes: 500,
      sessionName: "s-hoi300-01-ccc",
    });
    const tieNewer = createIncrementalCrateDir(debugInc, "schumazgg_lib-aaaaaaaa0004", {
      mtimeSec: baseSec + 30,
      sizeBytes: 600,
      sessionName: "s-hoi400-01-ddd",
    });
    const newest = createIncrementalCrateDir(debugInc, "schumazgg_lib-aaaaaaaa0005", {
      mtimeSec: baseSec + 40,
      sizeBytes: 700,
      sessionName: "s-hoi500-01-eee",
    });
    const raKeep = createIncrementalCrateDir(raInc, "schumazgg_lib-bbbbbbbb0001", {
      mtimeSec: baseSec + 10,
      sizeBytes: 200,
      sessionName: "s-hoi100-01-ra1",
    });
    mkdirSync(path.join(debugInc, "unrecognized_dir"), { recursive: true });

    expect(findIncrementalRoots(targetDir)).toEqual([debugInc, raInc]);

    const dry = pruneIncrementalOrphans({ rootDir, keepPerCrate: 2, dryRun: true, quiet: true });
    expect(dry.status).toBe("pruned");
    expect(dry.keptCount).toBe(3); // 2 in debug/incremental + 1 in rust-analyzer/debug/incremental
    expect(dry.removedCount).toBe(3);
    expect(dry.freedBytes).toBe(300 + 400 + 500);
    expect(existsSync(oldA)).toBe(true);

    const real = pruneIncrementalOrphans({ rootDir, keepPerCrate: 2, dryRun: false, quiet: true });
    expect(real.status).toBe("pruned");
    expect(real.removedCount).toBe(3);
    expect(existsSync(oldA)).toBe(false);
    expect(existsSync(oldB)).toBe(false);
    expect(existsSync(tieOlder)).toBe(false);
    expect(existsSync(tieNewer)).toBe(true);
    expect(existsSync(newest)).toBe(true);
    expect(existsSync(raKeep)).toBe(true);
    expect(existsSync(path.join(debugInc, "unrecognized_dir"))).toBe(true);
    expect(existsSync(activeDepFile)).toBe(true);
    expect(measureDirBytes(debugDeps)).toBe(1024);
  });

  it("builds cargo-sweep args and executes maintenance without wiping active deps", () => {
    expect(buildCargoSweepArgs({ timeDays: 30, toolchains: true, dryRun: true })).toEqual([
      "sweep",
      "--toolchains",
      "--time",
      "30",
      "--dry-run",
      "src-tauri",
    ]);

    const rootDir = createTempProject();
    const run = vi.fn();
    expect(runCargoTargetMaintenance({ rootDir, quiet: true, run }).status).toBe(
      "skipped-no-target"
    );
    expect(run).not.toHaveBeenCalled();

    const debugInc = path.join(rootDir, "src-tauri", "target", "debug", "incremental");
    mkdirSync(debugInc, { recursive: true });

    const missingRun = vi.fn(() => ({ status: 1 }));
    expect(
      runCargoTargetMaintenance({
        rootDir,
        timeDays: 30,
        allowMissing: true,
        quiet: true,
        run: missingRun,
      }).status
    ).toBe("skipped-missing-tool");

    const okRun = vi.fn(() => ({ status: 0 }));
    const res = runCargoTargetMaintenance({
      rootDir,
      pruneIncremental: true,
      timeDays: 30,
      toolchains: true,
      dryRun: true,
      quiet: true,
      run: okRun,
    });
    expect(res.status).toBe("swept");
    expect(okRun).toHaveBeenNthCalledWith(1, "cargo", ["sweep", "--version"], expect.any(Object));
    expect(okRun).toHaveBeenNthCalledWith(
      2,
      "cargo",
      ["sweep", "--toolchains", "--time", "30", "--dry-run", "src-tauri"],
      expect.objectContaining({ cwd: rootDir })
    );
  });
});
