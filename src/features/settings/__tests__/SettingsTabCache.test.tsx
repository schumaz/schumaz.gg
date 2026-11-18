import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import { I18nProvider } from "../../../i18n";
import { SettingsTab } from "../SettingsTab";
import { formatCacheBytes } from "../SettingsCacheSubsection";
import { DEFAULT_PREFERENCES } from "../../../types";

vi.mock("@tauri-apps/api/core", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@tauri-apps/api/core")>();
  return {
    ...actual,
    invoke: vi.fn(),
  };
});

describe("SettingsTab — Local Cache subsection", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    delete (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
  });

  afterEach(() => {
    delete (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
  });

  it("formats cache byte sizes across B, KB, MB, and GB", () => {
    expect(formatCacheBytes(0)).toBe("0 B");
    expect(formatCacheBytes(512)).toBe("512 B");
    expect(formatCacheBytes(1536)).toBe("1.5 KB");
    expect(formatCacheBytes(15 * 1024 * 1024)).toBe("15.0 MB");
    expect(formatCacheBytes(1024 * 1024 * 1024)).toBe("1.0 GB");
  });

  it("requires confirmation for all three cache clear options and supports back/escape", () => {
    const onClear = vi.fn();

    const { rerender } = render(
      <SettingsTab
        preferences={DEFAULT_PREFERENCES}
        onUpdatePreference={vi.fn()}
        cacheInfo={{
          totalBytes: 15 * 1024 * 1024,
          assetsBytes: 12 * 1024 * 1024,
          dataBytes: 3 * 1024 * 1024,
          fileCount: 42,
        }}
        cacheStatus="idle"
        onClearCache={onClear}
      />
    );

    const cacheGroup = screen.getByTestId("settings-subgroup-cache");
    expect(cacheGroup).toHaveTextContent("Cache Local");
    expect(screen.getByTestId("settings-cache-size")).toHaveTextContent("Espaço ocupado: 15.0 MB");
    expect(screen.queryByTestId("settings-cache-files")).not.toBeInTheDocument();
    expect(screen.getByTestId("settings-cache-status")).toHaveTextContent(
      "Imagens: 12.0 MB · Builds e dados: 3.0 MB"
    );

    const clearBtn = screen.getByTestId("settings-cache-clear-btn");
    expect(clearBtn).toBeEnabled();
    expect(clearBtn).toHaveTextContent("Limpar cache");
    fireEvent.click(clearBtn);
    expect(onClear).not.toHaveBeenCalled();

    const modal = screen.getByTestId("clear-cache-modal");
    expect(modal).toBeInTheDocument();
    expect(screen.getByTestId("clear-cache-option-builds")).toHaveTextContent("Recomendado");
    expect(screen.getByTestId("clear-cache-option-images")).toHaveTextContent("Não recomendado");
    expect(screen.getByTestId("clear-cache-option-all")).toBeEnabled();

    fireEvent.click(screen.getByTestId("clear-cache-cancel-btn"));
    expect(screen.queryByTestId("clear-cache-modal")).not.toBeInTheDocument();
    expect(onClear).not.toHaveBeenCalled();

    fireEvent.click(clearBtn);
    fireEvent.keyDown(window, { key: "Escape" });
    expect(screen.queryByTestId("clear-cache-modal")).not.toBeInTheDocument();

    fireEvent.click(clearBtn);
    fireEvent.click(screen.getByTestId("clear-cache-modal-backdrop"));
    expect(screen.queryByTestId("clear-cache-modal")).not.toBeInTheDocument();

    // Option 1: builds -> confirm prompt -> back -> Escape
    fireEvent.click(clearBtn);
    fireEvent.click(screen.getByTestId("clear-cache-option-builds"));
    expect(onClear).not.toHaveBeenCalled();
    expect(screen.getByTestId("clear-cache-confirm-prompt")).toHaveTextContent(
      "Confirmar exclusão do cache?"
    );
    expect(screen.getByTestId("clear-cache-confirm-prompt")).toHaveTextContent("3.0 MB");
    fireEvent.click(screen.getByTestId("clear-cache-confirm-back-btn"));
    expect(screen.queryByTestId("clear-cache-confirm-prompt")).not.toBeInTheDocument();
    expect(screen.getByTestId("clear-cache-option-builds")).toBeInTheDocument();

    // Option 2: images -> confirm prompt -> Escape returns to options -> confirm executes
    fireEvent.click(screen.getByTestId("clear-cache-option-images"));
    expect(onClear).not.toHaveBeenCalled();
    expect(screen.getByTestId("clear-cache-confirm-prompt")).toHaveTextContent("12.0 MB");
    fireEvent.keyDown(window, { key: "Escape" });
    expect(screen.queryByTestId("clear-cache-confirm-prompt")).not.toBeInTheDocument();
    expect(screen.getByTestId("clear-cache-modal")).toBeInTheDocument();

    fireEvent.click(screen.getByTestId("clear-cache-option-images"));
    fireEvent.click(screen.getByTestId("clear-cache-confirm-btn"));
    expect(screen.queryByTestId("clear-cache-modal")).not.toBeInTheDocument();
    expect(onClear).toHaveBeenCalledWith("images");

    // Option 1 confirmed
    fireEvent.click(clearBtn);
    fireEvent.click(screen.getByTestId("clear-cache-option-builds"));
    fireEvent.click(screen.getByTestId("clear-cache-confirm-btn"));
    expect(onClear).toHaveBeenCalledWith("builds");

    rerender(
      <SettingsTab
        preferences={DEFAULT_PREFERENCES}
        onUpdatePreference={vi.fn()}
        cacheInfo={{ totalBytes: 0, assetsBytes: 0, dataBytes: 0, fileCount: 0 }}
        cacheStatus="cleared"
        onClearCache={onClear}
      />
    );
    expect(screen.getByTestId("settings-cache-size")).toHaveTextContent("Espaço ocupado: 0 B");
    expect(screen.getByTestId("settings-cache-status")).toHaveTextContent(
      "Cache local limpo com sucesso."
    );
    expect(screen.getByTestId("settings-cache-clear-btn")).toBeDisabled();

    rerender(
      <I18nProvider locale="en_US">
        <SettingsTab
          preferences={{ ...DEFAULT_PREFERENCES, language: "en_US" }}
          onUpdatePreference={vi.fn()}
          locale="en_US"
          cacheInfo={{ totalBytes: 2048, assetsBytes: 2048, dataBytes: 0, fileCount: 1 }}
          cacheClearing={false}
          cacheStatus="error"
          onClearCache={onClear}
        />
      </I18nProvider>
    );
    expect(screen.getByTestId("settings-subgroup-cache")).toHaveTextContent("Local Cache");
    expect(screen.getByTestId("settings-cache-status")).toHaveTextContent(
      "Could not inspect or clear the local cache."
    );
    fireEvent.click(screen.getByTestId("settings-cache-clear-btn"));
    expect(screen.getByTestId("clear-cache-option-builds")).toBeDisabled();
    expect(screen.getByTestId("clear-cache-option-images")).toBeEnabled();
    expect(screen.getByTestId("clear-cache-option-images")).toHaveTextContent("Not recommended");
    expect(screen.getByTestId("clear-cache-option-builds")).toHaveTextContent("Recommended");

    // Option 3: all -> confirm prompt (EN)
    fireEvent.click(screen.getByTestId("clear-cache-option-all"));
    expect(onClear).not.toHaveBeenCalledWith("all");
    expect(screen.getByTestId("clear-cache-confirm-prompt")).toHaveTextContent(
      "Confirm cache deletion?"
    );
    expect(screen.getByTestId("clear-cache-confirm-prompt")).toHaveTextContent("2.0 KB");
    fireEvent.click(screen.getByTestId("clear-cache-confirm-btn"));
    expect(onClear).toHaveBeenCalledWith("all");

    rerender(
      <I18nProvider locale="en_US">
        <SettingsTab
          preferences={{ ...DEFAULT_PREFERENCES, language: "en_US" }}
          onUpdatePreference={vi.fn()}
          locale="en_US"
          cacheInfo={{ totalBytes: 2048, assetsBytes: 2048, dataBytes: 0, fileCount: 1 }}
          cacheClearing={true}
          cacheStatus="idle"
          onClearCache={onClear}
        />
      </I18nProvider>
    );
    expect(screen.getByTestId("settings-cache-clear-btn")).toHaveTextContent("Clearing...");
    expect(screen.getByTestId("settings-cache-clear-btn")).toBeDisabled();
  });

  it("loads local cache usage via IPC and clears selected scope after confirmation", async () => {
    (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ = {};
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === "get_local_cache_info") {
        return {
          totalBytes: 5 * 1024 * 1024,
          assetsBytes: 4 * 1024 * 1024,
          dataBytes: 1024 * 1024,
          fileCount: 18,
        };
      }
      if (cmd === "clear_local_cache") {
        return {
          totalBytes: 4 * 1024 * 1024,
          assetsBytes: 4 * 1024 * 1024,
          dataBytes: 0,
          fileCount: 12,
        };
      }
      return null;
    });

    render(
      <SettingsTab
        preferences={DEFAULT_PREFERENCES}
        onUpdatePreference={vi.fn()}
        isActive={true}
      />
    );

    await waitFor(() => {
      expect(screen.getByTestId("settings-cache-size")).toHaveTextContent("Espaço ocupado: 5.0 MB");
    });
    expect(invoke).toHaveBeenCalledWith("get_local_cache_info");

    const clearBtn = screen.getByTestId("settings-cache-clear-btn");
    expect(clearBtn).toBeEnabled();
    fireEvent.click(clearBtn);
    fireEvent.click(screen.getByTestId("clear-cache-option-builds"));
    expect(invoke).not.toHaveBeenCalledWith("clear_local_cache", { scope: "builds" });
    fireEvent.click(screen.getByTestId("clear-cache-confirm-btn"));

    await waitFor(() => {
      expect(screen.getByTestId("settings-cache-size")).toHaveTextContent("Espaço ocupado: 4.0 MB");
    });
    expect(invoke).toHaveBeenCalledWith("clear_local_cache", { scope: "builds" });
    expect(screen.getByTestId("settings-cache-status")).toHaveTextContent(
      "Cache local limpo com sucesso."
    );
    expect(screen.getByTestId("settings-cache-clear-btn")).toBeEnabled();
  });
});
