import { useState, useEffect, useRef, useCallback } from "react";
import { PlayerProfile, SummonerInfo, commands, events } from "../types";

export const PROFILE_REFRESH_COOLDOWN_SECS = 30;

export function usePlayerProfile(summoner: SummonerInfo | null) {
  const [profile, setProfile] = useState<PlayerProfile | null>(null);
  const [profileLoading, setProfileLoading] = useState(false);
  const [profileError, setProfileError] = useState<string | null>(null);
  const [refreshCooldownSecs, setRefreshCooldownSecs] = useState(0);

  const fetchSeqRef = useRef(0);
  const lastSummonerIdRef = useRef<number | null>(null);
  const isFetchingRef = useRef(false);

  useEffect(() => {
    if (refreshCooldownSecs <= 0) return;
    const timer = window.setInterval(() => {
      setRefreshCooldownSecs((prev) => (prev > 1 ? prev - 1 : 0));
    }, 1000);
    return () => window.clearInterval(timer);
  }, [refreshCooldownSecs > 0]);

  const fetchProfile = useCallback(
    async (withCooldown: boolean) => {
      if (!summoner) return;
      if (isFetchingRef.current) return;

      const seq = ++fetchSeqRef.current;
      isFetchingRef.current = true;
      setProfileLoading(true);
      setProfileError(null);

      if (withCooldown) {
        setRefreshCooldownSecs(PROFILE_REFRESH_COOLDOWN_SECS);
      }

      try {
        const data = await commands.getPlayerProfile();
        if (fetchSeqRef.current === seq && data) {
          setProfile(data);
        }
      } catch (err: unknown) {
        if (fetchSeqRef.current === seq) {
          setProfileError(
            typeof err === "string" ? err : "Não foi possível carregar os dados do perfil."
          );
        }
      } finally {
        isFetchingRef.current = false;
        if (fetchSeqRef.current === seq) {
          setProfileLoading(false);
        }
      }
    },
    [summoner]
  );

  const refreshProfile = useCallback(() => {
    if (refreshCooldownSecs > 0 || isFetchingRef.current) return;
    fetchProfile(true);
  }, [fetchProfile, refreshCooldownSecs]);

  useEffect(() => {
    if (!summoner) {
      fetchSeqRef.current += 1;
      lastSummonerIdRef.current = null;
      isFetchingRef.current = false;
      setProfile(null);
      setProfileLoading(false);
      setProfileError(null);
      setRefreshCooldownSecs(0);
      return;
    }

    if (lastSummonerIdRef.current !== summoner.summonerId) {
      lastSummonerIdRef.current = summoner.summonerId;
      setRefreshCooldownSecs(0);
      fetchProfile(false);
    }
  }, [summoner, fetchProfile]);

  useEffect(() => {
    let unlistenRefresh: (() => void) | undefined;

    const setup = async () => {
      try {
        unlistenRefresh = await events.profileRefreshRequested.listen(() => {
          if (summoner) {
            isFetchingRef.current = false;
            fetchProfile(false);
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
  };
}
