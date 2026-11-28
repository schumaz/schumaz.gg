import { useEffect } from "react";
import { settingsIpc } from "../services/ipc";
import type { CacheClearScope } from "../types";
import { clearAssetUrlCache } from "../utils/assets";

export interface UseCacheClearInvalidationOptions {
  onInvalidateBuilds: () => void;
}

/**
 * Subscribes to `cache-cleared` events (Tauri backend IPC + local settings fallback)
 * and flushes matching L1 RAM caches in the React frontend.
 */
export function useCacheClearInvalidation({
  onInvalidateBuilds,
}: UseCacheClearInvalidationOptions): void {
  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | null = null;

    const handleScope = (scope: CacheClearScope) => {
      if (scope === "builds" || scope === "all") {
        onInvalidateBuilds();
      }
      if (scope === "images" || scope === "all") {
        clearAssetUrlCache();
      }
    };

    void settingsIpc.onCacheCleared(handleScope).then((dispose) => {
      if (!active) {
        dispose();
        return;
      }
      unlisten = dispose;
    });

    return () => {
      active = false;
      unlisten?.();
    };
  }, [onInvalidateBuilds]);
}
