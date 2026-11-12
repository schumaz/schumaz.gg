import { SummonerInfo } from "../types";

interface IdleStateProps {
  summoner: SummonerInfo | null;
  errorMsg: string | null;
}

export function IdleState({ summoner: _summoner, errorMsg }: IdleStateProps) {
  return (
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
          Selecione ou confirme um campeão no League of Legends para carregar runas, feitiços e itens automaticamente.
        </p>

        {errorMsg && (
          <div className="alert alert-error" role="alert">
            <strong>Erro: </strong> {errorMsg}
          </div>
        )}
      </section>
    </div>
  );
}
