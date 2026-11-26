import { ScoreboardParticipant } from "../../types";
import { useTranslation } from "../../i18n";
import { resolveCachedAssetUrl } from "../../utils/assets";
import {
  AugmentMetadataMap,
  AugmentTooltipContent,
  lookupAugmentMetadata,
  lookupRuneMetadata,
  lookupSpellMetadata,
  RuneMetadataMap,
  RuneTooltipContent,
  SpellMetadataMap,
  SpellTooltipContent,
  Tooltip,
} from "../../components/ui";

function getAugmentRarityClass(rarity: number | undefined): string {
  if (rarity === 1) return "rarity-silver";
  if (rarity === 4) return "rarity-gold";
  if (rarity === 8) return "rarity-prismatic";
  return "";
}

export function ScoreboardAugmentsStrip({
  augmentIds,
  augmentMetadata,
}: {
  augmentIds: number[];
  augmentMetadata?: AugmentMetadataMap;
}) {
  const { t } = useTranslation();
  const validIds = augmentIds.filter((id) => id > 0).slice(0, 6);
  if (validIds.length === 0) return null;

  return (
    <div
      className={`sb-augments-grid${validIds.length > 4 ? " is-wide" : ""}`}
      aria-label={t("profile.augmentsAria")}
    >
      {validIds.map((augId, idx) => {
        const meta = lookupAugmentMetadata(augmentMetadata, augId);
        const alt = meta?.name || t("profile.augmentFallback", { id: augId });
        const iconSrc = meta?.icon_url ? resolveCachedAssetUrl(meta.icon_url) : "";
        return (
          <Tooltip
            key={`${augId}-${idx}`}
            content={<AugmentTooltipContent augmentId={augId} metadata={meta} />}
            triggerClassName="ui-tooltip-trigger-fill"
          >
            <div className={`sb-augment-frame ${getAugmentRarityClass(meta?.rarity)}`}>
              {iconSrc ? (
                <img src={iconSrc} alt={alt} className="sb-augment-icon" />
              ) : (
                <span className="sb-augment-fallback">{alt.charAt(0).toUpperCase()}</span>
              )}
            </div>
          </Tooltip>
        );
      })}
    </div>
  );
}

export function ScoreboardLoadoutCell({
  participant,
  runeMetadata,
  spellMetadata,
  augmentMetadata,
}: {
  participant: ScoreboardParticipant;
  runeMetadata?: RuneMetadataMap;
  spellMetadata?: SpellMetadataMap;
  augmentMetadata?: AugmentMetadataMap;
}) {
  const { t } = useTranslation();
  const spells = [
    { id: participant.spell1_id, url: participant.spell1_icon_url, alt: t("profile.spell1Alt") },
    { id: participant.spell2_id, url: participant.spell2_icon_url, alt: t("profile.spell2Alt") },
  ].filter((s) => s.id > 0 && Boolean(s.url));

  const augmentIds = (participant.augment_ids ?? []).filter((id) => id > 0);
  const primaryRuneId = participant.primary_rune_id ?? 0;
  const primaryStyleId = participant.primary_style_id ?? 0;
  const subStyleId = participant.sub_style_id ?? 0;
  const keystoneId = primaryRuneId > 0 ? primaryRuneId : primaryStyleId;
  const keystoneIcon =
    participant.primary_rune_icon_url || participant.primary_style_icon_url || "";
  const runeSlots =
    augmentIds.length === 0
      ? [
          { id: keystoneId, url: keystoneIcon, alt: t("profile.keystoneAlt"), cls: "is-keystone" },
          {
            id: subStyleId,
            url: participant.sub_style_icon_url || "",
            alt: t("profile.secondaryTreeAlt"),
            cls: "is-secondary",
          },
        ].filter((r) => r.id > 0 && Boolean(r.url))
      : [];

  return (
    <div className="sb-loadout-cell">
      {spells.length > 0 && (
        <div className="sb-spells-col">
          {spells.map((spell, idx) => {
            const meta = lookupSpellMetadata(spellMetadata, spell.id);
            const img = (
              <div className="dd-icon-frame sb-spell-frame">
                <img
                  src={resolveCachedAssetUrl(spell.url)}
                  alt={meta?.name || spell.alt}
                  className="sb-spell-icon"
                />
              </div>
            );
            return meta ? (
              <Tooltip key={`${spell.id}-${idx}`} content={<SpellTooltipContent spell={meta} />}>
                {img}
              </Tooltip>
            ) : (
              <div key={`${spell.id}-${idx}`}>{img}</div>
            );
          })}
        </div>
      )}

      {augmentIds.length > 0 ? (
        <ScoreboardAugmentsStrip augmentIds={augmentIds} augmentMetadata={augmentMetadata} />
      ) : (
        runeSlots.length > 0 && (
          <div className="sb-runes-col">
            {runeSlots.map((slot, idx) => {
              const meta = lookupRuneMetadata(runeMetadata, slot.id);
              const img = (
                <div className={`dd-icon-frame sb-rune-frame ${slot.cls}`}>
                  <img
                    src={resolveCachedAssetUrl(slot.url)}
                    alt={meta?.name || slot.alt}
                    className="sb-rune-icon"
                  />
                </div>
              );
              return meta ? (
                <Tooltip key={`${slot.cls}-${idx}`} content={<RuneTooltipContent rune={meta} />}>
                  {img}
                </Tooltip>
              ) : (
                <div key={`${slot.cls}-${idx}`}>{img}</div>
              );
            })}
          </div>
        )
      )}
    </div>
  );
}
