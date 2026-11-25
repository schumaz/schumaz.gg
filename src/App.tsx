import { CSSProperties, useEffect, useMemo, useRef, useState } from "react";
import "./App.css";
import appIconUrl from "./assets/app-icon.png";
import { ActiveTab, FlashKeyPreference } from "./types";
import { injectorIpc } from "./services/ipc";
import { I18nProvider } from "./i18n";
import { useAppBootstrap } from "./hooks/useAppBootstrap";
import { useAppUpdater } from "./hooks/useAppUpdater";
import { useWindowMinimized } from "./hooks/useWindowMinimized";
import { resolveCachedAssetUrl } from "./utils/assets";
import { computeOrderedSpells } from "./utils/spells";
import {
  ARAM_DOCK_ICON_URL,
  NavigationDock,
  StartupLoadingScreen,
  StatusDock,
} from "./components/ui";
import {
  FlashOnboardingModal,
  LanguageOnboardingModal,
  profileBlurPercentToPx,
  usePreferences,
} from "./features/settings";
import { ProfileWallpaperBackdrop, usePlayerProfile } from "./features/profile";
import { isInGamePhase, useLcuEvents } from "./features/champ-select";
import { useAram } from "./features/aram";
import { useArena } from "./features/arena";
import { useRanked } from "./features/ranked";
import { AppTabsViewport } from "./AppTabsViewport";

export * from "./types";
export * from "./utils/spells";

export interface AppProps {
  forceStartupLoading?: boolean;
}

