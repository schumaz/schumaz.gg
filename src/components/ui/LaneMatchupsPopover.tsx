import { useEffect, useId, useRef, useState } from "react";
import { LaneMatchupsSummary } from "../../types";
import { useTranslation } from "../../i18n";
import { resolveCachedAssetUrl } from "../../utils/assets";
import { LaneMatchupsCard } from "./LaneMatchupsCard";

interface LaneMatchupsPopoverProps {
  matchups?: LaneMatchupsSummary | null;
  championName?: string;
  laneLabel?: string;
  onSelectChampion?: (championId: number) => void;
  resetKey?: string | number;
}

/**
 * Compact header trigger displaying up to 3 top lane counter avatars that opens the
 * Hard vs Favorable lane matchups panel on demand in an anchored Dark Amethyst popover.
 */
export function LaneMatchupsPopover({
  matchups,
  championName,
  laneLabel,
  onSelectChampion,
  resetKey,
}: LaneMatchupsPopoverProps) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const wrapRef = useRef<HTMLDivElement | null>(null);
  const triggerRef = useRef<HTMLButtonElement | null>(null);
  const popoverId = useId();

  useEffect(() => {
    setOpen(false);
  }, [resetKey]);

  useEffect(() => {
    if (!open) return;

    const handlePointerDown = (event: MouseEvent) => {
      if (!wrapRef.current?.contains(event.target as Node)) {
        setOpen(false);
      }
    };
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setOpen(false);
        triggerRef.current?.focus();
      }
    };

    document.addEventListener("mousedown", handlePointerDown);
    document.addEventListener("keydown", handleKeyDown);
    return () => {
      document.removeEventListener("mousedown", handlePointerDown);
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [open]);

  if (!matchups) {
    return null;
  }

  const hardList = matchups.hard ?? [];
  const favorableList = matchups.favorable ?? [];
  if (hardList.length === 0 && favorableList.length === 0) {
    return null;
  }

  const isHardPreview = hardList.length > 0;
  const previewItems = (isHardPreview ? hardList : favorableList).slice(0, 3);

  return (
    <div
      ref={wrapRef}
      className={`lane-matchups-popover-wrap${open ? " is-open" : ""}`}
      data-testid="lane-matchups-popover-wrap"
    >
      <button
        ref={triggerRef}
        type="button"
        className={`lane-matchups-trigger${open ? " active" : ""}`}
        data-testid="lane-matchups-trigger"
        aria-expanded={open}
        aria-haspopup="dialog"
        aria-controls={open ? popoverId : undefined}
        title={t("matchups.togglePopoverTitle")}
        onClick={() => setOpen((prev) => !prev)}
      >
        <span className="lane-matchups-trigger-label">{t("matchups.triggerLabel")}</span>

        {previewItems.length > 0 && (
          <span
            className="lane-matchups-preview-avatars"
            aria-label={t("matchups.hardPreviewAria")}
          >
            {previewItems.map((entry) => {
              const name = entry.name?.trim() || `#${entry.champion_id}`;
              return (
                <span
                  key={`preview-${entry.champion_id}`}
                  className={`dd-icon-frame lane-matchups-preview-frame ${
                    isHardPreview ? "is-hard" : "is-favorable"
                  }`}
                >
                  {entry.icon_url ? (
                    <img
                      src={resolveCachedAssetUrl(entry.icon_url)}
                      alt={name}
                      className="lane-matchups-preview-avatar"
                      onError={(e) => {
                        (e.currentTarget as HTMLElement).style.display = "none";
                      }}
                    />
                  ) : (
                    <span className="lane-matchups-preview-fallback" aria-hidden="true">
                      {name.slice(0, 2).toUpperCase()}
                    </span>
                  )}
                </span>
              );
            })}
          </span>
        )}

        <svg
          className="lane-matchups-trigger-chevron"
          viewBox="0 0 24 24"
          width="14"
          height="14"
          fill="none"
          stroke="currentColor"
          strokeWidth="2.2"
          strokeLinecap="round"
          strokeLinejoin="round"
          aria-hidden="true"
        >
          <polyline points="6 9 12 15 18 9" />
        </svg>
      </button>

      {open && (
        <div
          id={popoverId}
          role="dialog"
          aria-label={t("matchups.title")}
          className="lane-matchups-popover-panel"
          data-testid="lane-matchups-popover"
        >
          <LaneMatchupsCard
            matchups={matchups}
            championName={championName}
            laneLabel={laneLabel}
            onSelectChampion={
              onSelectChampion
                ? (championId) => {
                    setOpen(false);
                    onSelectChampion(championId);
                  }
                : undefined
            }
            onClose={() => setOpen(false)}
          />
        </div>
      )}
    </div>
  );
}
