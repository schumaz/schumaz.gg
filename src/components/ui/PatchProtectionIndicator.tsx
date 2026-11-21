import { useState } from "react";

interface PatchProtectionIndicatorProps {
  remainingHoursShort: string | null;
  tooltipText: string;
  idPrefix?: string;
}

/**
 * Displays the active 48h patch volume protection indicator with hover/focus tooltip.
 */
export function PatchProtectionIndicator({
  remainingHoursShort,
  tooltipText,
  idPrefix = "",
}: PatchProtectionIndicatorProps) {
  const [showTooltip, setShowTooltip] = useState(false);
  const tooltipId = `${idPrefix}patch-protection-tooltip`;

  return (
    <div
      className="patch-protection-wrap"
      onMouseEnter={() => setShowTooltip(true)}
      onMouseLeave={() => setShowTooltip(false)}
    >
      <button
        type="button"
        className="patch-protection-indicator"
        data-testid={`${idPrefix}patch-protection-indicator`}
        aria-label={tooltipText}
        aria-describedby={showTooltip ? tooltipId : undefined}
        onFocus={() => setShowTooltip(true)}
        onBlur={() => setShowTooltip(false)}
      >
        !
      </button>
      {remainingHoursShort && (
        <span className="patch-protection-hours" data-testid={`${idPrefix}patch-protection-hours`}>
          {remainingHoursShort}
        </span>
      )}
      {showTooltip && (
        <div
          id={tooltipId}
          role="tooltip"
          className="patch-protection-tooltip"
          data-testid={tooltipId}
        >
          {tooltipText}
        </div>
      )}
    </div>
  );
}
