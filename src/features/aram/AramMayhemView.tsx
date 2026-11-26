import { useTranslation } from "../../i18n";
import { AramAugmentEntry, AramChampionRank, AramMode, ChampionBuild, FlashKeyPreference, SpellMetadata } from "../../types";
import { resolveCachedAssetUrl } from "../../utils/assets";
import { ListSkeleton, LoadingDots } from "../../components/ui/LoadingIndicators";
import { SearchInput } from "../../components/ui/SearchInput";
import { SkillOrderPopover } from "../../components/ui/SkillOrderPopover";
import { AramMayhemPanel, AugmentRarityFilter, getChampionTierClass, getChampionTierLabel } from "./aramHelpers";
import { AramMayhemAugmentsPanel } from "./AramMayhemAugmentsPanel";
import { AramMayhemBuildPanel } from "./AramMayhemBuildPanel";

interface AramMayhemViewProps {
  currentChampions: AramChampionRank[];
  filteredMayhemChampions: AramChampionRank[];
  isLoadingList: boolean;
  listError: string | null;
  mayhemChampSearch: string;
  onMayhemChampSearchChange: (val: string) => void;
  selectedChampionKey: string | null;
  selectedChampion: AramChampionRank | null;
  onSelectChampion: (key: string) => void;
  onRetryTierList: (mode: AramMode) => void;
  rarityFilter: AugmentRarityFilter;
  onSelectRarityFilter: (rarity: AugmentRarityFilter) => void;
  augmentSearch: string;
  onAugmentSearchChange: (val: string) => void;
  augmentsLoading: boolean;
  augmentsError: string | null;
  currentAugments: AramAugmentEntry[];
  onRetryAugments: () => void;
  activePanel?: AramMayhemPanel;
  onSelectPanel?: (panel: AramMayhemPanel) => void;
  mayhemBuild?: ChampionBuild | null;
  mayhemBuildLoading?: boolean;
  mayhemBuildError?: string | null;
  mayhemOrderedSpells?: SpellMetadata[];
  flashKey?: FlashKeyPreference;
  mayhemBuildIndex?: number;
  onSelectMayhemBuild?: (index: number) => void;
  onRetryMayhemBuild?: () => void;
}

