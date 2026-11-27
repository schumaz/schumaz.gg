import { memo } from "react";
import {
  ChampionBuild,
  ChampSelectedPayload,
  FlashKeyPreference,
  SpellMetadata,
  SummonerInfo,
} from "../../types";
import { ArenaAugmentsCard } from "../../components/ui/ArenaAugmentsCard";
import { ArenaSynergiesCard } from "../../components/ui/ArenaSynergiesCard";
import { FiltersHeader } from "../../components/ui/FiltersHeader";
import { ItemBlocks } from "../../components/ui/ItemBlocks";
import { RuneTree } from "../../components/ui/RuneTree";
import { SpellsBar } from "../../components/ui/SpellsBar";
import { isArenaBuildContext } from "../../components/ui/queueContext";
import { IdleState } from "./IdleState";
import { isSpellsLockedPhase } from "./lcuEventListeners";

interface ChampSelectTabProps {
  isActive: boolean;
  build: ChampionBuild | null;
  champSelect: ChampSelectedPayload | null;
  summoner: SummonerInfo | null;
  gameflowPhase?: string;
  loading: boolean;
  errorMsg: string | null;
  orderedSpells: SpellMetadata[];
  flashKey: FlashKeyPreference;
  selectedSpellPairIndex: number;
  selectedRunePageIndex: number;
  selectedBuildIndex: number;
  onFilterChange: (
    filterType: "position" | "tier" | "region" | "patch",
    value: string
  ) => void;
  onSelectRunePage: (pageIndex: number) => void;
  onInvertSpells: () => void;
  onSelectSpellPair: (pairIndex: number) => void;
  onSelectBuild: (index: number) => void;
  onSelectLobbySlot?: (slotIndex: number) => void;
}

export const ChampSelectTab = memo(function ChampSelectTab({
  isActive,
  build,
  champSelect,
  summoner,
  gameflowPhase = "",
  loading,
  errorMsg,
  orderedSpells,
  flashKey,
  selectedSpellPairIndex,
  selectedRunePageIndex,
  selectedBuildIndex,
  onFilterChange,
  onSelectRunePage,
  onInvertSpells,
  onSelectSpellPair,
  onSelectBuild,
  onSelectLobbySlot,
}: ChampSelectTabProps) {
  const isArena = isArenaBuildContext(build, champSelect);
  const isSpellsReadOnly =
    isSpellsLockedPhase(gameflowPhase, champSelect?.is_lobby_preselect) ||
    build?.supports_spells === false;

  return (
    <div
      className={`tab-content ${isActive ? "active-tab" : "inactive-tab"}`}
      data-testid="view-runes"
      aria-hidden={!isActive}
    >
      {build ? (
        <div className="dashboard-layout">
          <FiltersHeader
            build={build}
            champSelect={champSelect}
            loading={loading}
            onFilterChange={onFilterChange}
            onSelectLobbySlot={onSelectLobbySlot}
          />

          <div className={`dashboard-split-grid ${isArena ? "is-arena-grid" : ""}`}>
            <div className={`dashboard-col dashboard-col-left ${isArena ? "is-arena-col" : ""}`}>
              {isArena ? (
                <>
                  <ArenaAugmentsCard
                    augments={build.arena_augments ?? []}
                    showModeNotice={true}
                  />
                  <ArenaSynergiesCard synergies={build.arena_synergies ?? []} />
                </>
              ) : (
                <>
                  <RuneTree
                    build={build}
                    selectedRunePageIndex={selectedRunePageIndex}
                    onSelectRunePage={onSelectRunePage}
                  />

                  <SpellsBar
                    orderedSpells={orderedSpells}
                    spellOptions={build.spell_options}
                    selectedSpellPairIndex={selectedSpellPairIndex}
                    flashKey={flashKey}
                    onInvertSpells={isSpellsReadOnly ? undefined : onInvertSpells}
                    onSelectSpellPair={isSpellsReadOnly ? undefined : onSelectSpellPair}
                    readOnly={isSpellsReadOnly}
                  />
                </>
              )}
            </div>

            <div className="dashboard-col dashboard-col-right">
              <ItemBlocks
                build={build}
                selectedBuildIndex={selectedBuildIndex}
                onSelectBuild={onSelectBuild}
              />
            </div>
          </div>
        </div>
      ) : (
        <IdleState
          summoner={summoner}
          errorMsg={errorMsg}
          champSelect={champSelect}
          loading={loading}
        />
      )}
    </div>
  );
});
