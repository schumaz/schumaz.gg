import { Fragment } from "react";
import { ChampionBuild, ItemBlockKind } from "../../types";
import { useTranslation } from "../../i18n";
import { resolveCachedAssetUrl } from "../../utils/assets";
import { BuildSelector } from "./BuildSelector";

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
        />
      </div>

      <div className="grid-blocks">
        {build.item_blocks.map((block, bIndex) => {
          const kind: ItemBlockKind = block.kind ?? "other";
          const isStarterBlock = kind === "starter";
          const isCoreBlock = kind === "core";
          const isBootsBlock = kind === "boots";
          const isFinalBlock = kind === "final";
          const isSituationalBlock = kind === "situational";

          const localizedTitle = isStarterBlock
            ? t("items.blockStarter")
            : isCoreBlock
            ? t("items.blockCore")
            : isBootsBlock
            ? t("items.blockBoots")
            : isFinalBlock
            ? t("items.blockFinal")
            : isSituationalBlock
            ? t("items.blockSituational")
            : block.title;

          let activeIds = block.item_ids;
          let activeIcons = block.item_icon_urls;

          if (activeBuildOption) {
            if (isCoreBlock) {
              activeIds = activeBuildOption.core_item_ids;
              activeIcons = activeBuildOption.core_item_icon_urls;
            } else if (isFinalBlock) {
              activeIds = activeBuildOption.final_item_ids;
              activeIcons = activeBuildOption.final_item_icon_urls;
            } else if (
              isSituationalBlock &&
              activeBuildOption.situational_item_ids &&
              activeBuildOption.situational_item_ids.length > 0
            ) {
              activeIds = activeBuildOption.situational_item_ids;
              activeIcons = activeBuildOption.situational_item_icon_urls || [];
            }
          }

          const blockKindClass = isStarterBlock
            ? "block-kind-starter"
            : isCoreBlock
            ? "block-kind-core"
            : isBootsBlock
            ? "block-kind-boots"
            : isFinalBlock
            ? "block-kind-final"
            : isSituationalBlock
            ? "block-kind-situational"
            : "block-span-full";

          const blockTag = isCoreBlock
            ? t("items.coreTrioTag")
            : isFinalBlock
            ? t("items.itemsCountTag", { count: activeIds.length })
            : isSituationalBlock
            ? t("items.optionalTag")
            : null;

          return (
            <div key={`${kind}-${bIndex}`} className={`block-item ${blockKindClass}`}>
              <div className="block-header-row">
                <h4 title={`${bIndex + 1}. ${localizedTitle}`}>
                  {bIndex + 1}. {localizedTitle}
                </h4>
                {blockTag && (
                  <span className={`block-badge ${isCoreBlock ? "block-badge-accent" : ""}`}>
                    {blockTag}
                  </span>
                )}
              </div>
              <div className={`block-items-row ${isCoreBlock ? "block-items-flow" : ""}`}>
                {activeIds.map((itemId, iIndex) => {
                  const iconUrl = activeIcons?.[iIndex];
                  return (
                    <Fragment key={`${itemId}-${iIndex}`}>
                      {isCoreBlock && iIndex > 0 && (
                        <span className="item-flow-arrow" aria-hidden="true">
                          →
                        </span>
                      )}
                      <div className="item-icon-box" title={localizedTitle}>
                        {iconUrl ? (
                          <img
                            src={resolveCachedAssetUrl(iconUrl)}
                            alt={t("common.itemAlt", { id: itemId })}
                            className="shop-item-icon"
                            onError={(e) => {
                              (e.target as HTMLElement).style.display = "none";
                            }}
                          />
                        ) : (
                          <span className="item-id-fallback">{itemId}</span>
                        )}
                      </div>
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
