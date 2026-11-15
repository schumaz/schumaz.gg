import { useEffect, useMemo, useState } from "react";
import { AramAugmentEntry, AramChampionRank, AramMode } from "../../types";
import {
  AugmentRarityFilter,
  normalizeAramSearch,
  parseRarityFilter,
  sortAugmentsBestToWorst,
} from "./aramHelpers";
import { AramModeChooser } from "./AramModeChooser";
import { AramClassicView } from "./AramClassicView";
import { AramMayhemView } from "./AramMayhemView";

export * from "./aramHelpers";

interface AramTabProps {
  isActive: boolean;
  savedMode: AramMode | null;
  showChooser: boolean;
  tierLists: Partial<Record<AramMode, AramChampionRank[]>>;
  tierLoading: Partial<Record<AramMode, boolean>>;
  tierErrors: Partial<Record<AramMode, string | null>>;
  selectedChampionKey: string | null;
  augmentsByChampion: Record<string, AramAugmentEntry[]>;
  augmentsLoading: boolean;
  augmentsError: string | null;
  onSelectMode: (mode: AramMode) => void;
  onSelectChampion: (championKey: string) => void;
  onRetryTierList: (mode: AramMode) => void;
  onRetryAugments: () => void;
  savedRarityFilter?: string;
  onRarityFilterChange?: (rarity: AugmentRarityFilter) => void;
}

export function AramTab({
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
  savedRarityFilter,
  onRarityFilterChange,
}: AramTabProps) {
  const [aramSearch, setAramSearch] = useState("");
  const [mayhemChampSearch, setMayhemChampSearch] = useState("");
  const [augmentSearch, setAugmentSearch] = useState("");
  const [rarityFilter, setRarityFilter] = useState<AugmentRarityFilter>(() =>
    parseRarityFilter(savedRarityFilter)
  );

  useEffect(() => {
    if (savedRarityFilter !== undefined) {
      setRarityFilter(parseRarityFilter(savedRarityFilter));
    }
  }, [savedRarityFilter]);

  const handleSelectRarityFilter = (next: AugmentRarityFilter) => {
    setRarityFilter(next);
    onRarityFilterChange?.(next);
  };

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

  return (
    <div
      className={`tab-content ${isActive ? "active-tab" : "inactive-tab"}`}
      data-testid="view-aram"
      aria-hidden={!isActive}
    >
      {showChooser ? (
        <AramModeChooser savedMode={savedMode} onSelectMode={onSelectMode} />
      ) : activeMode === "aram" ? (
        <AramClassicView
          currentChampions={currentChampions}
          filteredAramChampions={filteredAramChampions}
          isLoadingList={isLoadingList}
          listError={listError}
          aramSearch={aramSearch}
          onSearchChange={setAramSearch}
          onRetryTierList={onRetryTierList}
        />
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
          onSelectRarityFilter={handleSelectRarityFilter}
          augmentSearch={augmentSearch}
          onAugmentSearchChange={setAugmentSearch}
          augmentsLoading={augmentsLoading}
          augmentsError={augmentsError}
          currentAugments={currentAugments}
          onRetryAugments={onRetryAugments}
        />
      )}
    </div>
  );
}
