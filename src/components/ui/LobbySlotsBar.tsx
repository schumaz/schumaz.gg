import { useTranslation } from "../../i18n";
import { LobbyPreselectSlot } from "../../types";
import { resolveCachedAssetUrl } from "../../utils/assets";

interface LobbySlotsBarProps {
  slots: LobbyPreselectSlot[];
  activeSlotIndex?: number | null;
  disabled?: boolean;
  onSelectSlot?: (slotIndex: number) => void;
}

const LANE_SHORT_KEYS = {
  top: "filters.lanes.topShort",
  jungle: "filters.lanes.jungleShort",
  mid: "filters.lanes.midShort",
  adc: "filters.lanes.adcShort",
  support: "filters.lanes.supportShort",
} as const;

export function LobbySlotsBar({
  slots,
  activeSlotIndex,
  disabled = false,
  onSelectSlot,
}: LobbySlotsBarProps) {
  const { t } = useTranslation();

  if (!slots || slots.length === 0) return null;

  const effectiveActiveIndex = activeSlotIndex ?? slots[0].slot_index;

  return (
    <div
      className="lobby-slots-bar"
      role="group"
      aria-label={t("filters.lobbySlotsAria")}
      data-testid="lobby-slots-bar"
    >
      {slots.map((slot) => {
        const isActive = slot.slot_index === effectiveActiveIndex;
        const slotRoleLabel =
          slot.slot_index === 0
            ? t("filters.lobbySlotPrimary")
            : slot.slot_index === 1
            ? t("filters.lobbySlotSecondary")
            : `#${slot.slot_index + 1}`;
        const posKey = (slot.assigned_position ?? "").trim().toLowerCase();
        const laneTranslationKey =
          posKey in LANE_SHORT_KEYS
            ? LANE_SHORT_KEYS[posKey as keyof typeof LANE_SHORT_KEYS]
            : null;
        const laneLabel = laneTranslationKey
          ? t(laneTranslationKey)
          : t("filters.lobbySlotFill");
        const champLabel = slot.champion_name?.trim() || `#${slot.champion_id}`;

        return (
          <button
            key={slot.slot_index}
            type="button"
            className={`lobby-slot-pill ${isActive ? "active" : ""}`}
            data-testid={`lobby-slot-btn-${slot.slot_index}`}
            aria-pressed={isActive}
            disabled={disabled || !onSelectSlot}
            onClick={() => onSelectSlot?.(slot.slot_index)}
          >
            {slot.champion_icon_url ? (
              <span className="dd-icon-frame lobby-slot-avatar-frame">
                <img
                  src={resolveCachedAssetUrl(slot.champion_icon_url)}
                  alt={champLabel}
                  className="lobby-slot-avatar"
                  decoding="async"
                  onError={(e) => {
                    (e.target as HTMLElement).style.display = "none";
                  }}
                />
              </span>
            ) : (
              <span className="lobby-slot-avatar-fallback" aria-hidden="true">
                {champLabel.slice(0, 2).toUpperCase()}
              </span>
            )}
            <span className="lobby-slot-role-tag">{slotRoleLabel}</span>
            <span className="lobby-slot-champ-name">{champLabel}</span>
            <span className="lobby-slot-lane">· {laneLabel}</span>
          </button>
        );
      })}
    </div>
  );
}
