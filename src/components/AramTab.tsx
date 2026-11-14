import { useMemo, useState } from "react";
import { AramAugmentEntry, AramChampionRank, AramMode } from "../types";
import { SearchInput } from "./SearchInput";

export type AugmentRarityFilter = "all" | 1 | 4 | 8;

export const LEAGUE_ARAM_ICON_URL =
  "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/content/src/leagueclient/gamemodeassets/aram/img/game-select-icon-active.png";

export const LEAGUE_ARAM_MAYHEM_ICON_URL =
  "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/assets/ux/kiwi/augments/icons/poroking_large.png";

interface AramTabProps {
  isActive: boolean;
  savedMode: AramMode | null;
  showChooser: boolean;
  tierLists: Partial<Record<AramMode, AramChampionRank[]>>;
  tierLoading: Partial<Record<AramMode, boolean>>;
  tierErrors: Partial<Record<AramMode, string | null>>;
  selectedChampionKey: string | null;
  augmentsByChampion: Record<string, AramAugmentEntry[]>;
  augmentsLoading: boolean;
  augmentsError: string | null;
  onSelectMode: (mode: AramMode) => void;
  onSelectChampion: (championKey: string) => void;
  onRetryTierList: (mode: AramMode) => void;
  onRetryAugments: () => void;
}

function normalizeSearch(value: string): string {
  return value
    .toLowerCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/\s+/g, "");
}

export function getChampionTierLabel(tier: number): string {
  if (tier <= 0) return "OP";
  return `Tier ${tier}`;
}

export function getChampionTierClass(tier: number): string {
  if (tier <= 0) return "tier-op";
  if (tier === 1) return "tier-1";
  if (tier === 2) return "tier-2";
  if (tier === 3) return "tier-3";
  if (tier === 4) return "tier-4";
  return "tier-5";
}

export function getAugmentTierLabel(tier: number): string {
  switch (tier) {
    case 0:
      return "S";
    case 1:
      return "A";
    case 2:
      return "B";
    case 3:
      return "C";
    case 4:
      return "D";
    default:
      return "E";
  }
}

export function getAugmentTierClass(tier: number): string {
  switch (tier) {
    case 0:
      return "aug-tier-s";
    case 1:
      return "aug-tier-a";
    case 2:
      return "aug-tier-b";
    case 3:
      return "aug-tier-c";
    default:
      return "aug-tier-low";
  }
}

export function getAugmentRarityInfo(rarity: number | null | undefined): {
  label: string;
  className: string;
} {
  switch (rarity) {
    case 1:
      return { label: "Prata", className: "rarity-silver" };
    case 4:
      return { label: "Ouro", className: "rarity-gold" };
    case 8:
      return { label: "Prismática", className: "rarity-prismatic" };
    default:
      return { label: "Especial", className: "rarity-neutral" };
  }
}

export function sortAugmentsBestToWorst(augments: AramAugmentEntry[]): AramAugmentEntry[] {
  return [...augments].sort((a, b) => {
    if (a.tier !== b.tier) return a.tier - b.tier;
    const popDiff = (b.popular ?? 0) - (a.popular ?? 0);
    if (Math.abs(popDiff) > 0.0001) return popDiff;
    const perfDiff = (b.performance ?? 0) - (a.performance ?? 0);
    if (Math.abs(perfDiff) > 0.0001) return perfDiff;
    return a.name.localeCompare(b.name);
  });
}

