import { useCallback, useEffect, useRef, useState } from "react";
import type { AutostartStatus } from "../../types";
import { assetsIpc, settingsIpc } from "../../services/ipc";

function isValidAutostartStatus(value: unknown): value is AutostartStatus {
  if (!value || typeof value !== "object") return false;
  const candidate = value as Record<string, unknown>;
  return (
    typeof candidate.enabled === "boolean" &&
    typeof candidate.supported === "boolean" &&
    typeof candidate.isPortable === "boolean"
  );
}

/**
 * Lazily queries and updates Windows autostart state only while SettingsTab is active.
 */
export function useAutostart(enabled: boolean) {
  const [status, setStatus] = useState<AutostartStatus | null>(null);
  const [loading, setLoading] = useState(false);
  const [updating, setUpdating] = useState(false);
  const [error, setError] = useState(false);
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
    setError(false);
    try {
      const next = await settingsIpc.getAutostartStatus();
      if (!isMountedRef.current) return;
      if (isValidAutostartStatus(next)) {
        setStatus(next);
      }
    } catch {
      if (isMountedRef.current) {
        setError(true);
      }
    } finally {
      if (isMountedRef.current) {
        setLoading(false);
      }
    }
  }, []);

  useEffect(() => {
    if (!enabled) return;
    void refresh();
  }, [enabled, refresh]);

  const setAutostartEnabled = useCallback(
    async (nextEnabled: boolean) => {
      if (updating) return;
      setUpdating(true);
      setError(false);
      try {
        const updated = await settingsIpc.setAutostartEnabled(nextEnabled);
        if (!isMountedRef.current) return;
        if (isValidAutostartStatus(updated)) {
          setStatus(updated);
        }
      } catch {
        if (isMountedRef.current) {
          setError(true);
        }
      } finally {
        if (isMountedRef.current) {
          setUpdating(false);
        }
      }
    },
    [updating]
  );

  return {
    status,
    loading,
    updating,
    error,
    refresh,
    setAutostartEnabled,
  };
}
