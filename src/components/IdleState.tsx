import { ReactNode } from "react";
import { SummonerInfo } from "../types";

interface IdleIslandProps {
  badge?: string;
  title: string;
  description: string;
  icon: ReactNode;
  errorMsg?: string | null;
  testId?: string;
}

export function IdleIsland({
  badge,
  title,
  description,
  icon,
  errorMsg,
  testId,
}: IdleIslandProps) {
  return (
    <div className="idle-state-container" data-testid={testId}>
      <section className="card idle-card">
        <div className="idle-icon-box" aria-hidden="true">
          {icon}
        </div>
        {badge && <span className="idle-brand-pill">{badge}</span>}
        <h2 className="idle-title">{title}</h2>
        <p className="card-desc idle-desc">{description}</p>

        {errorMsg && (
          <div className="alert alert-error idle-alert" role="alert">
            <strong>Erro: </strong> {errorMsg}
          </div>
        )}
      </section>
    </div>
  );
}

interface IdleStateProps {
  summoner: SummonerInfo | null;
  errorMsg: string | null;
}

export function IdleState({ summoner: _summoner, errorMsg }: IdleStateProps) {
  return (
    <IdleIsland
      badge="schumaz.gg · Runas"
      title="Aguardando Seleção de Campeões"
      description="Selecione ou confirme um campeão no LoL para carregar runas, feitiços e itens automaticamente."
      errorMsg={errorMsg}
      icon={
        <svg
          viewBox="0 0 24 24"
          width="42"
          height="42"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.75"
          strokeLinecap="round"
          strokeLinejoin="round"
        >
          <polygon points="12 2 2 7 12 12 22 7 12 2" />
          <polyline points="2 17 12 22 22 17" />
          <polyline points="2 12 12 17 22 12" />
        </svg>
      }
    />
  );
}
