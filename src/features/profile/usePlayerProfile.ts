import { useState, useEffect, useRef, useCallback } from "react";
import {
  AppLocale,
  CachedProfileResult,
  PlayerProfile,
  SummonerInfo,
} from "../../types";
import { profileIpc, type UnlistenFn } from "../../services/ipc";
import { translate } from "../../i18n";

export const PROFILE_REFRESH_COOLDOWN_SECS = 30;

function isValidCachedProfileResult(value: unknown): value is CachedProfileResult {
  if (!value || typeof value !== "object") return false;
  const candidate = value as Partial<CachedProfileResult>;
  return Boolean(
    candidate.profile &&
      typeof candidate.profile === "object" &&
      candidate.profile.summoner &&
      typeof candidate.profile.summoner.summonerId === "number"
  );
}

export function usePlayerProfile(
  summoner: SummonerInfo | null,
  locale: AppLocale = "pt_BR"
) {
  const [profile, setProfile] = useState<PlayerProfile | null>(null);
  const [profileLoading, setProfileLoading] = useState(false);
  const [profileError, setProfileError] = useState<string | null>(null);
  const [refreshCooldownSecs, setRefreshCooldownSecs] = useState(0);
  const [profileInitialReady, setProfileInitialReady] = useState(false);

  const fetchSeqRef = useRef(0);
  const lastSummonerIdRef = useRef<number | null>(null);
  const isFetchingRef = useRef(false);
  const prevLocaleRef = useRef<AppLocale>(locale);
  const summonerRef = useRef<SummonerInfo | null>(summoner);
  summonerRef.current = summoner;
  const profileRef = useRef<PlayerProfile | null>(profile);
  profileRef.current = profile;

  useEffect(() => {
    if (refreshCooldownSecs <= 0) return;
    const timer = window.setInterval(() => {
      setRefreshCooldownSecs((prev) => (prev > 1 ? prev - 1 : 0));
    }, 1000);
    return () => window.clearInterval(timer);
  }, [refreshCooldownSecs > 0]);

  const fetchProfile = useCallback(
    async (withCooldown: boolean, forceRefresh: boolean, showLoadingSpinner = true) => {
      if (!summoner || isFetchingRef.current) return;

      const seq = ++fetchSeqRef.current;
      isFetchingRef.current = true;
      if (showLoadingSpinner) setProfileLoading(true);
      setProfileError(null);
      if (withCooldown) setRefreshCooldownSecs(PROFILE_REFRESH_COOLDOWN_SECS);

      try {
        const data = await profileIpc.getPlayerProfile(forceRefresh);
        if (fetchSeqRef.current === seq && data && "summoner" in data) {
          setProfile(data);
        }
      } catch (err: unknown) {
        if (fetchSeqRef.current === seq) {
          setProfileError(
            typeof err === "string" ? err : translate(locale, "profile.fetchError")
          );
        }
      } finally {
        isFetchingRef.current = false;
        if (fetchSeqRef.current === seq) setProfileLoading(false);
      }
    },
    [summoner, locale]
  );

  // Instant 0ms startup hydration from disk cache before LCU connects
  useEffect(() => {
    if (summonerRef.current !== null) {
      setProfileInitialReady(true);
      return;
    }
    let active = true;
    profileIpc
      .getCachedPlayerProfile(null)
      .then((cached) => {
        if (
          active &&
          lastSummonerIdRef.current === null &&
          summonerRef.current === null &&
          isValidCachedProfileResult(cached)
        ) {
          setProfile(cached.profile);
        }
      })
      .catch(() => {})
      .finally(() => {
        if (active) setProfileInitialReady(true);
      });
    return () => {
      active = false;
    };
  }, []);

  const refreshProfile = useCallback(() => {
    if (!summoner || refreshCooldownSecs > 0 || isFetchingRef.current) return;
    fetchProfile(true, true, true);
  }, [summoner, fetchProfile, refreshCooldownSecs]);

  // Re-localize cached/live profile when language changes
  useEffect(() => {
    if (prevLocaleRef.current === locale) return;
    prevLocaleRef.current = locale;

    const targetSummonerId =
      summoner?.summonerId ?? profileRef.current?.summoner.summonerId ?? null;
    if (targetSummonerId !== null) {
      profileIpc
        .getCachedPlayerProfile(targetSummonerId)
        .then((cached) => {
          if (isValidCachedProfileResult(cached)) {
            setProfile(cached.profile);
          } else if (summoner) {
            isFetchingRef.current = false;
            fetchProfile(false, false, true);
          }
        })
        .catch(() => {
          if (summoner) {
            isFetchingRef.current = false;
            fetchProfile(false, false, true);
          }
        });
      return;
    }

    if (summoner) {
      isFetchingRef.current = false;
      fetchProfile(false, false, true);
    }
  }, [locale, summoner, fetchProfile]);

  // LCU connection / account switch handling with Stale-While-Revalidate (> 5 min)
  useEffect(() => {
    if (!summoner) {
      fetchSeqRef.current += 1;
      lastSummonerIdRef.current = null;
      isFetchingRef.current = false;
      setProfileLoading(false);
      setProfileError(null);
      setRefreshCooldownSecs(0);
      return;
    }

    if (lastSummonerIdRef.current !== summoner.summonerId) {
      lastSummonerIdRef.current = summoner.summonerId;
      setRefreshCooldownSecs(0);

      let cancelled = false;
      const connectSeq = ++fetchSeqRef.current;

      const hydrateAndRevalidate = async () => {
        try {
          const cached = await profileIpc.getCachedPlayerProfile(summoner.summonerId);
          if (cancelled || fetchSeqRef.current !== connectSeq) return;
          if (
            isValidCachedProfileResult(cached) &&
            cached.profile.summoner.summonerId === summoner.summonerId
          ) {
            setProfile(cached.profile);
            setProfileError(null);
            setProfileInitialReady(true);
            if (!cached.isStale) return;
            await fetchProfile(false, true, false);
            return;
          }
        } catch {
          // Fall through to live fetch
        }

        if (!cancelled && fetchSeqRef.current === connectSeq) {
          await fetchProfile(false, false, true);
        }
        if (!cancelled) {
          setProfileInitialReady(true);
        }
      };

      hydrateAndRevalidate();
      return () => {
        cancelled = true;
      };
    }
  }, [summoner, fetchProfile]);

  // Force refresh on post-match events (`profile-refresh-requested` / `match-ended`)
  useEffect(() => {
    let unlistenRefresh: UnlistenFn | undefined;
    const setup = async () => {
      try {
        unlistenRefresh = await profileIpc.onProfileRefreshRequested(() => {
          if (summoner) {
            isFetchingRef.current = false;
            fetchProfile(false, true, true);
          }
        });
      } catch {
        // Ignore in non-Tauri test environments
      }
    };

    setup();
    return () => {
      if (unlistenRefresh) unlistenRefresh();
    };
  }, [summoner, fetchProfile]);

  return {
    profile,
    profileLoading,
    profileError,
    refreshCooldownSecs,
    refreshProfile,
    profileInitialReady,
  };
}
