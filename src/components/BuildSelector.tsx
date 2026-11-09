import { UnifiedBuildOption } from "../types";

interface BuildSelectorProps {
  buildOptions?: UnifiedBuildOption[];
  selectedBuildIndex: number;
  onSelectBuild: (index: number) => void;
}

export function BuildSelector({
  buildOptions,
  selectedBuildIndex,
  onSelectBuild,
}: BuildSelectorProps) {
  if (!buildOptions || buildOptions.length <= 1) return null;

  return (
    <div className="unified-build-selector" data-testid="unified-build-selector">
      <span className="unified-build-label">Variação de Build:</span>
      <div className="unified-build-pills">
        {buildOptions.map((opt, optIdx) => (
          <button
            key={`build-opt-${optIdx}`}
            type="button"
            className={`unified-build-pill ${selectedBuildIndex === optIdx ? "active" : ""}`}
            data-testid={`build-opt-btn-${optIdx}`}
            onClick={() => onSelectBuild(optIdx)}
            title={`${opt.play.toLocaleString()} partidas (${opt.pick_rate}%)`}
          >
            <span className="build-pill-name">{opt.name}</span>
            <div className="build-pill-icons">
              {opt.core_item_icon_urls?.map((iconUrl, iconIdx) => (
                <img
                  key={`core-icon-${optIdx}-${iconIdx}`}
                  src={iconUrl}
                  alt={`Item ${opt.core_item_ids[iconIdx]}`}
                  className="build-pill-mini-icon"
                  onError={(e) => {
                    (e.target as HTMLElement).style.display = "none";
                  }}
                />
              ))}
            </div>
          </button>
        ))}
      </div>
    </div>
  );
}
