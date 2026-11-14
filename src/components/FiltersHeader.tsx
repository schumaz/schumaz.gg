import { ChampionBuild, ChampSelectedPayload } from "../types";
import { FilterDropdown, FilterDropdownOption } from "./FilterDropdown";
import { RegionFlag } from "./RegionFlag";

interface FiltersHeaderProps {
  build: ChampionBuild;
  champSelect: ChampSelectedPayload | null;
  loading: boolean;
  onFilterChange: (
    filterType: "position" | "tier" | "region" | "patch",
    value: string
  ) => void;
}

const CDRAGON_MINI_CRESTS_BASE =
  "https://raw.communitydragon.org/latest/plugins/rcp-fe-lol-static-assets/global/default/images/ranked-mini-crests";
const CDRAGON_POSITION_SVG_BASE =
  "https://raw.communitydragon.org/latest/plugins/rcp-fe-lol-static-assets/global/default/svg";

function buildIconImg(src: string, className = "filter-option-icon") {
  return (
    <img
      src={src}
      alt=""
      aria-hidden="true"
      className={className}
      onError={(e) => {
        (e.currentTarget as HTMLImageElement).style.display = "none";
      }}
    />
  );
}

export const TIER_OPTIONS: FilterDropdownOption[] = [
  {
    value: "all",
    label: "Todos os Elos",
    icon: buildIconImg(`${CDRAGON_MINI_CRESTS_BASE}/unranked.svg`),
  },
  {
    value: "challenger",
    label: "Desafiante",
    icon: buildIconImg(`${CDRAGON_MINI_CRESTS_BASE}/challenger.svg`),
  },
  {
    value: "grandmaster",
    label: "Grão-Mestre",
    icon: buildIconImg(`${CDRAGON_MINI_CRESTS_BASE}/grandmaster.svg`),
  },
  {
    value: "master_plus",
    label: "Mestre +",
    icon: buildIconImg(`${CDRAGON_MINI_CRESTS_BASE}/master.svg`),
  },
  {
    value: "diamond_plus",
    label: "Diamante +",
    icon: buildIconImg(`${CDRAGON_MINI_CRESTS_BASE}/diamond.svg`),
  },
  {
    value: "emerald_plus",
    label: "Esmeralda +",
    icon: buildIconImg(`${CDRAGON_MINI_CRESTS_BASE}/emerald.svg`),
  },
  {
    value: "platinum_plus",
    label: "Platina +",
    icon: buildIconImg(`${CDRAGON_MINI_CRESTS_BASE}/platinum.svg`),
  },
  {
    value: "gold_plus",
    label: "Ouro +",
    icon: buildIconImg(`${CDRAGON_MINI_CRESTS_BASE}/gold.svg`),
  },
  {
    value: "silver",
    label: "Prata",
    icon: buildIconImg(`${CDRAGON_MINI_CRESTS_BASE}/silver.svg`),
  },
  {
    value: "bronze",
    label: "Bronze",
    icon: buildIconImg(`${CDRAGON_MINI_CRESTS_BASE}/bronze.svg`),
  },
  {
    value: "iron",
    label: "Ferro",
    icon: buildIconImg(`${CDRAGON_MINI_CRESTS_BASE}/iron.svg`),
  },
];

export const POSITION_OPTIONS: FilterDropdownOption[] = [
  {
    value: "top",
    label: "Topo",
    icon: buildIconImg(`${CDRAGON_POSITION_SVG_BASE}/position-top-light.svg`),
  },
  {
    value: "jungle",
    label: "Selva",
    icon: buildIconImg(`${CDRAGON_POSITION_SVG_BASE}/position-jungle-light.svg`),
  },
  {
    value: "mid",
    label: "Meio",
    icon: buildIconImg(`${CDRAGON_POSITION_SVG_BASE}/position-middle-light.svg`),
  },
  {
    value: "adc",
    label: "Atirador (Bot)",
    icon: buildIconImg(`${CDRAGON_POSITION_SVG_BASE}/position-bottom-light.svg`),
  },
  {
    value: "support",
    label: "Suporte",
    icon: buildIconImg(`${CDRAGON_POSITION_SVG_BASE}/position-utility-light.svg`),
  },
];

