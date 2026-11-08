import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./App.css";

export type ActiveTab = "runes" | "settings";
export type FlashKeyPreference = "D" | "F";

export interface UserPreferences {
  autoInjectRunes: boolean;
  autoInjectSpells: boolean;
  autoInjectItems: boolean;
  flashKey: FlashKeyPreference;
}

export const DEFAULT_PREFERENCES: UserPreferences = {
  autoInjectRunes: true,
  autoInjectSpells: true,
  autoInjectItems: true,
  flashKey: "D",
};

const PREFERENCES_STORAGE_KEY = "schumaz_user_preferences";

export interface SummonerInfo {
  summonerId: number;
  gameName: string;
  tagLine: string;
  displayName: string;
  summonerLevel: number;
  profileIconId: number;
}

export interface InjectionReport {
  champion_name: string;
  runes_injected: boolean;
  spells_injected: boolean;
  items_injected: boolean;
  message: string;
}

export interface ChampSelectedPayload {
  champion_id: number;
  queue_id: number;
  queue_mode: string;
  locked_in: boolean;
  assigned_position?: string;
}

export interface RuneMetadata {
  id: number;
  key: string;
  name: string;
  icon_url: string;
}

export interface SpellMetadata {
  id: number;
  name: string;
  icon_url: string;
}

export interface ItemBlock {
  title: string;
  item_ids: number[];
  item_icon_urls?: string[];
}

export interface ChampionBuild {
  champion_id: number;
  champion_name: string;
  win_rate: number;
  pick_rate: number;
  primary_style_id: number;
  sub_style_id: number;
  selected_perk_ids: number[];
  spell1_id: number;
  spell2_id: number;
  item_blocks: ItemBlock[];
  champion_icon_url?: string;
  primary_style_name?: string;
  primary_style_icon_url?: string;
  sub_style_name?: string;
  sub_style_icon_url?: string;
  perks?: RuneMetadata[];
  spells?: SpellMetadata[];
  spell_choices?: SpellMetadata[];
  active_position?: string;
  active_tier?: string;
  active_region?: string;
  active_patch?: string;
  available_patches?: string[];
  patch_fallback_applied?: boolean;
}

function loadInitialPreferences(): UserPreferences {
  try {
    const raw = localStorage.getItem(PREFERENCES_STORAGE_KEY);
    if (raw) {
      const parsed = JSON.parse(raw);
      return {
        autoInjectRunes: typeof parsed.autoInjectRunes === "boolean" ? parsed.autoInjectRunes : DEFAULT_PREFERENCES.autoInjectRunes,
        autoInjectSpells: typeof parsed.autoInjectSpells === "boolean" ? parsed.autoInjectSpells : DEFAULT_PREFERENCES.autoInjectSpells,
        autoInjectItems: typeof parsed.autoInjectItems === "boolean" ? parsed.autoInjectItems : DEFAULT_PREFERENCES.autoInjectItems,
        flashKey: parsed.flashKey === "F" ? "F" : "D",
      };
    }
  } catch {
    // Fallback to default preferences if reading localStorage fails
  }
  return DEFAULT_PREFERENCES;
}

