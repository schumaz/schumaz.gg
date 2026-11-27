import { RecentMatchInfo } from "../../types";
import { useTranslation } from "../../i18n";
import { resolveCachedAssetUrl } from "../../utils/assets";
import {
  AugmentMetadataMap,
  AugmentTooltipContent,
  ItemMetadataMap,
  lookupAugmentMetadata,
  lookupRuneMetadata,
  lookupSpellMetadata,
  RuneMetadataMap,
  RuneTooltipContent,
  SpellMetadataMap,
  SpellTooltipContent,
  Tooltip,
} from "../../components/ui";
import { MatchItemsGrid } from "./MatchItemsGrid";
import {
  formatArenaPlacement,
  formatDuration,
  getArenaPlacementTone,
  getMultiKillBadge,
} from "./profileHelpers";

interface MatchRowSummaryProps {
  match: RecentMatchInfo;
  itemMetadata?: ItemMetadataMap;
  runeMetadata?: RuneMetadataMap;
  spellMetadata?: SpellMetadataMap;
  augmentMetadata?: AugmentMetadataMap;
}

function getAugmentRarityClass(rarity: number | undefined): string {
  if (rarity === 1) return "rarity-silver";
  if (rarity === 4) return "rarity-gold";
  if (rarity === 8) return "rarity-prismatic";
  return "";
}

/**
 * Compact match row body rendered inside `.match-row-btn`.
 * All tooltips pass `focusable={false}` to keep HTML inside `<button>` valid in React 19.
 */
