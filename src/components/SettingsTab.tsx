import { useEffect, useRef, useState } from "react";
import { UserPreferences } from "../types";
import { clampProfileBlur } from "../hooks/usePreferences";

export function isBlurSliderDisabled(wallpaperVisible: boolean): boolean {
  return !wallpaperVisible;
}

interface SettingsTabProps {
  preferences: UserPreferences;
  onUpdatePreference: <K extends keyof UserPreferences>(
    key: K,
    value: UserPreferences[K]
  ) => void;
  profileBlur?: number;
  onProfileBlurChange?: (blurPercent: number) => void;
  wallpaperVisible?: boolean;
  onWallpaperVisibleChange?: (visible: boolean) => void;
  isActive?: boolean;
}

export function SettingsTab({
  preferences,
  onUpdatePreference,
  profileBlur = 10,
  onProfileBlurChange,
  wallpaperVisible = true,
  onWallpaperVisibleChange,
  isActive = true,
}: SettingsTabProps) {
  const [isEditingBlur, setIsEditingBlur] = useState(false);
  const [draftBlur, setDraftBlur] = useState(String(profileBlur));
  const blurInputRef = useRef<HTMLInputElement | null>(null);
  const originalBlurRef = useRef(profileBlur);
  const blurDisabled = isBlurSliderDisabled(wallpaperVisible);

  useEffect(() => {
    if (!isEditingBlur) {
      setDraftBlur(String(profileBlur));
    }
  }, [profileBlur, isEditingBlur]);

  useEffect(() => {
    if (blurDisabled && isEditingBlur) {
      setIsEditingBlur(false);
      setDraftBlur(String(profileBlur));
    }
  }, [blurDisabled, isEditingBlur, profileBlur]);

  useEffect(() => {
    if (isEditingBlur) {
      blurInputRef.current?.focus();
      blurInputRef.current?.select();
    }
  }, [isEditingBlur]);

  const handleProfileBlurChange = (nextBlur: number) => {
    if (blurDisabled) return;
    onProfileBlurChange?.(nextBlur);
  };

  const startEditingBlur = () => {
    if (blurDisabled) return;
    originalBlurRef.current = profileBlur;
    setDraftBlur(String(profileBlur));
    setIsEditingBlur(true);
  };

  const commitBlurDraft = () => {
    if (blurDisabled) {
      setIsEditingBlur(false);
      return;
    }
    const trimmed = draftBlur.trim();
    if (trimmed === "") {
      handleProfileBlurChange(0);
    } else {
      const parsed = Number(trimmed);
      if (Number.isFinite(parsed)) {
        handleProfileBlurChange(clampProfileBlur(parsed));
      } else {
        handleProfileBlurChange(originalBlurRef.current);
      }
    }
    setIsEditingBlur(false);
  };

  const cancelBlurEdit = () => {
    if (!blurDisabled) {
      handleProfileBlurChange(originalBlurRef.current);
    }
    setDraftBlur(String(originalBlurRef.current));
    setIsEditingBlur(false);
  };

  return (
    <div
      className={`tab-content ${isActive ? "active-tab" : "inactive-tab"}`}
      data-testid="view-settings"
      aria-hidden={!isActive}
    >
      <div className="settings-container">
        {/* Group 1: Perfil (First Tab) */}
        <div className="settings-section-group" data-testid="settings-group-profile">
          <h2 className="settings-section-title">Perfil</h2>

          <section className="card settings-group-card" data-testid="settings-subgroup-appearance">
            <div className="settings-group-header">
              <h3>Aparência do Perfil</h3>
              <p className="card-desc">
                Ajuste a intensidade do desfoque e a exibição do plano de fundo da aba Perfil.
              </p>
            </div>

            <div className="settings-list">
              <div
                className={`settings-item settings-slider-item ${blurDisabled ? "is-disabled" : ""}`}
                aria-disabled={blurDisabled}
              >
                <div className="settings-item-text">
                  <span className="settings-item-title">Nível de Desfoque (Blur)</span>
                  <span className="settings-item-desc">
                    {blurDisabled
                      ? "Ative a opção Fundo Visível para ajustar o desfoque."
                      : "Deslize ou clique no número para digitar entre 0% e 100%."}
                  </span>
                </div>
                <div className="settings-slider-control">
                  <input
                    type="range"
                    min={0}
                    max={100}
                    step={1}
                    value={profileBlur}
                    disabled={blurDisabled}
                    onChange={(e) => handleProfileBlurChange(Number(e.target.value))}
                    className="blur-range-slider"
                    title={
                      blurDisabled
                        ? "Ative a opção Fundo Visível para ajustar o desfoque"
                        : undefined
                    }
                    aria-label="Nível de Desfoque da Interface do Perfil"
                    data-testid="profile-blur-slider"
                  />
                  {isEditingBlur && !blurDisabled ? (
                    <div className="slider-value-edit-wrap" data-testid="profile-blur-edit">
                      <input
                        ref={blurInputRef}
                        type="number"
                        min={0}
                        max={100}
                        step={1}
                        value={draftBlur}
                        onChange={(e) => {
                          const raw = e.target.value;
                          setDraftBlur(raw);
                          if (raw.trim() !== "") {
                            const parsed = Number(raw);
                            if (Number.isFinite(parsed)) {
                              handleProfileBlurChange(clampProfileBlur(parsed));
                            }
                          }
                        }}
                        onBlur={commitBlurDraft}
                        onKeyDown={(e) => {
                          if (e.key === "Enter") {
                            commitBlurDraft();
                          } else if (e.key === "Escape") {
                            cancelBlurEdit();
                          }
                        }}
                        className="slider-value-input"
                        aria-label="Digitar nível de desfoque em porcentagem"
                        data-testid="profile-blur-input"
                      />
                      <span className="slider-value-percent">%</span>
                    </div>
                  ) : (
                    <button
                      type="button"
                      className="slider-value-badge slider-value-btn"
                      onClick={startEditingBlur}
                      disabled={blurDisabled}
                      title={
                        blurDisabled
                          ? "Ative a opção Fundo Visível para ajustar o desfoque"
                          : "Clique para digitar o valor"
                      }
                      aria-label="Editar nível de desfoque em porcentagem"
                      data-testid="profile-blur-value"
                    >
                      {profileBlur}%
                    </button>
                  )}
                </div>
              </div>

              <label className="settings-item">
                <div className="settings-item-text">
                  <span className="settings-item-title">Fundo Visível</span>
                  <span className="settings-item-desc">
                    Exibe o papel de parede do seu perfil em tela inteira na aba Perfil.
                  </span>
                </div>
                <input
                  type="checkbox"
                  className="toggle-switch"
                  aria-label="Fundo Visível"
                  data-testid="wallpaper-visible-toggle"
                  checked={wallpaperVisible}
                  onChange={(e) => onWallpaperVisibleChange?.(e.target.checked)}
                />
              </label>
            </div>
          </section>
        </div>

        {/* Group 2: Runas (Second Tab) */}
        <div className="settings-section-group" data-testid="settings-group-runes">
          <h2 className="settings-section-title">Runas</h2>

          {/* Subgroup 2.1: Posicionamento do Flash */}
          <section className="card settings-group-card" data-testid="settings-subgroup-flash">
            <div className="settings-group-header">
              <h3>Posicionamento do Flash</h3>
              <p className="card-desc">
                Tecla designada para o feitiço Flash ao aplicar as recomendações.
              </p>
            </div>

            <div className="segmented-control-container">
              <div className="segmented-control" role="group" aria-label="Preferência de Tecla do Flash">
                <button
                  type="button"
                  className={`segment-btn ${preferences.flashKey === "D" ? "active" : ""}`}
                  onClick={() => onUpdatePreference("flashKey", "D")}
                  aria-pressed={preferences.flashKey === "D"}
                  aria-label="D"
                >
                  <span>D</span>
                </button>
                <button
                  type="button"
                  className={`segment-btn ${preferences.flashKey === "F" ? "active" : ""}`}
                  onClick={() => onUpdatePreference("flashKey", "F")}
                  aria-pressed={preferences.flashKey === "F"}
                  aria-label="F"
                >
                  <span>F</span>
                </button>
              </div>
            </div>
          </section>

          {/* Subgroup 2.2: Injeção Automática */}
          <section className="card settings-group-card" data-testid="settings-subgroup-auto-inject">
            <div className="settings-group-header">
              <h3>Injeção Automática</h3>
              <p className="card-desc">
                Sincronização e aplicação automática de dados no cliente durante a seleção de campeões.
              </p>
            </div>

            <div className="settings-list">
              <label className="settings-item">
                <div className="settings-item-text">
                  <span className="settings-item-title">Injetar Runas Automaticamente</span>
                  <span className="settings-item-desc">
                    Aplica a página de runas recomendada diretamente na conta do jogador.
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
                    Configura os feitiços de invocador otimizados para a rota e fila da partida.
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
                    Gera blocos de itens recomendados na loja in-game da partida.
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
        </div>
      </div>
    </div>
  );
}
