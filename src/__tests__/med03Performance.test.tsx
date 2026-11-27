import { ComponentProps, memo, useState } from "react";
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { act, fireEvent, render, renderHook, screen } from "@testing-library/react";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import * as i18n from "../i18n";
import { resolveCachedAssetUrl } from "../utils/assets";
import { StartupLoadingScreen } from "../components/ui/StartupLoadingScreen";
import { StatusDock } from "../components/ui/StatusDock";
import { NavigationDock } from "../components/ui/NavigationDock";
import { ProfileTab } from "../features/profile/ProfileTab";
import { ProfileHeroCard } from "../features/profile/ProfileHeroCard";
import { ProfileLeftColumn } from "../features/profile/ProfileLeftColumn";
import { ProfileMatchesCard } from "../features/profile/ProfileMatchesCard";
import { ProfileRefreshButton } from "../features/profile/ProfileRefreshButton";
import { ProfileWallpaperBackdrop } from "../features/profile/ProfileWallpaperBackdrop";
import { usePlayerProfile } from "../features/profile/usePlayerProfile";
import { ChampSelectTab } from "../features/champ-select/ChampSelectTab";
import { RankedTab } from "../features/ranked/RankedTab";
import { ArenaTab } from "../features/arena/ArenaTab";
import { AramTab } from "../features/aram/AramTab";
import { SettingsTab } from "../features/settings/SettingsTab";
import { PROFILE_BLUR_PERSIST_DEBOUNCE_MS, usePreferences } from "../features/settings/usePreferences";
import { DEFAULT_RANKED_FILTERS } from "../features/ranked/rankedStorage";
import { DEFAULT_PREFERENCES } from "../types";
import { mockProfile, mockSummoner } from "../features/profile/__tests__/profileFixtures";
import startupLoadingCss from "../components/ui/StartupLoadingScreen.css?raw";
import arenaPanelCss from "../components/ui/ArenaPanel.css?raw";
import profileTabCss from "../features/profile/ProfileTab.css?raw";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue({}),
  convertFileSrc: vi.fn((path: string, protocol = "asset") => `${protocol}://localhost/${path}`),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

const REACT_MEMO_SYMBOL = Symbol.for("react.memo");
const EMPTY_SPELLS: never[] = [];
const EMPTY_ARENA_FILTERS = { patch: null };
const EMPTY_RECORDS = {};
const noop = () => {};