export function MatchRowSummary({
  match,
  itemMetadata,
  runeMetadata,
  spellMetadata,
  augmentMetadata,
}: MatchRowSummaryProps) {
  const { t, locale, formatRelativeTime } = useTranslation();
  const isRemake = match.remake === true;
  const isArena = match.game_mode_key === "arena";
  const rawPlacement = match.arena_placement;
  const arenaPlacement =
    typeof rawPlacement === "number" && rawPlacement >= 1 && rawPlacement <= 8 ? rawPlacement : null;

  const multiKill = getMultiKillBadge(match);
  const kdaText =
    match.deaths === 0
      ? t("profile.perfectKda")
      : t("profile.kdaRatio", {
          ratio: ((match.kills + match.assists) / match.deaths).toFixed(2),
        });

  const spells = [
    { id: match.spell1_id, iconUrl: match.spell1_icon_url, fallbackAlt: t("profile.spell1Alt") },
    { id: match.spell2_id, iconUrl: match.spell2_icon_url, fallbackAlt: t("profile.spell2Alt") },
  ].filter((s) => Boolean(s.iconUrl));

  const augmentIds = (match.augment_ids ?? []).filter((id) => id > 0);
  const keystoneId =
    (match.primary_rune_id ?? 0) > 0 ? (match.primary_rune_id ?? 0) : (match.primary_style_id ?? 0);
  const keystoneIcon = match.primary_rune_icon_url || match.primary_style_icon_url || "";
  const subStyleIcon = match.sub_style_icon_url || "";
  const runeSlots =
    augmentIds.length === 0
      ? [
          { id: keystoneId, iconUrl: keystoneIcon, alt: t("profile.keystoneAlt"), slotClass: "is-keystone" },
          {
            id: match.sub_style_id ?? 0,
            iconUrl: subStyleIcon,
            alt: t("profile.secondaryTreeAlt"),
            slotClass: "is-secondary",
          },
        ].filter((slot) => Boolean(slot.iconUrl))
      : [];

  return (
    <>
      <div className="match-champ-col">
        {match.champion_icon_url ? (
          <div className="dd-icon-frame match-champ-frame">
            <img
              src={resolveCachedAssetUrl(match.champion_icon_url)}
              alt={match.champion_name}
              className="match-champ-icon"
              loading="lazy"
              decoding="async"
            />
          </div>
        ) : (
          <div className="match-champ-placeholder">{match.champion_name.charAt(0)}</div>
        )}

        {spells.length > 0 && (
          <div className="match-spells-col">
            {spells.map((spell, idx) => {
              const spellMeta = lookupSpellMetadata(spellMetadata, spell.id);
              const frame = (
                <div className="dd-icon-frame match-spell-frame">
                  <img
                    src={resolveCachedAssetUrl(spell.iconUrl)}
                    alt={spellMeta?.name || spell.fallbackAlt}
                    className="match-spell-icon"
                    loading="lazy"
                    decoding="async"
                  />
                </div>
              );
              return spellMeta ? (
                <Tooltip
                  key={`${spell.id}-${idx}`}
                  content={<SpellTooltipContent spell={spellMeta} />}
                  focusable={false}
                >
                  {frame}
                </Tooltip>
              ) : (
                <div key={`${spell.id}-${idx}`}>{frame}</div>
              );
            })}
          </div>
        )}

        {augmentIds.length > 0 && (
          <div
            className={`match-augments-grid${augmentIds.length > 4 ? " is-wide" : ""}`}
            data-testid={`match-augments-${match.game_id}`}
            aria-label={t("profile.augmentsAria")}
          >
            {augmentIds.slice(0, 6).map((augId, idx) => {
              const augMeta = lookupAugmentMetadata(augmentMetadata, augId);
              const augAlt = augMeta?.name || t("profile.augmentFallback", { id: augId });
              const iconSrc = augMeta?.icon_url ? resolveCachedAssetUrl(augMeta.icon_url) : "";
              return (
                <Tooltip
                  key={`${augId}-${idx}`}
                  content={<AugmentTooltipContent augmentId={augId} metadata={augMeta} />}
                  triggerClassName="ui-tooltip-trigger-fill"
                  focusable={false}
                >
                  <div className={`match-augment-frame ${getAugmentRarityClass(augMeta?.rarity)}`}>
                    {iconSrc ? (
                      <img src={iconSrc} alt={augAlt} className="match-augment-icon" loading="lazy" decoding="async" />
                    ) : (
                      <span className="match-augment-fallback">{augAlt.charAt(0).toUpperCase()}</span>
                    )}
                  </div>
                </Tooltip>
              );
            })}
          </div>
        )}

        {runeSlots.length > 0 && (
          <div
            className="match-runes-col"
            data-testid={`match-runes-${match.game_id}`}
            aria-label={t("profile.runesAria")}
          >
            {runeSlots.map((slot, idx) => {
              const runeMeta = lookupRuneMetadata(runeMetadata, slot.id);
              const frame = (
                <div className={`dd-icon-frame match-rune-frame ${slot.slotClass}`}>
                  <img
                    src={resolveCachedAssetUrl(slot.iconUrl)}
                    alt={runeMeta?.name || slot.alt}
                    className="match-rune-icon"
                    loading="lazy"
                    decoding="async"
                  />
                </div>
              );
              return runeMeta ? (
                <Tooltip
                  key={`${slot.slotClass}-${idx}`}
                  content={<RuneTooltipContent rune={runeMeta} />}
                  focusable={false}
                >
                  {frame}
                </Tooltip>
              ) : (
                <div key={`${slot.slotClass}-${idx}`}>{frame}</div>
              );
            })}
          </div>
        )}
      </div>

      <div className="match-main-info">
        <div className="match-top-line">
          {isRemake ? (
            <span className="match-result-tag remake" data-testid={`match-remake-badge-${match.game_id}`}>
              {t("profile.remake")}
            </span>
          ) : (
            <span className={`match-result-tag ${match.win ? "win" : "loss"}`}>
              {match.win ? t("profile.victory") : t("profile.defeat")}
            </span>
          )}
          {!isRemake && arenaPlacement !== null && (
            <span
              className={`match-placement-pill tone-${getArenaPlacementTone(arenaPlacement)}`}
              data-testid={`match-placement-${match.game_id}`}
            >
              {formatArenaPlacement(arenaPlacement, locale)}
            </span>
          )}
          <strong className="match-champ-name">{match.champion_name}</strong>
          <span className="match-mode-pill">{match.game_mode_label}</span>
        </div>
        <div className="match-sub-line">
          <span>{formatDuration(match.game_duration_secs)}</span>
          {match.game_creation_ms > 0 && <span>· {formatRelativeTime(match.game_creation_ms)}</span>}
          {!isRemake && multiKill && <span className="match-multikill-pill">{multiKill}</span>}
        </div>
      </div>

      <div className="match-kda-col">
        <span className="match-kda-score">
          {match.kills} / <span className="deaths">{match.deaths}</span> / {match.assists}
        </span>
        <span className="match-kda-ratio">{isArena ? kdaText : `${kdaText} · ${match.cs} CS`}</span>
      </div>

      <MatchItemsGrid
        itemIds={match.item_ids}
        itemIconUrls={match.item_icon_urls}
        trinketId={match.trinket_id ?? 0}
        trinketIconUrl={match.trinket_icon_url ?? ""}
        itemMetadata={itemMetadata}
      />
    </>
  );
}
