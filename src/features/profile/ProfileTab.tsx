import { CSSProperties, useEffect, useMemo, useState } from "react";
import { useTranslation } from "../../i18n";
import { PlayerProfile, SummonerInfo } from "../../types";
import { profileBlurPercentToPx } from "../settings/preferencesStorage";
import { IdleIsland } from "../champ-select/IdleState";
import {
  computeAvailableModes,
  computeHighlightedChampion,
  computeWinLossStats,
  filterValidMatchList,
} from "./profileHelpers";
import { ProfileHeroCard } from "./ProfileHeroCard";
import { ProfileLeftColumn } from "./ProfileLeftColumn";
import { ProfileMatchesCard } from "./ProfileMatchesCard";
import { useMatchHistoryPagination } from "./useMatchHistoryPagination";

interface ProfileTabProps {
  summoner: SummonerInfo | null;
  profile: PlayerProfile | null;
  loading: boolean;
  errorMsg: string | null;
  onRefresh: () => void;
  refreshCooldownSecs?: number;
  profileBlur?: number;
  isActive?: boolean;
  savedModeFilter?: string;
  onModeFilterChange?: (mode: string) => void;
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

export function ProfileTab({
  summoner,
  profile,
  loading,
  errorMsg,
  onRefresh,
  refreshCooldownSecs = 0,
  profileBlur = 10,
  isActive = true,
  savedModeFilter,
  onModeFilterChange,
}: ProfileTabProps) {
  const { t } = useTranslation();
  const [selectedMode, setSelectedMode] = useState<string>(savedModeFilter || "all");
  const [refreshCount, setRefreshCount] = useState(0);

  useEffect(() => {
    if (savedModeFilter) {
      setSelectedMode(savedModeFilter);
    }
  }, [savedModeFilter]);

  const handleSelectMode = (mode: string) => {
    setSelectedMode(mode);
    onModeFilterChange?.(mode);
  };

  const displaySummoner = summoner ?? profile?.summoner ?? null;
  const pagination = useMatchHistoryPagination(
    profile,
    displaySummoner?.summonerId ?? null,
    summoner !== null
  );
  const handleRefresh = () => {
    pagination.resetPagination();
    setRefreshCount((prev) => prev + 1);
    onRefresh();
  };
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

  const profileUiBlurPx = profileBlurPercentToPx(profileBlur);
  const glassBlurStyle: CSSProperties =
    profileUiBlurPx > 0
      ? {
          backdropFilter: `blur(${profileUiBlurPx}px)`,
          WebkitBackdropFilter: `blur(${profileUiBlurPx}px)`,
        }
      : {
          backdropFilter: "none",
          WebkitBackdropFilter: "none",
        };

  return (
    <div
      className={`tab-content ${isActive ? "active-tab" : "inactive-tab"}`}
      data-testid="view-profile"
      aria-hidden={!isActive}
      style={{ "--profile-ui-blur": `${profileUiBlurPx}px` } as CSSProperties}
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
            refreshCooldownSecs={refreshCooldownSecs}
            onRefresh={handleRefresh}
            glassBlurStyle={glassBlurStyle}
          />

          <div className="profile-split-grid">
            <ProfileLeftColumn
              profile={profile}
              filteredMatches={filteredMatches}
              winLossStats={winLossStats}
              highlightedChampion={highlightedChampion}
              glassBlurStyle={glassBlurStyle}
            />

            <ProfileMatchesCard
              validMatches={validMatches}
              filteredMatches={filteredMatches}
              availableModes={availableModes}
              activeMode={activeMode}
              onSelectMode={handleSelectMode}
              glassBlurStyle={glassBlurStyle}
              itemMetadata={pagination.itemMetadata}
              runeMetadata={pagination.runeMetadata}
              spellMetadata={pagination.spellMetadata}
              augmentMetadata={pagination.augmentMetadata}
              hasMore={pagination.hasMore}
              canLoadMore={pagination.canLoadMore && !loading}
              isLoadingMore={pagination.isLoadingMore}
              loadMoreError={pagination.loadMoreError}
              onLoadMore={pagination.loadMore}
              resetKey={matchesResetKey}
            />
          </div>
        </div>
      )}
    </div>
  );
}
