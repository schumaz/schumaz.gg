import { UnifiedBuildOption } from "../types";

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
  if (!buildOptions || buildOptions.length <= 1) return null;

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
              title={`${opt.name} · ${opt.play.toLocaleString()} partidas${
                typeof opt.pick_rate === "number" ? ` (${opt.pick_rate.toFixed(1)}%)` : ""
              }`}
            >
              <div className="build-pill-top">
                <span className="build-pill-name" title={opt.name}>
                  {opt.name}
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
                        alt={`Item ${opt.core_item_ids[iconIdx]}`}
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
