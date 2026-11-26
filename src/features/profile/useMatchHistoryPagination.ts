import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  ArenaAugmentMetadata,
  ItemMetadata,
  PlayerProfile,
  RecentMatchInfo,
  RuneMetadata,
  SpellMetadata,
} from "../../types";
import { profileIpc } from "../../services/ipc";
import { useTranslation } from "../../i18n";
import {
  AugmentMetadataMap,
  ItemMetadataMap,
  RuneMetadataMap,
  SpellMetadataMap,
} from "../../components/ui";

export const MATCH_HISTORY_PAGE_SIZE = 10;

interface ExtraMetadataState {
  items: Record<number, ItemMetadata>;
  runes: Record<number, RuneMetadata>;
  spells: Record<number, SpellMetadata>;
  augments: Record<number, ArenaAugmentMetadata>;
}

const EMPTY_EXTRA_METADATA: ExtraMetadataState = {
  items: {},
  runes: {},
  spells: {},
  augments: {},
};

function resolveInitialCursor(profile: PlayerProfile | null): {
  nextBegIndex: number;
  hasMore: boolean;
} {
  const fallbackLen = profile?.recent_matches?.length ?? 0;
  const nextBegIndex =
    typeof profile?.history_next_beg_index === "number"
      ? profile.history_next_beg_index
      : fallbackLen;
  return {
    nextBegIndex,
    hasMore: Boolean(profile?.history_has_more),
  };
}

function mergeMetadataMap<T>(
  base: Partial<Record<number, T>> | undefined | null,
  extra: Record<number, T>
): Partial<Record<number, T>> | undefined {
  const hasBase = Boolean(base && Object.keys(base).length > 0);
  const hasExtra = Object.keys(extra).length > 0;
  if (!hasBase && !hasExtra) return undefined;
  return { ...(base ?? {}), ...extra };
}

