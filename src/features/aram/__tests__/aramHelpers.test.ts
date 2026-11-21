import { describe, it, expect } from "vitest";
import { getChampionTierClass, getChampionTierLabel } from "../aramHelpers";

describe("aramHelpers", () => {
  it("maps champion tiers 0..4 to OP and Tier 1..4 labels and CSS classes", () => {
    expect(getChampionTierLabel(0)).toBe("OP");
    expect(getChampionTierClass(0)).toBe("tier-op");
    expect(getChampionTierLabel(1)).toBe("Tier 1");
    expect(getChampionTierClass(1)).toBe("tier-1");
    expect(getChampionTierLabel(2)).toBe("Tier 2");
    expect(getChampionTierClass(2)).toBe("tier-2");
    expect(getChampionTierLabel(3)).toBe("Tier 3");
    expect(getChampionTierClass(3)).toBe("tier-3");
    expect(getChampionTierLabel(4)).toBe("Tier 4");
    expect(getChampionTierClass(4)).toBe("tier-4");
    expect(getChampionTierLabel(5)).toBe("Tier 4");
    expect(getChampionTierClass(5)).toBe("tier-4");
  });
});
