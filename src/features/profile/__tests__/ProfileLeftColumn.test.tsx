import { afterEach, describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { I18nProvider } from "../../../i18n";
import type { PlayerProfile } from "../../../types";
import { ProfileTab } from "../ProfileTab";
import profileTabCss from "../ProfileTab.css?raw";
import { mockProfile, mockSummoner } from "./profileFixtures";

let injectedStyleEl: HTMLStyleElement | null = null;

afterEach(() => {
  injectedStyleEl?.remove();
  injectedStyleEl = null;
});

function buildStreakProfile(): PlayerProfile {
  return {
    ...mockProfile,
    recent_matches: [
      {
        ...mockProfile.recent_matches[0],
        game_id: 999,
        win: false,
        remake: true,
        kills: 0,
        deaths: 0,
        assists: 0,
      },
      ...mockProfile.recent_matches,
      {
        ...mockProfile.recent_matches[0],
        game_id: 1010,
        win: true,
        remake: false,
      },
    ],
  };
}

describe("P15.0 — ProfileLeftColumn Win/Loss & Recent Form Layout Regression", () => {
  it("renders all 10 non-remake W/L (V/D) streak pills inside [data-testid='recent-streak']", () => {
    const profile = buildStreakProfile();

    const { container, rerender } = render(
      <I18nProvider locale="en_US">
        <ProfileTab
          summoner={mockSummoner}
          profile={profile}
          loading={false}
          errorMsg={null}
          onRefresh={vi.fn()}
        />
      </I18nProvider>
    );

    const leftCol = container.querySelector(".profile-col-left")!;
    const winLossCard = screen.getByTestId("winloss-chart-card");
    const masteriesCard = screen.getByTestId("top-masteries-card");
    expect(leftCol).toContainElement(winLossCard);
    expect(leftCol).toContainElement(masteriesCard);

    const streakBlock = screen.getByTestId("recent-streak");
    expect(winLossCard).toContainElement(streakBlock);
    expect(streakBlock.querySelector(".recent-streak-label")).toHaveTextContent("Recent form");

    const enPills = Array.from(streakBlock.querySelectorAll(".recent-streak-pills .streak-pill"));
    expect(enPills).toHaveLength(10);
    expect(enPills.map((pill) => pill.textContent)).toEqual([
      "W",
      "W",
      "L",
      "W",
      "W",
      "W",
      "L",
      "L",
      "W",
      "L",
    ]);
    for (const pill of enPills) {
      expect(pill.classList.contains("win") || pill.classList.contains("loss")).toBe(true);
      expect(pill.getAttribute("title")?.trim().length).toBeGreaterThan(0);
    }

    rerender(
      <I18nProvider locale="pt_BR">
        <ProfileTab
          summoner={mockSummoner}
          profile={profile}
          loading={false}
          errorMsg={null}
          onRefresh={vi.fn()}
        />
      </I18nProvider>
    );

    expect(screen.getByTestId("recent-streak").querySelector(".recent-streak-label")).toHaveTextContent(
      "Sequência recente"
    );
    const ptPills = Array.from(
      screen.getByTestId("recent-streak").querySelectorAll(".recent-streak-pills .streak-pill")
    );
    expect(ptPills).toHaveLength(10);
    expect(ptPills.map((pill) => pill.textContent)).toEqual([
      "V",
      "V",
      "D",
      "V",
      "V",
      "V",
      "D",
      "D",
      "V",
      "D",
    ]);
  });

  it("prevents vertical shrinking on winloss-chart-card and keeps column-only scroll under viewport lock", () => {
    injectedStyleEl = document.createElement("style");
    injectedStyleEl.textContent = profileTabCss;
    document.head.appendChild(injectedStyleEl);

    const { container } = render(
      <I18nProvider locale="en_US">
        <ProfileTab
          summoner={mockSummoner}
          profile={mockProfile}
          loading={false}
          errorMsg={null}
          onRefresh={vi.fn()}
        />
      </I18nProvider>
    );

    const winLossStyle = window.getComputedStyle(screen.getByTestId("winloss-chart-card"));
    expect(winLossStyle.flexShrink).toBe("0");
    expect(winLossStyle.display).toBe("flex");
    expect(winLossStyle.flexDirection).toBe("column");

    const streakBlock = screen.getByTestId("recent-streak");
    expect(window.getComputedStyle(streakBlock).flexShrink).toBe("0");

    const firstPill = streakBlock.querySelector(".streak-pill")!;
    const firstPillStyle = window.getComputedStyle(firstPill);
    expect(firstPillStyle.flexShrink).toBe("0");
    expect(firstPillStyle.width).toBe("22px");
    expect(firstPillStyle.height).toBe("22px");

    const leftColStyle = window.getComputedStyle(container.querySelector(".profile-col-left")!);
    expect(leftColStyle.overflowY).toBe("auto");
    expect(leftColStyle.overflowX).toBe("hidden");

    const masteriesStyle = window.getComputedStyle(screen.getByTestId("top-masteries-card"));
    expect(masteriesStyle.flexShrink).toBe("0");

    expect(profileTabCss).toMatch(
      /\.profile-split-grid\s*\{[^}]*grid-template-rows:\s*minmax\(0,\s*1fr\);/
    );
    expect(profileTabCss).toMatch(
      /@media\s*\(max-width:\s*860px\)\s*\{[\s\S]*?\.profile-split-grid\s*\{[^}]*grid-template-rows:\s*auto;/
    );
  });
});
