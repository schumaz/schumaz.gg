import { ChampionBuild } from "../types";

interface RuneTreeProps {
  build: ChampionBuild;
  selectedRunePageIndex: number;
  onSelectRunePage: (pageIndex: number) => void;
}

export function RuneTree({
  build,
  selectedRunePageIndex,
  onSelectRunePage,
}: RuneTreeProps) {
  const activeRunePage =
    build.rune_pages && build.rune_pages.length > selectedRunePageIndex
      ? build.rune_pages[selectedRunePageIndex]
      : null;

  const primaryStyleName =
    activeRunePage?.primary_style_name || build.primary_style_name || "Árvore Primária";
  const primaryStyleIcon =
    activeRunePage?.primary_style_icon_url || build.primary_style_icon_url;
  const subStyleName =
    activeRunePage?.sub_style_name || build.sub_style_name || "Árvore Secundária";
  const subStyleIcon =
    activeRunePage?.sub_style_icon_url || build.sub_style_icon_url;
  const perksList = activeRunePage?.perks || build.perks || [];

  return (
    <section className="card runes-tree-card" data-testid="runes-tree">
      {build.rune_pages && build.rune_pages.length > 1 && (
        <div className="rune-pages-selector" data-testid="rune-pages-selector">
          <div className="rune-pages-selector-header">
            <span className="rune-pages-selector-title">Páginas de Runas</span>
            <span className="rune-pages-selector-desc">Escolha entre as mais jogadas</span>
          </div>
          <div className="rune-pages-options">
            {build.rune_pages.map((page, idx) => {
              const isSelected = selectedRunePageIndex === idx;
              const keystone = page.perks && page.perks.length > 0 ? page.perks[0] : null;
              const subIcon = page.sub_style_icon_url;
              const label = idx === 0 ? "Opção 1 (Mais Usada)" : `Opção ${idx + 1}`;
              return (
                <button
                  key={`rune-page-${idx}`}
                  type="button"
                  className={`rune-page-btn ${isSelected ? "active" : ""}`}
                  data-testid={`rune-page-btn-${idx}`}
                  onClick={() => onSelectRunePage(idx)}
                >
                  <div className="rune-page-btn-icons">
                    {keystone?.icon_url && (
                      <img
                        src={keystone.icon_url}
                        alt={keystone.name || "Keystone"}
                        className="rune-page-keystone-icon"
                      />
                    )}
                    {subIcon && (
                      <img
                        src={subIcon}
                        alt={page.sub_style_name || "Secundária"}
                        className="rune-page-sub-icon"
                      />
                    )}
                  </div>
                  <div className="rune-page-btn-info">
                    <span className="rune-page-label">{label}</span>
                    <span className="rune-page-games">
                      {page.play ? `${page.play.toLocaleString()} partidas` : `${page.pick_rate}%`}
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
            <h4>Fragmentos de Atributo</h4>
            <div className="stat-shards-list">
              {perksList.slice(6, 9).map((shard, index) => (
                <div key={`shard-${index}-${shard.id}`} className="shard-item" title={shard.name}>
                  {shard.icon_url && (
                    <img src={shard.icon_url} alt={shard.name} className="shard-icon" />
                  )}
                  <span className="shard-name">{shard.name}</span>
                </div>
              ))}
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
