import { useTranslation } from "../../i18n";
import {
  ChampionBuild,
  FlashKeyPreference,
  RankedChampionEntry,
  SpellMetadata,
} from "../../types";
import { resolveCachedAssetUrl } from "../../utils/assets";
import { FiltersHeader } from "../../components/ui/FiltersHeader";
import { ItemBlocks } from "../../components/ui/ItemBlocks";
import { LoadingDots } from "../../components/ui/LoadingIndicators";
import { RuneTree } from "../../components/ui/RuneTree";
import { SpellsBar } from "../../components/ui/SpellsBar";

interface RankedChampionDetailProps {
  selectedChampion: RankedChampionEntry;
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
  onSelectMatchupChampion?: (championId: number) => void;
  motionClassName?: string;
}

export function RankedChampionDetail({
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
  onSelectMatchupChampion,
  motionClassName,
}: RankedChampionDetailProps) {
  const { t } = useTranslation();

  return (
    <div
      className={`tierlist-detail-layout${motionClassName ? ` ${motionClassName}` : ""}`}
      data-testid="ranked-champion-detail"
    >
      {championBuild ? (
        <div className="dashboard-layout">
          <FiltersHeader
            build={championBuild}
            champSelect={null}
            loading={buildLoading}
            onFilterChange={onDetailFilterChange}
            onBackToTierList={onBackToTierList}
            statusTagText={detailStatusTag}
            queueModeLabel={t("filters.queueRankedSr")}
            readOnly={true}
            onSelectMatchupChampion={onSelectMatchupChampion}
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
        <div className="card tierlist-detail-state-card">
          <div className="tierlist-detail-fallback-top">
            <button
              type="button"
              className="tierlist-back-btn"
              data-testid="ranked-back-btn"
              onClick={onBackToTierList}
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
            <div className="tierlist-detail-champ-summary">
              <div className="dd-icon-frame tierlist-champ-frame">
                <img
                  src={resolveCachedAssetUrl(selectedChampion.icon_url)}
                  alt={selectedChampion.name}
                  className="tierlist-champ-avatar"
                  decoding="async"
                />
              </div>
              <span className="tierlist-detail-champ-name">{selectedChampion.name}</span>
            </div>
          </div>

          {buildError ? (
            <div className="tierlist-state-box">
              <span className="tierlist-error-text">{buildError}</span>
              <button type="button" className="tierlist-retry-btn" onClick={onRetryBuild}>
                {t("common.retry")}
              </button>
            </div>
          ) : (
            <div className="tierlist-state-box" data-testid="ranked-build-loading">
              <div className="loading-status-line">
                <div className="spinner" />
                <span>{t("ranked.loadingBuild", { name: selectedChampion.name })}</span>
                <LoadingDots size="sm" testId="ranked-build-loading-dots" />
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