describe("MED-03 React Rendering, IPC Debounce & Asset/CSS Performance", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    delete (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
    delete (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
  });

  it("isolates 1s profile refresh cooldown ticks to ProfileRefreshButton without re-rendering ProfileTab", async () => {
    vi.useFakeTimers();
    vi.mocked(invoke).mockImplementation((cmd: string) => {
      if (cmd === "get_cached_player_profile") {
        return Promise.resolve({ profile: mockProfile, createdAtUnix: 1700000000, isStale: false });
      }
      if (cmd === "get_player_profile") return Promise.resolve(mockProfile);
      return Promise.resolve(null);
    });

    const onProfileTabPropsRender = vi.fn();
    const ProfileTabProbe = memo(function ProfileTabProbe(props: ComponentProps<typeof ProfileTab>) {
      onProfileTabPropsRender();
      return <ProfileTab {...props} />;
    });

    function Harness() {
      const pp = usePlayerProfile(mockSummoner, "pt_BR");
      return (
        <ProfileTabProbe
          summoner={mockSummoner}
          profile={pp.profile}
          loading={pp.profileLoading}
          errorMsg={pp.profileError}
          onRefresh={pp.refreshProfile}
          refreshCooldownUntilMs={pp.refreshCooldownUntilMs}
        />
      );
    }

    render(<Harness />);
    await act(async () => {
      await Promise.resolve();
      await Promise.resolve();
    });

    const btn = screen.getByTestId("refresh-profile-btn");
    await act(async () => {
      fireEvent.click(btn);
      await Promise.resolve();
      await Promise.resolve();
    });

    expect(btn).toBeDisabled();
    expect(btn).toHaveTextContent("Atualizar (30s)");
    const rendersBeforeTicks = onProfileTabPropsRender.mock.calls.length;

    for (let sec = 29; sec >= 25; sec -= 1) {
      await act(async () => {
        await vi.advanceTimersByTimeAsync(1000);
      });
      expect(btn).toHaveTextContent(`Atualizar (${sec}s)`);
    }
    expect(onProfileTabPropsRender).toHaveBeenCalledTimes(rendersBeforeTicks);

    await act(async () => {
      await vi.advanceTimersByTimeAsync(25000);
    });
    expect(btn).toBeEnabled();
    expect(btn).toHaveTextContent("Atualizar");
    expect(onProfileTabPropsRender).toHaveBeenCalledTimes(rendersBeforeTicks);
  });

  it("memoizes all 6 tabs and shell components with React.memo to skip re-renders when parent updates", () => {
    const memoized = [
      ProfileTab, ChampSelectTab, RankedTab, ArenaTab, AramTab, SettingsTab, StatusDock,
      NavigationDock, ProfileWallpaperBackdrop, ProfileHeroCard, ProfileLeftColumn,
      ProfileMatchesCard, ProfileRefreshButton,
    ];
    for (const component of memoized) {
      expect((component as unknown as { $$typeof?: symbol }).$$typeof).toBe(REACT_MEMO_SYMBOL);
    }

    const useTranslationSpy = vi.spyOn(i18n, "useTranslation");
    function Parent() {
      const [tick, setTick] = useState(0);
      return (
        <div>
          <button type="button" data-testid="parent-tick" onClick={() => setTick((n) => n + 1)}>{tick}</button>
          <ProfileTab summoner={null} profile={null} loading={false} errorMsg={null} onRefresh={noop} isActive />
          <ChampSelectTab
            isActive={false} build={null} champSelect={null} summoner={null} loading={false} errorMsg={null}
            orderedSpells={EMPTY_SPELLS} flashKey="D" selectedSpellPairIndex={0} selectedRunePageIndex={0} selectedBuildIndex={0}
            onFilterChange={noop} onSelectRunePage={noop} onInvertSpells={noop} onSelectSpellPair={noop} onSelectBuild={noop}
          />
          <RankedTab
            isActive={false} filters={DEFAULT_RANKED_FILTERS} tierData={null} tierLoading={false} tierError={null}
            selectedChampion={null} championBuild={null} buildLoading={false} buildError={null} orderedSpells={EMPTY_SPELLS}
            selectedRunePageIndex={0} selectedBuildIndex={0} onTierListFilterChange={noop} onDetailFilterChange={noop}
            onSelectChampion={noop} onBackToTierList={noop} onRetryTierList={noop} onRetryBuild={noop}
            onSelectRunePage={noop} onSelectBuild={noop}
          />
          <ArenaTab
            isActive={false} filters={EMPTY_ARENA_FILTERS} tierData={null} tierLoading={false} tierError={null}
            selectedChampion={null} championBuild={null} buildLoading={false} buildError={null} selectedBuildIndex={0}
            onPatchChange={noop} onDetailFilterChange={noop} onSelectChampion={noop} onSelectSynergyChampion={noop}
            onBackToTierList={noop} onRetryTierList={noop} onRetryBuild={noop} onSelectBuild={noop}
          />
          <AramTab
            isActive={false} savedMode="aram" showChooser={false} tierLists={EMPTY_RECORDS} tierLoading={EMPTY_RECORDS}
            tierErrors={EMPTY_RECORDS} selectedChampionKey={null} augmentsByChampion={EMPTY_RECORDS} augmentsLoading={false}
            augmentsError={null} onSelectMode={noop} onSelectChampion={noop} onRetryTierList={noop} onRetryAugments={noop}
          />
          <SettingsTab isActive={false} preferences={DEFAULT_PREFERENCES} onUpdatePreference={noop} />
        </div>
      );
    }

    render(<Parent />);
    const callsAfterMount = useTranslationSpy.mock.calls.length;
    expect(callsAfterMount).toBeGreaterThan(0);

    for (let i = 0; i < 5; i += 1) fireEvent.click(screen.getByTestId("parent-tick"));
    expect(screen.getByTestId("parent-tick")).toHaveTextContent("5");
    expect(useTranslationSpy).toHaveBeenCalledTimes(callsAfterMount);
  });

  it("debounces rapid blur slider updates, supports immediate flush, and flushes dirty blur on unmount", async () => {
    vi.useFakeTimers();
    const { result, unmount } = renderHook(() => usePreferences());
    await act(async () => {
      await Promise.resolve();
      await Promise.resolve();
    });
    vi.mocked(invoke).mockClear();

    act(() => {
      result.current.updateProfileBlur(20);
      result.current.updateProfileBlur(40);
      result.current.updateProfileBlur(60);
      result.current.updateProfileBlur(80);
    });
    expect(result.current.profileBlur).toBe(80);

    await act(async () => { await vi.advanceTimersByTimeAsync(PROFILE_BLUR_PERSIST_DEBOUNCE_MS - 1); });
    expect(invoke).not.toHaveBeenCalledWith("update_profile_appearance", expect.anything());

    await act(async () => { await vi.advanceTimersByTimeAsync(1); });
    expect(invoke).toHaveBeenCalledTimes(1);
    expect(invoke).toHaveBeenLastCalledWith("update_profile_appearance", { blurPercent: 80, wallpaperVisible: true });

    act(() => { result.current.updateProfileBlur(50, { immediate: true }); });
    expect(invoke).toHaveBeenCalledTimes(2);
    expect(invoke).toHaveBeenLastCalledWith("update_profile_appearance", { blurPercent: 50, wallpaperVisible: true });

    await act(async () => { await vi.advanceTimersByTimeAsync(500); });
    expect(invoke).toHaveBeenCalledTimes(2);

    act(() => { result.current.updateProfileBlur(33); });
    expect(invoke).toHaveBeenCalledTimes(2);
    unmount();
    expect(invoke).toHaveBeenCalledTimes(3);
    expect(invoke).toHaveBeenLastCalledWith("update_profile_appearance", { blurPercent: 33, wallpaperVisible: true });
  });

  it("memoizes resolveCachedAssetUrl in Tauri runtime across repeated identical URLs", () => {
    (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ = {};
    const url = "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/champion/Med03CacheHit.png";
    const results = Array.from({ length: 5 }, () => resolveCachedAssetUrl(url));
    expect(new Set(results).size).toBe(1);
    expect(convertFileSrc).toHaveBeenCalledTimes(1);
  });

  it("animates StartupLoadingScreen progress with scaleX, uses async decoding, and consolidates CSS layers", () => {
    render(
      <>
        <StartupLoadingScreen progress={42} stageKey="assets" isExiting={false} />
        <StatusDock summoner={null} />
        <NavigationDock activeTab="profile" onTabChange={vi.fn()} />
      </>
    );

    const fill = screen.getByTestId("startup-progress-bar").querySelector(".startup-progress-fill") as HTMLElement;
    expect(fill.style.transform).toBe("scaleX(0.42)");
    expect(fill.style.width).toBe("");
    for (const img of Array.from(document.querySelectorAll("img"))) {
      expect(img).toHaveAttribute("decoding", "async");
    }

    expect(startupLoadingCss).toMatch(/transform-origin:\s*left center/);
    expect(startupLoadingCss).toMatch(/transition:\s*transform 220ms/);
    expect(arenaPanelCss).toMatch(/\.arena-augment-row\s*\{[^}]*content-visibility:\s*auto/);
    expect(profileTabCss.match(/[^-]backdrop-filter:\s*blur\(var\(--profile-ui-blur/g)).toHaveLength(1);
  });
});
