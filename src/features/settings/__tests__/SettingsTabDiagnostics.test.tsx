import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import { I18nProvider } from "../../../i18n";
import { SettingsTab } from "../SettingsTab";
import { DEFAULT_PREFERENCES, type DiagnosticSummary } from "../../../types";

vi.mock("@tauri-apps/api/core", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@tauri-apps/api/core")>();
  return {
    ...actual,
    invoke: vi.fn(),
  };
});

const SAMPLE_SUMMARY: DiagnosticSummary = {
  appVersion: "0.4.0",
  distributionMode: "NSIS",
  osVersion: "Windows 11 (22631)",
  architecture: "x86_64",
  locale: "pt_BR",
  gpuSaved: true,
  gpuActive: true,
  closeToTray: true,
  lcuConnected: false,
  gameflowPhase: "None",
  ddragonVersion: "16.22.1",
  activePatch: "16.22",
  recentIssues: [],
  formattedText: "schumaz.gg diagnostic summary\nApp: v0.4.0 (NSIS)",
};

describe("SettingsTab — Support & Diagnostics subsection", () => {
  const originalClipboard = navigator.clipboard;

  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    Object.defineProperty(navigator, "clipboard", {
      value: originalClipboard,
      configurable: true,
    });
  });

  it("renders Support & Diagnostics subsection in pt-BR and copies formatted summary with role=status feedback", async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", {
      value: { writeText },
      configurable: true,
    });

    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === "get_diagnostic_summary") return SAMPLE_SUMMARY;
      return null;
    });

    render(
      <I18nProvider locale="pt_BR">
        <SettingsTab
          preferences={{ ...DEFAULT_PREFERENCES, language: "pt_BR" }}
          onUpdatePreference={vi.fn()}
          cacheInfo={{ totalBytes: 0, assetsBytes: 0, dataBytes: 0, fileCount: 0 }}
        />
      </I18nProvider>
    );

    const group = screen.getByTestId("settings-subgroup-diagnostics");
    expect(group).toHaveTextContent("Suporte e Diagnóstico");
    expect(group).not.toHaveTextContent(/LeagueClientUx|OP\.GG/i);

    const copyBtn = screen.getByTestId("settings-diagnostics-copy-btn");
    expect(copyBtn).toHaveTextContent("Copiar resumo de diagnóstico");
    fireEvent.click(copyBtn);

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("get_diagnostic_summary");
      expect(writeText).toHaveBeenCalledWith(SAMPLE_SUMMARY.formattedText);
      const status = screen.getByTestId("settings-diagnostics-status");
      expect(status).toHaveAttribute("role", "status");
      expect(status).toHaveTextContent("Resumo copiado!");
      expect(status).toHaveClass("is-success");
    });
  });

  it("shows accessible error feedback when diagnostic summary copy fails (IPC or clipboard)", async () => {
    const writeText = vi.fn().mockRejectedValue(new Error("Clipboard blocked"));
    Object.defineProperty(navigator, "clipboard", {
      value: { writeText },
      configurable: true,
    });

    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === "get_diagnostic_summary") return SAMPLE_SUMMARY;
      return null;
    });

    render(
      <I18nProvider locale="pt_BR">
        <SettingsTab
          preferences={{ ...DEFAULT_PREFERENCES, language: "pt_BR" }}
          onUpdatePreference={vi.fn()}
          cacheInfo={{ totalBytes: 0, assetsBytes: 0, dataBytes: 0, fileCount: 0 }}
        />
      </I18nProvider>
    );

    fireEvent.click(screen.getByTestId("settings-diagnostics-copy-btn"));

    await waitFor(() => {
      const status = screen.getByTestId("settings-diagnostics-status");
      expect(status).toHaveTextContent("Não foi possível copiar o resumo de diagnóstico.");
      expect(status).toHaveClass("is-error");
    });

    vi.mocked(invoke).mockRejectedValueOnce(new Error("IPC failed"));
    fireEvent.click(screen.getByTestId("settings-diagnostics-copy-btn"));

    await waitFor(() => {
      expect(screen.getByTestId("settings-diagnostics-status")).toHaveClass("is-error");
    });
  });

  it("opens logs folder via backend IPC and reports folder errors cleanly", async () => {
    vi.mocked(invoke).mockResolvedValue(null);

    render(
      <I18nProvider locale="pt_BR">
        <SettingsTab
          preferences={{ ...DEFAULT_PREFERENCES, language: "pt_BR" }}
          onUpdatePreference={vi.fn()}
          cacheInfo={{ totalBytes: 0, assetsBytes: 0, dataBytes: 0, fileCount: 0 }}
        />
      </I18nProvider>
    );

    const openBtn = screen.getByTestId("settings-diagnostics-open-logs-btn");
    expect(openBtn).toHaveTextContent("Abrir pasta de logs");
    fireEvent.click(openBtn);

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("open_logs_folder");
    });

    vi.mocked(invoke).mockRejectedValueOnce(new Error("Explorer failed"));
    fireEvent.click(openBtn);

    await waitFor(() => {
      const status = screen.getByTestId("settings-diagnostics-status");
      expect(status).toHaveTextContent("Não foi possível abrir a pasta de logs.");
      expect(status).toHaveClass("is-error");
    });
  });

  it("renders bilingual en-US labels and status feedback without exposing external providers", async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", {
      value: { writeText },
      configurable: true,
    });
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === "get_diagnostic_summary") return SAMPLE_SUMMARY;
      return null;
    });

    render(
      <I18nProvider locale="en_US">
        <SettingsTab
          preferences={{ ...DEFAULT_PREFERENCES, language: "en_US" }}
          onUpdatePreference={vi.fn()}
          cacheInfo={{ totalBytes: 0, assetsBytes: 0, dataBytes: 0, fileCount: 0 }}
        />
      </I18nProvider>
    );

    const group = screen.getByTestId("settings-subgroup-diagnostics");
    expect(group).toHaveTextContent("Support & Diagnostics");
    expect(group).not.toHaveTextContent(/LeagueClientUx|OP\.GG/i);
    expect(screen.getByTestId("settings-diagnostics-copy-btn")).toHaveTextContent(
      "Copy diagnostic summary"
    );
    expect(screen.getByTestId("settings-diagnostics-open-logs-btn")).toHaveTextContent(
      "Open logs folder"
    );

    fireEvent.click(screen.getByTestId("settings-diagnostics-copy-btn"));

    await waitFor(() => {
      expect(screen.getByTestId("settings-diagnostics-status")).toHaveTextContent(
        "Summary copied!"
      );
    });
  });
});
