import { ChampionBuild } from "../types";
import { BuildSelector } from "./BuildSelector";

interface ItemBlocksProps {
  build: ChampionBuild;
  selectedBuildIndex: number;
  onSelectBuild: (index: number) => void;
}

export function ItemBlocks({
  build,
  selectedBuildIndex,
  onSelectBuild,
}: ItemBlocksProps) {
  return (
    <section className="card item-blocks-preview" data-testid="item-blocks">
      <div className="item-blocks-header">
        <h2>Build Recomendada na Loja</h2>
        <BuildSelector
          buildOptions={build.build_options}
          selectedBuildIndex={selectedBuildIndex}
          onSelectBuild={onSelectBuild}
        />
      </div>

      <div className="grid-blocks">
        {build.item_blocks.map((block, bIndex) => {
          const isCoreBlock = block.title.includes("Primeiros 3 Itens");
          const isFinalBlock = block.title.includes("Build Final");
          const isSituationalBlock =
            block.title.includes("Itens Situacionais") ||
            block.title.includes("Úteis") ||
            block.title.includes("Uteis");

          let activeIds = block.item_ids;
          let activeIcons = block.item_icon_urls;

          if (build.build_options && build.build_options.length > selectedBuildIndex) {
            const selectedBuild = build.build_options[selectedBuildIndex];
            if (isCoreBlock) {
              activeIds = selectedBuild.core_item_ids;
              activeIcons = selectedBuild.core_item_icon_urls;
            } else if (isFinalBlock) {
              activeIds = selectedBuild.final_item_ids;
              activeIcons = selectedBuild.final_item_icon_urls;
            } else if (
              isSituationalBlock &&
              selectedBuild.situational_item_ids &&
              selectedBuild.situational_item_ids.length > 0
            ) {
              activeIds = selectedBuild.situational_item_ids;
              activeIcons = selectedBuild.situational_item_icon_urls || [];
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

          return (
            <div key={block.title || bIndex} className="block-item">
              <div className="block-header-row">
                <h4>
                  {bIndex + 1}. {block.title}
                </h4>
              </div>
              <div className="block-items-row">
                {activeIds.map((itemId, iIndex) => {
                  const iconUrl = activeIcons?.[iIndex];
                  return (
                    <div
                      key={`${itemId}-${iIndex}`}
                      className="item-icon-box"
                      title={block.title}
                    >
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
