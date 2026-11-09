import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

import { ActiveTab, FlashKeyPreference } from "./types";
import { usePreferences } from "./hooks/usePreferences";
import { useLcuEvents } from "./hooks/useLcuEvents";
import { computeOrderedSpells } from "./utils/spells";
import {
  StatusDock,
  FiltersHeader,
  RuneTree,
  SpellsBar,
  ItemBlocks,
  SettingsTab,
  IdleState,
  NavigationDock,
} from "./components";

// Re-export all types and utilities for seamless backward compatibility
export * from "./types";
export * from "./utils/spells";

function App() {
  const [activeTab, setActiveTab] = useState<ActiveTab>("runes");

  const handleFlashKeyChange = (newFlashKey: FlashKeyPreference) => {
    const reordered = computeOrderedSpells(
      build?.spells,
      selectedSecondarySpellId,
      build?.spell_choices,
      newFlashKey,
      spellsInverted
    );
    if (reordered.length >= 2) {
      invoke("inject_summoner_spells", {
        spell1Id: reordered[0].id,
        spell2Id: reordered[1].id,
      }).catch(() => {});
    }
  };

  const { preferences, updatePreference } = usePreferences(handleFlashKeyChange);

  const {
    summoner,
    champSelect,
    build,
    spellsInverted,
    selectedSecondarySpellId,
    selectedRunePageIndex,
    selectedBuildIndex,
    loading,
    errorMsg,
    orderedSpells,
    handleFilterChange,
    handleSelectSecondarySpell,
    handleSelectRunePage,
    handleSelectBuild,
  } = useLcuEvents(preferences.flashKey);

  const handleInvertSpells = async () => {
    const nextFlashKey: FlashKeyPreference = preferences.flashKey === "D" ? "F" : "D";
    updatePreference("flashKey", nextFlashKey);
  };

  return (
    <main className="container">
      <StatusDock summoner={summoner} />

      {/* View 1: "Runas" Tab (Match Dashboard) */}
      {activeTab === "runes" && (
        <div className="tab-content" data-testid="view-runes">
          {build ? (
            <>
              <FiltersHeader
                build={build}
                champSelect={champSelect}
                loading={loading}
                onFilterChange={handleFilterChange}
              />

              <div className="runes-spells-grid">
                <RuneTree
                  build={build}
                  selectedRunePageIndex={selectedRunePageIndex}
                  onSelectRunePage={handleSelectRunePage}
                />

                <SpellsBar
                  orderedSpells={orderedSpells}
                  spellChoices={build.spell_choices}
                  selectedSecondarySpellId={selectedSecondarySpellId}
                  onInvertSpells={handleInvertSpells}
                  onSelectSecondarySpell={handleSelectSecondarySpell}
                />
              </div>

              <ItemBlocks
                build={build}
                selectedBuildIndex={selectedBuildIndex}
                onSelectBuild={handleSelectBuild}
              />
            </>
          ) : (
            <IdleState summoner={summoner} errorMsg={errorMsg} />
          )}
        </div>
      )}

      {/* View 2: "Configurações" Tab (Settings & Preferences) */}
      {activeTab === "settings" && (
        <SettingsTab
          preferences={preferences}
          onUpdatePreference={updatePreference}
        />
      )}

      {/* Floating Bottom Navigation Dock (iOS / Telegram Style Pill) */}
      <NavigationDock activeTab={activeTab} onTabChange={setActiveTab} />
    </main>
  );
}

export default App;
