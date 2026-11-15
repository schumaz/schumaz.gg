import { UnifiedBuildOption } from "../types";
import { useTranslation } from "../i18n";

interface BuildSelectorProps {
  buildOptions?: UnifiedBuildOption[];
  selectedBuildIndex: number;
  onSelectBuild: (index: number) => void;
}

function formatCompactGames(play: number): string {
  if (play >= 1000) {
    const thousands = play / 1000;
    return `${thousands >= 100 ? thousands.toFixed(0) : thousands.toFixed(1)}k`;
  }
  return `${play}`;
}

export function BuildSelector({
  buildOptions,
  selectedBuildIndex,
  onSelectBuild,
}: BuildSelectorProps) {
  const { t, formatNumber } = useTranslation();

  if (!buildOptions || buildOptions.length <= 1) return null;

  const resolveOptionName = (rawName: string, idx: number): string => {
    if (
      rawName === "Build 1 (Mais Feita)" ||
      rawName === "Build 1 (Most Popular)"
    ) {
      return t("items.build1MostPopular");
    }
    if (idx === 0 && !rawName) {
      return t("items.build1MostPopular");
    }
    return rawName || t("items.buildOptionNumber", { num: idx + 1 });
  };

  return (
    <div className="unified-build-selector" data-testid="unified-build-selector">
      <div
        className="unified-build-pills"
        style={{
          gridTemplateColumns: `repeat(${Math.min(buildOptions.length, 3)}, minmax(0, 1fr))`,
        }}
      >
        {buildOptions.map((opt, optIdx) => {
          const isSelected = selectedBuildIndex === optIdx;
          const displayName = resolveOptionName(opt.name, optIdx);
          const pickLabel =
            typeof opt.pick_rate === "number" && opt.pick_rate > 0
              ? `${opt.pick_rate.toFixed(1)}%`
              : opt.play > 0
              ? `${formatCompactGames(opt.play)}`
              : null;

          return (
            <button
              key={`build-opt-${optIdx}`}
              type="button"
              className={`unified-build-pill ${isSelected ? "active" : ""}`}
              data-testid={`build-opt-btn-${optIdx}`}
              onClick={() => onSelectBuild(optIdx)}
              title={`${displayName} · ${formatNumber(opt.play)} ${t(
                opt.play === 1 ? "common.gamesSingular" : "common.gamesPlural"
              )}${
                typeof opt.pick_rate === "number" ? ` (${opt.pick_rate.toFixed(1)}%)` : ""
              }`}
            >
              <div className="build-pill-top">
                <span className="build-pill-name" title={displayName}>
                  {displayName}
                </span>
              </div>

              <div className="build-pill-bottom">
                <div className="build-pill-icons">
                  {opt.core_item_icon_urls?.map((iconUrl, iconIdx) => (
                    <span
                      key={`core-icon-${optIdx}-${iconIdx}`}
                      className="dd-icon-frame build-pill-mini-frame"
                    >
                      <img
                        src={iconUrl}
                        alt={t("common.itemAlt", { id: opt.core_item_ids[iconIdx] })}
                        className="build-pill-mini-icon"
                        onError={(e) => {
                          (e.target as HTMLElement).style.display = "none";
                        }}
                      />
                    </span>
                  ))}
                </div>
                {pickLabel && <span className="build-pill-stat">{pickLabel}</span>}
              </div>
            </button>
          );
        })}
      </div>
    </div>
  );
}
