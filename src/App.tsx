import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./App.css";

export type ActiveTab = "runes" | "settings";

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
}

function App() {
  const [activeTab, setActiveTab] = useState<ActiveTab>("runes");
  const [summoner, setSummoner] = useState<SummonerInfo | null>(null);
  const [champSelect, setChampSelect] = useState<ChampSelectedPayload | null>(null);
  const [injectionReport, setInjectionReport] = useState<InjectionReport | null>(null);
  const [loading, setLoading] = useState(false);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  useEffect(() => {
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
    let unlistenInjected: (() => void) | undefined;

    const setupListeners = async () => {
      try {
        unlistenConnected = await listen<SummonerInfo>("lcu-connected", (event) => {
          setSummoner(event.payload);
          setErrorMsg(null);
        });

        unlistenDisconnected = await listen("lcu-disconnected", () => {
          setSummoner(null);
          setChampSelect(null);
        });

        unlistenChampSelected = await listen<ChampSelectedPayload>("champ-selected", (event) => {
          setChampSelect(event.payload);
        });

        unlistenChampSelectEnded = await listen("champ-select-ended", () => {
          setChampSelect(null);
        });

        unlistenInjected = await listen<InjectionReport>("build-injected", (event) => {
          setInjectionReport(event.payload);
        });
      } catch (e) {
        console.warn("Tauri event listener setup skipped or running in browser/test mode:", e);
      }
    };

    setupListeners();

    return () => {
      if (unlistenConnected) unlistenConnected();
      if (unlistenDisconnected) unlistenDisconnected();
      if (unlistenChampSelected) unlistenChampSelected();
      if (unlistenChampSelectEnded) unlistenChampSelectEnded();
      if (unlistenInjected) unlistenInjected();
    };
  }, []);

  const handleManualInject = async () => {
    setLoading(true);
    setErrorMsg(null);
    try {
      const report = await invoke<InjectionReport>("test_inject_runes");
      setInjectionReport(report);
    } catch (err: unknown) {
      setErrorMsg(typeof err === "string" ? err : JSON.stringify(err));
    } finally {
      setLoading(false);
    }
  };

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
          <section className="card champ-card">
            <h2>Seleção de Campeões</h2>
            {champSelect ? (
              <div className="champ-info">
                <p>
                  Campeão Selecionado: <strong>ID {champSelect.champion_id}</strong>
                </p>
                <p>
                  Modo de Jogo: <strong>{champSelect.queue_mode}</strong> (Fila {champSelect.queue_id})
                </p>
                <p>
                  Status:{" "}
                  <span className={champSelect.locked_in ? "tag locked" : "tag hovering"}>
                    {champSelect.locked_in ? "Confirmado (Lock-in)" : "Selecionando (Hover)"}
                  </span>
                </p>
              </div>
            ) : (
              <p className="text-muted">Nenhuma seleção de campeões ativa no momento.</p>
            )}
          </section>

          <section className="card injection-card">
            <h2>Teste de Injeção no Cliente</h2>
            <p className="card-desc">
              Injeta uma página de runas e conjuntos de itens recomendados com a nomenclatura obrigatória{" "}
              <code>"%Champname% | schumaz.gg"</code> e blocos personalizados na loja.
            </p>

            <button
              className="btn-primary"
              onClick={handleManualInject}
              disabled={loading || !summoner}
            >
              {loading ? "Injetando..." : "Testar Injeção de Runas no LoL"}
            </button>

            {!summoner && (
              <p className="hint-text">
                * Abra o cliente do LoL para habilitar a injeção direta no cliente.
              </p>
            )}

            {errorMsg && (
              <div className="alert alert-error" role="alert">
                <strong>Erro: </strong> {errorMsg}
              </div>
            )}

            {injectionReport && (
              <div className="report-box" data-testid="injection-report">
                <h3>Relatório de Injeção: {injectionReport.champion_name} | schumaz.gg</h3>
                <div className="report-items">
                  <span className={`badge ${injectionReport.runes_injected ? "badge-success" : "badge-fail"}`}>
                    Runas: {injectionReport.runes_injected ? "Injetadas com Sucesso" : "Falha"}
                  </span>
                  <span className={`badge ${injectionReport.items_injected ? "badge-success" : "badge-fail"}`}>
                    Itens na Loja: {injectionReport.items_injected ? "Injetados (4 Blocos)" : "Falha"}
                  </span>
                  <span className={`badge ${injectionReport.spells_injected ? "badge-success" : "badge-neutral"}`}>
                    Feitiços: {injectionReport.spells_injected ? "Injetados" : "Indisponíveis fora do Champ Select"}
                  </span>
                </div>
                <p className="report-msg">
                  <strong>Detalhes:</strong> {injectionReport.message}
                </p>
              </div>
            )}
          </section>

          <section className="card item-blocks-preview">
            <h2>Estrutura dos Conjuntos de Itens na Loja</h2>
            <div className="grid-blocks">
              <div className="block-item">
                <h4>1. Itens Iniciais</h4>
                <p>Primeiro item mais feito + poção</p>
              </div>
              <div className="block-item">
                <h4>2. Primeiros 3 Itens</h4>
                <p>Os 3 itens principais com maior win rate</p>
              </div>
              <div className="block-item">
                <h4>3. Build Final</h4>
                <p>Build final completa de late-game</p>
              </div>
              <div className="block-item">
                <h4>4. 3 Botas Mais Feitas</h4>
                <p>As 3 opções de botas mais populares</p>
              </div>
            </div>
          </section>
        </div>
      )}

      {/* View 2: "Configurações" Tab (Settings & Preferences) */}
      {activeTab === "settings" && (
        <div className="tab-content" data-testid="view-settings">
          <section className="card">
            <h2>Ajustes & Preferências</h2>
            <p className="card-desc">
              Personalize o comportamento da injeção automática, atalhos de feitiços e automações do cliente.
            </p>
            <p className="text-muted">
              Central de controles em desenvolvimento (Módulos 5.4, 6 e 7).
            </p>
          </section>
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
