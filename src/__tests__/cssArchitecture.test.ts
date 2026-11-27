// @ts-expect-error type error without @types/node package
import { readdirSync, readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

function collectFiles(dir: string, predicate: (filePath: string) => boolean): string[] {
  const results: string[] = [];
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const fullPath = `${dir}/${entry.name}`;
    if (entry.isDirectory()) {
      results.push(...collectFiles(fullPath, predicate));
    } else if (entry.isFile() && predicate(fullPath)) {
      results.push(fullPath);
    }
  }
  return results;
}

function countLines(content: string): number {
  const lines = content.split(/\r?\n/);
  if (lines[lines.length - 1] === "") lines.pop();
  return lines.length;
}

describe("MED-01 Modular CSS Architecture & Tokenization", () => {
  it("centralizes all CSS imports in src/App.css and removes direct CSS imports from components", () => {
    const appCss = readFileSync("src/App.css", "utf8") as string;
    const importedPaths = Array.from(appCss.matchAll(/@import\s+"\.\/([^"]+\.css)";/g)).map(
      (match) => `src/${match[1]}`
    );

    const allCssFiles = collectFiles("src", (p) => p.endsWith(".css") && p !== "src/App.css").sort();
    expect([...importedPaths].sort()).toEqual(allCssFiles);
    expect(importedPaths[importedPaths.length - 1]).toBe("src/styles/navigation-dock.css");

    const sourceFiles = collectFiles(
      "src",
      (p) => (p.endsWith(".ts") || p.endsWith(".tsx")) && !p.includes("__tests__") && !p.includes(".test.")
    );
    const directCssImporters = sourceFiles.filter((filePath) => {
      const content = readFileSync(filePath, "utf8") as string;
      return /import\s+["'][^"']+\.css["']/.test(content);
    });
    expect(directCssImporters).toEqual(["src/App.tsx"]);
  });

  it("removes all [data-testid] selectors from CSS and :nth-child(5) coupling from MatchScoreboard.css", () => {
    const cssFiles = collectFiles("src", (p) => p.endsWith(".css"));
    for (const filePath of cssFiles) {
      const content = readFileSync(filePath, "utf8") as string;
      expect(content, `${filePath} must not contain [data-testid] CSS selectors`).not.toContain(
        "[data-testid"
      );
    }

    const scoreboardCss = readFileSync("src/features/profile/MatchScoreboard.css", "utf8") as string;
    expect(scoreboardCss).not.toContain(":nth-child(5)");
    expect(scoreboardCss).toContain(".sb-col-vision");
  });

  it("unifies match loadout/item slots via --slot-size custom properties", () => {
    const slotsCss = readFileSync("src/features/profile/MatchLoadoutSlots.css", "utf8") as string;
    const matchesCss = readFileSync("src/features/profile/ProfileMatches.css", "utf8") as string;
    const scoreboardCss = readFileSync("src/features/profile/MatchScoreboard.css", "utf8") as string;

    expect(slotsCss).toContain("--slot-size");
    expect(matchesCss).toContain("var(--slot-size");
    expect(scoreboardCss).toMatch(/\.match-scoreboard-drawer\s*\{[^}]*--slot-size:\s*19px;/);
  });

  it("keeps modularized stylesheets within <= 450 lines and isolated by domain owner", () => {
    const caps = [
      "src/features/profile/ProfileTab.css",
      "src/features/aram/AramTab.css",
      "src/features/settings/SettingsTab.css",
      "src/styles/loading-skeleton.css",
    ];
    for (const filePath of caps) {
      const content = readFileSync(filePath, "utf8") as string;
      expect(countLines(content), `${filePath} exceeded 450 lines`).toBeLessThanOrEqual(450);
    }

    const profileCss = readFileSync("src/features/profile/ProfileTab.css", "utf8") as string;
    expect(profileCss).not.toContain(".blur-range-slider");
    expect(profileCss).not.toContain(".settings-modal-");
    expect(profileCss).not.toContain(".patch-protection-");

    const aramCss = readFileSync("src/features/aram/AramTab.css", "utf8") as string;
    expect(aramCss).not.toContain(".aram-search-");
    expect(aramCss).not.toContain(".aram-tier-pill");

    const skeletonCss = readFileSync("src/styles/loading-skeleton.css", "utf8") as string;
    expect(skeletonCss).not.toContain(".language-onboarding-");
    expect(skeletonCss).not.toContain(".settings-language-");
  });
});
