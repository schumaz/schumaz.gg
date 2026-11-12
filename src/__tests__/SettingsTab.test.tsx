import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { SettingsTab } from "../components/SettingsTab";
import { DEFAULT_PREFERENCES } from "../types";

describe("SettingsTab", () => {
  it("renders preferences and handles toggling checkboxes", () => {
    const handleUpdate = vi.fn();

    render(
      <SettingsTab
        preferences={DEFAULT_PREFERENCES}
        onUpdatePreference={handleUpdate}
      />
    );

    const runesToggle = screen.getByLabelText("Injetar Runas Automaticamente");
    expect(runesToggle).toBeChecked();

    fireEvent.click(runesToggle);
    expect(handleUpdate).toHaveBeenCalledWith("autoInjectRunes", false);

    const spellsToggle = screen.getByLabelText("Injetar Feitiços de Invocador");
    expect(spellsToggle).toBeChecked();

    fireEvent.click(spellsToggle);
    expect(handleUpdate).toHaveBeenCalledWith("autoInjectSpells", false);
  });

  it("handles changing Flash key preference between D and F", () => {
    const handleUpdate = vi.fn();

    render(
      <SettingsTab
        preferences={DEFAULT_PREFERENCES}
        onUpdatePreference={handleUpdate}
      />
    );

    const flashFBtn = screen.getByRole("button", { name: "F" });
    fireEvent.click(flashFBtn);

    expect(handleUpdate).toHaveBeenCalledWith("flashKey", "F");
  });
});
