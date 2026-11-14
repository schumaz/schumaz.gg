import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
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
  const [showDisablePatchProtectionModal, setShowDisablePatchProtectionModal] = useState(false);
  const blurInputRef = useRef<HTMLInputElement | null>(null);
  const originalBlurRef = useRef(profileBlur);
  const blurDisabled = isBlurSliderDisabled(wallpaperVisible);
  const patch48hProtection = preferences.patch48hProtection ?? true;

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

  useEffect(() => {
    if (!isActive || !patch48hProtection) {
      setShowDisablePatchProtectionModal(false);
    }
  }, [isActive, patch48hProtection]);

  useEffect(() => {
    if (!showDisablePatchProtectionModal) return;
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        setShowDisablePatchProtectionModal(false);
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [showDisablePatchProtectionModal]);

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

        {/* Group 2: Runas (Second Tab — single unified island) */}
        <div className="settings-section-group" data-testid="settings-group-runes">
          <h2 className="settings-section-title">Runas</h2>

          <section
            className="card settings-group-card settings-runes-card"
            data-testid="settings-subgroup-runes"
          >
            {/* Subsection 2.1: Posicionamento do Flash */}
            <div className="settings-subsection" data-testid="settings-subgroup-flash">
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
            </div>

            <div className="settings-card-divider" />

            {/* Subsection 2.2: Injeção Automática */}
            <div className="settings-subsection" data-testid="settings-subgroup-auto-inject">
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
            </div>

            <div className="settings-card-divider" />

            {/* Subsection 2.3: Proteção de Patch (48h) */}
            <div className="settings-subsection" data-testid="settings-subgroup-patch-protection">
              <div className="settings-group-header">
                <h3>Proteção de Patch (48h)</h3>
                <p className="card-desc">
                  Controle de integridade na coleta automática de dados ao lançar um novo patch.
                </p>
              </div>

              <div className="settings-list">
                <label className="settings-item">
                  <div className="settings-item-text">
                    <span className="settings-item-title">Proteção de 48h para Novos Patches</span>
                    <span className="settings-item-desc">
                      Mantém o patch anterior consolidado nas primeiras 48 horas após um novo patch quando o volume de dados ainda é baixo.
                    </span>
                  </div>
                  <input
                    type="checkbox"
                    className="toggle-switch"
                    aria-label="Proteção de 48h para Novos Patches"
                    data-testid="patch-48h-protection-toggle"
                    checked={patch48hProtection}
                    onChange={(e) => {
                      if (!e.target.checked && patch48hProtection) {
                        setShowDisablePatchProtectionModal(true);
                        return;
                      }
                      onUpdatePreference("patch48hProtection", e.target.checked);
                    }}
                  />
                </label>
              </div>
            </div>
          </section>
        </div>
      </div>

      {showDisablePatchProtectionModal &&
        createPortal(
          <div
            className="settings-modal-backdrop"
            data-testid="patch-48h-disable-modal-backdrop"
            onClick={(e) => {
              if (e.target === e.currentTarget) {
                setShowDisablePatchProtectionModal(false);
              }
            }}
          >
            <div
              className="card settings-modal-dialog"
              role="dialog"
              aria-modal="true"
              aria-labelledby="patch-48h-modal-title"
              aria-describedby="patch-48h-modal-desc"
              data-testid="patch-48h-protection-warning"
            >
              <div className="settings-modal-header">
                <span className="settings-warning-icon settings-modal-icon" aria-hidden="true">
                  !
                </span>
                <h3 id="patch-48h-modal-title" className="settings-modal-title">
                  Desativar Proteção de 48h?
                </h3>
              </div>

              <p id="patch-48h-modal-desc" className="settings-modal-desc">
                <strong>NÃO é recomendado</strong> desativar essa opção, pois desfavorece o uso do app devido ao baixo volume de dados no início do patch.
              </p>

              <div className="settings-modal-actions">
                <button
                  type="button"
                  className="settings-modal-btn settings-modal-btn-revert"
                  data-testid="patch-48h-revert-btn"
                  onClick={() => setShowDisablePatchProtectionModal(false)}
                >
                  Reverter
                </button>
                <button
                  type="button"
                  className="settings-modal-btn settings-modal-btn-accept"
                  data-testid="patch-48h-accept-btn"
                  onClick={() => {
                    setShowDisablePatchProtectionModal(false);
                    onUpdatePreference("patch48hProtection", false);
                  }}
                >
                  Aceitar desativação
                </button>
              </div>
            </div>
          </div>,
          document.body
        )}
    </div>
  );
}
