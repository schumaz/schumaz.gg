import { renderEasternRegionFlag } from "./regionFlagsEastern";
import { renderWesternRegionFlag } from "./regionFlagsWestern";

interface RegionFlagProps {
  region: string;
}

export function RegionFlag({ region }: RegionFlagProps) {
  const code = region.toLowerCase();

  return (
    renderWesternRegionFlag(code) ??
    renderEasternRegionFlag(code) ?? (
      <svg
        className="filter-globe-svg"
        viewBox="0 0 24 24"
        fill="none"
        aria-hidden="true"
        data-testid={`region-icon-${code}`}
      >
        <circle cx="12" cy="12" r="9" stroke="#9ca3af" strokeWidth="1.5" />
      </svg>
    )
  );
}
