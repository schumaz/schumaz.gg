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
  itemMetadata?: ItemMetadataMap;
}

/**
 * 6-slot final items grid for a recent match row with rich hover tooltips.
 * Uses `focusable={false}` because the grid renders inside the expandable row `<button>`.
 */
export function MatchItemsGrid({
  itemIds,
  itemIconUrls,
  itemMetadata,
}: MatchItemsGridProps) {
  const { t } = useTranslation();

  return (
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
              />
            </div>
          </Tooltip>
        );
      })}
    </div>
  );
}
