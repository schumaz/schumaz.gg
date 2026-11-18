import { existsSync, readdirSync, readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const DEFAULT_ROOT_DIR = path.resolve(__dirname, "..");

export const RUST_MAX_LINES = 350;
export const TS_MAX_LINES = 250;
export const EXCLUDED_FILES = new Set(["src/types/bindings.ts"]);

// Temporary P11.1 baseline for pre-existing files above the 350/250 ceiling.
// Prevents regressions in CI while allowing P11.1 to refactor them incrementally.
export const LEGACY_BASELINE = {
  "src-tauri/src/app_bootstrap.rs": 352,
  "src-tauri/src/ws_listener/polling.rs": 376,
  "src/App.tsx": 322,
  "src/hooks/useAppBootstrap.ts": 323,
  "src/hooks/useAppUpdater.ts": 296,
  "src/features/settings/SettingsMiscSection.tsx": 277,
  "src/features/settings/__tests__/SettingsTabMisc.test.tsx": 302,
  "src/__tests__/appTestFixtures.ts": 309,
  "src/__tests__/AramTab.test.tsx": 252,
  "src/__tests__/StartupLoadingScreen.test.tsx": 297,
  "src/__tests__/useAppUpdater.test.ts": 338,
};

export function countFileLines(content) {
  if (!content) {
    return 0;
  }
  const lines = content.split(/\r?\n/);
  if (lines[lines.length - 1] === "") {
    lines.pop();
  }
  return lines.length;
}

function walkFiles(dir, predicate, out = []) {
  if (!existsSync(dir)) {
    return out;
  }
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const fullPath = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      if (entry.name === "node_modules" || entry.name === "target" || entry.name === "dist") {
        continue;
      }
      walkFiles(fullPath, predicate, out);
    } else if (entry.isFile() && predicate(fullPath)) {
      out.push(fullPath);
    }
  }
  return out;
}

export function toPosixRelative(rootDir, fullPath) {
  return path.relative(rootDir, fullPath).replace(/\\/g, "/");
}

export function checkLineLimits({
  rootDir = DEFAULT_ROOT_DIR,
  strict = false,
  baseline = LEGACY_BASELINE,
} = {}) {
  const rustDir = path.join(rootDir, "src-tauri", "src");
  const frontendDir = path.join(rootDir, "src");

  if (!existsSync(rustDir)) {
    throw new Error(`Rust source directory not found: ${rustDir}`);
  }
  if (!existsSync(frontendDir)) {
    throw new Error(`Frontend source directory not found: ${frontendDir}`);
  }

  const rustFiles = walkFiles(rustDir, (file) => file.endsWith(".rs"));
  const tsFiles = walkFiles(
    frontendDir,
    (file) => file.endsWith(".ts") || file.endsWith(".tsx")
  );

  const targets = [
    ...rustFiles.map((fullPath) => ({ fullPath, baseLimit: RUST_MAX_LINES })),
    ...tsFiles.map((fullPath) => ({ fullPath, baseLimit: TS_MAX_LINES })),
  ];

  const violations = [];
  const grandfathered = [];
  let checkedCount = 0;

  for (const { fullPath, baseLimit } of targets) {
    const relPath = toPosixRelative(rootDir, fullPath);
    if (EXCLUDED_FILES.has(relPath)) {
      continue;
    }

    checkedCount += 1;
    const lineCount = countFileLines(readFileSync(fullPath, "utf8"));
    const baselineCap = !strict ? baseline[relPath] : undefined;
    const effectiveLimit =
      typeof baselineCap === "number" ? Math.max(baseLimit, baselineCap) : baseLimit;

    if (lineCount > effectiveLimit) {
      violations.push({
        file: relPath,
        lines: lineCount,
        limit: effectiveLimit,
        baseLimit,
        isBaselineRegression: typeof baselineCap === "number",
      });
    } else if (lineCount > baseLimit) {
      grandfathered.push({
        file: relPath,
        lines: lineCount,
        cap: baselineCap,
        baseLimit,
      });
    }
  }

  violations.sort((a, b) => a.file.localeCompare(b.file));
  grandfathered.sort((a, b) => a.file.localeCompare(b.file));

  return { checkedCount, violations, grandfathered };
}

function main() {
  const strict = process.argv.includes("--strict");
  const { checkedCount, violations, grandfathered } = checkLineLimits({ strict });

  console.log(
    `[line-limits] Checked ${checkedCount} files (Rust <= ${RUST_MAX_LINES}, TS/TSX <= ${TS_MAX_LINES}, mode=${strict ? "strict" : "baseline"}).`
  );

  if (grandfathered.length > 0) {
    console.log(`[line-limits] Legacy P11.1 baseline files (${grandfathered.length}):`);
    for (const item of grandfathered) {
      console.log(`  - ${item.file}: ${item.lines}/${item.cap} lines (target <= ${item.baseLimit})`);
    }
  }

  if (violations.length > 0) {
    console.error(`\n[line-limits] Found ${violations.length} file(s) exceeding line limits:`);
    for (const item of violations) {
      const suffix = item.isBaselineRegression
        ? ` (exceeded legacy baseline cap; target <= ${item.baseLimit})`
        : "";
      console.error(`  - ${item.file}: ${item.lines} lines (limit ${item.limit})${suffix}`);
    }
    process.exit(1);
  }

  console.log("[line-limits] All files are within allowed limits.");
}

const isMainModule =
  process.argv[1] && path.resolve(process.argv[1]) === path.resolve(__filename);
if (isMainModule) {
  main();
}
