/**
 * Canonical 0-based champion meta tier helpers (`0 = OP`, `1 = Tier 1`, ..., `4 = Tier 4`).
 */
export function getChampionTierLabel(tier: number): string {
  if (tier <= 0) return "OP";
  const clamped = Math.min(4, Math.floor(tier));
  return `Tier ${clamped}`;
}

export function getChampionTierClass(tier: number): string {
  if (tier <= 0) return "tier-op";
  if (tier === 1) return "tier-1";
  if (tier === 2) return "tier-2";
  if (tier === 3) return "tier-3";
  return "tier-4";
}
