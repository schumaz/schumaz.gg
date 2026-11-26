import { useCallback, useEffect, useRef, useState } from "react";
import { AppLocale, MatchScoreboardDetail } from "../../types";
import { profileIpc } from "../../services/ipc";

export type ScoreboardEntryState =
  | { status: "idle" | "loading" | "offline" | "error" }
  | { status: "ready"; data: MatchScoreboardDetail };

function isValidScoreboardDetail(value: unknown): value is MatchScoreboardDetail {
  if (!value || typeof value !== "object") return false;
  const candidate = value as Partial<MatchScoreboardDetail>;
  return (
    typeof candidate.game_id === "number" &&
    (Array.isArray(candidate.teams) || Array.isArray(candidate.arena_teams))
  );
}

/**
 * On-demand RAM-cached loader for `/lol-match-history/v1/games/{gameId}` scoreboards.
 * Deduplicates concurrent requests per `gameId` and resets cleanly on summoner/refresh/locale changes.
 */
export function useMatchScoreboard(
  isLcuConnected: boolean,
  resetKey?: unknown,
  locale?: AppLocale
) {
  const [entries, setEntries] = useState<Record<number, ScoreboardEntryState>>({});
  const cacheRef = useRef<Map<number, MatchScoreboardDetail>>(new Map());
  const inFlightRef = useRef<Map<number, Promise<void>>>(new Map());
  const epochRef = useRef(0);

  useEffect(() => {
    epochRef.current += 1;
    cacheRef.current.clear();
    inFlightRef.current.clear();
    setEntries({});
  }, [resetKey, locale]);

  const loadScoreboard = useCallback(
    async (gameId: number) => {
      if (gameId <= 0) return;

      const cached = cacheRef.current.get(gameId);
      if (cached) {
        setEntries((prev) =>
          prev[gameId]?.status === "ready"
            ? prev
            : { ...prev, [gameId]: { status: "ready", data: cached } }
        );
        return;
      }

      if (!isLcuConnected) {
        setEntries((prev) =>
          prev[gameId]?.status === "offline"
            ? prev
            : { ...prev, [gameId]: { status: "offline" } }
        );
        return;
      }

      if (inFlightRef.current.has(gameId)) {
        return;
      }

      const reqEpoch = epochRef.current;
      setEntries((prev) => ({ ...prev, [gameId]: { status: "loading" } }));

      const task = (async () => {
        try {
          const detail = await profileIpc.getMatchScoreboardDetail(gameId);
          if (epochRef.current !== reqEpoch) return;
          if (!isValidScoreboardDetail(detail)) {
            setEntries((prev) => ({ ...prev, [gameId]: { status: "error" } }));
            return;
          }
          cacheRef.current.set(gameId, detail);
          setEntries((prev) => ({
            ...prev,
            [gameId]: { status: "ready", data: detail },
          }));
        } catch {
          if (epochRef.current === reqEpoch) {
            setEntries((prev) => ({ ...prev, [gameId]: { status: "error" } }));
          }
        } finally {
          if (epochRef.current === reqEpoch) {
            inFlightRef.current.delete(gameId);
          }
        }
      })();

      inFlightRef.current.set(gameId, task);
      await task;
    },
    [isLcuConnected]
  );

  const retryScoreboard = useCallback(
    async (gameId: number) => {
      cacheRef.current.delete(gameId);
      await loadScoreboard(gameId);
    },
    [loadScoreboard]
  );

  const getEntry = useCallback(
    (gameId: number): ScoreboardEntryState => {
      const cached = cacheRef.current.get(gameId);
      if (cached) {
        return { status: "ready", data: cached };
      }
      if (!isLcuConnected) {
        return { status: "offline" };
      }
      return entries[gameId] ?? { status: "idle" };
    },
    [entries, isLcuConnected]
  );

  return {
    getEntry,
    loadScoreboard,
    retryScoreboard,
  };
}
