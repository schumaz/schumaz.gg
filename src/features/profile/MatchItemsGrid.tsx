import { useTranslation } from "../../i18n";
import { resolveCachedAssetUrl } from "../../utils/assets";
import {
  ItemMetadataMap,
  ItemTooltipContent,
  lookupItemMetadata,
  Tooltip,
} from "../../components/ui";

interface MatchItemsGridProps {
  itemIds: number[];
  itemIconUrls: string[];
  trinketId?: number;
  trinketIconUrl?: string;
  itemMetadata?: ItemMetadataMap;
}

/**
 * 2x3 final items grid (`item0..item5`) plus a centered dedicated trinket slot (`item6`)
 * with rich hover tooltips. Uses `focusable={false}` inside the expandable row `<button>`.
 */
export function MatchItemsGrid({
  itemIds,
  itemIconUrls,
  trinketId = 0,
  trinketIconUrl = "",
  itemMetadata,
}: MatchItemsGridProps) {
  const { t } = useTranslation();
  const hasTrinket = trinketId > 0 && Boolean(trinketIconUrl);
  const trinketMeta = hasTrinket ? lookupItemMetadata(itemMetadata, trinketId) : undefined;
  const trinketAlt = trinketMeta?.name || t("profile.trinketAria");

  return (
    <div className="match-items-wrap">
      <div className="match-items-grid" aria-label={t("profile.finalItemsAria")}>
        {Array.from({ length: 6 }).map((_, slotIdx) => {
          const iconUrl = itemIconUrls[slotIdx] || "";
          const itemId = itemIds[slotIdx] || 0;
          if (itemId <= 0 || !iconUrl) {
            return (
              <div key={slotIdx} className="match-item-slot">
                <span className="match-item-empty" />
              </div>
            );
          }

          const itemMeta = lookupItemMetadata(itemMetadata, itemId);
          const itemAlt = itemMeta?.name || t("common.itemAlt", { id: itemId });
          return (
            <Tooltip
              key={slotIdx}
              content={<ItemTooltipContent itemId={itemId} metadata={itemMeta} />}
              triggerClassName="ui-tooltip-trigger-fill"
              focusable={false}
            >
              <div className="match-item-slot">
                <img
                  src={resolveCachedAssetUrl(iconUrl)}
                  alt={itemAlt}
                  className="match-item-img"
                  loading="lazy"
                  decoding="async"
                />
              </div>
            </Tooltip>
          );
        })}
      </div>

      <div
        className="match-trinket-col"
        data-testid="match-trinket-slot"
        aria-label={t("profile.trinketAria")}
      >
        {hasTrinket ? (
          <Tooltip
            content={<ItemTooltipContent itemId={trinketId} metadata={trinketMeta} />}
            triggerClassName="ui-tooltip-trigger-fill"
            focusable={false}
          >
            <div className="match-trinket-slot">
              <img
                src={resolveCachedAssetUrl(trinketIconUrl)}
                alt={trinketAlt}
                className="match-item-img"
                loading="lazy"
                decoding="async"
              />
            </div>
          </Tooltip>
        ) : (
          <div className="match-trinket-slot">
            <span className="match-item-empty" />
          </div>
        )}
      </div>
    </div>
  );
}
