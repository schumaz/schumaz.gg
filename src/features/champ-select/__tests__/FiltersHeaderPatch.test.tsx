import { describe, it, expect, vi } from "vitest";
import { fireEvent, render, screen, within } from "@testing-library/react";
import { FiltersHeader } from "../FiltersHeader";
import { ChampionBuild, ChampSelectedPayload } from "../../../types";

const mockBuild: ChampionBuild = {
  champion_id: 103,
  champion_name: "Ahri",
  win_rate: 52.4,
  pick_rate: 8.5,
  primary_style_id: 8000,
  sub_style_id: 8100,
  selected_perk_ids: [8005, 9111, 9104, 8014, 8139, 8135, 5008, 5008, 5001],
  spell1_id: 4,
  spell2_id: 32,
  item_blocks: [],
};

const summonerRiftChampSelect: ChampSelectedPayload = {
  champion_id: 103,
  queue_id: 420,
  queue_mode: "SummonersRift",
  locked_in: true,
};

describe("FiltersHeader — Keyboard, Loading, and Patch Protection", () => {
  it("supports keyboard navigation (ArrowDown, ArrowUp, Home, End, Enter, Escape) and outside click", () => {
    const onFilterChange = vi.fn();
    render(
      <FiltersHeader
        build={{ ...mockBuild, active_position: "mid" }}
        champSelect={summonerRiftChampSelect}
        loading={false}
        onFilterChange={onFilterChange}
      />
    );

    const posTrigger = screen.getByTestId("filter-position");

    fireEvent.keyDown(posTrigger, { key: "ArrowDown" });
    expect(posTrigger).toHaveAttribute("aria-expanded", "true");
    expect(posTrigger).toHaveAttribute("aria-activedescendant", "filter-position-option-2");

    fireEvent.keyDown(posTrigger, { key: "ArrowDown" });
    expect(posTrigger).toHaveAttribute("aria-activedescendant", "filter-position-option-3");

    fireEvent.keyDown(posTrigger, { key: "End" });
    expect(posTrigger).toHaveAttribute("aria-activedescendant", "filter-position-option-4");

    fireEvent.keyDown(posTrigger, { key: "Home" });
    expect(posTrigger).toHaveAttribute("aria-activedescendant", "filter-position-option-0");

    fireEvent.keyDown(posTrigger, { key: "Escape" });
    expect(posTrigger).toHaveAttribute("aria-expanded", "false");
    expect(onFilterChange).not.toHaveBeenCalled();

    fireEvent.keyDown(posTrigger, { key: "Enter" });
    fireEvent.keyDown(posTrigger, { key: "ArrowDown" });
    fireEvent.keyDown(posTrigger, { key: "Enter" });
    expect(onFilterChange).toHaveBeenCalledWith("position", "adc");
    expect(posTrigger).toHaveAttribute("aria-expanded", "false");

    fireEvent.click(posTrigger);
    expect(posTrigger).toHaveAttribute("aria-expanded", "true");
    fireEvent.mouseDown(document.body);
    expect(posTrigger).toHaveAttribute("aria-expanded", "false");
  });

  it("disables filter dropdown triggers when loading is true", () => {
    render(
      <FiltersHeader
        build={mockBuild}
        champSelect={summonerRiftChampSelect}
        loading={true}
        onFilterChange={vi.fn()}
      />
    );

    const tierTrigger = screen.getByTestId("filter-tier");
    expect(tierTrigger).toBeDisabled();
    fireEvent.click(tierTrigger);
    expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
    expect(screen.getByTestId("filters-loading-bar")).toHaveAttribute("role", "progressbar");
  });

  it("renders custom Patch dropdown, labels the latest patch as '(Mais atual)', and hides 48h protection indicator when not active", () => {
    render(
      <FiltersHeader
        build={{
          ...mockBuild,
          active_patch: "16.22",
          available_patches: ["16.22", "16.21", "16.20"],
          patch_fallback_applied: false,
          patch_protection_remaining_hours: null,
        }}
        champSelect={summonerRiftChampSelect}
        loading={false}
        onFilterChange={vi.fn()}
      />
    );

    const patchTrigger = screen.getByTestId("filter-patch");
    expect(patchTrigger).toHaveAttribute("role", "combobox");
    expect(patchTrigger).toHaveAttribute("aria-haspopup", "listbox");
    expect(patchTrigger).toHaveAttribute("aria-expanded", "false");
    expect(patchTrigger).toHaveAttribute("data-value", "16.22");
    expect(patchTrigger).toHaveTextContent("Patch 16.22 (Mais atual)");

    fireEvent.click(patchTrigger);
    expect(patchTrigger).toHaveAttribute("aria-expanded", "true");

    const listbox = screen.getByRole("listbox", { name: "Patch:" });
    const options = within(listbox).getAllByRole("option");
    expect(options.map((opt) => opt.getAttribute("data-value"))).toEqual([
      "16.22",
      "16.21",
      "16.20",
    ]);
    expect(options.map((opt) => opt.textContent)).toEqual([
      "Patch 16.22 (Mais atual)",
      "Patch 16.21",
      "Patch 16.20",
    ]);

    expect(screen.queryByTestId("patch-protection-indicator")).not.toBeInTheDocument();
    expect(screen.queryByTestId("patch-fallback-badge")).not.toBeInTheDocument();
  });

  it("shows red '!' indicator, remaining hours, and tooltip on hover when 48h patch protection is active", () => {
    const onFilterChange = vi.fn();
    render(
      <FiltersHeader
        build={{
          ...mockBuild,
          active_patch: "16.21",
          available_patches: ["16.22", "16.21"],
          patch_fallback_applied: true,
          patch_protection_remaining_hours: 38,
        }}
        champSelect={summonerRiftChampSelect}
        loading={false}
        onFilterChange={onFilterChange}
      />
    );

    const indicator = screen.getByTestId("patch-protection-indicator");
    expect(indicator).toBeInTheDocument();
    expect(indicator).toHaveTextContent("!");

    expect(screen.getByTestId("patch-protection-hours")).toHaveTextContent("Faltam 38h");
    expect(screen.getByTestId("patch-fallback-badge")).toHaveTextContent(
      /Volume Protegido \(16\.21\).*Faltam 38h/i
    );

    expect(screen.queryByTestId("patch-protection-tooltip")).not.toBeInTheDocument();
    fireEvent.mouseEnter(indicator);

    const tooltip = screen.getByTestId("patch-protection-tooltip");
    expect(tooltip).toBeInTheDocument();
    expect(tooltip).toHaveAttribute("role", "tooltip");
    expect(tooltip).toHaveTextContent(/48h/i);
    expect(tooltip).toHaveTextContent(/integridade dos dados/i);
    expect(tooltip).toHaveTextContent(/escolha manual/i);
    expect(tooltip).toHaveTextContent(/Faltam 38 horas/i);

    fireEvent.mouseLeave(indicator);
    expect(screen.queryByTestId("patch-protection-tooltip")).not.toBeInTheDocument();

    const patchTrigger = screen.getByTestId("filter-patch");
    expect(patchTrigger).toHaveAttribute("data-value", "16.21");
    fireEvent.click(patchTrigger);
    const listbox = screen.getByRole("listbox", { name: "Patch:" });
    fireEvent.click(within(listbox).getByRole("option", { name: "Patch 16.22 (Mais atual)" }));
    expect(onFilterChange).toHaveBeenCalledWith("patch", "16.22");
  });
});
