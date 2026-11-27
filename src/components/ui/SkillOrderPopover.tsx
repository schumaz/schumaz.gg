import { Fragment, useEffect, useId, useRef, useState } from "react";
import { ChampionAbilityMetadata, SkillOrderRecommendation } from "../../types";
import { useTranslation } from "../../i18n";
import { resolveCachedAssetUrl } from "../../utils/assets";
import { SkillOrderCard } from "./SkillOrderCard";

interface SkillOrderPopoverProps {
  skillOrder?: SkillOrderRecommendation | null;
  readOnly?: boolean;
  resetKey?: string | number;
}

/**
 * Compact header trigger displaying skill maxing priority (`Q > W > E`) that opens
 * the full 1–18 leveling grid on demand in an anchored Dark Amethyst popover.
 */
export function SkillOrderPopover({
  skillOrder,
  readOnly = false,
  resetKey,
}: SkillOrderPopoverProps) {
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

  if (!skillOrder) {
    return null;
  }

  const priority = (skillOrder.priority ?? [])
    .map((key) => key.trim().toUpperCase())
    .filter(Boolean);
  const order = (skillOrder.order ?? [])
    .map((key) => key.trim().toUpperCase())
    .filter(Boolean);

  if (priority.length === 0 && order.length === 0) {
    return null;
  }

  const abilitiesByKey = new Map<string, ChampionAbilityMetadata>();
  for (const ability of skillOrder.abilities ?? []) {
    const normalized = ability.key.trim().toUpperCase();
    if (normalized) {
      abilitiesByKey.set(normalized, ability);
    }
  }

  return (
    <div
      ref={wrapRef}
      className={`skill-order-popover-wrap${open ? " is-open" : ""}`}
      data-testid="skill-order-popover-wrap"
    >
      <button
        ref={triggerRef}
        type="button"
        className={`skill-order-trigger${open ? " active" : ""}`}
        data-testid="skill-order-trigger"
        aria-expanded={open}
        aria-haspopup="dialog"
        aria-controls={open ? popoverId : undefined}
        title={t("skills.togglePopoverTitle")}
        onClick={() => setOpen((prev) => !prev)}
      >
        <span className="skill-order-trigger-label">{t("skills.title")}</span>

        {priority.length > 0 && (
          <span
            className="skill-priority-chain"
            data-testid="skill-priority-chain"
            aria-label={t("skills.priorityAriaLabel")}
          >
            {priority.map((key, idx) => {
              const ability = abilitiesByKey.get(key);
              const label = ability?.name?.trim() || t("skills.abilityFallback", { key });
              return (
                <Fragment key={`hdr-prio-${key}-${idx}`}>
                  {idx > 0 && (
                    <span className="skill-priority-sep" aria-hidden="true">
                      &gt;
                    </span>
                  )}
                  <span
                    className={`skill-priority-pill skill-key-${key.toLowerCase()}${
                      ability?.icon_url ? "" : " is-text-only"
                    }`}
                  >
                    {ability?.icon_url && (
                      <span className="dd-icon-frame skill-priority-icon-frame">
                        <img
                          src={resolveCachedAssetUrl(ability.icon_url)}
                          alt={label}
                          className="skill-ability-icon"
                          decoding="async"
                          onError={(e) => {
                            (e.currentTarget as HTMLElement).style.display = "none";
                          }}
                        />
                      </span>
                    )}
                    <span className="skill-priority-key">{key}</span>
                  </span>
                </Fragment>
              );
            })}
          </span>
        )}

        <svg
          className="skill-order-trigger-chevron"
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
          aria-label={t("skills.title")}
          className="skill-order-popover-panel"
          data-testid="skill-order-popover"
        >
          <SkillOrderCard
            skillOrder={skillOrder}
            readOnly={readOnly}
            onClose={() => setOpen(false)}
          />
        </div>
      )}
    </div>
  );
}