function App() {
  const [activeTab, setActiveTab] = useState<ActiveTab>("runes");
  const [preferences, setPreferences] = useState<UserPreferences>(loadInitialPreferences);
  const [summoner, setSummoner] = useState<SummonerInfo | null>(null);
  const [champSelect, setChampSelect] = useState<ChampSelectedPayload | null>(null);
  const [build, setBuild] = useState<ChampionBuild | null>(null);
  const [spellsInverted, setSpellsInverted] = useState(false);
  const [selectedSecondarySpellId, setSelectedSecondarySpellId] = useState<number | null>(null);
  const [loading, setLoading] = useState(false);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  const computeOrderedSpells = (
    baseSpells: SpellMetadata[] | undefined,
    secondarySpellId: number | null,
    spellChoices: SpellMetadata[] | undefined,
    flashKey: FlashKeyPreference,
    inverted: boolean
  ): SpellMetadata[] => {
    if (!baseSpells || baseSpells.length < 2) return baseSpells || [];
    let spellsCopy = [...baseSpells];

    if (secondarySpellId && spellChoices && spellChoices.length > 0) {
      const chosen = spellChoices.find((c) => c.id === secondarySpellId);
      if (chosen) {
        const secondaryIds = new Set(spellChoices.map((c) => c.id));
        const idxToReplace = spellsCopy.findIndex((s) => secondaryIds.has(s.id));
        if (idxToReplace !== -1) {
          spellsCopy[idxToReplace] = chosen;
        } else {
          const nonAnchorIdx = spellsCopy.findIndex((s) => s.id !== 4 && s.id !== 11);
          if (nonAnchorIdx !== -1) {
            spellsCopy[nonAnchorIdx] = chosen;
          } else {
            spellsCopy[1] = chosen;
          }
        }
      }
    }

    const flashIndex = spellsCopy.findIndex((s) => s.id === 4);
    if (flashIndex !== -1) {
      if (flashKey === "F" && flashIndex === 0) {
        spellsCopy.reverse();
      } else if (flashKey === "D" && flashIndex === 1) {
        spellsCopy.reverse();
      }
    }

    if (inverted) {
      spellsCopy.reverse();
    }

    return spellsCopy;
  };

  // Sync preferences state with localStorage and Tauri backend
  const updatePreference = <K extends keyof UserPreferences>(key: K, value: UserPreferences[K]) => {
    setPreferences((prev) => {
      const next = { ...prev, [key]: value };
      try {
        localStorage.setItem(PREFERENCES_STORAGE_KEY, JSON.stringify(next));
      } catch {
        // Ignore localStorage quota errors
      }
      invoke("sync_user_preferences", { preferences: next }).catch(() => {});
      return next;
    });

    if (key === "flashKey") {
      const newFlashKey = value as FlashKeyPreference;
      const reordered = computeOrderedSpells(
        build?.spells,
        selectedSecondarySpellId,
        build?.spell_choices,
        newFlashKey,
        spellsInverted
      );
      if (reordered.length >= 2) {
        invoke("inject_summoner_spells", {
          spell1Id: reordered[0].id,
          spell2Id: reordered[1].id,
        }).catch(() => {});
      }
    }
  };

  const handleFilterChange = async (
    filterType: "position" | "tier" | "region" | "patch",
    value: string
  ) => {
    if (!build) return;
    const champId = champSelect?.champion_id || build.champion_id;
    const qId = champSelect?.queue_id || 420;

    setLoading(true);
    setErrorMsg(null);
    try {
      const updatedBuild = await invoke<ChampionBuild>("update_build_filters", {
        championId: champId,
        queueId: qId,
        position: filterType === "position" ? value : (build.active_position || null),
        tier: filterType === "tier" ? value : (build.active_tier || null),
        region: filterType === "region" ? value : (build.active_region || null),
        patch: filterType === "patch" ? value : (build.active_patch || null),
      });
      if (updatedBuild) {
        setBuild(updatedBuild);
      }
    } catch (err: unknown) {
      console.error("Falha ao atualizar filtros da build:", err);
      setErrorMsg(
        typeof err === "string"
          ? err
          : "Erro ao carregar build com os filtros selecionados."
      );
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    // Sync initial preferences with backend
    invoke("sync_user_preferences", { preferences }).catch(() => {});

    // Initial check for summoner status
    invoke<SummonerInfo | null>("get_summoner_status")
      .then((data) => {
        if (data) setSummoner(data);
      })
      .catch(() => {});

    // Listen to real-time events emitted from Tauri backend
    let unlistenConnected: (() => void) | undefined;
    let unlistenDisconnected: (() => void) | undefined;
    let unlistenChampSelected: (() => void) | undefined;
    let unlistenChampSelectEnded: (() => void) | undefined;
    let unlistenBuild: (() => void) | undefined;

    const setupListeners = async () => {
      try {
        unlistenConnected = await listen<SummonerInfo>("lcu-connected", (event) => {
          setSummoner(event.payload);
          setErrorMsg(null);
        });

        unlistenDisconnected = await listen("lcu-disconnected", () => {
          setSummoner(null);
          setChampSelect(null);
          setBuild(null);
        });

        unlistenChampSelected = await listen<ChampSelectedPayload>("champ-selected", async (event) => {
          setChampSelect(event.payload);
          setSpellsInverted(false);
          setSelectedSecondarySpellId(null);

          // Fetch recommended build for selected champion if not already received
          try {
            if (event.payload.assigned_position) {
              const recommended = await invoke<ChampionBuild>("update_build_filters", {
                championId: event.payload.champion_id,
                queueId: event.payload.queue_id || 420,
                position: event.payload.assigned_position,
                tier: null,
                region: null,
                patch: null,
              });
              if (recommended) {
                setBuild(recommended);
              }
            } else {
              const recommended = await invoke<ChampionBuild>("get_recommended_build_for_champion", {
                championId: event.payload.champion_id,
                queueId: event.payload.queue_id || 420,
              });
              if (recommended) {
                setBuild(recommended);
              }
            }
          } catch {
            // Ignore background fetch error, ws_listener will emit recommended-build
          }
        });

        unlistenChampSelectEnded = await listen("champ-select-ended", () => {
          setChampSelect(null);
          setBuild(null);
          setSpellsInverted(false);
          setSelectedSecondarySpellId(null);
        });

        unlistenBuild = await listen<ChampionBuild>("recommended-build", (event) => {
          setBuild(event.payload);
          setSelectedSecondarySpellId(null);
        });

        // Injection report is not displayed in the UI
      } catch (e) {
        console.warn("Tauri event listener setup skipped or running in test mode:", e);
      }
    };

    setupListeners();

    return () => {
      if (unlistenConnected) unlistenConnected();
      if (unlistenDisconnected) unlistenDisconnected();
      if (unlistenChampSelected) unlistenChampSelected();
      if (unlistenChampSelectEnded) unlistenChampSelectEnded();
      if (unlistenBuild) unlistenBuild();
      // unlistenInjected removed
    };
  }, []);

  const orderedSpells = computeOrderedSpells(
    build?.spells,
    selectedSecondarySpellId,
    build?.spell_choices,
    preferences.flashKey,
    spellsInverted
  );

  const handleInvertSpells = async () => {
    const nextFlashKey: FlashKeyPreference = preferences.flashKey === "D" ? "F" : "D";
    updatePreference("flashKey", nextFlashKey);
  };

  const handleSelectSecondarySpell = async (spellId: number) => {
    setSelectedSecondarySpellId(spellId);
    const reordered = computeOrderedSpells(
      build?.spells,
      spellId,
      build?.spell_choices,
      preferences.flashKey,
      spellsInverted
    );
    if (reordered.length >= 2) {
      try {
        await invoke("inject_summoner_spells", {
          spell1Id: reordered[0].id,
          spell2Id: reordered[1].id,
        });
      } catch (e) {
        console.warn("Failed to inject secondary spell change:", e);
      }
    }
  };

  const patchOptions =
    build?.available_patches && build.available_patches.length > 0
      ? build.available_patches
      : build?.active_patch
      ? [build.active_patch]
      : ["16.22", "16.21"];

  return (
    <main className="container">
      <header className="header">
        <h1 className="title">schumaz.gg</h1>
        <p className="subtitle">Assistente de Alto Desempenho para League of Legends</p>

        <div className="status-badge-container">
          {summoner ? (
            <div className="status-badge connected" data-testid="status-connected">
              <span className="dot dot-green"></span>
              <span>
                Conectado: <strong>{summoner.gameName || summoner.displayName}</strong>
                {summoner.tagLine && <small> #{summoner.tagLine}</small>} (Nível {summoner.summonerLevel})
              </span>
            </div>
          ) : (
            <div className="status-badge disconnected" data-testid="status-disconnected">
              <span className="dot dot-orange"></span>
              <span>Aguardando cliente do LoL (LeagueClientUx.exe)...</span>
            </div>
          )}
        </div>
      </header>

      {/* View 1: "Runas" Tab (Match Dashboard) */}
      {activeTab === "runes" && (
        <div className="tab-content" data-testid="view-runes">
          {build ? (
            <>
              {/* Champion Hero Overview Card */}
              <section className="card champ-hero-card" data-testid="champ-hero">
                <div className="champ-hero-top">
                  <div className="champ-hero-left">
                    {build.champion_icon_url ? (
                      <img
                        src={build.champion_icon_url}
                        alt={build.champion_name}
                        className="champ-avatar"
                        onError={(e) => {
                          (e.target as HTMLElement).style.display = "none";
                        }}
                      />
                    ) : (
                      <div className="champ-avatar-placeholder">
                        {build.champion_name.slice(0, 2).toUpperCase()}
                      </div>
                    )}

                    <div className="champ-hero-details">
                      <div className="champ-hero-title-row">
                        <h2 className="champ-name">{build.champion_name}</h2>
                        <span className={`tag ${champSelect?.locked_in ? "locked" : "hovering"}`}>
                          {champSelect?.locked_in ? "Confirmado (Lock-in)" : "Selecionando (Hover)"}
                        </span>
                      </div>

                      <div className="champ-meta-row">
                        <span className="badge-meta">
                          {champSelect?.queue_mode === "Aram"
                            ? "ARAM (Howling Abyss)"
                            : champSelect?.queue_mode === "PracticeTool"
                            ? "Modo Treino (Build Ranqueada)"
                            : "Summoner's Rift (Ranqueada / Normal)"}
                        </span>
                        {build.pick_rate > 0 && (
                          <span className="badge badge-neutral badge-stat">
                            {build.pick_rate.toFixed(1)}% Pick Rate
                          </span>
                        )}
                        {build.patch_fallback_applied && (
                          <span
                            className="badge badge-warning badge-stat patch-fallback-badge"
                            title="Patch recente (<48h). Usando dados consolidados do patch anterior."
                            data-testid="patch-fallback-badge"
                          >
                            🛡️ Volume Protegido ({build.active_patch})
                          </span>
                        )}
                      </div>
                    </div>
                  </div>
                </div>

                {/* 4 Filters: Elo Base, Rota, Servidor, Patch */}
                <div className="champ-filters-bar" data-testid="champ-filters-bar">
                  <div className="filter-group">
                    <label htmlFor="filter-tier" className="filter-label">
                      Elo:
                    </label>
                    <select
                      id="filter-tier"
                      data-testid="filter-tier"
                      className="filter-select"
                      value={build.active_tier || "emerald_plus"}
                      disabled={loading}
                      onChange={(e) => handleFilterChange("tier", e.target.value)}
                    >
                      <option value="all">Todos os Elos</option>
                      <option value="challenger">Desafiante</option>
                      <option value="grandmaster">Grão-Mestre</option>
                      <option value="master_plus">Mestre +</option>
                      <option value="diamond_plus">Diamante +</option>
                      <option value="emerald_plus">Esmeralda +</option>
                      <option value="platinum_plus">Platina +</option>
                      <option value="gold_plus">Ouro +</option>
                      <option value="silver">Prata</option>
                      <option value="bronze">Bronze</option>
                      <option value="iron">Ferro</option>
                    </select>
                  </div>

                  <div className="filter-group">
                    <label htmlFor="filter-position" className="filter-label">
                      Rota:
                    </label>
                    <select
                      id="filter-position"
                      data-testid="filter-position"
                      className="filter-select"
                      value={build.active_position || "mid"}
                      disabled={loading}
                      onChange={(e) => handleFilterChange("position", e.target.value)}
                    >
                      <option value="top">Topo</option>
                      <option value="jungle">Selva</option>
                      <option value="mid">Meio</option>
                      <option value="adc">Atirador (Bot)</option>
                      <option value="support">Suporte</option>
                    </select>
                  </div>

                  <div className="filter-group">
                    <label htmlFor="filter-region" className="filter-label">
                      Servidor:
                    </label>
                    <select
                      id="filter-region"
                      data-testid="filter-region"
                      className="filter-select"
                      value={build.active_region || "global"}
                      disabled={loading}
                      onChange={(e) => handleFilterChange("region", e.target.value)}
                    >
                      <option value="global">Mundial (Global)</option>
                      <option value="br">Brasil (BR)</option>
                      <option value="kr">Coreia (KR)</option>
                      <option value="na">América do Norte (NA)</option>
                      <option value="euw">Europa Ocidental (EUW)</option>
                      <option value="eune">Europa Nórdica e Oriental (EUNE)</option>
                      <option value="lan">América Latina Norte (LAN)</option>
                      <option value="las">América Latina Sul (LAS)</option>
                      <option value="oce">Oceania (OCE)</option>
                      <option value="tr">Turquia (TR)</option>
                      <option value="ru">Rússia (RU)</option>
                      <option value="jp">Japão (JP)</option>
                      <option value="me">Oriente Médio (ME)</option>
                      <option value="sg">Singapura (SG)</option>
                      <option value="tw">Taiwan (TW)</option>
                      <option value="vn">Vietnã (VN)</option>
                    </select>
                  </div>

                  <div className="filter-group">
                    <label htmlFor="filter-patch" className="filter-label">
                      Patch:
                    </label>
                    <select
                      id="filter-patch"
                      data-testid="filter-patch"
                      className="filter-select"
                      value={build.active_patch || ""}
                      disabled={loading}
                      onChange={(e) => handleFilterChange("patch", e.target.value)}
                    >
                      {patchOptions.map((p) => (
                        <option key={p} value={p}>
                          Patch {p}
                        </option>
                      ))}
                    </select>
                  </div>
                </div>
              </section>

              {/* Runes Tree & Spells Layout */}
              <div className="runes-spells-grid">
                {/* Visual Rune Tree Card */}
                <section className="card runes-tree-card" data-testid="runes-tree">
                  <div className="runes-tree-columns">
                    {/* Primary Rune Tree */}
                    <div className="rune-column primary-column">
                      <div className="tree-header">
                        {build.primary_style_icon_url && (
                          <img
                            src={build.primary_style_icon_url}
                            alt={build.primary_style_name || "Primária"}
                            className="tree-icon"
                          />
                        )}
                        <h3>{build.primary_style_name || "Árvore Primária"}</h3>
                      </div>

                      {/* Primary Keystone */}
                      {build.perks && build.perks.length > 0 && (
                        <div className="keystone-item" title={build.perks[0].name}>
                          {build.perks[0].icon_url && (
                            <img
                              src={build.perks[0].icon_url}
                              alt={build.perks[0].name}
                              className="keystone-icon"
                            />
                          )}
                          <span className="keystone-name">{build.perks[0].name}</span>
                        </div>
                      )}

                      {/* Primary Minor Perks (Slots 1, 2, 3) */}
                      <div className="minor-perks-list">
                        {build.perks?.slice(1, 4).map((perk, index) => (
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
                        {build.sub_style_icon_url && (
                          <img
                            src={build.sub_style_icon_url}
                            alt={build.sub_style_name || "Secundária"}
                            className="tree-icon"
                          />
                        )}
                        <h3>{build.sub_style_name || "Árvore Secundária"}</h3>
                      </div>

                      {/* Secondary Minor Perks (Slots 4, 5) */}
                      <div className="minor-perks-list">
                        {build.perks?.slice(4, 6).map((perk, index) => (
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
                          {build.perks?.slice(6, 9).map((shard, index) => (
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

                {/* Summoner Spells Card */}
                <section className="card spells-card" data-testid="spells-card">
                  <div className="spells-card-header">
                    <h2>Feitiços de Invocador</h2>
                    <button
                      type="button"
                      className="invert-spells-btn"
                      data-testid="invert-spells-btn"
                      onClick={handleInvertSpells}
                      title="Inverter a posição dos feitiços entre as teclas D e F"
                    >
                      <span className="invert-spells-icon">⇄</span> Inverter Feitiços
                    </button>
                  </div>
                  <p className="card-desc">Feitiços recomendados adaptados para a fila atual.</p>

                  <div className="spells-list">
                    {orderedSpells && orderedSpells.length >= 2 ? (
                      orderedSpells.map((spell, index) => (
                        <div key={`spell-${index}-${spell.id}`} className="spell-item">
                          <span className="spell-key-indicator">{index === 0 ? "D" : "F"}</span>
                          {spell.icon_url && (
                            <img src={spell.icon_url} alt={spell.name} className="spell-icon" />
                          )}
                          <div className="spell-info">
                            <span className="spell-name">{spell.name}</span>
                          </div>
                        </div>
                      ))
                    ) : (
                      <div className="spells-fallback">
                        <div className="spell-item">
                          <span className="spell-key-indicator">D</span>
                          <span className="spell-name">Feitiço 1</span>
                        </div>
                        <div className="spell-item">
                          <span className="spell-key-indicator">F</span>
                          <span className="spell-name">Feitiço 2</span>
                        </div>
                      </div>
                    )}
                  </div>

                  {build.spell_choices && build.spell_choices.length > 0 && (
                    <div className="secondary-spell-selector" data-testid="secondary-spell-selector">
                      <h4 className="secondary-spell-title">Escolha de Segundo Feitiço</h4>
                      <div className="secondary-spell-options">
                        {build.spell_choices.map((choice, idx) => {
                          const currentSecondaryId =
                            selectedSecondarySpellId ?? build.spell_choices?.[0]?.id;
                          const isSelected = currentSecondaryId === choice.id;
                          return (
                            <button
                              key={`sec-spell-${choice.id}-${idx}`}
                              type="button"
                              className={`secondary-spell-btn ${isSelected ? "active" : ""}`}
                              data-testid={`sec-spell-btn-${choice.id}`}
                              onClick={() => handleSelectSecondarySpell(choice.id)}
                              title={idx === 0 ? "Mais Usado" : "Segundo Mais Usado"}
                            >
                              {choice.icon_url && (
                                <img
                                  src={choice.icon_url}
                                  alt={choice.name}
                                  className="secondary-spell-icon"
                                />
                              )}
                              <span className="secondary-spell-name">{choice.name}</span>
                            </button>
                          );
                        })}
                      </div>
                    </div>
                  )}
                </section>
              </div>

              {/* Conjuntos de Itens na Loja */}
              <section className="card item-blocks-preview" data-testid="item-blocks">
                <h2>Build Recomendada na Loja</h2>
                <div className="grid-blocks">
                  {build.item_blocks.map((block, bIndex) => (
                    <div key={block.title || bIndex} className="block-item">
                      <h4>
                        {bIndex + 1}. {block.title}
                      </h4>
                      <div className="block-items-row">
                        {block.item_ids.map((itemId, iIndex) => {
                          const iconUrl = block.item_icon_urls?.[iIndex];
                          return (
                            <div key={`${itemId}-${iIndex}`} className="item-icon-box" title={block.title}>
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
                  ))}
                </div>
              </section>
            </>
          ) : (
            /* Idle / Waiting for Champion Select State */
            <div className="idle-state-container">
              <section className="card idle-card">
                <div className="idle-icon-box">
                  <svg viewBox="0 0 24 24" width="48" height="48" fill="none" stroke="currentColor" strokeWidth="1.5">
                    <polygon points="12 2 2 7 12 12 22 7 12 2" />
                    <polyline points="2 17 12 22 22 17" />
                    <polyline points="2 12 12 17 22 12" />
                  </svg>
                </div>
                <h2>Aguardando Seleção de Campeões</h2>
                <p className="card-desc">
                  Entre em uma partida ou sala de draft no League of Legends. Assim que você selecionar ou travar um
                  campeão, as runas recomendadas, feitiços e itens da build mais populares serão carregados e aplicados
                  automaticamente a partir dos dados do OP.GG.
                </p>

                {!summoner && (
                  <p className="hint-text">
                    * Abra o cliente do LoL (LeagueClientUx.exe) para habilitar a conexão e injeção direta.
                  </p>
                )}

                {errorMsg && (
                  <div className="alert alert-error" role="alert">
                    <strong>Erro: </strong> {errorMsg}
                  </div>
                )}
              </section>
            </div>
          )}
        </div>
      )}

      {/* View 2: "Configurações" Tab (Settings & Preferences) */}
      {activeTab === "settings" && (
        <div className="tab-content" data-testid="view-settings">
          <div className="settings-container">
            {/* Group 1: Automatic Injection */}
            <section className="card settings-group-card">
              <div className="settings-group-header">
                <h2>Injeção Automática</h2>
                <p className="card-desc">
                  Escolha quais elementos da recomendação de build devem ser aplicados automaticamente no cliente local durante o Champion Select.
                </p>
              </div>

              <div className="settings-list">
                <label className="settings-item">
                  <div className="settings-item-text">
                    <span className="settings-item-title">Injetar Runas Automaticamente</span>
                    <span className="settings-item-desc">
                      Aplica a página de runas oficial mais utilizada diretamente no cliente do jogo.
                    </span>
                  </div>
                  <input
                    type="checkbox"
                    className="toggle-switch"
                    aria-label="Injetar Runas Automaticamente"
                    checked={preferences.autoInjectRunes}
                    onChange={(e) => updatePreference("autoInjectRunes", e.target.checked)}
                  />
                </label>

                <label className="settings-item">
                  <div className="settings-item-text">
                    <span className="settings-item-title">Injetar Feitiços de Invocador</span>
                    <span className="settings-item-desc">
                      Aplica os feitiços recomendados para a rota e fila atual (ex: Golpear na Selva, Bola de Neve no ARAM).
                    </span>
                  </div>
                  <input
                    type="checkbox"
                    className="toggle-switch"
                    aria-label="Injetar Feitiços de Invocador"
                    checked={preferences.autoInjectSpells}
                    onChange={(e) => updatePreference("autoInjectSpells", e.target.checked)}
                  />
                </label>

                <label className="settings-item">
                  <div className="settings-item-text">
                    <span className="settings-item-title">Injetar Conjuntos de Itens na Loja</span>
                    <span className="settings-item-desc">
                      Organiza a build recomendada em blocos prontos na loja da sua partida.
                    </span>
                  </div>
                  <input
                    type="checkbox"
                    className="toggle-switch"
                    aria-label="Injetar Conjuntos de Itens na Loja"
                    checked={preferences.autoInjectItems}
                    onChange={(e) => updatePreference("autoInjectItems", e.target.checked)}
                  />
                </label>
              </div>
            </section>

            {/* Group 2: Flash Key Positioning */}
            <section className="card settings-group-card">
              <div className="settings-group-header">
                <h2>Posicionamento do Flash</h2>
                <p className="card-desc">
                  Escolha se prefere posicionar o Flash na tecla D ou F por padrão ao aplicar os feitiços recomendados.
                </p>
              </div>

              <div className="segmented-control-container">
                <div className="segmented-control" role="group" aria-label="Preferência de Tecla do Flash">
                  <button
                    type="button"
                    className={`segment-btn ${preferences.flashKey === "D" ? "active" : ""}`}
                    onClick={() => updatePreference("flashKey", "D")}
                    aria-pressed={preferences.flashKey === "D"}
                  >
                    <span>Flash no D</span>
                  </button>
                  <button
                    type="button"
                    className={`segment-btn ${preferences.flashKey === "F" ? "active" : ""}`}
                    onClick={() => updatePreference("flashKey", "F")}
                    aria-pressed={preferences.flashKey === "F"}
                  >
                    <span>Flash no F</span>
                  </button>
                </div>
                <span className="segmented-caption">
                  Tecla ativa: <strong>Flash no {preferences.flashKey}</strong> (invertendo feitiços se necessário).
                </span>
              </div>
            </section>
          </div>
        </div>
      )}

      {/* Floating Bottom Navigation Dock (iOS / Telegram Style Pill) */}
      <nav className="floating-dock" aria-label="Navegação Principal">
        <button
          type="button"
          className={`dock-btn ${activeTab === "runes" ? "active" : ""}`}
          onClick={() => setActiveTab("runes")}
          aria-current={activeTab === "runes" ? "page" : undefined}
        >
          <svg className="dock-icon" viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <polygon points="12 2 2 7 12 12 22 7 12 2" />
            <polyline points="2 17 12 22 22 17" />
            <polyline points="2 12 12 17 22 12" />
          </svg>
          <span>Runas</span>
        </button>

        <button
          type="button"
          className={`dock-btn ${activeTab === "settings" ? "active" : ""}`}
          onClick={() => setActiveTab("settings")}
          aria-current={activeTab === "settings" ? "page" : undefined}
        >
          <svg className="dock-icon" viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <circle cx="12" cy="12" r="3" />
            <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z" />
          </svg>
          <span>Configurações</span>
        </button>
      </nav>
    </main>
  );
}

export default App;
