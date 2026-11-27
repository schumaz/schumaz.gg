import { describe, expect, it } from "vitest";
import tokensShellCss from "../styles/tokens-shell.css?raw";
import tierListSharedCss from "../styles/TierListShared.css?raw";
import aramMayhemCss from "../features/aram/AramMayhem.css?raw";
import settingsTabCss from "../features/settings/SettingsTab.css?raw";
import profileTabCss from "../features/profile/ProfileTab.css?raw";
import profileMatchesCss from "../features/profile/ProfileMatches.css?raw";
import matchScoreboardCss from "../features/profile/MatchScoreboard.css?raw";
import champSelectCss from "../features/champ-select/ChampSelect.css?raw";
import arenaPanelCss from "../components/ui/ArenaPanel.css?raw";
import itemBlocksCss from "../components/ui/ItemBlocks.css?raw";

function extractRuleBlock(css: string, selectorPattern: RegExp): string {
  const match = css.match(selectorPattern);
  expect(match, `Expected CSS rule matching ${selectorPattern}`).not.toBeNull();
  return match![0];
}

describe("Tab layout centering, grid track sizing, and block-only intrinsic sizing", () => {
  it("keeps .header padding symmetric and places the 16px brand inset on .header-brand", () => {
    const headerRule = extractRuleBlock(tokensShellCss, /\.header\s*\{[^}]*\}/);
    expect(headerRule).toMatch(/padding:\s*0\s*;/);

    const brandRule = extractRuleBlock(tokensShellCss, /\.header-brand\s*\{[^}]*\}/);
    expect(brandRule).toMatch(/padding:\s*6px 8px 6px 16px\s*;/);
  });

  it("centers Settings groups inside a full-width container with symmetric scrollbar gutters", () => {
    const containerRule = extractRuleBlock(settingsTabCss, /\.settings-container\s*\{[^}]*\}/);
    expect(containerRule).toMatch(/width:\s*100%\s*;/);
    expect(containerRule).toMatch(/align-items:\s*center\s*;/);
    expect(containerRule).toMatch(/scrollbar-gutter:\s*stable both-edges\s*;/);

    const groupRule = extractRuleBlock(settingsTabCss, /\.settings-section-group\s*\{[^}]*\}/);
    expect(groupRule).toMatch(/width:\s*100%\s*;/);
    expect(groupRule).toMatch(/max-width:\s*804px\s*;/);
  });

  it("uses minmax(0, ...) with >=1fr tracks and min-width: 0 on split grids and columns", () => {
    const aramGridRule = extractRuleBlock(
      aramMayhemCss,
      /\/\* ARAM Mayhem Split Layout \*\/\s*\.aram-mayhem-grid\s*\{[^}]*\}/
    );
    expect(aramGridRule).toMatch(
      /grid-template-columns:\s*minmax\(0,\s*85fr\)\s+minmax\(0,\s*135fr\)\s*;/
    );
    const aramColRule = extractRuleBlock(aramMayhemCss, /\.aram-mayhem-col\s*\{[^}]*\}/);
    expect(aramColRule).toMatch(/min-width:\s*0\s*;/);

    const profileGridRule = extractRuleBlock(
      profileTabCss,
      /\/\* Profile 2-Column Split Grid \*\/\s*\.profile-split-grid\s*\{[^}]*\}/
    );
    expect(profileGridRule).toMatch(
      /grid-template-columns:\s*minmax\(0,\s*92fr\)\s+minmax\(0,\s*138fr\)\s*;/
    );
    const profileColRule = extractRuleBlock(profileTabCss, /\.profile-col\s*\{[^}]*\}/);
    expect(profileColRule).toMatch(/min-width:\s*0\s*;/);

    const dashboardGridRule = extractRuleBlock(
      champSelectCss,
      /\.dashboard-split-grid\s*\{[^}]*\}/
    );
    expect(dashboardGridRule).toMatch(
      /grid-template-columns:\s*minmax\(0,\s*1fr\)\s+minmax\(0,\s*1\.04fr\)\s*;/
    );
    const dashboardColRule = extractRuleBlock(champSelectCss, /\.dashboard-col\s*\{[^}]*\}/);
    expect(dashboardColRule).toMatch(/min-width:\s*0\s*;/);

    const arenaBlocksRule = extractRuleBlock(
      arenaPanelCss,
      /\.grid-blocks\.grid-blocks-arena\s*\{[^}]*\}/
    );
    expect(arenaBlocksRule).toMatch(
      /grid-template-columns:\s*minmax\(0,\s*1\.55fr\)\s+minmax\(0,\s*1fr\)\s*;/
    );

    const itemBlocksRule = extractRuleBlock(itemBlocksCss, /\.grid-blocks\s*\{[^}]*\}/);
    expect(itemBlocksRule).toMatch(
      /grid-template-columns:\s*minmax\(0,\s*1fr\)\s+minmax\(0,\s*1\.38fr\)\s*;/
    );

    // Ensure no grid-template-columns or grid-template-rows uses sub-1fr tracks (0.xfr)
    for (const css of [
      aramMayhemCss,
      tierListSharedCss,
      profileTabCss,
      profileMatchesCss,
      matchScoreboardCss,
      champSelectCss,
      arenaPanelCss,
      itemBlocksCss,
    ]) {
      expect(css).not.toMatch(/grid-template-(?:columns|rows):[^;]*\b0\.\d+fr\b/);
    }
  });

  it("uses contain-intrinsic-block-size instead of two-axis contain-intrinsic-size across tabs and rows", () => {
    const stylesheets = [
      { name: "tokens-shell.css", css: tokensShellCss },
      { name: "TierListShared.css", css: tierListSharedCss },
      { name: "AramMayhem.css", css: aramMayhemCss },
      { name: "ProfileMatches.css", css: profileMatchesCss },
      { name: "ArenaPanel.css", css: arenaPanelCss },
      { name: "MatchScoreboard.css", css: matchScoreboardCss },
    ];

    for (const { name, css } of stylesheets) {
      expect(css, `${name} must not lock horizontal intrinsic size`).not.toMatch(
        /contain-intrinsic-size\s*:/
      );
      expect(css, `${name} must declare contain-intrinsic-block-size`).toMatch(
        /contain-intrinsic-block-size\s*:/
      );
    }
  });
});
