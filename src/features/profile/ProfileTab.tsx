import { CSSProperties, memo, useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "../../i18n";
import { PlayerProfile, SummonerInfo } from "../../types";
import { profileBlurPercentToPx } from "../settings/preferencesStorage";
import { IdleIsland } from "../champ-select/IdleState";
import {
  computeAvailableModes,
  computeHighlightedChampion,
  computeWinLossStats,
  filterValidMatchList,
  type ProfileChampionTarget,
  resolveAllMasteries,
} from "./profileHelpers";
import { ProfileHeroCard } from "./ProfileHeroCard";
import { ProfileLeftColumn } from "./ProfileLeftColumn";
import { ProfileMasteriesModal } from "./ProfileMasteriesModal";
import { ProfileMatchesCard } from "./ProfileMatchesCard";
import { useMatchHistoryPagination } from "./useMatchHistoryPagination";

interface ProfileTabProps {
  summoner: SummonerInfo | null;
  profile: PlayerProfile | null;
  loading: boolean;
  errorMsg: string | null;
  onRefresh: () => void;
  refreshCooldownUntilMs?: number;
  refreshCooldownSecs?: number;
  profileBlur?: number;
  wallpaperVisible?: boolean;
  isActive?: boolean;
  savedModeFilter?: string;
  onModeFilterChange?: (mode: string) => void;
  onOpenChampionBuild?: (target: ProfileChampionTarget) => void;
}

const PROFILE_IDLE_ICON = (
  <svg
    viewBox="0 0 24 24"
    width="42"
    height="42"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.75"
    strokeLinecap="round"
    strokeLinejoin="round"
  >
    <path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2" />
    <circle cx="12" cy="7" r="4" />
  </svg>
);

export const ProfileTab = memo(function ProfileTab({
  summoner,
  profile,
  loading,
  errorMsg,
  onRefresh,
  refreshCooldownUntilMs,
  refreshCooldownSecs,
  profileBlur = 10,
  wallpaperVisible = true,
  isActive = true,
  savedModeFilter,
  onModeFilterChange,
  onOpenChampionBuild,
}: ProfileTabProps) {
  const { t } = useTranslation();
  const [selectedMode, setSelectedMode] = useState<string>(savedModeFilter || "all");
  const [refreshCount, setRefreshCount] = useState(0);
  const [masteriesModalOpen, setMasteriesModalOpen] = useState(false);

  useEffect(() => {
    if (savedModeFilter) {
      setSelectedMode(savedModeFilter);
    }
  }, [savedModeFilter]);

  const handleSelectMode = useCallback(
    (mode: string) => {
      setSelectedMode(mode);
      onModeFilterChange?.(mode);
    },
    [onModeFilterChange]
  );

  const displaySummoner = summoner ?? profile?.summoner ?? null;
  const allMasteries = useMemo(() => resolveAllMasteries(profile), [profile]);

  useEffect(() => {
    setMasteriesModalOpen(false);
  }, [isActive, displaySummoner?.summonerId]);

  const handleOpenChampionBuild = useMemo(
    () =>
      onOpenChampionBuild
        ? (target: ProfileChampionTarget) => {
            setMasteriesModalOpen(false);
            onOpenChampionBuild(target);
          }
        : undefined,
    [onOpenChampionBuild]
  );
  const pagination = useMatchHistoryPagination(
    profile,
    displaySummoner?.summonerId ?? null,
    summoner !== null
  );
  const resetPagination = pagination.resetPagination;
  const handleRefresh = useCallback(() => {
    resetPagination();
    setRefreshCount((prev) => prev + 1);
    onRefresh();
  }, [resetPagination, onRefresh]);
  const handleOpenAllMasteries = useCallback(() => setMasteriesModalOpen(true), []);
  const handleCloseAllMasteries = useCallback(() => setMasteriesModalOpen(false), []);
  const handleMasteriesModalSelect = useCallback(
    (target: ProfileChampionTarget) => {
      if (handleOpenChampionBuild) {
        handleOpenChampionBuild(target);
      } else {
        setMasteriesModalOpen(false);
      }
    },
    [handleOpenChampionBuild]
  );
  const matchesResetKey = useMemo(
    () => ({
      summonerId: displaySummoner?.summonerId ?? null,
      profile,
      refreshCount,
    }),
    [displaySummoner?.summonerId, profile, refreshCount]
  );

  const validMatches = useMemo(
    () => filterValidMatchList(pagination.matches),
    [pagination.matches]
  );
  const availableModes = useMemo(() => computeAvailableModes(validMatches), [validMatches]);

  const activeMode =
    selectedMode === "all" || availableModes.some((m) => m.key === selectedMode)
      ? selectedMode
      : "all";

  const filteredMatches = useMemo(() => {
    if (activeMode === "all") return validMatches;
    return validMatches.filter((m) => m.game_mode_key === activeMode);
  }, [validMatches, activeMode]);

  const winLossStats = useMemo(() => computeWinLossStats(filteredMatches), [filteredMatches]);
  const highlightedChampion = useMemo(
    () => computeHighlightedChampion(filteredMatches),
    [filteredMatches]
  );

  const effectiveProfileUiBlurPx = wallpaperVisible ? profileBlurPercentToPx(profileBlur) : 0;
  const isBlurOff = effectiveProfileUiBlurPx <= 0;
  const viewStyle = useMemo(
    () => ({ "--profile-ui-blur": `${effectiveProfileUiBlurPx}px` } as CSSProperties),
    [effectiveProfileUiBlurPx]
  );

  return (
    <div
      className={`tab-content profile-view ${isActive ? "active-tab" : "inactive-tab"}${
        isBlurOff ? " profile-blur-off" : ""
      }`}
      data-tab="profile"
      data-testid="view-profile"
      aria-hidden={!isActive}
      style={viewStyle}
    >
      {!displaySummoner ? (
        <IdleIsland
          testId="profile-waiting-client"
          mode="waiting"
          badge={t("profile.badge")}
          title={t("profile.waitingTitle")}
          description={t("profile.waitingDesc")}
          errorMsg={errorMsg}
          icon={PROFILE_IDLE_ICON}
        />
      ) : loading && !profile ? (
        <IdleIsland
          testId="profile-loading"
          mode="loading"
          badge={t("profile.badge")}
          title={t("profile.loadingTitle")}
          description={t("profile.loadingDesc")}
          icon={PROFILE_IDLE_ICON}
        />
      ) : (
        <div className="profile-layout" data-testid="profile-loaded">
          <ProfileHeroCard
            summoner={summoner}
            displaySummoner={displaySummoner}
            profile={profile}
            loading={loading}
            refreshCooldownUntilMs={refreshCooldownUntilMs}
            refreshCooldownSecs={refreshCooldownSecs}
            onRefresh={handleRefresh}
          />

          <div className="profile-split-grid">
            <ProfileLeftColumn
              profile={profile}
              filteredMatches={filteredMatches}
              winLossStats={winLossStats}
              highlightedChampion={highlightedChampion}
              allMasteriesCount={allMasteries.length}
              onOpenAllMasteries={handleOpenAllMasteries}
              onOpenChampionBuild={handleOpenChampionBuild}
            />

            <ProfileMatchesCard
              validMatches={validMatches}
              filteredMatches={filteredMatches}
              availableModes={availableModes}
              activeMode={activeMode}
              onSelectMode={handleSelectMode}
              itemMetadata={pagination.itemMetadata}
              runeMetadata={pagination.runeMetadata}
              spellMetadata={pagination.spellMetadata}
              augmentMetadata={pagination.augmentMetadata}
              isLcuConnected={summoner !== null}
              hasMore={pagination.hasMore}
              canLoadMore={pagination.canLoadMore && !loading}
              isLoadingMore={pagination.isLoadingMore}
              loadMoreError={pagination.loadMoreError}
              onLoadMore={pagination.loadMore}
              onOpenChampionBuild={handleOpenChampionBuild}
              resetKey={matchesResetKey}
            />
          </div>

          {masteriesModalOpen && allMasteries.length > 0 && (
            <ProfileMasteriesModal
              masteries={allMasteries}
              onClose={handleCloseAllMasteries}
              onSelectChampion={handleMasteriesModalSelect}
            />
          )}
        </div>
      )}
    </div>
  );
});
