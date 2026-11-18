import { FilterDropdownOption } from "./FilterDropdown";
import { TranslatorFn, defaultTranslate } from "./filterOptions";

export function computePatchUiMeta(params: {
  activePatch?: string | null;
  availablePatches?: string[] | null;
  patchFallbackApplied?: boolean | null;
  remainingHours?: number | null;
  t?: TranslatorFn;
}) {
  const t = params.t ?? defaultTranslate;
  const patchOptions =
    params.availablePatches && params.availablePatches.length > 0
      ? params.availablePatches
      : params.activePatch
      ? [params.activePatch]
      : ["16.22", "16.21"];

  const patchDropdownOptions: FilterDropdownOption[] = patchOptions.map((p, idx) => ({
    value: p,
    label:
      idx === 0
        ? t("filters.latestPatch", { patch: p })
        : t("filters.patchOption", { patch: p }),
  }));

  const hasKnownLatest = Boolean(params.availablePatches && params.availablePatches.length > 0);
  const latestPatch = patchOptions[0];
  const isPatchProtectionActive = Boolean(
    params.patchFallbackApplied && (!hasKnownLatest || params.activePatch !== latestPatch)
  );

  const remainingHours =
    isPatchProtectionActive &&
    typeof params.remainingHours === "number" &&
    params.remainingHours > 0
      ? params.remainingHours
      : null;

  const remainingHoursShort =
    remainingHours !== null
      ? remainingHours === 1
        ? t("filters.remaining1h")
        : t("filters.remainingNh", { hours: remainingHours })
      : null;

  const protectionTooltipText =
    remainingHours !== null
      ? remainingHours === 1
        ? t("filters.protectionTooltipWithHours1")
        : t("filters.protectionTooltipWithHoursN", { hours: remainingHours })
      : t("filters.protectionTooltipBase");

  return {
    patchOptions,
    patchDropdownOptions,
    isPatchProtectionActive,
    remainingHoursShort,
    protectionTooltipText,
  };
}
