import { useTranslation } from "../../i18n";
import { AramAugmentEntry, AramChampionRank, AramMode } from "../../types";
import { resolveCachedAssetUrl } from "../../utils/assets";
import { ListSkeleton, LoadingDots } from "../../components/ui/LoadingIndicators";
import { SearchInput } from "../../components/ui/SearchInput";
import {
  AugmentRarityFilter,
  getAugmentRarityInfo,
  getAugmentTierClass,
  getAugmentTierLabel,
  getChampionTierClass,
  getChampionTierLabel,
} from "./aramHelpers";

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
}

export function AramMayhemView({
  currentChampions,
  filteredMayhemChampions,
  isLoadingList,
  listError,
  mayhemChampSearch,
  onMayhemChampSearchChange,
  selectedChampionKey,
  selectedChampion,
  onSelectChampion,
  onRetryTierList,
  rarityFilter,
  onSelectRarityFilter,
  augmentSearch,
  onAugmentSearchChange,
  augmentsLoading,
  augmentsError,
  currentAugments,
  onRetryAugments,
}: AramMayhemViewProps) {
  const { t } = useTranslation();

  const rarityFilterOptions = [
    { id: "all" as const, label: t("aram.rarityAll"), dot: "" },
    { id: 1 as const, label: t("aram.raritySilver"), dot: "dot-silver" },
    { id: 4 as const, label: t("aram.rarityGold"), dot: "dot-gold" },
    { id: 8 as const, label: t("aram.rarityPrismatic"), dot: "dot-prismatic" },
  ];

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
                    <img src={resolveCachedAssetUrl(champ.icon_url)} alt={champ.name} className="aram-champ-avatar" loading="lazy" />
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
                    <span className="aram-hero-sub">{t("aram.augmentsSubtitle")}</span>
                  </div>
                  {selectedChampion && (
                    <span className={`aram-tier-pill aram-hero-rank-pill ${getChampionTierClass(selectedChampion.tier)}`}>
                      #{selectedChampion.rank} · {getChampionTierLabel(selectedChampion.tier)}
                    </span>
                  )}
                </div>

                <div className="aram-augments-controls">
                  <div className="segmented-control aram-rarity-filter" role="group" aria-label={t("aram.filterRarityAria")}>
                    {rarityFilterOptions.map((opt) => (
                      <button
                        key={String(opt.id)}
                        type="button"
                        className={`segment-btn ${rarityFilter === opt.id ? "active" : ""}`}
                        onClick={() => onSelectRarityFilter(opt.id)}
                      >
                        {opt.dot && <span className={`aram-rarity-dot ${opt.dot}`} />}
                        <span>{opt.label}</span>
                      </button>
                    ))}
                  </div>

                  <SearchInput
                    value={augmentSearch}
                    onChange={onAugmentSearchChange}
                    placeholder={t("aram.searchAugmentPlaceholder")}
                    ariaLabel={t("aram.searchAugmentAria")}
                    clearAriaLabel={t("aram.clearSearchAugmentAria")}
                    className="aram-aug-search"
                  />
                </div>
              </div>

              <div className="aram-scroll-body" data-testid="aram-augments-list">
                {augmentsLoading ? (
                  <div className="loading-skeleton-shell" data-testid="aram-augments-loading">
                    <div className="loading-banner">
                      <div className="spinner aram-spinner" />
                      <span>{t("aram.loadingAugments")}</span>
                      <LoadingDots size="sm" testId="aram-augments-loading-dots" />
                    </div>
                    <ListSkeleton rows={5} variant="augments" testId="aram-augments-skeleton" />
                  </div>
                ) : augmentsError ? (
                  <div className="aram-state-box">
                    <span className="aram-error-text">{augmentsError}</span>
                    <button type="button" className="aram-retry-btn" onClick={onRetryAugments}>
                      {t("common.retry")}
                    </button>
                  </div>
                ) : currentAugments.length === 0 ? (
                  <div className="aram-state-box">
                    <span>{t("aram.noAugmentsFound")}</span>
                  </div>
                ) : (
                  currentAugments.map((aug) => {
                    const rarityInfo = getAugmentRarityInfo(aug.rarity, t);
                    return (
                      <div key={aug.id} className="aram-augment-row">
                        <div className={`aram-augment-icon-frame ${rarityInfo.className}`}>
                          <img src={resolveCachedAssetUrl(aug.icon_url)} alt={aug.name} className="aram-augment-icon" loading="lazy" />
                        </div>

                        <span className={`aram-aug-tier-badge ${getAugmentTierClass(aug.tier)}`}>
                          {getAugmentTierLabel(aug.tier)}
                        </span>

                        <div className="aram-augment-info">
                          <div className="aram-augment-top">
                            <strong className="aram-augment-name">{aug.name}</strong>
                            <span className={`aram-rarity-tag ${rarityInfo.className}`}>{rarityInfo.label}</span>
                            {(aug.popular ?? 0) > 0 && (
                              <span className="aram-aug-pick">
                                {t("aram.augmentPickRate", { rate: (aug.popular ?? 0).toFixed(1) })}
                              </span>
                            )}
                          </div>
                          {aug.desc && <p className="aram-augment-desc">{aug.desc}</p>}
                        </div>
                      </div>
                    );
                  })
                )}
              </div>
            </>
          )}
        </section>
      </div>
    </div>
  );
}
