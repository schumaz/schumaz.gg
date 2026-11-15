import { ChampionBuild } from "../types";
import { TranslationKey, useTranslation } from "../i18n";

interface RuneTreeProps {
  build: ChampionBuild;
  selectedRunePageIndex: number;
  onSelectRunePage: (pageIndex: number) => void;
  readOnly?: boolean;
}

function getSimplifiedShardKey(id: number): TranslationKey | null {
  switch (id) {
    case 5008:
      return "runes.shards.adaptive";
    case 5005:
      return "runes.shards.attackSpeed";
    case 5007:
      return "runes.shards.abilityHaste";
    case 5002:
      return "runes.shards.armor";
    case 5003:
      return "runes.shards.magicResist";
    case 5001:
    case 5013:
      return "runes.shards.healthScaling";
    case 5010:
      return "runes.shards.moveSpeed";
    case 5011:
      return "runes.shards.health";
    case 5012:
      return "runes.shards.tenacity";
    default:
      return null;
  }
}

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
                        src={keystone.icon_url}
                        alt={keystone.name || t("runes.keystoneFallback")}
                        className="rune-page-keystone-icon"
                      />
                    )}
                    {subIcon && (
                      <img
                        src={subIcon}
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
        {/* Primary Rune Tree */}
        <div className="rune-column primary-column">
          <div className="tree-header">
            {primaryStyleIcon && (
              <img
                src={primaryStyleIcon}
                alt={primaryStyleName}
                className="tree-icon"
              />
            )}
            <h3>{primaryStyleName}</h3>
          </div>

          {/* Primary Keystone */}
          {perksList.length > 0 && (
            <div className="keystone-item" title={perksList[0].name}>
              {perksList[0].icon_url && (
                <img
                  src={perksList[0].icon_url}
                  alt={perksList[0].name}
                  className="keystone-icon"
                />
              )}
              <span className="keystone-name">{perksList[0].name}</span>
            </div>
          )}

          {/* Primary Minor Perks (Slots 1, 2, 3) */}
          <div className="minor-perks-list">
            {perksList.slice(1, 4).map((perk, index) => (
              <div key={`primary-${index}-${perk.id}`} className="perk-item" title={perk.name}>
                {perk.icon_url && (
                  <img src={perk.icon_url} alt={perk.name} className="perk-icon" />
                )}
                <span className="perk-name">{perk.name}</span>
              </div>
            ))}
          </div>
        </div>

        {/* Secondary Rune Tree & Stat Shards */}
        <div className="rune-column secondary-column">
          <div className="tree-header">
            {subStyleIcon && (
              <img
                src={subStyleIcon}
                alt={subStyleName}
                className="tree-icon"
              />
            )}
            <h3>{subStyleName}</h3>
          </div>

          {/* Secondary Minor Perks (Slots 4, 5) */}
          <div className="minor-perks-list">
            {perksList.slice(4, 6).map((perk, index) => (
              <div key={`sub-${index}-${perk.id}`} className="perk-item" title={perk.name}>
                {perk.icon_url && (
                  <img src={perk.icon_url} alt={perk.name} className="perk-icon" />
                )}
                <span className="perk-name">{perk.name}</span>
              </div>
            ))}
          </div>

          {/* Stat Shards (Slots 6, 7, 8) */}
          <div className="stat-shards-section">
            <h4>{t("runes.statShardsTitle")}</h4>
            <div className="stat-shards-list">
              {perksList.slice(6, 9).map((shard, index) => {
                const shardKey = getSimplifiedShardKey(shard.id);
                const shardLabel = shardKey ? t(shardKey) : shard.name;
                return (
                  <div key={`shard-${index}-${shard.id}`} className="shard-item" title={shard.name}>
                    {shard.icon_url && (
                      <img src={shard.icon_url} alt={shard.name} className="shard-icon" />
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
