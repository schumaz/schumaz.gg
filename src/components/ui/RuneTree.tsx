import { ChampionBuild } from "../../types";
import { TranslationKey, useTranslation } from "../../i18n";
import { resolveCachedAssetUrl } from "../../utils/assets";

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

export function RuneTree({
  build,
  selectedRunePageIndex,
  onSelectRunePage,
  readOnly = false,
}: RuneTreeProps) {
  const { t, formatNumber } = useTranslation();

  if (build.supports_runes === false) {
    return (
      <section className="card runes-tree-card mayhem-notice-card" data-testid="runes-tree">
        <div className="mayhem-notice-content">
          <div className="mayhem-badge">{t("runes.mayhemBadge")}</div>
          <h3 className="mayhem-title">{t("runes.mayhemDisabledTitle")}</h3>
          <p className="card-desc">
            {t("runes.mayhemDisabledPrefix")}
            <strong>{t("runes.mayhemDisabledMode")}</strong>
            {t("runes.mayhemDisabledMiddle")}
            <strong>{t("runes.mayhemDisabledAugments")}</strong>
            {t("runes.mayhemDisabledSuffix")}
          </p>
          <div className="mayhem-info-box">
            <div className="mayhem-info-item">
              <span className="mayhem-check">✓</span>
              <span>{t("runes.mayhemInfoSpells")}</span>
            </div>
            <div className="mayhem-info-item">
              <span className="mayhem-check">✓</span>
              <span>{t("runes.mayhemInfoItems")}</span>
            </div>
          </div>
        </div>
      </section>
    );
  }

  const activeRunePage =
    build.rune_pages && build.rune_pages.length > selectedRunePageIndex
      ? build.rune_pages[selectedRunePageIndex]
      : null;

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
      {build.rune_pages && build.rune_pages.length > 1 && (
        <div className="rune-pages-selector" data-testid="rune-pages-selector">
          <div className="rune-pages-options">
            {build.rune_pages.map((page, idx) => {
              const isSelected = selectedRunePageIndex === idx;
              const keystone = page.perks && page.perks.length > 0 ? page.perks[0] : null;
              const subIcon = page.sub_style_icon_url;
              const label = readOnly
                ? idx === 0
                  ? t("runes.pageMostUsed")
                  : t("runes.pageNumber", { num: idx + 1 })
                : idx === 0
                  ? t("runes.optionMostUsed")
                  : t("runes.optionNumber", { num: idx + 1 });
              return (
                <button
                  key={`rune-page-${idx}`}
                  type="button"
                  className={`rune-page-btn ${isSelected ? "active" : ""}`}
                  data-testid={`rune-page-btn-${idx}`}
                  onClick={() => onSelectRunePage(idx)}
                  title={label}
                >
                  <div className="rune-page-btn-icons">
                    {keystone?.icon_url && (
                      <img
                        src={resolveCachedAssetUrl(keystone.icon_url)}
                        alt={keystone.name || t("runes.keystoneFallback")}
                        className="rune-page-keystone-icon"
                      />
                    )}
                    {subIcon && (
                      <img
                        src={resolveCachedAssetUrl(subIcon)}
                        alt={page.sub_style_name || t("runes.secondaryFallback")}
                        className="rune-page-sub-icon"
                      />
                    )}
                  </div>
                  <div className="rune-page-btn-info">
                    <span className="rune-page-label">{label}</span>
                    <span className="rune-page-games">
                      {page.play
                        ? `${formatNumber(page.play)} ${t(
                            page.play === 1 ? "common.gamesSingular" : "common.gamesPlural"
                          )}`
                        : `${page.pick_rate}%`}
                    </span>
                  </div>
                </button>
              );
            })}
          </div>
        </div>
      )}

      <div className="runes-tree-columns">
        <div className="rune-column primary-column">
          <div className="tree-header">
            {primaryStyleIcon && (
              <img
                src={resolveCachedAssetUrl(primaryStyleIcon)}
                alt={primaryStyleName}
                className="tree-icon"
              />
            )}
            <h3>{primaryStyleName}</h3>
          </div>

          {perksList.length > 0 && (
            <div className="keystone-item" title={perksList[0].name}>
              {perksList[0].icon_url && (
                <img
                  src={resolveCachedAssetUrl(perksList[0].icon_url)}
                  alt={perksList[0].name}
                  className="keystone-icon"
                />
              )}
              <span className="keystone-name">{perksList[0].name}</span>
            </div>
          )}

          <div className="minor-perks-list">
            {perksList.slice(1, 4).map((perk, index) => (
              <div key={`primary-${index}-${perk.id}`} className="perk-item" title={perk.name}>
                {perk.icon_url && (
                  <img
                    src={resolveCachedAssetUrl(perk.icon_url)}
                    alt={perk.name}
                    className="perk-icon"
                  />
                )}
                <span className="perk-name">{perk.name}</span>
              </div>
            ))}
          </div>
        </div>

        <div className="rune-column secondary-column">
          <div className="tree-header">
            {subStyleIcon && (
              <img
                src={resolveCachedAssetUrl(subStyleIcon)}
                alt={subStyleName}
                className="tree-icon"
              />
            )}
            <h3>{subStyleName}</h3>
          </div>

          <div className="minor-perks-list">
            {perksList.slice(4, 6).map((perk, index) => (
              <div key={`sub-${index}-${perk.id}`} className="perk-item" title={perk.name}>
                {perk.icon_url && (
                  <img
                    src={resolveCachedAssetUrl(perk.icon_url)}
                    alt={perk.name}
                    className="perk-icon"
                  />
                )}
                <span className="perk-name">{perk.name}</span>
              </div>
            ))}
          </div>

          <div className="stat-shards-section">
            <h4>{t("runes.statShardsTitle")}</h4>
            <div className="stat-shards-list">
              {perksList.slice(6, 9).map((shard, index) => {
                const shardKey = SHARD_KEY_BY_ID[shard.id] ?? null;
                const shardLabel = shardKey ? t(shardKey) : shard.name;
                return (
                  <div key={`shard-${index}-${shard.id}`} className="shard-item" title={shard.name}>
                    {shard.icon_url && (
                      <img
                        src={resolveCachedAssetUrl(shard.icon_url)}
                        alt={shard.name}
                        className="shard-icon"
                      />
                    )}
                    <span className="shard-name">{shardLabel}</span>
                  </div>
                );
              })}
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
