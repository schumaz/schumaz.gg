import { commands, type DiagnosticSummary_Serialize } from "../../types/bindings";

export type DiagnosticSummary = DiagnosticSummary_Serialize;

/**
 * Typed IPC adapter for sanitized local diagnostics export and opening the local `Logs/` folder.
 */
export const diagnosticsIpc = {
  async getDiagnosticSummary(): Promise<DiagnosticSummary> {
    return commands.getDiagnosticSummary();
  },

  async openLogsFolder(): Promise<void> {
    await commands.openLogsFolder();
  },
};
