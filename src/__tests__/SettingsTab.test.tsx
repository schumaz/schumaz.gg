import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { SettingsTab, isBlurSliderDisabled } from "../features/settings/SettingsTab";
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

  it("shows confirmation popup only when clicking to disable 48h patch protection, supporting Reverter and Aceitar desativação", () => {
    const handleUpdate = vi.fn();

    const { rerender } = render(
      <SettingsTab
        preferences={DEFAULT_PREFERENCES}
        onUpdatePreference={handleUpdate}
      />
    );

    const toggle = screen.getByTestId("patch-48h-protection-toggle");
    expect(toggle).toBeChecked();
    expect(screen.queryByTestId("patch-48h-protection-warning")).not.toBeInTheDocument();

    fireEvent.click(toggle);
    expect(handleUpdate).not.toHaveBeenCalled();

    const popup = screen.getByTestId("patch-48h-protection-warning");
    expect(popup).toBeInTheDocument();
    expect(popup).toHaveAttribute("role", "dialog");
    expect(popup).toHaveTextContent(/NÃO é recomendado/i);
    expect(popup).toHaveTextContent(/baixo volume de dados no início do patch/i);

    fireEvent.click(screen.getByTestId("patch-48h-revert-btn"));
    expect(screen.queryByTestId("patch-48h-protection-warning")).not.toBeInTheDocument();
    expect(handleUpdate).not.toHaveBeenCalled();
    expect(toggle).toBeChecked();

    fireEvent.click(toggle);
    expect(screen.getByTestId("patch-48h-protection-warning")).toBeInTheDocument();
    fireEvent.click(screen.getByTestId("patch-48h-accept-btn"));
    expect(screen.queryByTestId("patch-48h-protection-warning")).not.toBeInTheDocument();
    expect(handleUpdate).toHaveBeenCalledWith("patch48hProtection", false);

    rerender(
      <SettingsTab
        preferences={{ ...DEFAULT_PREFERENCES, patch48hProtection: false }}
        onUpdatePreference={handleUpdate}
      />
    );

    const disabledToggle = screen.getByTestId("patch-48h-protection-toggle");
    expect(disabledToggle).not.toBeChecked();
    expect(screen.queryByTestId("patch-48h-protection-warning")).not.toBeInTheDocument();

    fireEvent.click(disabledToggle);
    expect(screen.queryByTestId("patch-48h-protection-warning")).not.toBeInTheDocument();
    expect(handleUpdate).toHaveBeenCalledWith("patch48hProtection", true);
  });

  it("organizes settings into 4 semantic groups with expected subsections and profile blur controls", () => {
    const handleBlurChange = vi.fn();
    const handleWallpaperToggle = vi.fn();

    const { container } = render(
      <SettingsTab
        preferences={DEFAULT_PREFERENCES}
        onUpdatePreference={vi.fn()}
        onProfileBlurChange={handleBlurChange}
        wallpaperVisible={true}
        onWallpaperVisibleChange={handleWallpaperToggle}
      />
    );

    const slider = screen.getByTestId("profile-blur-slider") as HTMLInputElement;
    expect(slider.min).toBe("0");
    expect(slider.max).toBe("100");
    expect(slider.value).toBe("10");
    expect(slider).toBeEnabled();

    const valueBtn = screen.getByTestId("profile-blur-value");
    expect(valueBtn).toHaveTextContent("10%");
    expect(valueBtn).toBeEnabled();

    fireEvent.change(slider, { target: { value: "80" } });
    expect(handleBlurChange).toHaveBeenCalledWith(80);

    fireEvent.click(valueBtn);
    const numInput = screen.getByTestId("profile-blur-input") as HTMLInputElement;
    expect(numInput).toHaveFocus();
    fireEvent.change(numInput, { target: { value: "65" } });
    expect(handleBlurChange).toHaveBeenCalledWith(65);
    fireEvent.keyDown(numInput, { key: "Enter" });
    expect(screen.queryByTestId("profile-blur-input")).not.toBeInTheDocument();

    const wpToggle = screen.getByTestId("wallpaper-visible-toggle");
    expect(wpToggle).toBeChecked();
    fireEvent.click(wpToggle);
    expect(handleWallpaperToggle).toHaveBeenCalledWith(false);

    const groups = Array.from(container.querySelectorAll(".settings-section-group")).map((el) =>
      el.getAttribute("data-testid")
    );
    expect(groups).toEqual([
      "settings-group-general",
      "settings-group-automation",
      "settings-group-appearance",
      "settings-group-system",
    ]);

    const cards = Array.from(container.querySelectorAll(".settings-group-card")).map((el) =>
      el.getAttribute("data-testid")
    );
    expect(cards).toEqual([
      "settings-subgroup-general",
      "settings-subgroup-automation",
      "settings-subgroup-appearance",
      "settings-subgroup-system",
    ]);

    const subsectionsOf = (cardTestId: string) =>
      Array.from(screen.getByTestId(cardTestId).querySelectorAll(".settings-subsection")).map((el) =>
        el.getAttribute("data-testid")
      );

    expect(subsectionsOf("settings-subgroup-general")).toEqual([
      "settings-subgroup-language",
      "settings-subgroup-startup-tray",
    ]);
    expect(subsectionsOf("settings-subgroup-automation")).toEqual([
      "settings-subgroup-flash",
      "settings-subgroup-auto-inject",
      "settings-subgroup-patch-protection",
      "settings-subgroup-daily-missions",
    ]);
    expect(subsectionsOf("settings-subgroup-appearance")).toEqual([
      "settings-subgroup-profile-appearance",
      "settings-subgroup-graphics",
    ]);
    expect(subsectionsOf("settings-subgroup-system")).toEqual([
      "settings-subgroup-cache",
      "settings-subgroup-updates",
    ]);
  });

  it("blocks blur slider and percentage editing when Fundo Visível is unchecked", () => {
    expect(isBlurSliderDisabled(false)).toBe(true);
    expect(isBlurSliderDisabled(true)).toBe(false);

    const handleBlurChange = vi.fn();

    const { rerender } = render(
      <SettingsTab
        preferences={DEFAULT_PREFERENCES}
        onUpdatePreference={vi.fn()}
        profileBlur={25}
        onProfileBlurChange={handleBlurChange}
        wallpaperVisible={false}
        onWallpaperVisibleChange={vi.fn()}
      />
    );

    const slider = screen.getByTestId("profile-blur-slider") as HTMLInputElement;
    const valueBtn = screen.getByTestId("profile-blur-value");

    expect(slider).toBeDisabled();
    expect(valueBtn).toBeDisabled();
    expect(
      screen.getByText("Ative a opção Fundo Visível para ajustar o desfoque.")
    ).toBeInTheDocument();

    fireEvent.change(slider, { target: { value: "70" } });
    fireEvent.click(valueBtn);
    expect(handleBlurChange).not.toHaveBeenCalled();
    expect(screen.queryByTestId("profile-blur-input")).not.toBeInTheDocument();

    rerender(
      <SettingsTab
        preferences={DEFAULT_PREFERENCES}
        onUpdatePreference={vi.fn()}
        profileBlur={25}
        onProfileBlurChange={handleBlurChange}
        wallpaperVisible={true}
        onWallpaperVisibleChange={vi.fn()}
      />
    );

    expect(screen.getByTestId("profile-blur-slider")).toBeEnabled();
    expect(screen.getByTestId("profile-blur-value")).toBeEnabled();

    fireEvent.click(screen.getByTestId("profile-blur-value"));
    expect(screen.getByTestId("profile-blur-input")).toBeInTheDocument();

    rerender(
      <SettingsTab
        preferences={DEFAULT_PREFERENCES}
        onUpdatePreference={vi.fn()}
        profileBlur={25}
        onProfileBlurChange={handleBlurChange}
        wallpaperVisible={false}
        onWallpaperVisibleChange={vi.fn()}
      />
    );

    expect(screen.queryByTestId("profile-blur-input")).not.toBeInTheDocument();
    expect(screen.getByTestId("profile-blur-slider")).toBeDisabled();
  });
});