export function AramTab({
  isActive,
  savedMode,
  showChooser,
  tierLists,
  tierLoading,
  tierErrors,
  selectedChampionKey,
  augmentsByChampion,
  augmentsLoading,
  augmentsError,
  onSelectMode,
  onSelectChampion,
  onRetryTierList,
  onRetryAugments,
}: AramTabProps) {
  const [aramSearch, setAramSearch] = useState("");
  const [mayhemChampSearch, setMayhemChampSearch] = useState("");
  const [augmentSearch, setAugmentSearch] = useState("");
  const [rarityFilter, setRarityFilter] = useState<AugmentRarityFilter>("all");

  const activeMode = savedMode ?? "aram";
  const currentChampions = tierLists[activeMode] ?? [];
  const isLoadingList = Boolean(tierLoading[activeMode]);
  const listError = tierErrors[activeMode] ?? null;

  const filteredAramChampions = useMemo(() => {
    const query = normalizeSearch(aramSearch);
    if (!query) return currentChampions;
    return currentChampions.filter((c) => normalizeSearch(c.name).includes(query));
  }, [currentChampions, aramSearch]);

  const filteredMayhemChampions = useMemo(() => {
    const query = normalizeSearch(mayhemChampSearch);
    if (!query) return currentChampions;
    return currentChampions.filter((c) => normalizeSearch(c.name).includes(query));
  }, [currentChampions, mayhemChampSearch]);

  const selectedChampion = useMemo(() => {
    if (!selectedChampionKey) return null;
    return currentChampions.find((c) => c.key === selectedChampionKey) ?? null;
  }, [currentChampions, selectedChampionKey]);

  const currentAugments = useMemo(() => {
    if (!selectedChampionKey) return [];
    const raw = augmentsByChampion[selectedChampionKey] ?? [];
    const sorted = sortAugmentsBestToWorst(raw);
    const query = normalizeSearch(augmentSearch);

    return sorted.filter((aug) => {
      if (rarityFilter !== "all" && aug.rarity !== rarityFilter) {
        return false;
      }
      if (query && !normalizeSearch(aug.name).includes(query)) {
        return false;
      }
      return true;
    });
  }, [selectedChampionKey, augmentsByChampion, rarityFilter, augmentSearch]);

  return (
    <div
      className={`tab-content ${isActive ? "active-tab" : "inactive-tab"}`}
      data-testid="view-aram"
      aria-hidden={!isActive}
    >
      {showChooser ? (
        <div className="aram-chooser-wrapper" data-testid="aram-mode-chooser">
          <div className="aram-chooser-header">
            <h2 className="aram-chooser-title">Escolha o modo ARAM</h2>
            <p className="aram-chooser-subtitle">
              Sua escolha fica salva para os próximos acessos. Para trocar depois, clique novamente
              em Aram na barra inferior.
            </p>
          </div>

          <div className="aram-chooser-grid">
            <button
              type="button"
              className={`aram-choice-card ${savedMode === "aram" ? "is-saved" : ""}`}
              data-testid="aram-choose-aram"
              onClick={() => onSelectMode("aram")}
            >
              <div className="aram-choice-top">
                <div className="aram-choice-icon" aria-hidden="true">
                  <img
                    src={LEAGUE_ARAM_ICON_URL}
                    alt=""
                    className="aram-choice-icon-img"
                  />
                </div>
                {savedMode === "aram" && <span className="aram-saved-pill">Salva</span>}
              </div>
              <h3 className="aram-choice-name">ARAM</h3>
              <p className="aram-choice-desc">
                Lista de campeões ordenados pelo desempenho geral em Howling Abyss.
              </p>
            </button>

            <button
              type="button"
              className={`aram-choice-card ${savedMode === "aram_mayhem" ? "is-saved" : ""}`}
              data-testid="aram-choose-mayhem"
              onClick={() => onSelectMode("aram_mayhem")}
            >
              <div className="aram-choice-top">
                <div className="aram-choice-icon is-mayhem" aria-hidden="true">
                  <img
                    src={LEAGUE_ARAM_MAYHEM_ICON_URL}
                    alt=""
                    className="aram-choice-icon-img"
                  />
                </div>
                {savedMode === "aram_mayhem" && <span className="aram-saved-pill">Salva</span>}
              </div>
              <h3 className="aram-choice-name">ARAM Mayhem</h3>
              <p className="aram-choice-desc">
                Ranking de campeões e as melhores cartinhas de aprimoramento por raridade.
              </p>
            </button>
          </div>
        </div>
      ) : activeMode === "aram" ? (
        <div className="aram-layout" data-testid="aram-classic-view">
          <div className="card aram-toolbar-card">
            <div className="aram-toolbar-left">
              <h2 className="page-hero-title">Ranking de Campeões</h2>
            </div>

            <div className="aram-hero-right">
              <span className="aram-mode-badge">ARAM</span>

              <SearchInput
                value={aramSearch}
                onChange={setAramSearch}
                placeholder="Buscar campeão..."
                ariaLabel="Buscar campeão no ARAM"
                clearAriaLabel="Limpar busca de campeão no ARAM"
              />
            </div>
          </div>

          <div className="card aram-classic-list-card">
            {isLoadingList && currentChampions.length === 0 ? (
              <div className="aram-state-box">
                <div className="spinner aram-spinner" />
                <span>Carregando ranking do ARAM...</span>
              </div>
            ) : listError && currentChampions.length === 0 ? (
              <div className="aram-state-box">
                <span className="aram-error-text">{listError}</span>
                <button
                  type="button"
                  className="aram-retry-btn"
                  onClick={() => onRetryTierList("aram")}
                >
                  Tentar novamente
                </button>
              </div>
            ) : filteredAramChampions.length === 0 ? (
              <div className="aram-state-box">
                <span>Nenhum campeão encontrado para &quot;{aramSearch}&quot;.</span>
              </div>
            ) : (
              <div className="aram-classic-list" data-testid="aram-classic-list">
                {filteredAramChampions.map((champ) => (
                  <div key={champ.id} className="aram-classic-row">
                    <span className="aram-rank-num">#{champ.rank}</span>
                    <div className="dd-icon-frame aram-champ-frame">
                      <img
                        src={champ.icon_url}
                        alt={champ.name}
                        className="aram-champ-avatar"
                        loading="lazy"
                      />
                    </div>
                    <span className="aram-champ-name">{champ.name}</span>

                    <div className="aram-classic-stats">
                      {champ.win_rate !== null && champ.win_rate !== undefined && (
                        <span className="aram-stat-pill">
                          <span className="aram-stat-label">WR</span>
                          <span
                            className={`aram-stat-val ${
                              champ.win_rate >= 52
                                ? "is-high"
                                : champ.win_rate < 48
                                  ? "is-low"
                                  : ""
                            }`}
                          >
                            {champ.win_rate.toFixed(1)}%
                          </span>
                        </span>
                      )}
                      {champ.pick_rate !== null && champ.pick_rate !== undefined && (
                        <span className="aram-stat-pill">
                          <span className="aram-stat-label">PR</span>
                          <span className="aram-stat-val">{champ.pick_rate.toFixed(1)}%</span>
                        </span>
                      )}
                      <span className={`aram-tier-pill ${getChampionTierClass(champ.tier)}`}>
                        {getChampionTierLabel(champ.tier)}
                      </span>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>
      ) : (
        <div className="aram-layout" data-testid="aram-mayhem-view">
          <div className="aram-mayhem-grid">
            {/* Left Column: Ranked Champions for Augment Lookup */}
            <section className="card aram-mayhem-col aram-mayhem-champions-col">
              <div className="aram-panel-header">
                <div className="aram-panel-title-row">
                  <h2 className="aram-mayhem-hero-title">Cartinhas</h2>
                  <span className="aram-mode-badge is-mayhem">ARAM Mayhem</span>
                </div>
                <SearchInput
                  value={mayhemChampSearch}
                  onChange={setMayhemChampSearch}
                  placeholder="Buscar campeão..."
                  ariaLabel="Buscar campeão no ARAM Mayhem"
                  clearAriaLabel="Limpar busca de campeão no ARAM Mayhem"
                  className="is-full"
                />
              </div>

              <div className="aram-scroll-body" data-testid="aram-mayhem-champions-list">
                {isLoadingList && currentChampions.length === 0 ? (
                  <div className="aram-state-box">
                    <div className="spinner aram-spinner" />
                    <span>Carregando campeões...</span>
                  </div>
                ) : listError && currentChampions.length === 0 ? (
                  <div className="aram-state-box">
                    <span className="aram-error-text">{listError}</span>
                    <button
                      type="button"
                      className="aram-retry-btn"
                      onClick={() => onRetryTierList("aram_mayhem")}
                    >
                      Tentar novamente
                    </button>
                  </div>
                ) : filteredMayhemChampions.length === 0 ? (
                  <div className="aram-state-box">
                    <span>Nenhum campeão encontrado.</span>
                  </div>
                ) : (
                  filteredMayhemChampions.map((champ) => {
                    const isSelected = champ.key === selectedChampionKey;
                    return (
                      <button
                        key={champ.id}
                        type="button"
                        className={`aram-mayhem-champ-row ${isSelected ? "selected" : ""}`}
                        onClick={() => onSelectChampion(champ.key)}
                        data-testid={`mayhem-champ-${champ.key}`}
                      >
                        <span className="aram-rank-num">#{champ.rank}</span>
                        <div className="dd-icon-frame aram-champ-frame">
                          <img
                            src={champ.icon_url}
                            alt={champ.name}
                            className="aram-champ-avatar"
                            loading="lazy"
                          />
                        </div>
                        <span className="aram-champ-name">{champ.name}</span>
                        <span className={`aram-tier-pill ${getChampionTierClass(champ.tier)}`}>
                          {getChampionTierLabel(champ.tier)}
                        </span>
                      </button>
                    );
                  })
                )}
              </div>
            </section>

            {/* Right Column: Champion Augments ("Cartinhas") */}
            <section className="card aram-mayhem-col aram-mayhem-augments-col">
              {!selectedChampionKey ? (
                <div className="aram-state-box" data-testid="aram-mayhem-empty-augments">
                  <span>Selecione um campeão ao lado para ver as melhores cartinhas.</span>
                </div>
              ) : (
                <>
                  <div className="aram-panel-header">
                    <div className="aram-augments-hero">
                      {selectedChampion && (
                        <div className="dd-icon-frame aram-hero-frame">
                          <img
                            src={selectedChampion.icon_url}
                            alt={selectedChampion.name}
                            className="aram-hero-avatar"
                          />
                        </div>
                      )}
                      <div className="aram-hero-meta">
                        <h3 className="aram-panel-title">
                          {selectedChampion?.name ?? selectedChampionKey}
                        </h3>
                        <span className="aram-hero-sub">
                          Melhores cartinhas do rank mais forte ao mais fraco
                        </span>
                      </div>
                      {selectedChampion && (
                        <span
                          className={`aram-tier-pill aram-hero-rank-pill ${getChampionTierClass(
                            selectedChampion.tier
                          )}`}
                        >
                          #{selectedChampion.rank} · {getChampionTierLabel(selectedChampion.tier)}
                        </span>
                      )}
                    </div>

                    <div className="aram-augments-controls">
                      <div
                        className="segmented-control aram-rarity-filter"
                        role="group"
                        aria-label="Filtrar cartinhas por raridade"
                      >
                        {(
                          [
                            { id: "all", label: "Todas", dot: "" },
                            { id: 1, label: "Prata", dot: "dot-silver" },
                            { id: 4, label: "Ouro", dot: "dot-gold" },
                            { id: 8, label: "Prismática", dot: "dot-prismatic" },
                          ] as const
                        ).map((opt) => (
                          <button
                            key={String(opt.id)}
                            type="button"
                            className={`segment-btn ${rarityFilter === opt.id ? "active" : ""}`}
                            onClick={() => setRarityFilter(opt.id)}
                          >
                            {opt.dot && <span className={`aram-rarity-dot ${opt.dot}`} />}
                            <span>{opt.label}</span>
                          </button>
                        ))}
                      </div>

                      <SearchInput
                        value={augmentSearch}
                        onChange={setAugmentSearch}
                        placeholder="Buscar cartinha..."
                        ariaLabel="Buscar cartinha"
                        clearAriaLabel="Limpar busca de cartinha"
                        className="aram-aug-search"
                      />
                    </div>
                  </div>

                  <div className="aram-scroll-body" data-testid="aram-augments-list">
                    {augmentsLoading ? (
                      <div className="aram-state-box">
                        <div className="spinner aram-spinner" />
                        <span>Carregando cartinhas...</span>
                      </div>
                    ) : augmentsError ? (
                      <div className="aram-state-box">
                        <span className="aram-error-text">{augmentsError}</span>
                        <button
                          type="button"
                          className="aram-retry-btn"
                          onClick={onRetryAugments}
                        >
                          Tentar novamente
                        </button>
                      </div>
                    ) : currentAugments.length === 0 ? (
                      <div className="aram-state-box">
                        <span>Nenhuma cartinha encontrada com esses filtros.</span>
                      </div>
                    ) : (
                      currentAugments.map((aug) => {
                        const rarityInfo = getAugmentRarityInfo(aug.rarity);
                        return (
                          <div key={aug.id} className="aram-augment-row">
                            <div className={`aram-augment-icon-frame ${rarityInfo.className}`}>
                              <img
                                src={aug.icon_url}
                                alt={aug.name}
                                className="aram-augment-icon"
                                loading="lazy"
                              />
                            </div>

                            <span className={`aram-aug-tier-badge ${getAugmentTierClass(aug.tier)}`}>
                              {getAugmentTierLabel(aug.tier)}
                            </span>

                            <div className="aram-augment-info">
                              <div className="aram-augment-top">
                                <strong className="aram-augment-name">{aug.name}</strong>
                                <span className={`aram-rarity-tag ${rarityInfo.className}`}>
                                  {rarityInfo.label}
                                </span>
                                {(aug.popular ?? 0) > 0 && (
                                  <span className="aram-aug-pick">
                                    {(aug.popular ?? 0).toFixed(1)}% escolha
                                  </span>
                                )}
                              </div>
                              {aug.desc && <p className="aram-augment-desc">{aug.desc}</p>}
                            </div>
                          </div>
                        );
                      })
                    )}
                  </div>
                </>
              )}
            </section>
          </div>
        </div>
      )}
    </div>
  );
}