export const REGION_OPTIONS: FilterDropdownOption[] = [
  {
    value: "global",
    label: "Global",
    title: "Mundial (Global)",
    icon: <RegionFlag region="global" />,
  },
  {
    value: "br",
    label: "BR",
    title: "Brasil (BR)",
    icon: <RegionFlag region="br" />,
  },
  {
    value: "kr",
    label: "KR",
    title: "Coreia (KR)",
    icon: <RegionFlag region="kr" />,
  },
  {
    value: "na",
    label: "NA",
    title: "América do Norte (NA)",
    icon: <RegionFlag region="na" />,
  },
  {
    value: "euw",
    label: "EUW",
    title: "Europa Ocidental (EUW)",
    icon: <RegionFlag region="euw" />,
  },
  {
    value: "eune",
    label: "EUNE",
    title: "Europa Nórdica e Oriental (EUNE)",
    icon: <RegionFlag region="eune" />,
  },
  {
    value: "lan",
    label: "LAN",
    title: "América Latina Norte (LAN)",
    icon: <RegionFlag region="lan" />,
  },
  {
    value: "las",
    label: "LAS",
    title: "América Latina Sul (LAS)",
    icon: <RegionFlag region="las" />,
  },
  {
    value: "oce",
    label: "OCE",
    title: "Oceania (OCE)",
    icon: <RegionFlag region="oce" />,
  },
  {
    value: "tr",
    label: "TR",
    title: "Turquia (TR)",
    icon: <RegionFlag region="tr" />,
  },
  {
    value: "ru",
    label: "RU",
    title: "Rússia (RU)",
    icon: <RegionFlag region="ru" />,
  },
  {
    value: "jp",
    label: "JP",
    title: "Japão (JP)",
    icon: <RegionFlag region="jp" />,
  },
  {
    value: "me",
    label: "ME",
    title: "Oriente Médio (ME)",
    icon: <RegionFlag region="me" />,
  },
  {
    value: "sg",
    label: "SG",
    title: "Singapura (SG)",
    icon: <RegionFlag region="sg" />,
  },
  {
    value: "tw",
    label: "TW",
    title: "Taiwan (TW)",
    icon: <RegionFlag region="tw" />,
  },
  {
    value: "vn",
    label: "VN",
    title: "Vietnã (VN)",
    icon: <RegionFlag region="vn" />,
  },
];

export function FiltersHeader({
  build,
  champSelect,
  loading,
  onFilterChange,
}: FiltersHeaderProps) {
  const isAram =
    champSelect?.queue_mode === "Aram" ||
    champSelect?.queue_mode === "AramMayhem" ||
    champSelect?.queue_id === 450 ||
    champSelect?.queue_id === 2400 ||
    build.supports_runes === false;

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
            <div className="dd-icon-frame champ-avatar-frame">
              <img
                src={build.champion_icon_url}
                alt={build.champion_name}
                className="champ-avatar"
                onError={(e) => {
                  (e.target as HTMLElement).style.display = "none";
                }}
              />
            </div>
          ) : (
            <div className="champ-avatar-placeholder">
              {build.champion_name.slice(0, 2).toUpperCase()}
            </div>
          )}

          <h2 className="champ-name" title={build.champion_name}>
            {build.champion_name}
          </h2>
        </div>

        <div className="champ-hero-right">
          <span className={`tag ${champSelect?.locked_in ? "locked" : "hovering"}`}>
            {champSelect?.locked_in ? "Confirmado (Lock-in)" : "Selecionando (Hover)"}
          </span>

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
            {(build.pick_rate ?? 0) > 0 && (
              <span className="badge badge-neutral badge-stat">
                {(build.pick_rate ?? 0).toFixed(1)}% Pick Rate
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

      {/* Filters: Elo Base, Rota (SR only), Servidor, Patch */}
      <div className="champ-filters-bar" data-testid="champ-filters-bar">
        <div className="filter-group">
          <label id="filter-tier-label" htmlFor="filter-tier" className="filter-label">
            Elo:
          </label>
          <FilterDropdown
            id="filter-tier"
            labelId="filter-tier-label"
            testId="filter-tier"
            value={build.active_tier || "emerald_plus"}
            options={TIER_OPTIONS}
            disabled={loading}
            onChange={(val) => onFilterChange("tier", val)}
          />
        </div>

        {!isAram && (
          <div className="filter-group">
            <label id="filter-position-label" htmlFor="filter-position" className="filter-label">
              Rota:
            </label>
            <FilterDropdown
              id="filter-position"
              labelId="filter-position-label"
              testId="filter-position"
              value={build.active_position || "mid"}
              options={POSITION_OPTIONS}
              disabled={loading}
              onChange={(val) => onFilterChange("position", val)}
            />
          </div>
        )}

        <div className="filter-group">
          <label id="filter-region-label" htmlFor="filter-region" className="filter-label">
            Servidor:
          </label>
          <FilterDropdown
            id="filter-region"
            labelId="filter-region-label"
            testId="filter-region"
            value={build.active_region || "global"}
            options={REGION_OPTIONS}
            disabled={loading}
            onChange={(val) => onFilterChange("region", val)}
          />
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
