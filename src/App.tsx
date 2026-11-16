import { CSSProperties, useEffect, useMemo, useRef, useState } from "react";
import "./App.css";
import appIconUrl from "./assets/app-icon.png";
import { ActiveTab, FlashKeyPreference } from "./types";
import { injectorIpc } from "./services/ipc";
import { I18nProvider } from "./i18n";
import { useAppBootstrap } from "./hooks/useAppBootstrap";
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
  SettingsTab,
  profileBlurPercentToPx,
  usePreferences,
} from "./features/settings";
import { ProfileTab, ProfileWallpaperBackdrop, usePlayerProfile } from "./features/profile";
import { ChampSelectTab, useLcuEvents } from "./features/champ-select";
import { AramTab, useAram } from "./features/aram";
import { RankedTab, useRanked } from "./features/ranked";

export * from "./types";
export * from "./utils/spells";

export interface AppProps {
  forceStartupLoading?: boolean;
}

function App({ forceStartupLoading }: AppProps = {}) {
  const [activeTab, setActiveTab] = useState<ActiveTab>("profile");
  const isAppMinimized = useWindowMinimized();
  const rankedPatchChangeRef = useRef<(() => void) | null>(null);

  const handleFlashKeyChange = (newFlashKey: FlashKeyPreference) => {
    const reordered = computeOrderedSpells(
      lcu.build?.spells,
      lcu.selectedSecondarySpellId,
      lcu.build?.spell_choices,
      newFlashKey,
      lcu.spellsInverted
    );
    if (reordered.length >= 2) {
      injectorIpc.injectSummonerSpells(reordered[0].id, reordered[1].id, null, null).catch(() => {});
    }
  };

  const handlePatchProtectionChange = () => {
    if (lcu.build) lcu.handleFilterChange("patch", "");
    rankedPatchChangeRef.current?.();
  };

  const prefs = usePreferences(handleFlashKeyChange, handlePatchProtectionChange);
  const { locale, preferences, navigationState, updateNavigationState } = prefs;

  const lcu = useLcuEvents(preferences.flashKey, {
    onChampionPulled: () => setActiveTab("runes"),
    onMatchEnded: () => setActiveTab("profile"),
    locale,
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

  const aram = useAram(activeTab === "aram", currentChampionId, locale, aramPersistence);
  const ranked = useRanked(
    activeTab === "ranked",
    preferences.flashKey,
    preferences.patch48hProtection ?? true,
    locale,
    rankedPersistence
  );

  useEffect(() => {
    rankedPatchChangeRef.current = ranked.handlePatchProtectionChange;
  }, [ranked.handlePatchProtectionChange]);

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

        <StatusDock summoner={lcu.summoner} />

        <div className="tabs-viewport" aria-hidden={boot.isBooting || undefined}>
          <ProfileTab
            summoner={lcu.summoner}
            profile={playerProfile.profile}
            loading={playerProfile.profileLoading}
            errorMsg={playerProfile.profileError}
            onRefresh={playerProfile.refreshProfile}
            refreshCooldownSecs={playerProfile.refreshCooldownSecs}
            profileBlur={prefs.profileBlur}
            isActive={activeTab === "profile"}
            savedModeFilter={navigationState?.profileMatchModeFilter}
            onModeFilterChange={(mode) => updateNavigationState({ profileMatchModeFilter: mode })}
          />

          <ChampSelectTab
            isActive={activeTab === "runes"}
            build={lcu.build}
            champSelect={lcu.champSelect}
            summoner={lcu.summoner}
            loading={lcu.loading}
            errorMsg={lcu.errorMsg}
            orderedSpells={lcu.orderedSpells}
            selectedSecondarySpellId={lcu.selectedSecondarySpellId}
            selectedRunePageIndex={lcu.selectedRunePageIndex}
            selectedBuildIndex={lcu.selectedBuildIndex}
            onFilterChange={lcu.handleFilterChange}
            onSelectRunePage={lcu.handleSelectRunePage}
            onInvertSpells={handleInvertSpells}
            onSelectSecondarySpell={lcu.handleSelectSecondarySpell}
            onSelectBuild={lcu.handleSelectBuild}
          />

          <RankedTab
            isActive={activeTab === "ranked"}
            filters={ranked.filters}
            tierData={ranked.tierData}
            tierLoading={ranked.tierLoading}
            tierError={ranked.tierError}
            selectedChampion={ranked.selectedChampion}
            championBuild={ranked.championBuild}
            buildLoading={ranked.buildLoading}
            buildError={ranked.buildError}
            orderedSpells={ranked.orderedSpells}
            selectedRunePageIndex={ranked.selectedRunePageIndex}
            selectedBuildIndex={ranked.selectedBuildIndex}
            onTierListFilterChange={ranked.handleTierListFilterChange}
            onDetailFilterChange={ranked.handleDetailFilterChange}
            onSelectChampion={ranked.selectChampion}
            onBackToTierList={ranked.clearSelectedChampion}
            onRetryTierList={ranked.retryTierList}
            onRetryBuild={ranked.retryChampionBuild}
            onSelectRunePage={ranked.setSelectedRunePageIndex}
            onSelectBuild={ranked.setSelectedBuildIndex}
            savedChampionTierFilter={navigationState?.rankedChampionTierFilter}
            onChampionTierFilterChange={(tier) => updateNavigationState({ rankedChampionTierFilter: tier })}
          />

          <AramTab
            isActive={activeTab === "aram"}
            savedMode={aram.savedMode}
            showChooser={aram.showChooser}
            tierLists={aram.tierLists}
            tierLoading={aram.tierLoading}
            tierErrors={aram.tierErrors}
            selectedChampionKey={aram.selectedChampionKey}
            augmentsByChampion={aram.augmentsByChampion}
            augmentsLoading={aram.augmentsLoading}
            augmentsError={aram.augmentsError}
            onSelectMode={aram.selectMode}
            onSelectChampion={aram.selectChampion}
            onRetryTierList={aram.retryTierList}
            onRetryAugments={aram.retryAugments}
            savedRarityFilter={navigationState?.aramMayhemRarityFilter}
            onRarityFilterChange={(rarity) => updateNavigationState({ aramMayhemRarityFilter: String(rarity) })}
          />

          <SettingsTab
            preferences={preferences}
            onUpdatePreference={prefs.updatePreference}
            locale={locale}
            onChangeLanguage={prefs.changeLanguage}
            profileBlur={prefs.profileBlur}
            onProfileBlurChange={prefs.updateProfileBlur}
            wallpaperVisible={prefs.wallpaperVisible}
            onWallpaperVisibleChange={prefs.updateWallpaperVisible}
            hardwareAcceleration={prefs.hardwareAcceleration}
            onHardwareAccelerationChange={prefs.updateHardwareAcceleration}
            gpuRestartRequired={prefs.gpuRestartRequired}
            onRestartApp={prefs.restartApp}
            isActive={activeTab === "settings"}
          />
        </div>

        <NavigationDock
          activeTab={activeTab}
          onTabChange={setActiveTab}
          onAramClick={handleAramDockClick}
          onRankedClick={handleRankedDockClick}
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
