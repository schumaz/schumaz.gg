import { check, type DownloadEvent, type Update } from "@tauri-apps/plugin-updater";
import { openUrl } from "@tauri-apps/plugin-opener";
import { commands, type AppDistributionInfo } from "../../types/bindings";
import { isTauriRuntime } from "./assets.ipc";

export type InstallOptions = Parameters<Update["install"]>[0];
export type { AppDistributionInfo, DownloadEvent };

export interface UpdaterUpdateHandle {
  version: string;
  currentVersion: string;
  date?: string;
  body?: string;
  download: (onEvent?: (event: DownloadEvent) => void) => Promise<void>;
  install: (options?: InstallOptions) => Promise<void>;
  close: () => Promise<void>;
}

/**
 * Typed IPC adapter for Tauri v2 background updates (`@tauri-apps/plugin-updater`),
 * portable runtime detection, and external release URL opening (`@tauri-apps/plugin-opener`).
 */
export const updaterIpc = {
  async getDistributionInfo(): Promise<AppDistributionInfo | null> {
    if (!isTauriRuntime() || typeof commands?.getAppDistributionInfo !== "function") {
      return null;
    }
    try {
      return await commands.getAppDistributionInfo();
    } catch {
      return null;
    }
  },

  async checkForUpdate(): Promise<UpdaterUpdateHandle | null> {
    if (!isTauriRuntime()) {
      return null;
    }
    const update = await check();
    if (!update) {
      return null;
    }
    return {
      version: update.version,
      currentVersion: update.currentVersion,
      date: update.date,
      body: update.body,
      download: (onEvent) => update.download(onEvent),
      install: (options) => update.install(options),
      close: () => update.close(),
    };
  },

  async openReleasesPage(url: string): Promise<void> {
    if (!isTauriRuntime()) {
      return;
    }
    await openUrl(url);
  },
};
