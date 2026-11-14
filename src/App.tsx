import { CSSProperties, useEffect, useState } from "react";
import "./App.css";

import { ActiveTab, FlashKeyPreference, commands } from "./types";
import { profileBlurPercentToPx, usePreferences } from "./hooks/usePreferences";
import { useLcuEvents } from "./hooks/useLcuEvents";
import { usePlayerProfile } from "./hooks/usePlayerProfile";
import { useAram } from "./hooks/useAram";
import { computeOrderedSpells } from "./utils/spells";
import {
  StatusDock,
  ProfileTab,
  FiltersHeader,
  RuneTree,
  SpellsBar,
  ItemBlocks,
  SettingsTab,
  IdleState,
  NavigationDock,
  AramTab,
} from "./components";

// Re-export all types and utilities for seamless backward compatibility
export * from "./types";
export * from "./utils/spells";

function App() {
  const [activeTab, setActiveTab] = useState<ActiveTab>("profile");
  const [splashFallbackUsed, setSplashFallbackUsed] = useState(false);
  const [splashBroken, setSplashBroken] = useState(false);

  const handleFlashKeyChange = (newFlashKey: FlashKeyPreference) => {
    const reordered = computeOrderedSpells(
      build?.spells,
      selectedSecondarySpellId,
      build?.spell_choices,
      newFlashKey,
      spellsInverted
    );
    if (reordered.length >= 2) {
      commands.injectSummonerSpells(reordered[0].id, reordered[1].id, null, null).catch(() => {});
    }
  };

  const handlePatchProtectionChange = () => {
    if (build) {
      handleFilterChange("patch", "");
    }
  };

  const {
    preferences,
    updatePreference,
    profileBlur,
    updateProfileBlur,
    wallpaperVisible,
    updateWallpaperVisible,
  } = usePreferences(handleFlashKeyChange, handlePatchProtectionChange);

  const {
    summoner,
    champSelect,
    build,
    spellsInverted,
    selectedSecondarySpellId,
    selectedRunePageIndex,
    selectedBuildIndex,
    loading,
    errorMsg,
    orderedSpells,
    handleFilterChange,
    handleSelectSecondarySpell,
    handleSelectRunePage,
    handleSelectBuild,
  } = useLcuEvents(preferences.flashKey, {
    onChampionPulled: () => setActiveTab("runes"),
    onMatchEnded: () => setActiveTab("profile"),
  });

  const currentChampionId = champSelect?.champion_id ?? build?.champion_id ?? null;
  const aram = useAram(activeTab === "aram", currentChampionId);

  const {
    profile,
    profileLoading,
    profileError,
    refreshCooldownSecs,
    refreshProfile,
  } = usePlayerProfile(summoner);

  useEffect(() => {
    setSplashFallbackUsed(false);
    setSplashBroken(false);
  }, [profile?.background_splash_url]);

  const activeSplashUrl = splashFallbackUsed
    ? profile?.background_splash_fallback_url || ""
    : profile?.background_splash_url || "";

  const showFullScreenWallpaper =
    activeTab === "profile" &&
    Boolean(summoner) &&
    wallpaperVisible &&
    Boolean(activeSplashUrl) &&
    !splashBroken;

  const profileUiBlurPx = profileBlurPercentToPx(profileBlur);

  const handleInvertSpells = async () => {
    const nextFlashKey: FlashKeyPreference = preferences.flashKey === "D" ? "F" : "D";
    updatePreference("flashKey", nextFlashKey);
  };

  const handleAramDockClick = () => {
    const wasAlreadyOnAram = activeTab === "aram";
    setActiveTab("aram");
    aram.handleAramDockClick(wasAlreadyOnAram);
  };

  return (
    <main
      className={`container ${activeTab === "profile" ? "profile-active" : ""}`}
      style={
        activeTab === "profile"
          ? ({ "--profile-ui-blur": `${profileUiBlurPx}px` } as CSSProperties)
          : undefined
      }
    >
      {/* Full-Screen Profile Wallpaper Backdrop (fixed 1px blur) */}
      {showFullScreenWallpaper && (
        <div className="profile-wallpaper-backdrop" data-testid="profile-wallpaper">
          <img
            src={activeSplashUrl}
            alt=""
            aria-hidden="true"
            className="profile-wallpaper-img"
            style={{ filter: "blur(1px) saturate(1.05)" }}
            onError={() => {
              if (
                !splashFallbackUsed &&
                profile?.background_splash_fallback_url &&
                profile.background_splash_fallback_url !== activeSplashUrl
              ) {
                setSplashFallbackUsed(true);
              } else {
                setSplashBroken(true);
              }
            }}
          />
          <div className="profile-wallpaper-scrim" />
        </div>
      )}

      <StatusDock summoner={summoner} />

      {/* Main Content Area with Seamless Crossfade Transition */}
      <div className="tabs-viewport">
        {/* View 1: "Perfil" Tab (Default Startup Screen) */}
        <ProfileTab
          summoner={summoner}
          profile={profile}
          loading={profileLoading}
          errorMsg={profileError}
          onRefresh={refreshProfile}
          refreshCooldownSecs={refreshCooldownSecs}
          profileBlur={profileBlur}
          isActive={activeTab === "profile"}
        />

        {/* View 2: "Runas" Tab (Match Dashboard) */}
        <div
          className={`tab-content ${activeTab === "runes" ? "active-tab" : "inactive-tab"}`}
          data-testid="view-runes"
          aria-hidden={activeTab !== "runes"}
        >
          {build ? (
            <div className="dashboard-layout">
              <FiltersHeader
                build={build}
                champSelect={champSelect}
                loading={loading}
                onFilterChange={handleFilterChange}
              />

              <div className="dashboard-split-grid">
                <div className="dashboard-col dashboard-col-left">
                  <RuneTree
                    build={build}
                    selectedRunePageIndex={selectedRunePageIndex}
                    onSelectRunePage={handleSelectRunePage}
                  />

                  <SpellsBar
                    orderedSpells={orderedSpells}
                    spellChoices={build.spell_choices}
                    selectedSecondarySpellId={selectedSecondarySpellId}
                    onInvertSpells={handleInvertSpells}
                    onSelectSecondarySpell={handleSelectSecondarySpell}
                  />
                </div>

                <div className="dashboard-col dashboard-col-right">
                  <ItemBlocks
                    build={build}
                    selectedBuildIndex={selectedBuildIndex}
                    onSelectBuild={handleSelectBuild}
                  />
                </div>
              </div>
            </div>
          ) : (
            <IdleState summoner={summoner} errorMsg={errorMsg} />
          )}
        </div>

        {/* View 3: "Aram" Tab (ARAM & ARAM Mayhem) */}
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
        />

        {/* View 4: "Configurações" Tab (Settings & Preferences) */}
        <SettingsTab
          preferences={preferences}
          onUpdatePreference={updatePreference}
          profileBlur={profileBlur}
          onProfileBlurChange={updateProfileBlur}
          wallpaperVisible={wallpaperVisible}
          onWallpaperVisibleChange={updateWallpaperVisible}
          isActive={activeTab === "settings"}
        />
      </div>

      {/* Bottom Navigation Dock */}
      <NavigationDock
        activeTab={activeTab}
        onTabChange={setActiveTab}
        onAramClick={handleAramDockClick}
      />
    </main>
  );
}

export default App;
