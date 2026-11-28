import { useCallback, useEffect, useRef, useState } from "react";
import type { CacheClearScope, LocalCacheInfo } from "../../types";
import { assetsIpc, settingsIpc } from "../../services/ipc";

export type LocalCacheStatus = "idle" | "cleared" | "error";

const EMPTY_CACHE_INFO: LocalCacheInfo = {
  totalBytes: 0,
  assetsBytes: 0,
  dataBytes: 0,
  fileCount: 0,
};

function isValidLocalCacheInfo(value: unknown): value is LocalCacheInfo {
  if (!value || typeof value !== "object") return false;
  const candidate = value as Record<string, unknown>;
  return (
    typeof candidate.totalBytes === "number" &&
    typeof candidate.assetsBytes === "number" &&
    typeof candidate.dataBytes === "number" &&
    typeof candidate.fileCount === "number"
  );
}

export function useLocalCache(
  enabled: boolean,
  onCacheCleared?: (scope: CacheClearScope) => void
) {
  const [cacheInfo, setCacheInfo] = useState<LocalCacheInfo | null>(null);
  const [loading, setLoading] = useState(false);
  const [clearing, setClearing] = useState(false);
  const [status, setStatus] = useState<LocalCacheStatus>("idle");
  const isMountedRef = useRef(true);

  useEffect(() => {
    isMountedRef.current = true;
    return () => {
      isMountedRef.current = false;
    };
  }, []);

  const refresh = useCallback(async () => {
    if (!assetsIpc.isTauriRuntime()) return;
    setLoading(true);
    try {
      const info = await settingsIpc.getLocalCacheInfo();
      if (!isMountedRef.current) return;
      if (isValidLocalCacheInfo(info)) {
        setCacheInfo(info);
      } else {
        setCacheInfo(EMPTY_CACHE_INFO);
      }
    } catch {
      if (isMountedRef.current) {
        setStatus("error");
      }
    } finally {
      if (isMountedRef.current) {
        setLoading(false);
      }
    }
  }, []);

  useEffect(() => {
    if (!enabled) return;
    setStatus("idle");
    void refresh();
  }, [enabled, refresh]);

  const clearCache = useCallback(
    async (scope: CacheClearScope) => {
      if (clearing) return;
      setClearing(true);
      setStatus("idle");
      try {
        const info = await settingsIpc.clearLocalCache(scope);
        onCacheCleared?.(scope);
        if (!isMountedRef.current) return;
        setCacheInfo(isValidLocalCacheInfo(info) ? info : EMPTY_CACHE_INFO);
        setStatus("cleared");
      } catch {
        if (isMountedRef.current) {
          setStatus("error");
        }
      } finally {
        if (isMountedRef.current) {
          setClearing(false);
        }
      }
    },
    [clearing, onCacheCleared]
  );

  return {
    cacheInfo,
    loading,
    clearing,
    status,
    refresh,
    clearCache,
  };
}
