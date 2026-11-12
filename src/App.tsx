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

      {/* Main Content Area with Seamless Crossfade Transition */}
      <div className="tabs-viewport">
        {/* View 1: "Runas" Tab (Match Dashboard) */}
        <div
          className={`tab-content ${activeTab === "runes" ? "active-tab" : "inactive-tab"}`}
          data-testid="view-runes"
          aria-hidden={activeTab !== "runes"}
        >
          {build ? (
            <div className="dashboard-layout">
              <FiltersHeader
                build={build}
                champSelect={champSelect}
                loading={loading}
                onFilterChange={handleFilterChange}
              />

              <div className="dashboard-split-grid">
                <div className="dashboard-col dashboard-col-left">
                  <RuneTree
                    build={build}
                    selectedRunePageIndex={selectedRunePageIndex}
                    onSelectRunePage={handleSelectRunePage}
                  />
                </div>

                <div className="dashboard-col dashboard-col-right">
                  <SpellsBar
                    orderedSpells={orderedSpells}
                    spellChoices={build.spell_choices}
                    selectedSecondarySpellId={selectedSecondarySpellId}
                    onInvertSpells={handleInvertSpells}
                    onSelectSecondarySpell={handleSelectSecondarySpell}
                  />

                  <ItemBlocks
                    build={build}
                    selectedBuildIndex={selectedBuildIndex}
                    onSelectBuild={handleSelectBuild}
                  />
                </div>
              </div>
            </div>
          ) : (
            <IdleState summoner={summoner} errorMsg={errorMsg} />
          )}
        </div>

        {/* View 2: "Configurações" Tab (Settings & Preferences) */}
        <SettingsTab
          preferences={preferences}
          onUpdatePreference={updatePreference}
          isActive={activeTab === "settings"}
        />
      </div>

      {/* Bottom Navigation Dock */}
      <NavigationDock activeTab={activeTab} onTabChange={setActiveTab} />
    </main>
  );
}

export default App;
