import { convertFileSrc } from "@tauri-apps/api/core";

export const SCHUMAZ_ASSET_PROTOCOL = "schumaz-asset";

/**
 * Returns true when running inside the Tauri WebView runtime.
 */
export function isTauriRuntime(): boolean {
  if (typeof window === "undefined") {
    return false;
  }
  return Boolean((window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__);
}

/**
 * Typed IPC adapter for Tauri custom protocol asset resolution (`schumaz-asset://`).
 */
export const assetsIpc = {
  isTauriRuntime,

  convertAssetFileSrc(segment: string, protocol = SCHUMAZ_ASSET_PROTOCOL): string {
    return convertFileSrc(segment, protocol);
  },
};
