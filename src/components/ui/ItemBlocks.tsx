import { Fragment } from "react";
import { ChampionBuild, ItemBlockKind } from "../../types";
import { useTranslation } from "../../i18n";
import { resolveCachedAssetUrl } from "../../utils/assets";
import { BuildSelector } from "./BuildSelector";
import { ItemTooltipContent, lookupItemMetadata } from "./BuildTooltips";
import { isArenaBuildContext } from "./queueContext";
import { Tooltip } from "./Tooltip";

interface ItemBlocksProps {
  build: ChampionBuild;
  selectedBuildIndex: number;
  onSelectBuild: (index: number) => void;
  readOnly?: boolean;
}

export function ItemBlocks({
  build,
  selectedBuildIndex,
  onSelectBuild,
  readOnly = false,
}: ItemBlocksProps) {
  const { t, formatNumber } = useTranslation();

  const isArena = isArenaBuildContext(build);
  const activeBuildOption =
    build.build_options && build.build_options.length > selectedBuildIndex
      ? build.build_options[selectedBuildIndex]
      : null;

  const activeBuildMetaParts: string[] = [];
  if (activeBuildOption) {
    if (typeof activeBuildOption.pick_rate === "number" && activeBuildOption.pick_rate > 0) {
      activeBuildMetaParts.push(
        `${activeBuildOption.pick_rate.toFixed(1)}% ${
          readOnly ? t("items.usageLabel") : t("items.pickLabel")
        }`
      );
    }
    if (activeBuildOption.play > 0) {
      activeBuildMetaParts.push(
        `${formatNumber(activeBuildOption.play)} ${t(
          activeBuildOption.play === 1 ? "common.gamesSingular" : "common.gamesPlural"
        )}`
      );
    }
  }
  const activeBuildMetaText = activeBuildMetaParts.join(" · ");

  const resolvedBlocks = build.item_blocks
    .map((block) => {
      const kind: ItemBlockKind = block.kind ?? "other";
      let activeIds = block.item_ids;
      let activeIcons = block.item_icon_urls;

      if (activeBuildOption) {
        if (kind === "core") {
          activeIds = activeBuildOption.core_item_ids;
          activeIcons = activeBuildOption.core_item_icon_urls;
        } else if (kind === "final") {
          activeIds = activeBuildOption.final_item_ids;
          activeIcons = activeBuildOption.final_item_icon_urls;
        } else if (
          kind === "situational" &&
          activeBuildOption.situational_item_ids &&
          activeBuildOption.situational_item_ids.length > 0
        ) {
          activeIds = activeBuildOption.situational_item_ids;
          activeIcons = activeBuildOption.situational_item_icon_urls || [];
        }
      }

      return { block, kind, activeIds, activeIcons };
    })
    .filter((entry) => !isArena || entry.activeIds.length > 0);

  const hasPrismaticBlock = resolvedBlocks.some((entry) => entry.kind === "prismatic");
  const hasBootsBlock = resolvedBlocks.some((entry) => entry.kind === "boots");

  return (
    <section className="card item-blocks-preview" data-testid="item-blocks">
      <div className="item-blocks-header">
        <div className="item-blocks-title-row">
          <h2>{readOnly ? t("items.titleReadOnly") : t("items.titleShop")}</h2>
          {activeBuildMetaText && (
            <span className="item-blocks-meta-pill" data-testid="active-build-meta">
              {activeBuildMetaText}
            </span>
          )}
        </div>
        <BuildSelector
          buildOptions={build.build_options}
          selectedBuildIndex={selectedBuildIndex}
          onSelectBuild={onSelectBuild}
          itemMetadata={build.item_metadata}
        />
      </div>

      <div className={`grid-blocks ${hasPrismaticBlock ? "grid-blocks-arena" : ""}`}>
        {resolvedBlocks.map(({ block, kind, activeIds, activeIcons }, bIndex) => {
          const isStarterBlock = kind === "starter";
          const isPrismaticBlock = kind === "prismatic";
          const isCoreBlock = kind === "core";
          const isBootsBlock = kind === "boots";
          const isFinalBlock = kind === "final";
          const isSituationalBlock = kind === "situational";

          const localizedTitle = isStarterBlock
            ? t("items.blockStarter")
            : isPrismaticBlock
            ? t("items.blockPrismatic")
            : isCoreBlock
            ? t("items.blockCore")
            : isBootsBlock
            ? isArena
              ? t("items.blockArenaBoots")
              : t("items.blockBoots")
            : isFinalBlock
            ? t("items.blockFinal")
            : isSituationalBlock
            ? t("items.blockSituational")
            : block.title;

          const blockKindClass = isStarterBlock
            ? "block-kind-starter"
            : isPrismaticBlock
            ? `block-kind-prismatic${hasBootsBlock ? "" : " block-span-full"}`
            : isCoreBlock
            ? "block-kind-core"
            : isBootsBlock
            ? "block-kind-boots"
            : isFinalBlock
            ? "block-kind-final"
            : isSituationalBlock
            ? "block-kind-situational"
            : "block-span-full";

          const blockTag = isPrismaticBlock
            ? t("items.prismaticTag")
            : isCoreBlock
            ? t("items.coreTrioTag")
            : isFinalBlock
            ? t("items.itemsCountTag", { count: activeIds.length })
            : isSituationalBlock
            ? t("items.optionalTag")
            : null;

          const badgeVariantClass = isPrismaticBlock
            ? "block-badge-prismatic"
            : isCoreBlock
            ? "block-badge-accent"
            : "";

          return (
            <div key={`${kind}-${bIndex}`} className={`block-item ${blockKindClass}`}>
              <div className="block-header-row">
                <h4 title={`${bIndex + 1}. ${localizedTitle}`}>
                  {bIndex + 1}. {localizedTitle}
                </h4>
                {blockTag && (
                  <span className={`block-badge ${badgeVariantClass}`}>
                    {blockTag}
                  </span>
                )}
              </div>
              <div className={`block-items-row ${isCoreBlock ? "block-items-flow" : ""}`}>
                {activeIds.map((itemId, iIndex) => {
                  const iconUrl = activeIcons?.[iIndex];
                  const itemMeta = lookupItemMetadata(build.item_metadata, itemId);
                  const itemAlt = itemMeta?.name || t("common.itemAlt", { id: itemId });
                  return (
                    <Fragment key={`${itemId}-${iIndex}`}>
                      {isCoreBlock && iIndex > 0 && (
                        <span className="item-flow-arrow" aria-hidden="true">
                          →
                        </span>
                      )}
                      <Tooltip
                        content={<ItemTooltipContent itemId={itemId} metadata={itemMeta} />}
                        ariaLabel={itemAlt}
                      >
                        <div className="item-icon-box">
                          {iconUrl ? (
                            <img
                              src={resolveCachedAssetUrl(iconUrl)}
                              alt={itemAlt}
                              className="shop-item-icon"
                              decoding="async"
                              onError={(e) => {
                                (e.target as HTMLElement).style.display = "none";
                              }}
                            />
                          ) : (
                            <span className="item-id-fallback">{itemId}</span>
                          )}
                        </div>
                      </Tooltip>
                    </Fragment>
                  );
                })}
              </div>
            </div>
          );
        })}
      </div>
    </section>
  );
}
