import { SpellMetadata } from "../types";

interface SpellsBarProps {
  orderedSpells: SpellMetadata[];
  spellChoices?: SpellMetadata[];
  selectedSecondarySpellId: number | null;
  onInvertSpells: () => void;
  onSelectSecondarySpell: (spellId: number) => void;
}

export function SpellsBar({
  orderedSpells,
  spellChoices,
  selectedSecondarySpellId,
  onInvertSpells,
  onSelectSecondarySpell,
}: SpellsBarProps) {
  return (
    <section className="card spells-card" data-testid="spells-card">
      <div className="spells-card-header">
        <h2>Feitiços de Invocador</h2>
        <button
          type="button"
          className="invert-spells-btn"
          data-testid="invert-spells-btn"
          onClick={onInvertSpells}
          title="Inverter a posição dos feitiços entre as teclas D e F"
        >
          <span className="invert-spells-icon">⇄</span> Inverter Feitiços
        </button>
      </div>
      <p className="card-desc">Feitiços recomendados adaptados para a fila atual.</p>

      <div className="spells-list">
        {orderedSpells && orderedSpells.length >= 2 ? (
          orderedSpells.map((spell, index) => (
            <div key={`spell-${index}-${spell.id}`} className="spell-item">
              <span className="spell-key-indicator">{index === 0 ? "D" : "F"}</span>
              {spell.icon_url && (
                <img src={spell.icon_url} alt={spell.name} className="spell-icon" />
              )}
              <div className="spell-info">
                <span className="spell-name">{spell.name}</span>
              </div>
            </div>
          ))
        ) : (
          <div className="spells-fallback">
            <div className="spell-item">
              <span className="spell-key-indicator">D</span>
              <span className="spell-name">Feitiço 1</span>
            </div>
            <div className="spell-item">
              <span className="spell-key-indicator">F</span>
              <span className="spell-name">Feitiço 2</span>
            </div>
          </div>
        )}
      </div>

      {spellChoices && spellChoices.length > 0 && (
        <div className="secondary-spell-selector" data-testid="secondary-spell-selector">
          <h4 className="secondary-spell-title">Escolha de Segundo Feitiço</h4>
          <div className="secondary-spell-options">
            {spellChoices.map((choice, idx) => {
              const currentSecondaryId =
                selectedSecondarySpellId ?? spellChoices[0]?.id;
              const isSelected = currentSecondaryId === choice.id;
              return (
                <button
                  key={`sec-spell-${choice.id}-${idx}`}
                  type="button"
                  className={`secondary-spell-btn ${isSelected ? "active" : ""}`}
                  data-testid={`sec-spell-btn-${choice.id}`}
                  onClick={() => onSelectSecondarySpell(choice.id)}
                  title={idx === 0 ? "Mais Usado" : "Segundo Mais Usado"}
                >
                  {choice.icon_url && (
                    <img
                      src={choice.icon_url}
                      alt={choice.name}
                      className="secondary-spell-icon"
                    />
                  )}
                  <span className="secondary-spell-name">{choice.name}</span>
                </button>
              );
            })}
          </div>
        </div>
      )}
    </section>
  );
}
