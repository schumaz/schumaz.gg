import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  buildCargoSweepArgs,
  DEFAULT_MAX_SIZE,
  measureDirBytes,
  parseSweepArgs,
  runCargoSweep,
} from "./sweep-cargo-target.mjs";

const tempDirs = [];

function createTempProject() {
  const dir = mkdtempSync(path.join(tmpdir(), "schumazgg-sweep-test-"));
  tempDirs.push(dir);
  return dir;
}

afterEach(() => {
  while (tempDirs.length > 0) {
    const dir = tempDirs.pop();
    rmSync(dir, { recursive: true, force: true });
  }
});

describe("sweep-cargo-target CLI and helpers", () => {
  it("parses default and explicit CLI options", () => {
    expect(parseSweepArgs([])).toEqual({
      maxSize: DEFAULT_MAX_SIZE,
      ifOverGb: null,
      dryRun: false,
      allowMissing: false,
      quiet: false,
      help: false,
    });

    expect(
      parseSweepArgs(["--maxsize=3GB", "--if-over-gb", "6", "--dry-run", "--allow-missing", "-q"])
    ).toEqual({
      maxSize: "3GB",
      ifOverGb: 6,
      dryRun: true,
      allowMissing: true,
      quiet: true,
      help: false,
    });
  });

  it("rejects invalid or unknown arguments", () => {
    expect(() => parseSweepArgs(["--maxsize"])).toThrow("Missing value for --maxsize");
    expect(() => parseSweepArgs(["--if-over-gb=0"])).toThrow("Invalid positive number");
    expect(() => parseSweepArgs(["--bogus"])).toThrow("Unknown option: --bogus");
  });

  it("builds positional cargo-sweep arguments targeting src-tauri", () => {
    expect(buildCargoSweepArgs({ maxSize: "4GB", dryRun: false })).toEqual([
      "sweep",
      "--maxsize",
      "4GB",
      "src-tauri",
    ]);
    expect(buildCargoSweepArgs({ maxSize: "2GB", dryRun: true })).toEqual([
      "sweep",
      "--maxsize",
      "2GB",
      "--dry-run",
      "src-tauri",
    ]);
  });

  it("skips when target directory is missing or under threshold", () => {
    const rootDir = createTempProject();
    const run = vi.fn();

    expect(runCargoSweep({ rootDir, quiet: true, run }).status).toBe("skipped-no-target");
    expect(run).not.toHaveBeenCalled();

    const targetDir = path.join(rootDir, "src-tauri", "target", "debug");
    mkdirSync(targetDir, { recursive: true });
    writeFileSync(path.join(targetDir, "sample.bin"), Buffer.alloc(1024));

    expect(measureDirBytes(path.join(rootDir, "src-tauri", "target"))).toBe(1024);
    expect(runCargoSweep({ rootDir, ifOverGb: 1, quiet: true, run }).status).toBe(
      "skipped-under-threshold"
    );
    expect(run).not.toHaveBeenCalled();
  });

  it("handles missing cargo-sweep gracefully when allowMissing is true and invokes sweep when present", () => {
    const rootDir = createTempProject();
    const targetDir = path.join(rootDir, "src-tauri", "target", "debug");
    mkdirSync(targetDir, { recursive: true });
    writeFileSync(path.join(targetDir, "sample.bin"), Buffer.alloc(2048));

    const missingRun = vi.fn(() => ({ status: 1 }));
    expect(
      runCargoSweep({ rootDir, allowMissing: true, quiet: true, run: missingRun }).status
    ).toBe("skipped-missing-tool");

    const okRun = vi.fn(() => ({ status: 0 }));
    expect(
      runCargoSweep({ rootDir, maxSize: "4GB", dryRun: true, quiet: true, run: okRun }).status
    ).toBe("swept");
    expect(okRun).toHaveBeenNthCalledWith(1, "cargo", ["sweep", "--version"], expect.any(Object));
    expect(okRun).toHaveBeenNthCalledWith(
      2,
      "cargo",
      ["sweep", "--maxsize", "4GB", "--dry-run", "src-tauri"],
      expect.objectContaining({ cwd: rootDir })
    );
  });
});
