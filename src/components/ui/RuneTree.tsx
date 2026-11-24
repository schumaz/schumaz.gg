import { ChampionBuild, RuneMetadata } from "../../types";
import { TranslationKey, useTranslation } from "../../i18n";
import { resolveCachedAssetUrl } from "../../utils/assets";
import { RuneTooltipContent } from "./BuildTooltips";
import { RuneMayhemNotice } from "./RuneMayhemNotice";
import { RunePagesSelector } from "./RunePagesSelector";
import { Tooltip } from "./Tooltip";

interface RuneTreeProps {
  build: ChampionBuild;
  selectedRunePageIndex: number;
  onSelectRunePage: (pageIndex: number) => void;
  readOnly?: boolean;
}

const SHARD_KEY_BY_ID: Record<number, TranslationKey> = {
  5008: "runes.shards.adaptive",
  5005: "runes.shards.attackSpeed",
  5007: "runes.shards.abilityHaste",
  5002: "runes.shards.armor",
  5003: "runes.shards.magicResist",
  5001: "runes.shards.healthScaling",
  5013: "runes.shards.healthScaling",
  5010: "runes.shards.moveSpeed",
  5011: "runes.shards.health",
  5012: "runes.shards.tenacity",
};

function renderMinorPerk(perk: RuneMetadata, keyPrefix: string, index: number) {
  const fallbackInitial = perk.name.trim().slice(0, 2).toUpperCase() || "?";

  return (
    <Tooltip
      key={`${keyPrefix}-${index}-${perk.id}`}
      content={<RuneTooltipContent rune={perk} />}
      triggerClassName="ui-tooltip-trigger-block"
      ariaLabel={perk.name}
    >
      <div className="perk-item">
        <div className="rune-medallion">
          {perk.icon_url ? (
            <img
              src={resolveCachedAssetUrl(perk.icon_url)}
              alt={perk.name}
              className="rune-medallion-icon"
            />
          ) : (
            <span className="rune-medallion-fallback" aria-hidden="true">
              {fallbackInitial}
            </span>
          )}
        </div>
        <span className="perk-name">{perk.name}</span>
      </div>
    </Tooltip>
  );
}

function renderTreeHeader(
  styleName: string,
  styleIcon: string | undefined,
  badgeLabel: string,
  accentBadge: boolean
) {
  return (
    <div className="rune-subcard-header">
      <div className="tree-header-identity">
        {styleIcon && (
          <img src={resolveCachedAssetUrl(styleIcon)} alt={styleName} className="tree-icon" />
        )}
        <h3>{styleName}</h3>
      </div>
      <span className={`block-badge${accentBadge ? " block-badge-accent" : ""}`}>
        {badgeLabel}
      </span>
    </div>
  );
}

/**
 * Structured rune tree panel with sub-cards matching ItemBlocks surface language and rich tooltips.
 */
