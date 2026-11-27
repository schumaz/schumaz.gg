// @ts-expect-error type error without @types/node package
import { existsSync, readdirSync, readFileSync } from "node:fs";
import { afterEach, describe, expect, it, vi } from "vitest";
import { render } from "@testing-library/react";
import { NavigationDock } from "../components/ui/NavigationDock";
import type { ActiveTab } from "../types";
import tokensShellCss from "../styles/tokens-shell.css?raw";
import settingsTabCss from "../features/settings/SettingsTab.css?raw";
import profileTabCss from "../features/profile/ProfileTab.css?raw";
import navigationDockCss from "../styles/navigation-dock.css?raw";

let injectedStyleEl: HTMLStyleElement | null = null;

afterEach(() => {
  injectedStyleEl?.remove();
  injectedStyleEl = null;
});

function collectCssFiles(dir: string): string[] {
  const results: string[] = [];
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const fullPath = `${dir}/${entry.name}`;
    if (entry.isDirectory()) {
      results.push(...collectCssFiles(fullPath));
    } else if (entry.isFile() && entry.name.endsWith(".css")) {
      results.push(fullPath);
    }
  }
  return results;
}

describe("NavigationDock unified static amethyst stylesheet", () => {
  it("keeps all dock styling in src/styles/navigation-dock.css and removes duplicates", () => {
    const appCss = readFileSync("src/App.css", "utf8") as string;
    expect(appCss).toContain('@import "./styles/navigation-dock.css";');

    expect(existsSync("src/components/ui/NavigationDock.css")).toBe(false);

    expect(navigationDockCss).toContain("--dock-surface-base: #0d0e12;");
    expect(navigationDockCss).toContain("--dock-surface-gradient:");
    expect(navigationDockCss).toContain("--dock-border-gradient:");
    expect(navigationDockCss).toContain("--dock-shadow:");
    expect(navigationDockCss).toContain("background-color: var(--dock-surface-base);");
    expect(navigationDockCss).toContain("background-clip: padding-box, border-box;");
    expect(navigationDockCss).not.toContain("backdrop-filter");

    const dockSelectorPattern = /\.(floating-dock|dock-btn|dock-icon|dock-icon-img)\b/;
    const cssFiles = collectCssFiles("src");
    const matchingFiles = cssFiles.filter((filePath) => {
      const content = readFileSync(filePath, "utf8") as string;
      return dockSelectorPattern.test(content);
    });

    expect(matchingFiles).toEqual(["src/styles/navigation-dock.css"]);
  });

  it("renders identical dock surface, gradient, border, shadow, and backdropFilter across all shell states", () => {
    injectedStyleEl = document.createElement("style");
    injectedStyleEl.textContent = [
      tokensShellCss,
      settingsTabCss,
      profileTabCss,
      navigationDockCss,
    ].join("\n");
    document.head.appendChild(injectedStyleEl);

    const states: Array<{ className: string; activeTab: ActiveTab }> = [
      { className: "container profile-active", activeTab: "profile" },
      { className: "container profile-active profile-blur-off", activeTab: "profile" },
      { className: "container gpu-effects-disabled", activeTab: "runes" },
      { className: "container", activeTab: "runes" },
    ];

    const computedSnapshots = states.map(({ className, activeTab }) => {
      const { container, unmount } = render(
        <main className={className}>
          <NavigationDock activeTab={activeTab} onTabChange={vi.fn()} />
        </main>
      );
      const dock = container.querySelector(".floating-dock") as HTMLElement;
      expect(dock).not.toBeNull();
      const style = window.getComputedStyle(dock);
      const snapshot = {
        backgroundColor: style.backgroundColor,
        backgroundImage: style.backgroundImage,
        borderTopColor: style.borderTopColor,
        boxShadow: style.boxShadow,
        backdropFilter: style.backdropFilter,
      };
      unmount();
      return snapshot;
    });

    const [profileSnapshot, ...otherSnapshots] = computedSnapshots;
    expect(profileSnapshot.backgroundColor).not.toBe("");
    expect(profileSnapshot.backgroundColor).not.toBe("transparent");
    expect(profileSnapshot.backgroundColor).not.toBe("rgba(0, 0, 0, 0)");

    for (const snapshot of otherSnapshots) {
      expect(snapshot).toEqual(profileSnapshot);
    }
  });
});
