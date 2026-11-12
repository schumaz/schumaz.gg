import { ChampionBuild, ChampSelectedPayload } from "../types";

interface FiltersHeaderProps {
  build: ChampionBuild;
  champSelect: ChampSelectedPayload | null;
  loading: boolean;
  onFilterChange: (
    filterType: "position" | "tier" | "region" | "patch",
    value: string
  ) => void;
}

export function FiltersHeader({
  build,
  champSelect,
  loading,
  onFilterChange,
}: FiltersHeaderProps) {
  const patchOptions =
    build?.available_patches && build.available_patches.length > 0
      ? build.available_patches
      : build?.active_patch
      ? [build.active_patch]
      : ["16.22", "16.21"];

  return (
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
                {champSelect?.queue_mode === "AramMayhem"
                  ? "ARAM Mayhem (Howling Abyss)"
                  : champSelect?.queue_mode === "Aram"
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
            onChange={(e) => onFilterChange("tier", e.target.value)}
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
            onChange={(e) => onFilterChange("position", e.target.value)}
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
            onChange={(e) => onFilterChange("region", e.target.value)}
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
            onChange={(e) => onFilterChange("patch", e.target.value)}
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
  );
}