export function RuneTree({
  build,
  selectedRunePageIndex,
  onSelectRunePage,
  readOnly = false,
}: RuneTreeProps) {
  const { t, formatNumber } = useTranslation();

  if (build.supports_runes === false) {
    return <RuneMayhemNotice />;
  }

  const activeRunePage =
    build.rune_pages && build.rune_pages.length > selectedRunePageIndex
      ? build.rune_pages[selectedRunePageIndex]
      : null;

  const activeRuneMetaParts: string[] = [];
  if (activeRunePage) {
    if (typeof activeRunePage.pick_rate === "number" && activeRunePage.pick_rate > 0) {
      activeRuneMetaParts.push(
        `${activeRunePage.pick_rate.toFixed(1)}% ${
          readOnly ? t("items.usageLabel") : t("items.pickLabel")
        }`
      );
    }
    if (activeRunePage.play > 0) {
      activeRuneMetaParts.push(
        `${formatNumber(activeRunePage.play)} ${t(
          activeRunePage.play === 1 ? "common.gamesSingular" : "common.gamesPlural"
        )}`
      );
    }
  }
  const activeRuneMetaText = activeRuneMetaParts.join(" · ");

  const primaryStyleName =
    activeRunePage?.primary_style_name ||
    build.primary_style_name ||
    t("runes.primaryTreeFallback");
  const primaryStyleIcon =
    activeRunePage?.primary_style_icon_url || build.primary_style_icon_url;
  const subStyleName =
    activeRunePage?.sub_style_name || build.sub_style_name || t("runes.subTreeFallback");
  const subStyleIcon =
    activeRunePage?.sub_style_icon_url || build.sub_style_icon_url;
  const perksList = activeRunePage?.perks || build.perks || [];

  return (
    <section className="card runes-tree-card" data-testid="runes-tree">
      <div className="runes-card-header">
        <div className="runes-title-row">
          <h2>{readOnly ? t("runes.titleReadOnly") : t("runes.titleActive")}</h2>
          {activeRuneMetaText && (
            <span className="runes-meta-pill" data-testid="active-rune-meta">
              {activeRuneMetaText}
            </span>
          )}
        </div>

        {build.rune_pages && build.rune_pages.length > 1 && (
          <RunePagesSelector
            runePages={build.rune_pages}
            selectedRunePageIndex={selectedRunePageIndex}
            onSelectRunePage={onSelectRunePage}
            readOnly={readOnly}
          />
        )}
      </div>

      <div className="runes-tree-columns">
        <div className="rune-subcard rune-subcard-primary">
          {renderTreeHeader(primaryStyleName, primaryStyleIcon, t("runes.primaryBadge"), true)}

          {perksList.length > 0 && (
            <Tooltip
              content={<RuneTooltipContent rune={perksList[0]} />}
              triggerClassName="ui-tooltip-trigger-block"
              ariaLabel={perksList[0].name}
            >
              <div className="keystone-item">
                {perksList[0].icon_url && (
                  <img
                    src={resolveCachedAssetUrl(perksList[0].icon_url)}
                    alt={perksList[0].name}
                    className="keystone-icon"
                  />
                )}
                <span className="keystone-name">{perksList[0].name}</span>
              </div>
            </Tooltip>
          )}

          <div className="minor-perks-list" data-testid="primary-rune-medallions">
            {perksList
              .slice(1, 4)
              .map((perk, index) => renderMinorPerk(perk, "primary", index))}
          </div>
        </div>

        <div className="rune-secondary-stack">
          <div className="rune-subcard rune-subcard-secondary">
            {renderTreeHeader(subStyleName, subStyleIcon, t("runes.secondaryBadge"), false)}

            <div className="minor-perks-list" data-testid="secondary-rune-medallions">
              {perksList
                .slice(4, 6)
                .map((perk, index) => renderMinorPerk(perk, "sub", index))}
            </div>
          </div>

          <div className="rune-subcard rune-subcard-shards">
            <div className="rune-subcard-header">
              <h4>{t("runes.statShardsTitle")}</h4>
            </div>
            <div className="stat-shards-list" data-testid="stat-shards-medallions">
              {perksList.slice(6, 9).map((shard, index) => {
                const shardKey = SHARD_KEY_BY_ID[shard.id] ?? null;
                const shardLabel = shardKey ? t(shardKey) : shard.name;
                return (
                  <Tooltip
                    key={`shard-${index}-${shard.id}`}
                    content={<RuneTooltipContent rune={shard} />}
                    triggerClassName="ui-tooltip-trigger-block"
                    ariaLabel={shard.name}
                  >
                    <div className="shard-item">
                      <div className="shard-medallion">
                        {shard.icon_url && (
                          <img
                            src={resolveCachedAssetUrl(shard.icon_url)}
                            alt={shard.name}
                            className="shard-medallion-icon"
                          />
                        )}
                      </div>
                      <span className="shard-name">{shardLabel}</span>
                    </div>
                  </Tooltip>
                );
              })}
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
