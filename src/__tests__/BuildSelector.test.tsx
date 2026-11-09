import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { BuildSelector } from "../components/BuildSelector";
import { UnifiedBuildOption } from "../types";

describe("BuildSelector", () => {
  const mockOptions: UnifiedBuildOption[] = [
    {
      name: "Build 1 (Mais Feita)",
      core_item_ids: [3089, 3157, 3135],
      core_item_icon_urls: ["rabadon.png", "zhonya.png", "void.png"],
      final_item_ids: [3089, 3157, 3135, 3020, 3165, 3102],
      final_item_icon_urls: ["1.png", "2.png", "3.png", "4.png", "5.png", "6.png"],
      play: 10000,
      win: 5500,
      pick_rate: 60.0,
    },
    {
      name: "Build 2",
      core_item_ids: [6653, 3157, 3089],
      core_item_icon_urls: ["liandry.png", "zhonya.png", "rabadon.png"],
      final_item_ids: [6653, 3157, 3089, 3020, 3165, 3102],
      final_item_icon_urls: ["1.png", "2.png", "3.png", "4.png", "5.png", "6.png"],
      play: 5000,
      win: 2600,
      pick_rate: 30.0,
    },
  ];

  it("renders build pills with names and handles selection", () => {
    const handleSelect = vi.fn();

    render(
      <BuildSelector
        buildOptions={mockOptions}
        selectedBuildIndex={0}
        onSelectBuild={handleSelect}
      />
    );

    expect(screen.getByTestId("unified-build-selector")).toBeInTheDocument();
    expect(screen.getByText("Build 1 (Mais Feita)")).toBeInTheDocument();
    expect(screen.getByText("Build 2")).toBeInTheDocument();

    const secondPill = screen.getByTestId("build-opt-btn-1");
    fireEvent.click(secondPill);

    expect(handleSelect).toHaveBeenCalledWith(1);
  });

  it("returns null if options are missing or single", () => {
    const { container } = render(
      <BuildSelector
        buildOptions={[mockOptions[0]]}
        selectedBuildIndex={0}
        onSelectBuild={vi.fn()}
      />
    );

    expect(container.firstChild).toBeNull();
  });
});
