import { describe, it, expect, vi } from "vitest";
import { fireEvent, render, screen, within } from "@testing-library/react";
import { FiltersHeader } from "../components/FiltersHeader";
import { ChampionBuild, ChampSelectedPayload } from "../types";

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

describe("FiltersHeader", () => {
  it("renders ARAM Mayhem badge and hides Rota selector when queue_mode is AramMayhem", () => {
    const mockChampSelect: ChampSelectedPayload = {
      champion_id: 103,
      queue_id: 2400,
      queue_mode: "AramMayhem",
      locked_in: true,
    };

    render(
      <FiltersHeader
        build={mockBuild}
        champSelect={mockChampSelect}
        loading={false}
        onFilterChange={vi.fn()}
      />
    );

    expect(screen.getByText("ARAM Mayhem (Howling Abyss)")).toBeInTheDocument();
    expect(screen.queryByTestId("filter-position")).not.toBeInTheDocument();
    expect(screen.queryByLabelText("Rota:")).not.toBeInTheDocument();
    expect(screen.getByTestId("filter-tier")).toBeInTheDocument();
    expect(screen.getByTestId("filter-region")).toBeInTheDocument();
    expect(screen.getByTestId("filter-patch")).toBeInTheDocument();
  });

  it("renders traditional ARAM badge and hides Rota selector when queue_mode is Aram", () => {
    const mockChampSelect: ChampSelectedPayload = {
      champion_id: 103,
      queue_id: 450,
      queue_mode: "Aram",
      locked_in: true,
    };

    render(
      <FiltersHeader
        build={mockBuild}
        champSelect={mockChampSelect}
        loading={false}
        onFilterChange={vi.fn()}
      />
    );

    expect(screen.getByText("ARAM (Howling Abyss)")).toBeInTheDocument();
    expect(screen.queryByTestId("filter-position")).not.toBeInTheDocument();
    expect(screen.queryByLabelText("Rota:")).not.toBeInTheDocument();
  });

  it("renders Rota selector when queue_mode is SummonersRift", () => {
    render(
      <FiltersHeader
        build={{ ...mockBuild, active_position: "mid" }}
        champSelect={summonerRiftChampSelect}
        loading={false}
        onFilterChange={vi.fn()}
      />
    );

    expect(screen.getByTestId("filter-position")).toBeInTheDocument();
    expect(screen.getByLabelText("Rota:")).toBeInTheDocument();
  });

  it("renders custom Elo dropdown with official tier icons for all 11 options and calls onFilterChange", () => {
    const onFilterChange = vi.fn();
    render(
      <FiltersHeader
        build={{ ...mockBuild, active_tier: "emerald_plus" }}
        champSelect={summonerRiftChampSelect}
        loading={false}
        onFilterChange={onFilterChange}
      />
    );

    const tierTrigger = screen.getByTestId("filter-tier");
    expect(tierTrigger).toHaveAttribute("role", "combobox");
    expect(tierTrigger).toHaveAttribute("aria-haspopup", "listbox");
    expect(tierTrigger).toHaveAttribute("aria-expanded", "false");
    expect(tierTrigger).toHaveTextContent("Esmeralda +");

    const selectedImg = tierTrigger.querySelector("img");
    expect(selectedImg).toBeInTheDocument();
    expect(selectedImg).toHaveAttribute("src", expect.stringContaining("emerald.svg"));

    fireEvent.click(tierTrigger);
    expect(tierTrigger).toHaveAttribute("aria-expanded", "true");

    const listbox = screen.getByRole("listbox", { name: "Elo:" });
    const options = within(listbox).getAllByRole("option");
    expect(options.map((opt) => opt.getAttribute("data-value"))).toEqual([
      "all",
      "challenger",
      "grandmaster",
      "master_plus",
      "diamond_plus",
      "emerald_plus",
      "platinum_plus",
      "gold_plus",
      "silver",
      "bronze",
      "iron",
    ]);
    expect(options.map((opt) => opt.textContent)).toEqual([
      "Todos os Elos",
      "Desafiante",
      "Grão-Mestre",
      "Mestre +",
      "Diamante +",
      "Esmeralda +",
      "Platina +",
      "Ouro +",
      "Prata",
      "Bronze",
      "Ferro",
    ]);

    for (const option of options) {
      const img = option.querySelector("img");
      expect(img).toBeInTheDocument();
      expect(img?.getAttribute("src")).toMatch(/ranked-mini-crests\/.+\.svg$/);
    }

    const diamondOpt = within(listbox).getByRole("option", { name: "Diamante +" });
    fireEvent.click(diamondOpt);

    expect(onFilterChange).toHaveBeenCalledWith("tier", "diamond_plus");
    expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
  });

  it("renders custom Rota dropdown with position icons and calls onFilterChange", () => {
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
    expect(posTrigger).toHaveTextContent("Meio");
    expect(posTrigger.querySelector("img")).toHaveAttribute(
      "src",
      expect.stringContaining("position-middle-light.svg")
    );

    fireEvent.click(posTrigger);
    const listbox = screen.getByRole("listbox", { name: "Rota:" });
    const options = within(listbox).getAllByRole("option");

    expect(options.map((opt) => opt.getAttribute("data-value"))).toEqual([
      "top",
      "jungle",
      "mid",
      "adc",
      "support",
    ]);
    expect(options.map((opt) => opt.textContent)).toEqual([
      "Topo",
      "Selva",
      "Meio",
      "Atirador (Bot)",
      "Suporte",
    ]);

    for (const option of options) {
      const img = option.querySelector("img");
      expect(img).toBeInTheDocument();
      expect(img?.getAttribute("src")).toMatch(/position-.*-light\.svg$/);
    }

    fireEvent.click(within(listbox).getByRole("option", { name: "Atirador (Bot)" }));
    expect(onFilterChange).toHaveBeenCalledWith("position", "adc");
  });

  it("renders custom Servidor dropdown with SVG flags and acronym labels (BR, KR, Global, etc.)", () => {
    const onFilterChange = vi.fn();
    render(
      <FiltersHeader
        build={{ ...mockBuild, active_region: "br" }}
        champSelect={summonerRiftChampSelect}
        loading={false}
        onFilterChange={onFilterChange}
      />
    );

    const regionTrigger = screen.getByTestId("filter-region");
    expect(regionTrigger).toHaveTextContent("BR");
    expect(regionTrigger).not.toHaveTextContent("Brasil");
    expect(within(regionTrigger).getByTestId("region-icon-br").tagName.toLowerCase()).toBe("svg");

    fireEvent.click(regionTrigger);
    const listbox = screen.getByRole("listbox", { name: "Servidor:" });
    const options = within(listbox).getAllByRole("option");

    expect(options.map((opt) => opt.textContent)).toEqual([
      "Global",
      "BR",
      "KR",
      "NA",
      "EUW",
      "EUNE",
      "LAN",
      "LAS",
      "OCE",
      "TR",
      "RU",
      "JP",
      "ME",
      "SG",
      "TW",
      "VN",
    ]);

    for (const option of options) {
      const svg = option.querySelector("svg");
      expect(svg).toBeInTheDocument();
    }

    fireEvent.click(within(listbox).getByRole("option", { name: "KR" }));
    expect(onFilterChange).toHaveBeenCalledWith("region", "kr");
  });

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

    // Open with ArrowDown -> starts highlighted on current selection ("mid", index 2)
    fireEvent.keyDown(posTrigger, { key: "ArrowDown" });
    expect(posTrigger).toHaveAttribute("aria-expanded", "true");
    expect(posTrigger).toHaveAttribute("aria-activedescendant", "filter-position-option-2");

    // ArrowDown -> moves to "adc" (index 3)
    fireEvent.keyDown(posTrigger, { key: "ArrowDown" });
    expect(posTrigger).toHaveAttribute("aria-activedescendant", "filter-position-option-3");

    // End -> moves to "support" (index 4)
    fireEvent.keyDown(posTrigger, { key: "End" });
    expect(posTrigger).toHaveAttribute("aria-activedescendant", "filter-position-option-4");

    // Home -> moves to "top" (index 0)
    fireEvent.keyDown(posTrigger, { key: "Home" });
    expect(posTrigger).toHaveAttribute("aria-activedescendant", "filter-position-option-0");

    // Escape closes without committing
    fireEvent.keyDown(posTrigger, { key: "Escape" });
    expect(posTrigger).toHaveAttribute("aria-expanded", "false");
    expect(onFilterChange).not.toHaveBeenCalled();

    // Open with Enter, ArrowDown twice -> "adc", Enter commits
    fireEvent.keyDown(posTrigger, { key: "Enter" });
    fireEvent.keyDown(posTrigger, { key: "ArrowDown" });
    fireEvent.keyDown(posTrigger, { key: "Enter" });
    expect(onFilterChange).toHaveBeenCalledWith("position", "adc");
    expect(posTrigger).toHaveAttribute("aria-expanded", "false");

    // Open and click outside -> closes
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

    // Tooltip appears on hover/mouseEnter and explains 48h lock, manual override, and remaining hours
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

    // User can manually override to the newest patch via custom Patch dropdown
    const patchTrigger = screen.getByTestId("filter-patch");
    expect(patchTrigger).toHaveAttribute("data-value", "16.21");
    fireEvent.click(patchTrigger);
    const listbox = screen.getByRole("listbox", { name: "Patch:" });
    fireEvent.click(within(listbox).getByRole("option", { name: "Patch 16.22 (Mais atual)" }));
    expect(onFilterChange).toHaveBeenCalledWith("patch", "16.22");
  });
});