function App({ forceStartupLoading }: AppProps = {}) {
  const [activeTab, setActiveTab] = useState<ActiveTab>("profile");
  const isAppMinimized = useWindowMinimized();
  const patchChangeHandlersRef = useRef<Array<() => void>>([]);

  const handleFlashKeyChange = (newFlashKey: FlashKeyPreference) => {
    if (
      isInGamePhase(lcu.gameflowPhase) ||
      lcu.build?.supports_spells === false ||
      lcu.champSelect?.queue_mode === "Arena"
    ) {
      return;
    }
    const reordered = computeOrderedSpells(
      lcu.build?.spells,
      lcu.selectedSpellPairIndex,
      lcu.build?.spell_options,
      newFlashKey,
      lcu.spellsInverted
    );
    if (reordered.length >= 2) {
      injectorIpc.injectSummonerSpells(reordered[0].id, reordered[1].id, null).catch(() => {});
    }
  };

  const handlePatchProtectionChange = () => {
    if (lcu.build) lcu.handleFilterChange("patch", "");
    for (const handler of patchChangeHandlersRef.current) handler();
  };

  const prefs = usePreferences(handleFlashKeyChange, handlePatchProtectionChange);
  const { locale, preferences, navigationState, updateNavigationState } = prefs;

  const lcu = useLcuEvents(preferences.flashKey, {
    onChampionPulled: () => setActiveTab("runes"),
    onMatchEnded: () => setActiveTab("profile"),
    locale,
    localeReady: prefs.preferencesReady,
  });

  const updater = useAppUpdater({
    enabled: import.meta.env.MODE !== "test",
    lcuInitialChecked: lcu.lcuInitialChecked,
    gameflowPhase: lcu.gameflowPhase,
    isInChampSelect: lcu.champSelect !== null,
  });

  const currentChampionId = lcu.champSelect?.champion_id ?? lcu.build?.champion_id ?? null;

  const aramPersistence = useMemo(
    () => ({
      savedAramMode: navigationState?.savedAramMode ?? null,
      savedLastChampion: navigationState?.aramMayhemLastChampion ?? null,
      onNavigationPatch: updateNavigationState,
    }),
    [navigationState?.savedAramMode, navigationState?.aramMayhemLastChampion, updateNavigationState]
  );

  const rankedPersistence = useMemo(
    () => ({
      savedFilters: navigationState
        ? {
            region: navigationState.rankedFilterRegion,
            tier: navigationState.rankedFilterTier,
            lane: navigationState.rankedFilterLane,
            patch: navigationState.rankedFilterPatch ?? null,
          }
        : undefined,
      onNavigationPatch: updateNavigationState,
    }),
    [navigationState, updateNavigationState]
  );

  const patch48hProtection = preferences.patch48hProtection ?? true;
  const aram = useAram(activeTab === "aram", currentChampionId, locale, aramPersistence, preferences.flashKey);
  const ranked = useRanked(activeTab === "ranked", preferences.flashKey, patch48hProtection, locale, rankedPersistence);
  const arena = useArena(activeTab === "arena", patch48hProtection, locale);

  useEffect(() => {
    patchChangeHandlersRef.current = [ranked.handlePatchProtectionChange, arena.handlePatchProtectionChange, aram.handlePatchProtectionChange];
  }, [ranked.handlePatchProtectionChange, arena.handlePatchProtectionChange, aram.handlePatchProtectionChange]);

  const playerProfile = usePlayerProfile(lcu.summoner, locale);

  const startupEnabled = forceStartupLoading ?? import.meta.env.MODE !== "test";
  const prefersReducedMotion =
    typeof window !== "undefined" &&
    typeof window.matchMedia === "function" &&
    window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  const criticalImageUrls = useMemo(() => {
    const urls = [appIconUrl, resolveCachedAssetUrl(ARAM_DOCK_ICON_URL)];
    if (prefs.wallpaperVisible && playerProfile.profile?.background_splash_url) {
      urls.push(resolveCachedAssetUrl(playerProfile.profile.background_splash_url));
    }
    return urls;
  }, [prefs.wallpaperVisible, playerProfile.profile?.background_splash_url]);

  const boot = useAppBootstrap({
    enabled: startupEnabled,
    preferencesReady: prefs.preferencesReady,
    lcuInitialChecked: lcu.lcuInitialChecked,
    profileInitialReady: playerProfile.profileInitialReady,
    criticalImageUrls,
    reducedMotion: prefersReducedMotion || !prefs.hardwareAcceleration,
  });

  const profileUiBlurPx = profileBlurPercentToPx(prefs.profileBlur);
  const profileBlurOff = activeTab === "profile" && profileUiBlurPx <= 0;

  const handleInvertSpells = async () => {
    prefs.updatePreference("flashKey", preferences.flashKey === "D" ? "F" : "D");
  };

  const handleAramDockClick = () => {
    const wasAlreadyOnAram = activeTab === "aram";
    setActiveTab("aram");
    aram.handleAramDockClick(wasAlreadyOnAram);
  };

  const handleRankedDockClick = () => {
    const wasAlreadyOnRanked = activeTab === "ranked";
    setActiveTab("ranked");
    if (wasAlreadyOnRanked && ranked.selectedChampion) ranked.clearSelectedChampion();
  };

  const handleArenaDockClick = () => {
    const wasAlreadyOnArena = activeTab === "arena";
    setActiveTab("arena");
    if (wasAlreadyOnArena && arena.selectedChampion) arena.clearSelectedChampion();
  };

  return (
    <I18nProvider locale={locale} onLocaleChange={prefs.changeLanguage}>
      <main
        className={`container ${activeTab === "profile" ? "profile-active" : ""} ${
          profileBlurOff ? "profile-blur-off" : ""
        } ${isAppMinimized ? "app-minimized" : ""} ${
          !prefs.hardwareAcceleration ? "gpu-effects-disabled" : ""
        }`}
        style={activeTab === "profile" ? ({ "--profile-ui-blur": `${profileUiBlurPx}px` } as CSSProperties) : undefined}
      >
        <ProfileWallpaperBackdrop
          activeTab={activeTab}
          summoner={lcu.summoner}
          profile={playerProfile.profile}
          wallpaperVisible={prefs.wallpaperVisible}
        />

        <StatusDock
          summoner={lcu.summoner}
          updateState={updater.updateState}
          onRestartUpdate={updater.installAndRestart}
          onOpenPortableRelease={updater.openPortableRelease}
          onDismissUpdate={updater.dismissUpdate}
        />

        <AppTabsViewport
          activeTab={activeTab}
          isBooting={boot.isBooting}
          prefs={prefs}
          lcu={lcu}
          playerProfile={playerProfile}
          ranked={ranked}
          arena={arena}
          aram={aram}
          updater={updater}
          onInvertSpells={handleInvertSpells}
        />

        <NavigationDock
          activeTab={activeTab}
          onTabChange={setActiveTab}
          onRankedClick={handleRankedDockClick}
          onArenaClick={handleArenaDockClick}
          onAramClick={handleAramDockClick}
          animationsPaused={isAppMinimized || !prefs.hardwareAcceleration || boot.isBooting}
          aramIconSrc={resolveCachedAssetUrl(ARAM_DOCK_ICON_URL)}
        />

        {boot.isBooting ? (
          <StartupLoadingScreen
            progress={boot.progress}
            stageKey={boot.stageKey}
            isExiting={boot.isExiting}
          />
        ) : !prefs.preferencesReady ? null : prefs.needsLanguageOnboarding ? (
          <LanguageOnboardingModal
            selectedLocale={locale}
            onSelectLocale={prefs.previewOnboardingLocale}
            onConfirm={prefs.confirmLanguageOnboarding}
          />
        ) : prefs.needsFlashOnboarding ? (
          <FlashOnboardingModal
            selectedFlashKey={preferences.flashKey}
            onSelectFlashKey={prefs.previewOnboardingFlashKey}
            onConfirm={prefs.confirmFlashOnboarding}
          />
        ) : null}
      </main>
    </I18nProvider>
  );
}

export default App;
