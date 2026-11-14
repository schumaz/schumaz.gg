import { Fragment } from "react";
import { ChampionBuild } from "../types";
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
  const activeBuildOption =
    build.build_options && build.build_options.length > selectedBuildIndex
      ? build.build_options[selectedBuildIndex]
      : null;

  const activeBuildMetaParts: string[] = [];
  if (activeBuildOption) {
    if (typeof activeBuildOption.pick_rate === "number" && activeBuildOption.pick_rate > 0) {
      activeBuildMetaParts.push(
        `${activeBuildOption.pick_rate.toFixed(1)}% ${readOnly ? "uso" : "escolha"}`
      );
    }
    if (activeBuildOption.play > 0) {
      activeBuildMetaParts.push(`${activeBuildOption.play.toLocaleString()} partidas`);
    }
  }
  const activeBuildMetaText = activeBuildMetaParts.join(" · ");

  return (
    <section className="card item-blocks-preview" data-testid="item-blocks">
      <div className="item-blocks-header">
        <div className="item-blocks-title-row">
          <h2>{readOnly ? "Build Recomendada" : "Build na Loja"}</h2>
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
          const isStarterBlock = block.title.includes("Itens Iniciais");
          const isCoreBlock = block.title.includes("Primeiros 3 Itens");
          const isBootsBlock = block.title.includes("Botas");
          const isFinalBlock = block.title.includes("Build Final");
          const isSituationalBlock =
            block.title.includes("Itens Situacionais") ||
            block.title.includes("Úteis") ||
            block.title.includes("Uteis");

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
          } else if (
            isCoreBlock &&
            build.core_build_options &&
            build.core_build_options.length > selectedBuildIndex
          ) {
            activeIds = build.core_build_options[selectedBuildIndex].item_ids;
            activeIcons = build.core_build_options[selectedBuildIndex].item_icon_urls;
          } else if (
            isFinalBlock &&
            build.final_build_options &&
            build.final_build_options.length > selectedBuildIndex
          ) {
            activeIds = build.final_build_options[selectedBuildIndex].item_ids;
            activeIcons = build.final_build_options[selectedBuildIndex].item_icon_urls;
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
            ? "Core Trio"
            : isFinalBlock
            ? `${activeIds.length} Itens`
            : isSituationalBlock
            ? "Opcional"
            : null;

          return (
            <div key={block.title || bIndex} className={`block-item ${blockKindClass}`}>
              <div className="block-header-row">
                <h4 title={`${bIndex + 1}. ${block.title}`}>
                  {bIndex + 1}. {block.title}
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
                      <div className="item-icon-box" title={block.title}>
                        {iconUrl ? (
                          <img
                            src={iconUrl}
                            alt={`Item ${itemId}`}
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
