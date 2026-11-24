import { useTranslation } from "../../i18n";
import type { ArenaChampionEntry, ChampionBuild } from "../../types";
import { resolveCachedAssetUrl } from "../../utils/assets";
import { ArenaAugmentsCard } from "../../components/ui/ArenaAugmentsCard";
import { ArenaSynergiesCard } from "../../components/ui/ArenaSynergiesCard";
import { FiltersHeader } from "../../components/ui/FiltersHeader";
import { ItemBlocks } from "../../components/ui/ItemBlocks";
import { LoadingDots } from "../../components/ui/LoadingIndicators";

interface ArenaChampionDetailProps {
  selectedChampion: ArenaChampionEntry;
  championBuild: ChampionBuild | null;
  buildLoading: boolean;
  buildError: string | null;
  detailStatusTag: string;
  selectedBuildIndex: number;
  onDetailFilterChange: (
    filterType: "position" | "tier" | "region" | "patch",
    value: string
  ) => void;
  onBackToTierList: () => void;
  onRetryBuild: () => void;
  onSelectBuild: (index: number) => void;
  onSelectSynergyChampion: (championId: number) => void;
}

/**
 * Detailed Arena (`3x6`) champion consultation view displaying Augments, Best Synergies, and Arena Item Blocks.
 */
export function ArenaChampionDetail({
  selectedChampion,
  championBuild,
  buildLoading,
  buildError,
  detailStatusTag,
  selectedBuildIndex,
  onDetailFilterChange,
  onBackToTierList,
  onRetryBuild,
  onSelectBuild,
  onSelectSynergyChampion,
}: ArenaChampionDetailProps) {
  const { t } = useTranslation();

  return (
    <div className="arena-detail-layout" data-testid="arena-champion-detail">
      {championBuild ? (
        <div className="dashboard-layout">
          <FiltersHeader
            build={championBuild}
            champSelect={null}
            loading={buildLoading}
            onFilterChange={onDetailFilterChange}
            onBackToTierList={onBackToTierList}
            backButtonTitle={t("filters.backToArenaTierListTitle")}
            statusTagText={detailStatusTag}
            queueModeLabel={t("filters.queueArena")}
            readOnly={true}
          />

          <div className="dashboard-split-grid is-arena-grid">
            <div className="dashboard-col dashboard-col-left is-arena-col">
              <ArenaAugmentsCard
                augments={championBuild.arena_augments ?? []}
                showModeNotice={false}
              />

              <ArenaSynergiesCard
                synergies={championBuild.arena_synergies ?? []}
                onSelectChampion={onSelectSynergyChampion}
              />
            </div>

            <div className="dashboard-col dashboard-col-right">
              <ItemBlocks
                build={championBuild}
                selectedBuildIndex={selectedBuildIndex}
                onSelectBuild={onSelectBuild}
                readOnly={true}
              />
            </div>
          </div>
        </div>
      ) : (
        <div className="card ranked-detail-state-card">
          <div className="ranked-detail-fallback-top">
            <button
              type="button"
              className="ranked-back-btn"
              data-testid="arena-back-btn"
              onClick={onBackToTierList}
              title={t("filters.backToArenaTierListTitle")}
            >
              <svg
                viewBox="0 0 24 24"
                width="16"
                height="16"
                fill="none"
                stroke="currentColor"
                strokeWidth="2.2"
                strokeLinecap="round"
                strokeLinejoin="round"
                aria-hidden="true"
              >
                <polyline points="15 18 9 12 15 6" />
              </svg>
              <span>{t("common.back")}</span>
            </button>
            <div className="ranked-detail-champ-summary">
              <div className="dd-icon-frame aram-champ-frame">
                <img
                  src={resolveCachedAssetUrl(selectedChampion.icon_url)}
                  alt={selectedChampion.name}
                  className="aram-champ-avatar"
                />
              </div>
              <span className="ranked-detail-champ-name">{selectedChampion.name}</span>
            </div>
          </div>

          {buildError ? (
            <div className="aram-state-box">
              <span className="aram-error-text">{buildError}</span>
              <button type="button" className="aram-retry-btn" onClick={onRetryBuild}>
                {t("common.retry")}
              </button>
            </div>
          ) : (
            <div className="aram-state-box" data-testid="arena-build-loading">
              <div className="loading-status-line">
                <div className="spinner aram-spinner" />
                <span>{t("arenaTab.loadingBuild", { name: selectedChampion.name })}</span>
                <LoadingDots size="sm" testId="arena-build-loading-dots" />
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
