import { SummonerInfo } from "../types";
import { LoadingDots } from "./LoadingIndicators";

interface StatusDockProps {
  summoner: SummonerInfo | null;
}

export function StatusDock({ summoner }: StatusDockProps) {
  return (
    <header className="header" data-tauri-drag-region>
      <h1 className="title sr-only" data-tauri-drag-region>schumaz.gg</h1>

      <div className="status-badge-container" data-tauri-drag-region>
        {summoner ? (
          <div className="status-badge connected" data-testid="status-connected">
            <span className="dot dot-green"></span>
            <span>Conectado ao cliente do LoL</span>
          </div>
        ) : (
          <div className="status-badge disconnected" data-testid="status-disconnected">
            <span className="dot dot-orange dot-pulsing"></span>
            <span>Aguardando cliente do LoL</span>
            <LoadingDots variant="warning" size="sm" testId="status-waiting-dots" />
          </div>
        )}
      </div>
    </header>
  );
}
