import { useMemo, useState } from "react";
import type { ArenaAugmentEntry } from "../../types";
import { useTranslation } from "../../i18n";
import { resolveCachedAssetUrl } from "../../utils/assets";

export type ArenaAugmentRarityFilter = "all" | 1 | 4 | 8;

export interface ArenaAugmentsCardProps {
  augments: ArenaAugmentEntry[];
  showModeNotice?: boolean;
  initialRarityFilter?: ArenaAugmentRarityFilter;
}

function getRarityVisualMeta(
  rarity: number,
  t: ReturnType<typeof useTranslation>["t"]
): { className: string; label: string } {
  if (rarity === 1) return { className: "rarity-silver", label: t("arena.raritySilver") };
  if (rarity === 4) return { className: "rarity-gold", label: t("arena.rarityGold") };
  if (rarity === 8) return { className: "rarity-prismatic", label: t("arena.rarityPrismatic") };
  return { className: "rarity-neutral", label: "" };
}

/**
 * Shared Arena (`CHERRY` / 3x6) augments card with rarity filtering and an optional compact notice
 * indicating that runes and summoner spells are managed by Arena mode.
 */
export function ArenaAugmentsCard({
  augments,
  showModeNotice = false,
  initialRarityFilter = "all",
}: ArenaAugmentsCardProps) {
  const { t } = useTranslation();
  const [rarityFilter, setRarityFilter] = useState<ArenaAugmentRarityFilter>(initialRarityFilter);

  const rarityOptions = useMemo(
    () => [
      { id: "all" as const, label: t("arena.rarityAll"), dot: "" },
      { id: 1 as const, label: t("arena.raritySilver"), dot: "dot-silver" },
      { id: 4 as const, label: t("arena.rarityGold"), dot: "dot-gold" },
      { id: 8 as const, label: t("arena.rarityPrismatic"), dot: "dot-prismatic" },
    ],
    [t]
  );

  const visibleAugments = useMemo(() => {
    if (rarityFilter === "all") return augments;
    return augments.filter((aug) => aug.rarity === rarityFilter);
  }, [augments, rarityFilter]);

  return (
    <section className="card arena-augments-card" data-testid="arena-augments-card">
      {showModeNotice && (
        <div className="arena-mode-notice" data-testid="arena-mode-notice">
          <span className="arena-mode-notice-badge">{t("arena.modeNoticeBadge")}</span>
          <span className="arena-mode-notice-text">{t("arena.modeNoticeText")}</span>
        </div>
      )}

      <div className="arena-panel-header">
        <div className="arena-panel-title-row">
          <h2>{t("arena.augmentsTitle")}</h2>
          {augments.length > 0 && (
            <span className="arena-count-pill">{visibleAugments.length}</span>
          )}
        </div>

        <div
          className="segmented-control arena-rarity-filter"
          role="group"
          aria-label={t("arena.rarityFilterAria")}
          data-testid="arena-rarity-filter"
        >
          {rarityOptions.map((opt) => (
            <button
              key={String(opt.id)}
              type="button"
              className={`segment-btn ${rarityFilter === opt.id ? "active" : ""}`}
              data-testid={`arena-rarity-btn-${opt.id}`}
              onClick={() => setRarityFilter(opt.id)}
            >
              {opt.dot && <span className={`aram-rarity-dot ${opt.dot}`} />}
              <span>{opt.label}</span>
            </button>
          ))}
        </div>
      </div>

      <div className="arena-augments-list" data-testid="arena-augments-list">
        {visibleAugments.length === 0 ? (
          <div className="arena-empty-box" data-testid="arena-augments-empty">
            <span>
              {augments.length === 0
                ? t("arena.augmentsEmpty")
                : t("arena.augmentsFilterEmpty")}
            </span>
          </div>
        ) : (
          visibleAugments.map((aug) => {
            const rarityMeta = getRarityVisualMeta(aug.rarity, t);
            const hasTop3 = typeof aug.win_rate === "number" && aug.win_rate > 0;
            const hasFirst = typeof aug.first_place_rate === "number" && aug.first_place_rate > 0;
            const hasAvg = typeof aug.avg_placement === "number" && aug.avg_placement > 0;

            return (
              <div
                key={aug.id}
                className="arena-augment-row"
                data-testid={`arena-augment-${aug.id}`}
              >
                <div className={`arena-augment-icon-frame ${rarityMeta.className}`}>
                  {aug.icon_url ? (
                    <img
                      src={resolveCachedAssetUrl(aug.icon_url)}
                      alt={aug.name}
                      className="arena-augment-icon"
                      loading="lazy"
                      decoding="async"
                      onError={(e) => {
                        (e.target as HTMLElement).style.display = "none";
                      }}
                    />
                  ) : (
                    <span className="arena-icon-fallback">
                      {aug.name.slice(0, 2).toUpperCase()}
                    </span>
                  )}
                </div>

                <div className="arena-augment-info">
                  <div className="arena-augment-top">
                    <strong className="arena-augment-name" title={aug.name}>
                      {aug.name}
                    </strong>
                    {rarityMeta.label && (
                      <span className={`aram-rarity-tag ${rarityMeta.className}`}>
                        {rarityMeta.label}
                      </span>
                    )}
                    <div className="arena-augment-stats">
                      {hasTop3 && (
                        <span
                          className={`arena-stat-chip ${(aug.win_rate ?? 0) >= 55 ? "is-high" : ""}`}
                        >
                          {(aug.win_rate ?? 0).toFixed(1)}% {t("arena.top3Short")}
                        </span>
                      )}
                      {hasFirst && (
                        <span className="arena-stat-chip is-accent">
                          {(aug.first_place_rate ?? 0).toFixed(1)}% {t("arena.firstPlaceShort")}
                        </span>
                      )}
                      {!hasFirst && hasAvg && (
                        <span className="arena-stat-chip">
                          #{(aug.avg_placement ?? 0).toFixed(1)} {t("arena.avgPlaceShort")}
                        </span>
                      )}
                    </div>
                  </div>
                  {aug.desc && (
                    <p className="arena-augment-desc" title={aug.desc}>
                      {aug.desc}
                    </p>
                  )}
                </div>
              </div>
            );
          })
        )}
      </div>
    </section>
  );
}
