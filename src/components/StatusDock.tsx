import { SummonerInfo } from "../types";

interface StatusDockProps {
  summoner: SummonerInfo | null;
}

export function StatusDock({ summoner }: StatusDockProps) {
  return (
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
  );
}
