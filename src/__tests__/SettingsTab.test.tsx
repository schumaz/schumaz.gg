import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { SettingsTab, isBlurSliderDisabled } from "../components/SettingsTab";
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

  it("renders 10% default blur slider, clickable/editable percentage number, wallpaper toggle, and Perfil -> Runas order", () => {
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

    // Default blur is 10%
    const slider = screen.getByTestId("profile-blur-slider") as HTMLInputElement;
    expect(slider.min).toBe("0");
    expect(slider.max).toBe("100");
    expect(slider.value).toBe("10");
    expect(slider).toBeEnabled();

    const valueBtn = screen.getByTestId("profile-blur-value");
    expect(valueBtn).toHaveTextContent("10%");
    expect(valueBtn).toBeEnabled();

    // Slider change works
    fireEvent.change(slider, { target: { value: "80" } });
    expect(handleBlurChange).toHaveBeenCalledWith(80);

    // Clicking the percentage number opens editable input
    fireEvent.click(valueBtn);
    const numInput = screen.getByTestId("profile-blur-input") as HTMLInputElement;
    expect(numInput).toHaveFocus();
    fireEvent.change(numInput, { target: { value: "65" } });
    expect(handleBlurChange).toHaveBeenCalledWith(65);
    fireEvent.keyDown(numInput, { key: "Enter" });
    expect(screen.queryByTestId("profile-blur-input")).not.toBeInTheDocument();

    // Fundo Visível toggle sits below the blur slider inside Aparência do Perfil
    const wpToggle = screen.getByTestId("wallpaper-visible-toggle");
    expect(wpToggle).toBeChecked();
    fireEvent.click(wpToggle);
    expect(handleWallpaperToggle).toHaveBeenCalledWith(false);

    // Check subgroup hierarchy & exact order
    const groups = Array.from(container.querySelectorAll(".settings-section-group"));
    expect(groups).toHaveLength(2);
    expect(groups[0]).toHaveAttribute("data-testid", "settings-group-profile");
    expect(groups[1]).toHaveAttribute("data-testid", "settings-group-runes");

    const cards = Array.from(container.querySelectorAll(".settings-group-card")).map((el) =>
      el.getAttribute("data-testid")
    );
    expect(cards).toEqual([
      "settings-subgroup-appearance",
      "settings-subgroup-flash",
      "settings-subgroup-auto-inject",
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

    // Even if a change event is dispatched, the guard prevents updating blur
    fireEvent.change(slider, { target: { value: "70" } });
    fireEvent.click(valueBtn);
    expect(handleBlurChange).not.toHaveBeenCalled();
    expect(screen.queryByTestId("profile-blur-input")).not.toBeInTheDocument();

    // Re-enabling Fundo Visível unlocks the slider and exits/allows editing cleanly
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

    // Start editing while visible, then uncheck Fundo Visível -> edit closes cleanly
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