export function useMatchHistoryPagination(
  profile: PlayerProfile | null,
  summonerId: number | null,
  isLcuConnected: boolean
) {
  const { t } = useTranslation();
  const initialCursor = resolveInitialCursor(profile);

  const [extraMatches, setExtraMatches] = useState<RecentMatchInfo[]>([]);
  const [extraMeta, setExtraMeta] = useState<ExtraMetadataState>(EMPTY_EXTRA_METADATA);
  const [nextBegIndex, setNextBegIndex] = useState<number>(initialCursor.nextBegIndex);
  const [hasMore, setHasMore] = useState<boolean>(initialCursor.hasMore);
  const [isLoadingMore, setIsLoadingMore] = useState<boolean>(false);
  const [hasLoadMoreError, setHasLoadMoreError] = useState<boolean>(false);

  const epochRef = useRef(0);
  const isLoadingMoreRef = useRef(false);
  const nextBegIndexRef = useRef(initialCursor.nextBegIndex);
  const hasMoreRef = useRef(initialCursor.hasMore);
  const profileRef = useRef(profile);
  profileRef.current = profile;

  const resetPagination = useCallback(() => {
    const initial = resolveInitialCursor(profileRef.current);
    epochRef.current += 1;
    isLoadingMoreRef.current = false;
    nextBegIndexRef.current = initial.nextBegIndex;
    hasMoreRef.current = initial.hasMore;
    setExtraMatches([]);
    setExtraMeta(EMPTY_EXTRA_METADATA);
    setNextBegIndex(initial.nextBegIndex);
    setHasMore(initial.hasMore);
    setIsLoadingMore(false);
    setHasLoadMoreError(false);
  }, []);

  useEffect(() => {
    const initial = resolveInitialCursor(profile);
    epochRef.current += 1;
    isLoadingMoreRef.current = false;
    nextBegIndexRef.current = initial.nextBegIndex;
    hasMoreRef.current = initial.hasMore;
    setExtraMatches([]);
    setExtraMeta(EMPTY_EXTRA_METADATA);
    setNextBegIndex(initial.nextBegIndex);
    setHasMore(initial.hasMore);
    setIsLoadingMore(false);
    setHasLoadMoreError(false);
  }, [profile, summonerId]);

  const loadMore = useCallback(async () => {
    if (!isLcuConnected || !hasMoreRef.current || isLoadingMoreRef.current) {
      return;
    }

    const requestEpoch = epochRef.current;
    const begIndex = nextBegIndexRef.current;
    isLoadingMoreRef.current = true;
    setIsLoadingMore(true);
    setHasLoadMoreError(false);

    try {
      const page = await profileIpc.getPlayerMatchHistoryPage(begIndex, MATCH_HISTORY_PAGE_SIZE);
      if (epochRef.current !== requestEpoch) return;
      if (!page || !Array.isArray(page.matches)) {
        setHasLoadMoreError(true);
        return;
      }

      setExtraMatches((prev) => {
        const seen = new Set<number>();
        for (const m of profileRef.current?.recent_matches ?? []) {
          seen.add(m.game_id);
        }
        for (const m of prev) {
          seen.add(m.game_id);
        }
        const incoming = page.matches.filter((m) => {
          if (seen.has(m.game_id)) return false;
          seen.add(m.game_id);
          return true;
        });
        return incoming.length > 0 ? [...prev, ...incoming] : prev;
      });

      setExtraMeta((prev) => ({
        items: { ...prev.items, ...(page.item_metadata ?? {}) },
        runes: { ...prev.runes, ...(page.rune_metadata ?? {}) },
        spells: { ...prev.spells, ...(page.spell_metadata ?? {}) },
        augments: { ...prev.augments, ...(page.augment_metadata ?? {}) },
      }));

      const nextCursor =
        typeof page.next_beg_index === "number" ? page.next_beg_index : begIndex;
      const canContinue = nextCursor > begIndex && Boolean(page.has_more);
      nextBegIndexRef.current = nextCursor;
      hasMoreRef.current = canContinue;
      setNextBegIndex(nextCursor);
      setHasMore(canContinue);
    } catch {
      if (epochRef.current === requestEpoch) {
        setHasLoadMoreError(true);
      }
    } finally {
      if (epochRef.current === requestEpoch) {
        isLoadingMoreRef.current = false;
        setIsLoadingMore(false);
      }
    }
  }, [isLcuConnected]);

  const matches = useMemo(() => {
    const base = profile?.recent_matches ?? [];
    if (extraMatches.length === 0) return base;
    const seen = new Set<number>();
    const combined: RecentMatchInfo[] = [];
    for (const m of base) {
      if (!seen.has(m.game_id)) {
        seen.add(m.game_id);
        combined.push(m);
      }
    }
    for (const m of extraMatches) {
      if (!seen.has(m.game_id)) {
        seen.add(m.game_id);
        combined.push(m);
      }
    }
    return combined;
  }, [profile, extraMatches]);

  const itemMetadata = useMemo(
    () => mergeMetadataMap(profile?.item_metadata, extraMeta.items) as ItemMetadataMap | undefined,
    [profile?.item_metadata, extraMeta.items]
  );
  const runeMetadata = useMemo(
    () => mergeMetadataMap(profile?.rune_metadata, extraMeta.runes) as RuneMetadataMap | undefined,
    [profile?.rune_metadata, extraMeta.runes]
  );
  const spellMetadata = useMemo(
    () =>
      mergeMetadataMap(profile?.spell_metadata, extraMeta.spells) as SpellMetadataMap | undefined,
    [profile?.spell_metadata, extraMeta.spells]
  );
  const augmentMetadata = useMemo(
    () =>
      mergeMetadataMap(profile?.augment_metadata, extraMeta.augments) as
        | AugmentMetadataMap
        | undefined,
    [profile?.augment_metadata, extraMeta.augments]
  );

  return {
    matches,
    itemMetadata,
    runeMetadata,
    spellMetadata,
    augmentMetadata,
    nextBegIndex,
    hasMore,
    canLoadMore: isLcuConnected && hasMore,
    isLoadingMore,
    loadMoreError: hasLoadMoreError ? t("profile.loadMoreError") : null,
    loadMore,
    resetPagination,
  };
}
