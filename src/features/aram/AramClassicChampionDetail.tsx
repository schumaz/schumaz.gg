import { useTranslation } from "../../i18n";
import {
  AramChampionRank,
  ChampionBuild,
  FlashKeyPreference,
  SpellMetadata,
} from "../../types";
import { resolveCachedAssetUrl } from "../../utils/assets";
import { FiltersHeader } from "../../components/ui/FiltersHeader";
import { ItemBlocks } from "../../components/ui/ItemBlocks";
import { LoadingDots } from "../../components/ui/LoadingIndicators";
import { RuneTree } from "../../components/ui/RuneTree";
import { SpellsBar } from "../../components/ui/SpellsBar";

interface AramClassicChampionDetailProps {
  selectedChampion: AramChampionRank;
  championBuild: ChampionBuild | null;
  buildLoading: boolean;
  buildError: string | null;
  detailStatusTag: string;
  orderedSpells: SpellMetadata[];
  flashKey?: FlashKeyPreference;
  selectedRunePageIndex: number;
  selectedBuildIndex: number;
  onDetailFilterChange: (
    filterType: "position" | "tier" | "region" | "patch",
    value: string
  ) => void;
  onBackToTierList: () => void;
  onRetryBuild: () => void;
  onSelectRunePage: (pageIndex: number) => void;
  onSelectBuild: (index: number) => void;
}

/**
 * Read-only Classic ARAM champion build detail screen reusing FiltersHeader,
 * SkillOrderPopover/Card, RuneTree, SpellsBar, and ItemBlocks.
 */
export function AramClassicChampionDetail({
  selectedChampion,
  championBuild,
  buildLoading,
  buildError,
  detailStatusTag,
  orderedSpells,
  flashKey,
  selectedRunePageIndex,
  selectedBuildIndex,
  onDetailFilterChange,
  onBackToTierList,
  onRetryBuild,
  onSelectRunePage,
  onSelectBuild,
}: AramClassicChampionDetailProps) {
  const { t } = useTranslation();

  return (
    <div className="aram-detail-layout" data-testid="aram-classic-champion-detail">
      {championBuild ? (
        <div className="dashboard-layout">
          <FiltersHeader
            build={championBuild}
            champSelect={null}
            loading={buildLoading}
            onFilterChange={onDetailFilterChange}
            onBackToTierList={onBackToTierList}
            backButtonTitle={t("filters.backToAramTierListTitle")}
            statusTagText={detailStatusTag}
            queueModeLabel={t("filters.queueAram")}
            isAramMode={true}
            readOnly={true}
          />

          <div className="dashboard-split-grid">
            <div className="dashboard-col dashboard-col-left">
              <RuneTree
                build={championBuild}
                selectedRunePageIndex={selectedRunePageIndex}
                onSelectRunePage={onSelectRunePage}
                readOnly={true}
              />

              <SpellsBar
                orderedSpells={orderedSpells}
                spellOptions={championBuild.spell_options}
                flashKey={flashKey}
                readOnly={true}
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
        <div className="card aram-detail-state-card">
          <div className="aram-detail-fallback-top">
            <button
              type="button"
              className="ranked-back-btn"
              data-testid="aram-back-btn"
              onClick={onBackToTierList}
              title={t("filters.backToAramTierListTitle")}
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
            <div className="aram-detail-champ-summary">
              <div className="dd-icon-frame aram-champ-frame">
                <img
                  src={resolveCachedAssetUrl(selectedChampion.icon_url)}
                  alt={selectedChampion.name}
                  className="aram-champ-avatar"
                />
              </div>
              <span className="aram-detail-champ-name">{selectedChampion.name}</span>
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
            <div className="aram-state-box" data-testid="aram-classic-build-loading">
              <div className="loading-status-line">
                <div className="spinner aram-spinner" />
                <span>{t("aram.loadingBuild", { name: selectedChampion.name })}</span>
                <LoadingDots size="sm" testId="aram-classic-build-loading-dots" />
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
