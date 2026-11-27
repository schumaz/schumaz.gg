import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  commands,
  events,
  type BootstrapProgress,
  type BootstrapStatus,
  type WindowMinimizedChanged,
} from "../../types/bindings";
import type { UnlistenFn } from "./lcu.ipc";

export function isTauriWindowAvailable(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

/**
 * Typed IPC adapter for Tauri window state events, native controls, and app lifecycle commands.
 */
export const windowIpc = {
  async getBootstrapStatus(): Promise<BootstrapStatus | null> {
    if (typeof commands?.getBootstrapStatus !== "function") {
      return null;
    }
    return commands.getBootstrapStatus();
  },

  async onBootstrapProgress(
    handler: (status: BootstrapProgress) => void
  ): Promise<UnlistenFn> {
    const listener = events?.bootstrapProgress?.listen;
    if (typeof listener !== "function") {
      return () => {};
    }
    return listener((event) => {
      if (event?.payload) {
        handler(event.payload);
      }
    });
  },

  async onWindowMinimizedChanged(
    handler: (minimized: WindowMinimizedChanged) => void
  ): Promise<UnlistenFn> {
    const listener = events?.windowMinimizedChanged?.listen;
    if (typeof listener !== "function") {
      return () => {};
    }
    return listener((event) => {
      handler(Boolean(event.payload));
    });
  },

  async isWindowFocused(): Promise<boolean | null> {
    if (!isTauriWindowAvailable()) {
      return null;
    }
    try {
      return await getCurrentWindow().isFocused();
    } catch {
      return null;
    }
  },

  async isWindowMinimized(): Promise<boolean | null> {
    if (!isTauriWindowAvailable()) {
      return null;
    }
    try {
      return await getCurrentWindow().isMinimized();
    } catch {
      return null;
    }
  },

  async onWindowFocusChanged(handler: (focused: boolean) => void): Promise<UnlistenFn> {
    if (!isTauriWindowAvailable()) {
      return () => {};
    }
    try {
      return await getCurrentWindow().onFocusChanged(({ payload }) => {
        handler(Boolean(payload));
      });
    } catch {
      return () => {};
    }
  },

  async isWindowMaximized(): Promise<boolean> {
    if (!isTauriWindowAvailable()) {
      return false;
    }
    try {
      return await getCurrentWindow().isMaximized();
    } catch {
      return false;
    }
  },

  async minimizeWindow(): Promise<void> {
    if (!isTauriWindowAvailable()) {
      return;
    }
    await getCurrentWindow().minimize();
  },

  async toggleMaximizeWindow(): Promise<void> {
    if (!isTauriWindowAvailable()) {
      return;
    }
    await getCurrentWindow().toggleMaximize();
  },

  async closeWindow(): Promise<void> {
    if (!isTauriWindowAvailable()) {
      return;
    }
    await getCurrentWindow().close();
  },

  async restartApp(): Promise<void> {
    return commands.restartApp();
  },
};