export function AramMayhemView({
  currentChampions, filteredMayhemChampions, isLoadingList, listError, mayhemChampSearch,
  onMayhemChampSearchChange, selectedChampionKey, selectedChampion, onSelectChampion, onRetryTierList,
  rarityFilter, onSelectRarityFilter, augmentSearch, onAugmentSearchChange, augmentsLoading,
  augmentsError, currentAugments, onRetryAugments, activePanel = "augments", onSelectPanel,
  mayhemBuild = null, mayhemBuildLoading = false, mayhemBuildError = null,
  mayhemOrderedSpells = [], flashKey, mayhemBuildIndex = 0, onSelectMayhemBuild, onRetryMayhemBuild,
}: AramMayhemViewProps) {
  const { t } = useTranslation();

  return (
    <div className="aram-layout" data-testid="aram-mayhem-view">
      <div className="aram-mayhem-grid">
        <section className="card aram-mayhem-col aram-mayhem-champions-col">
          <div className="aram-panel-header">
            <div className="aram-panel-title-row">
              <span className="aram-mode-badge is-mayhem">{t("aram.mayhemTitle")}</span>
              <h2 className="aram-mayhem-hero-title">{t("aram.augmentsColumnTitle")}</h2>
            </div>
            <SearchInput
              value={mayhemChampSearch}
              onChange={onMayhemChampSearchChange}
              placeholder={t("common.searchChampionPlaceholder")}
              ariaLabel={t("aram.searchMayhemAria")}
              clearAriaLabel={t("aram.clearSearchMayhemAria")}
              className="is-full"
            />
          </div>

          <div className="aram-scroll-body" data-testid="aram-mayhem-champions-list">
            {isLoadingList && currentChampions.length === 0 ? (
              <div className="loading-skeleton-shell" data-testid="aram-mayhem-champions-loading">
                <div className="loading-banner">
                  <div className="spinner aram-spinner" />
                  <span>{t("aram.loadingChampions")}</span>
                  <LoadingDots size="sm" testId="aram-mayhem-champions-loading-dots" />
                </div>
                <ListSkeleton rows={7} variant="compact" testId="aram-mayhem-champions-skeleton" />
              </div>
            ) : listError && currentChampions.length === 0 ? (
              <div className="aram-state-box">
                <span className="aram-error-text">{listError}</span>
                <button type="button" className="aram-retry-btn" onClick={() => onRetryTierList("aram_mayhem")}>
                  {t("common.retry")}
                </button>
              </div>
            ) : filteredMayhemChampions.length === 0 ? (
              <div className="aram-state-box">
                <span>{t("aram.noChampFound")}</span>
              </div>
            ) : (
              filteredMayhemChampions.map((champ) => (
                <button
                  key={champ.id}
                  type="button"
                  className={`aram-mayhem-champ-row ${champ.key === selectedChampionKey ? "selected" : ""}`}
                  onClick={() => onSelectChampion(champ.key)}
                  data-testid={`mayhem-champ-${champ.key}`}
                >
                  <span className="aram-rank-num">#{champ.rank}</span>
                  <div className="dd-icon-frame aram-champ-frame">
                    <img src={resolveCachedAssetUrl(champ.icon_url)} alt={champ.name} className="aram-champ-avatar" loading="lazy" decoding="async" />
                  </div>
                  <span className="aram-champ-name">{champ.name}</span>
                  <span className={`aram-tier-pill ${getChampionTierClass(champ.tier)}`}>
                    {getChampionTierLabel(champ.tier)}
                  </span>
                </button>
              ))
            )}
          </div>
        </section>

        <section className="card aram-mayhem-col aram-mayhem-augments-col">
          {!selectedChampionKey ? (
            <div className="aram-state-box" data-testid="aram-mayhem-empty-augments">
              <span>{t("aram.selectChampPrompt")}</span>
            </div>
          ) : (
            <>
              <div className="aram-panel-header">
                <div className="aram-augments-hero">
                  {selectedChampion && (
                    <div className="dd-icon-frame aram-hero-frame">
                      <img src={resolveCachedAssetUrl(selectedChampion.icon_url)} alt={selectedChampion.name} className="aram-hero-avatar" />
                    </div>
                  )}
                  <div className="aram-hero-meta">
                    <h3 className="aram-panel-title">{selectedChampion?.name ?? selectedChampionKey}</h3>
                    <span className="aram-hero-sub">
                      {activePanel === "build" ? t("aram.mayhemBuildSubtitle") : t("aram.augmentsSubtitle")}
                    </span>
                  </div>
                  {selectedChampion && (
                    <span className={`aram-tier-pill aram-hero-rank-pill ${getChampionTierClass(selectedChampion.tier)}`}>
                      #{selectedChampion.rank} · {getChampionTierLabel(selectedChampion.tier)}
                    </span>
                  )}
                </div>

                <div className="aram-mayhem-panel-switch-row">
                  <div className="segmented-control aram-mayhem-panel-switch" role="tablist" aria-label={t("aram.mayhemPanelTabsAria")}>
                    <button
                      type="button"
                      role="tab"
                      aria-selected={activePanel === "augments"}
                      className={`segment-btn ${activePanel === "augments" ? "active" : ""}`}
                      data-testid="mayhem-tab-augments"
                      onClick={() => onSelectPanel?.("augments")}
                    >
                      {t("aram.mayhemTabAugments")}
                    </button>
                    <button
                      type="button"
                      role="tab"
                      aria-selected={activePanel === "build"}
                      className={`segment-btn ${activePanel === "build" ? "active" : ""}`}
                      data-testid="mayhem-tab-build"
                      onClick={() => onSelectPanel?.("build")}
                    >
                      {t("aram.mayhemTabBuild")}
                    </button>
                  </div>

                  {activePanel === "build" && mayhemBuild?.skill_order && (
                    <SkillOrderPopover
                      skillOrder={mayhemBuild.skill_order}
                      readOnly={true}
                      resetKey={`mayhem-${selectedChampionKey}-${mayhemBuild.active_patch ?? ""}`}
                    />
                  )}
                </div>
              </div>

              {activePanel === "augments" ? (
                <AramMayhemAugmentsPanel
                  rarityFilter={rarityFilter}
                  onSelectRarityFilter={onSelectRarityFilter}
                  augmentSearch={augmentSearch}
                  onAugmentSearchChange={onAugmentSearchChange}
                  augmentsLoading={augmentsLoading}
                  augmentsError={augmentsError}
                  currentAugments={currentAugments}
                  onRetryAugments={onRetryAugments}
                />
              ) : (
                <AramMayhemBuildPanel
                  championName={selectedChampion?.name ?? selectedChampionKey}
                  build={mayhemBuild}
                  loading={mayhemBuildLoading}
                  error={mayhemBuildError}
                  orderedSpells={mayhemOrderedSpells}
                  flashKey={flashKey}
                  selectedBuildIndex={mayhemBuildIndex}
                  onSelectBuild={(idx) => onSelectMayhemBuild?.(idx)}
                  onRetryBuild={() => onRetryMayhemBuild?.()}
                />
              )}
            </>
          )}
        </section>
      </div>
    </div>
  );
}
