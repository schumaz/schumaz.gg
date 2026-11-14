import { ActiveTab } from "../types";

interface NavigationDockProps {
  activeTab: ActiveTab;
  onTabChange: (tab: ActiveTab) => void;
  onRankedClick?: () => void;
  onAramClick?: () => void;
}

const ARAM_DOCK_ICON_URL =
  "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/content/src/leagueclient/gamemodeassets/aram/img/game-select-icon-active.png";

export function NavigationDock({
  activeTab,
  onTabChange,
  onRankedClick,
  onAramClick,
}: NavigationDockProps) {
  return (
    <nav className="floating-dock" aria-label="Navegação Principal">
      <button
        type="button"
        className={`dock-btn ${activeTab === "profile" ? "active" : ""}`}
        onClick={() => onTabChange("profile")}
        aria-current={activeTab === "profile" ? "page" : undefined}
      >
        <svg
          className="dock-icon"
          viewBox="0 0 24 24"
          width="18"
          height="18"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
        >
          <path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2" />
          <circle cx="12" cy="7" r="4" />
        </svg>
        <span>Perfil</span>
      </button>

      <button
        type="button"
        className={`dock-btn ${activeTab === "runes" ? "active" : ""}`}
        onClick={() => onTabChange("runes")}
        aria-current={activeTab === "runes" ? "page" : undefined}
      >
        <svg
          className="dock-icon"
          viewBox="0 0 24 24"
          width="18"
          height="18"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
        >
          <polygon points="12 2 2 7 12 12 22 7 12 2" />
          <polyline points="2 17 12 22 22 17" />
          <polyline points="2 12 12 17 22 12" />
        </svg>
        <span>Runas</span>
      </button>

      <button
        type="button"
        className={`dock-btn ${activeTab === "ranked" ? "active" : ""}`}
        onClick={() => {
          if (onRankedClick) {
            onRankedClick();
          } else {
            onTabChange("ranked");
          }
        }}
        aria-current={activeTab === "ranked" ? "page" : undefined}
      >
        <svg
          className="dock-icon"
          viewBox="0 0 24 24"
          width="18"
          height="18"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
        >
          <path d="M6 9H4.5a2.5 2.5 0 0 1 0-5H6" />
          <path d="M18 9h1.5a2.5 2.5 0 0 0 0-5H18" />
          <path d="M4 22h16" />
          <path d="M10 14.66V17c0 .55-.47.98-.97 1.21C7.85 18.75 7 20.24 7 22" />
          <path d="M14 14.66V17c0 .55.47.98.97 1.21C16.15 18.75 17 20.24 17 22" />
          <path d="M18 2H6v7a6 6 0 0 0 12 0V2Z" />
        </svg>
        <span>Ranqueada</span>
      </button>

      <button
        type="button"
        className={`dock-btn ${activeTab === "aram" ? "active" : ""}`}
        onClick={() => {
          if (onAramClick) {
            onAramClick();
          } else {
            onTabChange("aram");
          }
        }}
        aria-current={activeTab === "aram" ? "page" : undefined}
      >
        <img
          src={ARAM_DOCK_ICON_URL}
          alt=""
          className="dock-icon dock-icon-img"
          width={18}
          height={18}
        />
        <span>Aram</span>
      </button>

      <button
        type="button"
        className={`dock-btn ${activeTab === "settings" ? "active" : ""}`}
        onClick={() => onTabChange("settings")}
        aria-current={activeTab === "settings" ? "page" : undefined}
      >
        <svg
          className="dock-icon"
          viewBox="0 0 24 24"
          width="18"
          height="18"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
        >
          <circle cx="12" cy="12" r="3" />
          <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z" />
        </svg>
        <span>Configurações</span>
      </button>
    </nav>
  );
}
