import { memo, useMemo, useState } from "react";
import { useTranslation } from "../../i18n";
import { useSubviewTransition } from "../../hooks/useSubviewTransition";
import { computeOrderedSpells } from "../../utils/spells";
import {
  AramMayhemPanel,
  AramTabProps,
  AugmentRarityFilter,
  getChampionTierLabel,
  normalizeAramSearch,
  sortAugmentsBestToWorst,
} from "./aramHelpers";
import { AramModeChooser } from "./AramModeChooser";
import { AramClassicView } from "./AramClassicView";
import { AramClassicChampionDetail } from "./AramClassicChampionDetail";
import { AramMayhemView } from "./AramMayhemView";

export * from "./aramHelpers";

export const AramTab = memo(function AramTab({
  isActive,
  savedMode,
  showChooser,
  tierLists,
  tierLoading,
  tierErrors,
  selectedChampionKey,
  augmentsByChampion,
  augmentsLoading,
  augmentsError,
  onSelectMode,
  onSelectChampion,
  onRetryTierList,
  onRetryAugments,
  selectedClassicChampion = null,
  classicBuild = null,
  classicBuildLoading = false,
  classicBuildError = null,
  classicOrderedSpells,
  classicRunePageIndex,
  classicBuildIndex,
  onSelectClassicChampion,
  onBackToClassicList,
  onClassicDetailFilterChange,
  onRetryClassicBuild,
  onSelectClassicRunePage,
  onSelectClassicBuild,
  mayhemPanel,
  onSelectMayhemPanel,
  mayhemBuildsByChampion,
  mayhemBuildLoading = false,
  mayhemBuildError = null,
  mayhemOrderedSpells,
  mayhemBuildIndex,
  onSelectMayhemBuild,
  onRetryMayhemBuild,
  flashKey = "D",
}: AramTabProps) {
  const { t } = useTranslation();
  const [aramSearch, setAramSearch] = useState("");
  const [mayhemChampSearch, setMayhemChampSearch] = useState("");
  const [augmentSearch, setAugmentSearch] = useState("");
  const [rarityFilter, setRarityFilter] = useState<AugmentRarityFilter>("all");
  const [localMayhemPanel, setLocalMayhemPanel] = useState<AramMayhemPanel>("augments");
  const [localClassicRuneIdx, setLocalClassicRuneIdx] = useState(0);
  const [localClassicBuildIdx, setLocalClassicBuildIdx] = useState(0);
  const [localMayhemBuildIdx, setLocalMayhemBuildIdx] = useState(0);

  const activeMode = savedMode ?? "aram";
  const currentChampions = tierLists[activeMode] ?? [];
  const isLoadingList = Boolean(tierLoading[activeMode]);
  const listError = tierErrors[activeMode] ?? null;

  const filteredAramChampions = useMemo(() => {
    const query = normalizeAramSearch(aramSearch);
    if (!query) return currentChampions;
    return currentChampions.filter((c) => normalizeAramSearch(c.name).includes(query));
  }, [currentChampions, aramSearch]);

  const filteredMayhemChampions = useMemo(() => {
    const query = normalizeAramSearch(mayhemChampSearch);
    if (!query) return currentChampions;
    return currentChampions.filter((c) => normalizeAramSearch(c.name).includes(query));
  }, [currentChampions, mayhemChampSearch]);

  const selectedChampion = useMemo(() => {
    if (!selectedChampionKey) return null;
    return currentChampions.find((c) => c.key === selectedChampionKey) ?? null;
  }, [currentChampions, selectedChampionKey]);

  const currentAugments = useMemo(() => {
    if (!selectedChampionKey) return [];
    const raw = augmentsByChampion[selectedChampionKey] ?? [];
    const sorted = sortAugmentsBestToWorst(raw);
    const query = normalizeAramSearch(augmentSearch);

    return sorted.filter((aug) => {
      if (rarityFilter !== "all" && aug.rarity !== rarityFilter) return false;
      if (query && !normalizeAramSearch(aug.name).includes(query)) return false;
      return true;
    });
  }, [selectedChampionKey, augmentsByChampion, rarityFilter, augmentSearch]);

  const resolvedMayhemPanel = mayhemPanel ?? localMayhemPanel;
  const handleSelectMayhemPanel = (panel: AramMayhemPanel) => {
    if (mayhemPanel === undefined) setLocalMayhemPanel(panel);
    onSelectMayhemPanel?.(panel);
  };

  const resolvedClassicRuneIdx = classicRunePageIndex ?? localClassicRuneIdx;
  const resolvedClassicBuildIdx = classicBuildIndex ?? localClassicBuildIdx;
  const resolvedMayhemBuildIdx = mayhemBuildIndex ?? localMayhemBuildIdx;

  const activeMayhemBuild = selectedChampionKey
    ? mayhemBuildsByChampion?.[selectedChampionKey.trim().toLowerCase()] ?? null
    : null;

  const resolvedClassicSpells = useMemo(
    () =>
      classicOrderedSpells ??
      computeOrderedSpells(classicBuild?.spells, 0, classicBuild?.spell_options, flashKey, false),
    [classicOrderedSpells, classicBuild?.spells, classicBuild?.spell_options, flashKey]
  );

  const resolvedMayhemSpells = useMemo(
    () =>
      mayhemOrderedSpells ??
      computeOrderedSpells(activeMayhemBuild?.spells, 0, activeMayhemBuild?.spell_options, flashKey, false),
    [mayhemOrderedSpells, activeMayhemBuild?.spells, activeMayhemBuild?.spell_options, flashKey]
  );

  const classicDetailStatusTag = selectedClassicChampion
    ? selectedClassicChampion.rank > 0
      ? t("aram.classicDetailStatus", {
          tier: getChampionTierLabel(selectedClassicChampion.tier),
          rank: selectedClassicChampion.rank,
        })
      : t("aram.classicDetailStatusFallback")
    : t("aram.classicDetailStatusFallback");

  const subviewKey = showChooser
    ? "chooser"
    : activeMode === "aram"
      ? selectedClassicChampion
        ? `classic-detail:${selectedClassicChampion.id}`
        : "classic-list"
      : "mayhem";
  const subviewRank = showChooser
    ? 0
    : activeMode === "aram" && selectedClassicChampion
      ? 2
      : 1;
  const subviewMotionClass = useSubviewTransition(subviewKey, subviewRank, isActive, "depth");

  return (
    <div
      className={`tab-content ${isActive ? "active-tab" : "inactive-tab"}`}
      data-tab="aram"
      data-testid="view-aram"
      aria-hidden={!isActive}
    >
      {showChooser ? (
        <AramModeChooser
          savedMode={savedMode}
          onSelectMode={onSelectMode}
          motionClassName={subviewMotionClass}
        />
      ) : activeMode === "aram" ? (
        selectedClassicChampion ? (
          <AramClassicChampionDetail
            selectedChampion={selectedClassicChampion}
            championBuild={classicBuild}
            buildLoading={classicBuildLoading}
            buildError={classicBuildError}
            detailStatusTag={classicDetailStatusTag}
            orderedSpells={resolvedClassicSpells}
            flashKey={flashKey}
            selectedRunePageIndex={resolvedClassicRuneIdx}
            selectedBuildIndex={resolvedClassicBuildIdx}
            onDetailFilterChange={(filterType, val) => onClassicDetailFilterChange?.(filterType, val)}
            onBackToTierList={() => onBackToClassicList?.()}
            onRetryBuild={() => onRetryClassicBuild?.()}
            onSelectRunePage={(idx) => {
              if (classicRunePageIndex === undefined) setLocalClassicRuneIdx(idx);
              onSelectClassicRunePage?.(idx);
            }}
            onSelectBuild={(idx) => {
              if (classicBuildIndex === undefined) setLocalClassicBuildIdx(idx);
              onSelectClassicBuild?.(idx);
            }}
            motionClassName={subviewMotionClass}
          />
        ) : (
          <AramClassicView
            currentChampions={currentChampions}
            filteredAramChampions={filteredAramChampions}
            isLoadingList={isLoadingList}
            listError={listError}
            aramSearch={aramSearch}
            onSearchChange={setAramSearch}
            onSelectChampion={onSelectClassicChampion}
            onRetryTierList={onRetryTierList}
            motionClassName={subviewMotionClass}
          />
        )
      ) : (
        <AramMayhemView
          currentChampions={currentChampions}
          filteredMayhemChampions={filteredMayhemChampions}
          isLoadingList={isLoadingList}
          listError={listError}
          mayhemChampSearch={mayhemChampSearch}
          onMayhemChampSearchChange={setMayhemChampSearch}
          selectedChampionKey={selectedChampionKey}
          selectedChampion={selectedChampion}
          onSelectChampion={onSelectChampion}
          onRetryTierList={onRetryTierList}
          rarityFilter={rarityFilter}
          onSelectRarityFilter={setRarityFilter}
          augmentSearch={augmentSearch}
          onAugmentSearchChange={setAugmentSearch}
          augmentsLoading={augmentsLoading}
          augmentsError={augmentsError}
          currentAugments={currentAugments}
          onRetryAugments={onRetryAugments}
          activePanel={resolvedMayhemPanel}
          onSelectPanel={handleSelectMayhemPanel}
          mayhemBuild={activeMayhemBuild}
          mayhemBuildLoading={mayhemBuildLoading}
          mayhemBuildError={mayhemBuildError}
          mayhemOrderedSpells={resolvedMayhemSpells}
          flashKey={flashKey}
          mayhemBuildIndex={resolvedMayhemBuildIdx}
          onSelectMayhemBuild={(idx) => {
            if (mayhemBuildIndex === undefined) setLocalMayhemBuildIdx(idx);
            onSelectMayhemBuild?.(idx);
          }}
          onRetryMayhemBuild={onRetryMayhemBuild}
          isTabActive={isActive}
          motionClassName={subviewMotionClass}
        />
      )}
    </div>
  );
});
