import { UserPreferences } from "../types";

interface SettingsTabProps {
  preferences: UserPreferences;
  onUpdatePreference: <K extends keyof UserPreferences>(
    key: K,
    value: UserPreferences[K]
  ) => void;
}

export function SettingsTab({ preferences, onUpdatePreference }: SettingsTabProps) {
  return (
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
                onChange={(e) => onUpdatePreference("autoInjectRunes", e.target.checked)}
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
                onChange={(e) => onUpdatePreference("autoInjectSpells", e.target.checked)}
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
                onChange={(e) => onUpdatePreference("autoInjectItems", e.target.checked)}
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
                onClick={() => onUpdatePreference("flashKey", "D")}
                aria-pressed={preferences.flashKey === "D"}
              >
                <span>Flash no D</span>
              </button>
              <button
                type="button"
                className={`segment-btn ${preferences.flashKey === "F" ? "active" : ""}`}
                onClick={() => onUpdatePreference("flashKey", "F")}
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
  );
}
