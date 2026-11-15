import { describe, it, expect } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import {
  en,
  pt,
  I18nProvider,
  useTranslation,
  translate,
  formatNumberForLocale,
  formatCompactNumberForLocale,
  formatRelativeTimeForLocale,
} from "../i18n";

function collectLeafPaths(obj: unknown, prefix = ""): Array<{ key: string; value: unknown }> {
  if (!obj || typeof obj !== "object") {
    return [{ key: prefix, value: obj }];
  }
  const result: Array<{ key: string; value: unknown }> = [];
  for (const [k, v] of Object.entries(obj as Record<string, unknown>)) {
    const nextPrefix = prefix ? `${prefix}.${k}` : k;
    if (v && typeof v === "object" && !Array.isArray(v)) {
      result.push(...collectLeafPaths(v, nextPrefix));
    } else {
      result.push({ key: nextPrefix, value: v });
    }
  }
  return result;
}

function extractPlaceholders(str: string): string[] {
  const matches = str.match(/\{[a-zA-Z0-9_]+\}/g) ?? [];
  return Array.from(new Set(matches)).sort();
}

describe("i18n dictionaries and translation helpers", () => {
  it("has 100% recursive key parity between pt.ts and en.ts with non-empty strings", () => {
    const ptLeaves = collectLeafPaths(pt);
    const enLeaves = collectLeafPaths(en);

    const ptKeys = ptLeaves.map((entry) => entry.key).sort();
    const enKeys = enLeaves.map((entry) => entry.key).sort();

    expect(ptKeys).toEqual(enKeys);
    expect(ptKeys.length).toBeGreaterThan(100);

    for (const entry of ptLeaves) {
      expect(typeof entry.value).toBe("string");
      expect((entry.value as string).trim().length).toBeGreaterThan(0);
    }

    for (const entry of enLeaves) {
      expect(typeof entry.value).toBe("string");
      expect((entry.value as string).trim().length).toBeGreaterThan(0);
    }
  });

  it("has matching interpolation placeholders across all pt.ts and en.ts keys", () => {
    const ptMap = new Map(collectLeafPaths(pt).map((e) => [e.key, String(e.value)]));
    const enMap = new Map(collectLeafPaths(en).map((e) => [e.key, String(e.value)]));

    for (const [key, ptVal] of ptMap.entries()) {
      const enVal = enMap.get(key) ?? "";
      expect(extractPlaceholders(enVal), `Placeholder mismatch for key "${key}"`).toEqual(
        extractPlaceholders(ptVal)
      );
    }
  });

  it("translates and interpolates parameters accurately for pt_BR and en_US", () => {
    expect(translate("pt_BR", "ranked.loadingBuild", { name: "Ahri" })).toBe(
      "Carregando runas e build de Ahri"
    );
    expect(translate("en_US", "ranked.loadingBuild", { name: "Ahri" })).toBe(
      "Loading runes and build for Ahri"
    );
    expect(translate("en_US", "filters.remainingNh", { hours: 12 })).toBe("12h left");
    expect(translate("pt_BR", "filters.remainingNh", { hours: 12 })).toBe("Faltam 12h");
  });

  it("formats numbers, compact damage/gold values, and relative times per locale", () => {
    expect(formatNumberForLocale(452310, "pt_BR")).toBe("452.310");
    expect(formatNumberForLocale(452310, "en_US")).toBe("452,310");

    expect(formatCompactNumberForLocale(24500, "pt_BR")).toBe("24,5k");
    expect(formatCompactNumberForLocale(24500, "en_US")).toBe("24.5k");

    const twoHoursAgo = Date.now() - 2 * 60 * 60 * 1000;
    expect(formatRelativeTimeForLocale(twoHoursAgo, "pt_BR")).toBe("há 2h");
    expect(formatRelativeTimeForLocale(twoHoursAgo, "en_US")).toBe("2h ago");
  });

  it("switches UI translations dynamically via I18nProvider and useTranslation", () => {
    function SampleConsumer() {
      const { locale, setLocale, t } = useTranslation();
      return (
        <div>
          <span data-testid="locale-code">{locale}</span>
          <span data-testid="greeting">{t("onboarding.title")}</span>
          <button type="button" onClick={() => setLocale("pt_BR")}>
            PT
          </button>
          <button type="button" onClick={() => setLocale("en_US")}>
            EN
          </button>
        </div>
      );
    }

    render(
      <I18nProvider locale={undefined}>
        <SampleConsumer />
      </I18nProvider>
    );

    // setupTests defaults to pt_BR
    expect(screen.getByTestId("locale-code")).toHaveTextContent("pt_BR");
    expect(screen.getByTestId("greeting")).toHaveTextContent("Escolha seu idioma");

    fireEvent.click(screen.getByRole("button", { name: "EN" }));
    expect(screen.getByTestId("locale-code")).toHaveTextContent("en_US");
    expect(screen.getByTestId("greeting")).toHaveTextContent("Choose your language");

    fireEvent.click(screen.getByRole("button", { name: "PT" }));
    expect(screen.getByTestId("locale-code")).toHaveTextContent("pt_BR");
    expect(screen.getByTestId("greeting")).toHaveTextContent("Escolha seu idioma");
  });
});
