import { RunePageOption } from "../../types";
import { useTranslation } from "../../i18n";
import { resolveCachedAssetUrl } from "../../utils/assets";

interface RunePagesSelectorProps {
  runePages: RunePageOption[];
  selectedRunePageIndex: number;
  onSelectRunePage: (pageIndex: number) => void;
  readOnly?: boolean;
}

/**
 * Compact single-line pill selector for switching between top recommended rune pages.
 */
export function RunePagesSelector({
  runePages,
  selectedRunePageIndex,
  onSelectRunePage,
  readOnly = false,
}: RunePagesSelectorProps) {
  const { t, formatNumber, formatCompactNumber } = useTranslation();

  if (runePages.length <= 1) {
    return null;
  }

  return (
    <div className="rune-pages-selector" data-testid="rune-pages-selector">
      <div className="rune-pages-options">
        {runePages.map((page, idx) => {
          const isSelected = selectedRunePageIndex === idx;
          const keystone = page.perks && page.perks.length > 0 ? page.perks[0] : null;
          const subIcon = page.sub_style_icon_url;
          const label = readOnly
            ? idx === 0
              ? t("runes.pageMostUsed")
              : t("runes.pageNumber", { num: idx + 1 })
            : idx === 0
              ? t("runes.optionMostUsed")
              : t("runes.optionNumber", { num: idx + 1 });

          const gamesFull = page.play
            ? `${formatNumber(page.play)} ${t(
                page.play === 1 ? "common.gamesSingular" : "common.gamesPlural"
              )}`
            : `${page.pick_rate ?? 0}%`;

          const gamesCompact = page.play
            ? formatCompactNumber(page.play)
            : `${page.pick_rate ?? 0}%`;

          return (
            <button
              key={`rune-page-${idx}`}
              type="button"
              className={`rune-page-btn ${isSelected ? "active" : ""}`}
              data-testid={`rune-page-btn-${idx}`}
              onClick={() => onSelectRunePage(idx)}
              title={`${label} · ${gamesFull}`}
              aria-pressed={isSelected}
            >
              <div className="rune-page-btn-icons">
                {keystone?.icon_url && (
                  <img
                    src={resolveCachedAssetUrl(keystone.icon_url)}
                    alt={keystone.name || t("runes.keystoneFallback")}
                    className="rune-page-keystone-icon"
                  />
                )}
                {subIcon && (
                  <img
                    src={resolveCachedAssetUrl(subIcon)}
                    alt={page.sub_style_name || t("runes.secondaryFallback")}
                    className="rune-page-sub-icon"
                  />
                )}
              </div>
              <span className="rune-page-label">{label}</span>
              <span className="rune-page-games">{gamesCompact}</span>
            </button>
          );
        })}
      </div>
    </div>
  );
}
