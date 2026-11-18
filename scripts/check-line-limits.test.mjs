import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { checkLineLimits, countFileLines } from "./check-line-limits.mjs";

const tempDirs = [];

function createTempProject() {
  const dir = mkdtempSync(path.join(tmpdir(), "schumazgg-line-limits-"));
  tempDirs.push(dir);
  mkdirSync(path.join(dir, "src-tauri", "src"), { recursive: true });
  mkdirSync(path.join(dir, "src", "types"), { recursive: true });
  return dir;
}

function makeLines(count) {
  return Array.from({ length: count }, (_, i) => `// line ${i + 1}`).join("\n") + "\n";
}

afterEach(() => {
  while (tempDirs.length > 0) {
    rmSync(tempDirs.pop(), { recursive: true, force: true });
  }
});

describe("check-line-limits", () => {
  it("counts lines consistently with and without trailing newline", () => {
    expect(countFileLines("")).toBe(0);
    expect(countFileLines("a\nb")).toBe(2);
    expect(countFileLines("a\nb\n")).toBe(2);
    expect(countFileLines("a\r\nb\r\n")).toBe(2);
  });

  it("enforces 350 Rust / 250 TS-TSX limits and excludes src/types/bindings.ts", () => {
    const rootDir = createTempProject();
    writeFileSync(path.join(rootDir, "src-tauri", "src", "ok.rs"), makeLines(350), "utf8");
    writeFileSync(path.join(rootDir, "src-tauri", "src", "too_long.rs"), makeLines(351), "utf8");
    writeFileSync(path.join(rootDir, "src", "ok.ts"), makeLines(250), "utf8");
    writeFileSync(path.join(rootDir, "src", "too_long.tsx"), makeLines(251), "utf8");
    writeFileSync(path.join(rootDir, "src", "types", "bindings.ts"), makeLines(900), "utf8");

    const result = checkLineLimits({ rootDir, baseline: {} });
    expect(result.checkedCount).toBe(4);
    expect(result.violations).toEqual([
      {
        file: "src-tauri/src/too_long.rs",
        lines: 351,
        limit: 350,
        baseLimit: 350,
        isBaselineRegression: false,
      },
      {
        file: "src/too_long.tsx",
        lines: 251,
        limit: 250,
        baseLimit: 250,
        isBaselineRegression: false,
      },
    ]);
  });

  it("allows legacy baseline files at or below cap and fails if they grow or in strict mode", () => {
    const rootDir = createTempProject();
    writeFileSync(path.join(rootDir, "src-tauri", "src", "legacy.rs"), makeLines(360), "utf8");
    writeFileSync(path.join(rootDir, "src", "legacy.tsx"), makeLines(275), "utf8");

    const baseline = {
      "src-tauri/src/legacy.rs": 360,
      "src/legacy.tsx": 270,
    };

    const baselineResult = checkLineLimits({ rootDir, baseline });
    expect(baselineResult.grandfathered).toEqual([
      {
        file: "src-tauri/src/legacy.rs",
        lines: 360,
        cap: 360,
        baseLimit: 350,
      },
    ]);
    expect(baselineResult.violations).toEqual([
      {
        file: "src/legacy.tsx",
        lines: 275,
        limit: 270,
        baseLimit: 250,
        isBaselineRegression: true,
      },
    ]);

    const strictResult = checkLineLimits({ rootDir, strict: true, baseline });
    expect(strictResult.violations).toHaveLength(2);
  });
});
