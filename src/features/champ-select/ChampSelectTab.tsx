import {
  ChampionBuild,
  ChampSelectedPayload,
  SpellMetadata,
  SummonerInfo,
} from "../../types";
import { ItemBlocks } from "../../components/ui/ItemBlocks";
import { RuneTree } from "../../components/ui/RuneTree";
import { SpellsBar } from "../../components/ui/SpellsBar";
import { FiltersHeader } from "./FiltersHeader";
import { IdleState } from "./IdleState";

interface ChampSelectTabProps {
  isActive: boolean;
  build: ChampionBuild | null;
  champSelect: ChampSelectedPayload | null;
  summoner: SummonerInfo | null;
  loading: boolean;
  errorMsg: string | null;
  orderedSpells: SpellMetadata[];
  selectedSecondarySpellId: number | null;
  selectedRunePageIndex: number;
  selectedBuildIndex: number;
  onFilterChange: (
    filterType: "position" | "tier" | "region" | "patch",
    value: string
  ) => void;
  onSelectRunePage: (pageIndex: number) => void;
  onInvertSpells: () => void;
  onSelectSecondarySpell: (spellId: number) => void;
  onSelectBuild: (index: number) => void;
}

export function ChampSelectTab({
  isActive,
  build,
  champSelect,
  summoner,
  loading,
  errorMsg,
  orderedSpells,
  selectedSecondarySpellId,
  selectedRunePageIndex,
  selectedBuildIndex,
  onFilterChange,
  onSelectRunePage,
  onInvertSpells,
  onSelectSecondarySpell,
  onSelectBuild,
}: ChampSelectTabProps) {
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
          />

          <div className="dashboard-split-grid">
            <div className="dashboard-col dashboard-col-left">
              <RuneTree
                build={build}
                selectedRunePageIndex={selectedRunePageIndex}
                onSelectRunePage={onSelectRunePage}
              />

              <SpellsBar
                orderedSpells={orderedSpells}
                spellChoices={build.spell_choices}
                selectedSecondarySpellId={selectedSecondarySpellId}
                onInvertSpells={onInvertSpells}
                onSelectSecondarySpell={onSelectSecondarySpell}
              />
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
}
