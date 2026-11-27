import { ScoreboardParticipant } from "../../types";
import { useTranslation } from "../../i18n";
import { resolveCachedAssetUrl } from "../../utils/assets";
import {
  ItemMetadataMap,
  ItemTooltipContent,
  lookupItemMetadata,
  Tooltip,
} from "../../components/ui";
import type { ProfileChampionTarget } from "./profileHelpers";

export function ScoreboardChampCell({
  participant,
  onOpenChampionBuild,
}: {
  participant: ScoreboardParticipant;
  onOpenChampionBuild?: (target: ProfileChampionTarget) => void;
}) {
  const { t } = useTranslation();
  const content = (
    <>
      {participant.champion_icon_url ? (
        <div className="dd-icon-frame sb-champ-frame">
          <img
            src={resolveCachedAssetUrl(participant.champion_icon_url)}
            alt={participant.champion_name}
            className="sb-champ-icon"
            loading="lazy"
            decoding="async"
          />
        </div>
      ) : (
        <div className="sb-champ-placeholder">{participant.champion_name.charAt(0)}</div>
      )}
      <span className="sb-level-badge">{participant.champ_level}</span>
    </>
  );

  if (onOpenChampionBuild && participant.champion_id > 0) {
    return (
      <button
        type="button"
        className="sb-champ-cell sb-champ-btn"
        data-testid={`scoreboard-champ-btn-${participant.participant_id}`}
        aria-label={t("profile.openRankedBuildAria", { name: participant.champion_name })}
        title={t("profile.openRankedBuildAria", { name: participant.champion_name })}
        onClick={() =>
          onOpenChampionBuild({
            championId: participant.champion_id,
            championName: participant.champion_name,
            championIconUrl: participant.champion_icon_url,
          })
        }
      >
        {content}
      </button>
    );
  }

  return <div className="sb-champ-cell">{content}</div>;
}

export function ScoreboardPlayerName({
  participant,
  onOpenChampionBuild,
}: {
  participant: ScoreboardParticipant;
  onOpenChampionBuild?: (target: ProfileChampionTarget) => void;
}) {
  const { t } = useTranslation();
  const fullName = participant.riot_id_tag
    ? `${participant.riot_id_name}#${participant.riot_id_tag}`
    : participant.riot_id_name;

  return (
    <div className="sb-player-name-wrap">
      <span className="sb-riot-id" title={fullName}>
        {participant.riot_id_name}
        {participant.riot_id_tag && <span className="sb-riot-tag">#{participant.riot_id_tag}</span>}
      </span>
      <div className="sb-player-sub">
        {onOpenChampionBuild && participant.champion_id > 0 ? (
          <button
            type="button"
            className="sb-champ-name-btn"
            aria-label={t("profile.openRankedBuildAria", { name: participant.champion_name })}
            title={t("profile.openRankedBuildAria", { name: participant.champion_name })}
            onClick={() =>
              onOpenChampionBuild({
                championId: participant.champion_id,
                championName: participant.champion_name,
                championIconUrl: participant.champion_icon_url,
              })
            }
          >
            {participant.champion_name}
          </button>
        ) : (
          <span>{participant.champion_name}</span>
        )}
        {participant.is_local_player && (
          <span className="sb-local-badge">{t("profile.localPlayerBadge")}</span>
        )}
      </div>
    </div>
  );
}

export function ScoreboardDamageBar({
  participantId,
  damage,
  maxDamage,
}: {
  participantId: number;
  damage: number;
  maxDamage: number;
}) {
  const { formatCompactNumber } = useTranslation();
  const pct =
    maxDamage > 0 ? Math.min(100, Math.max(damage > 0 ? 3 : 0, Math.round((damage / maxDamage) * 100))) : 0;

  return (
    <div className="sb-dmg-cell">
      <span className="sb-dmg-val">{formatCompactNumber(damage)}</span>
      <div className="sb-dmg-track" aria-hidden="true">
        <div
          className="sb-dmg-fill"
          data-testid={`scoreboard-dmg-bar-${participantId}`}
          style={{ width: `${pct}%` }}
        />
      </div>
    </div>
  );
}

export function ScoreboardItemsStrip({
  itemIds,
  itemIconUrls,
  trinketId,
  trinketIconUrl,
  itemMetadata,
  showTrinket = true,
}: {
  itemIds: number[];
  itemIconUrls: string[];
  trinketId: number;
  trinketIconUrl: string;
  itemMetadata?: ItemMetadataMap;
  showTrinket?: boolean;
}) {
  const { t } = useTranslation();
  const hasTrinket = trinketId > 0 && Boolean(trinketIconUrl);
  const trinketMeta = hasTrinket ? lookupItemMetadata(itemMetadata, trinketId) : undefined;

  return (
    <div className="sb-items-strip">
      {Array.from({ length: 6 }).map((_, slotIdx) => {
        const itemId = itemIds[slotIdx] || 0;
        const iconUrl = itemIconUrls[slotIdx] || "";
        if (itemId <= 0 || !iconUrl) {
          return (
            <div key={slotIdx} className="sb-item-slot">
              <span className="match-item-empty" />
            </div>
          );
        }
        const meta = lookupItemMetadata(itemMetadata, itemId);
        return (
          <Tooltip
            key={slotIdx}
            content={<ItemTooltipContent itemId={itemId} metadata={meta} />}
            triggerClassName="ui-tooltip-trigger-fill"
          >
            <div className="sb-item-slot">
              <img
                src={resolveCachedAssetUrl(iconUrl)}
                alt={meta?.name || t("common.itemAlt", { id: itemId })}
                className="match-item-img"
                loading="lazy"
                decoding="async"
              />
            </div>
          </Tooltip>
        );
      })}

      {showTrinket &&
        (hasTrinket ? (
          <Tooltip
            content={<ItemTooltipContent itemId={trinketId} metadata={trinketMeta} />}
            triggerClassName="ui-tooltip-trigger-fill"
          >
            <div className="sb-trinket-slot">
              <img
                src={resolveCachedAssetUrl(trinketIconUrl)}
                alt={trinketMeta?.name || t("profile.trinketAria")}
                className="match-item-img"
                loading="lazy"
                decoding="async"
              />
            </div>
          </Tooltip>
        ) : (
          <div className="sb-trinket-slot">
            <span className="match-item-empty" />
          </div>
        ))}
    </div>
  );
}
