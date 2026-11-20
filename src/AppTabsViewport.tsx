import type { ActiveTab } from "./types";
import type { useAppUpdater } from "./hooks/useAppUpdater";
import { SettingsTab, type usePreferences } from "./features/settings";
import { ProfileTab, type usePlayerProfile } from "./features/profile";
import { ChampSelectTab, type useLcuEvents } from "./features/champ-select";
import { AramTab, type useAram } from "./features/aram";
import { ArenaTab, type useArena } from "./features/arena";
import { RankedTab, type useRanked } from "./features/ranked";

export interface AppTabsViewportProps {
  activeTab: ActiveTab;
  isBooting: boolean;
  prefs: ReturnType<typeof usePreferences>;
  lcu: ReturnType<typeof useLcuEvents>;
  playerProfile: ReturnType<typeof usePlayerProfile>;
  ranked: ReturnType<typeof useRanked>;
  arena: ReturnType<typeof useArena>;
  aram: ReturnType<typeof useAram>;
  updater: ReturnType<typeof useAppUpdater>;
  onInvertSpells: () => void;
}

export function AppTabsViewport({
  activeTab,
  isBooting,
  prefs,
  lcu,
  playerProfile,
  ranked,
  arena,
  aram,
  updater,
  onInvertSpells,
}: AppTabsViewportProps) {
  const { locale, preferences, navigationState, updateNavigationState } = prefs;

  return (
    <div className="tabs-viewport" aria-hidden={isBooting || undefined}>
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
        onInvertSpells={onInvertSpells}
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

      <ArenaTab
        isActive={activeTab === "arena"}
        filters={arena.filters}
        tierData={arena.tierData}
        tierLoading={arena.tierLoading}
        tierError={arena.tierError}
        selectedChampion={arena.selectedChampion}
        championBuild={arena.championBuild}
        buildLoading={arena.buildLoading}
        buildError={arena.buildError}
        selectedBuildIndex={arena.selectedBuildIndex}
        onPatchChange={arena.handlePatchChange}
        onDetailFilterChange={arena.handleDetailFilterChange}
        onSelectChampion={arena.selectChampion}
        onSelectSynergyChampion={arena.selectChampionById}
        onBackToTierList={arena.clearSelectedChampion}
        onRetryTierList={arena.retryTierList}
        onRetryBuild={arena.retryChampionBuild}
        onSelectBuild={arena.setSelectedBuildIndex}
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
        updateState={updater.updateState}
        distributionInfo={updater.distributionInfo}
        lastUpdateCheckResult={updater.lastCheckResult}
        onCheckForUpdates={() => {
          void updater.checkForUpdates({ manual: true });
        }}
        onRestartToUpdate={() => {
          void updater.installAndRestart();
        }}
        onOpenPortableRelease={() => {
          void updater.openPortableRelease();
        }}
      />
    </div>
  );
}
