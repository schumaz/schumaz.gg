import { commands, events, type WindowMinimizedChanged } from "../../types/bindings";
import type { UnlistenFn } from "./lcu.ipc";

/**
 * Typed IPC adapter for Tauri window state events and app lifecycle commands.
 */
export const windowIpc = {
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

  async restartApp(): Promise<void> {
    return commands.restartApp();
  },
};
