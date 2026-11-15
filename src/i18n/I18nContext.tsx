import { createContext, ReactNode, useCallback, useContext, useMemo, useState } from "react";
import { en } from "./locales/en";
import { pt } from "./locales/pt";
import {
  AppLocale,
  DEFAULT_ONBOARDING_LOCALE,
  getSavedAppLocale,
  TranslationDictionary,
} from "./types";

const LOCALES: Record<AppLocale, TranslationDictionary> = {
  pt_BR: pt,
  en_US: en,
};

type LeafKeys<T, Prefix extends string = ""> = {
  [K in keyof T & string]: T[K] extends string
    ? `${Prefix}${K}`
    : LeafKeys<T[K], `${Prefix}${K}.`>;
}[keyof T & string];

export type TranslationKey = LeafKeys<TranslationDictionary>;

export type TranslationParams = Record<string, string | number>;

function resolveNestedKey(dict: TranslationDictionary, key: string): string {
  const parts = key.split(".");
  let current: unknown = dict;
  for (const part of parts) {
    if (current && typeof current === "object" && part in current) {
      current = (current as Record<string, unknown>)[part];
    } else {
      return key;
    }
  }
  return typeof current === "string" ? current : key;
}

export function translate(
  locale: AppLocale,
  key: TranslationKey,
  params?: TranslationParams
): string {
  const dict = LOCALES[locale] ?? LOCALES.en_US;
  let template = resolveNestedKey(dict, key);
  if (params) {
    for (const [paramKey, paramValue] of Object.entries(params)) {
      template = template.replace(new RegExp(`\\{${paramKey}\\}`, "g"), String(paramValue));
    }
  }
  return template;
}

export function formatRelativeTimeForLocale(creationMs: number, locale: AppLocale): string {
  if (!creationMs) return "";
  const diffMs = Date.now() - creationMs;
  const mins = Math.floor(diffMs / 60000);
  if (mins < 60) {
    const val = Math.max(1, mins);
    return locale === "pt_BR" ? `há ${val}min` : `${val}m ago`;
  }
  const hours = Math.floor(mins / 60);
  if (hours < 24) {
    return locale === "pt_BR" ? `há ${hours}h` : `${hours}h ago`;
  }
  const days = Math.floor(hours / 24);
  return locale === "pt_BR" ? `há ${days}d` : `${days}d ago`;
}

export function formatCompactNumberForLocale(val: number, locale: AppLocale): string {
  if (val >= 1000) {
    const formatted = (val / 1000).toFixed(1);
    return locale === "pt_BR" ? `${formatted.replace(".", ",")}k` : `${formatted}k`;
  }
  return String(val);
}

export function formatNumberForLocale(val: number, locale: AppLocale): string {
  return val.toLocaleString(locale === "pt_BR" ? "pt-BR" : "en-US");
}

export interface I18nContextValue {
  locale: AppLocale;
  setLocale: (locale: AppLocale) => void;
  t: (key: TranslationKey, params?: TranslationParams) => string;
  formatNumber: (val: number) => string;
  formatCompactNumber: (val: number) => string;
  formatRelativeTime: (creationMs: number) => string;
}

const I18nContext = createContext<I18nContextValue | null>(null);

interface I18nProviderProps {
  locale?: AppLocale;
  onLocaleChange?: (locale: AppLocale) => void;
  children: ReactNode;
}

export function I18nProvider({
  locale: controlledLocale,
  onLocaleChange,
  children,
}: I18nProviderProps) {
  const [internalLocale, setInternalLocale] = useState<AppLocale>(
    () => getSavedAppLocale() ?? DEFAULT_ONBOARDING_LOCALE
  );

  const activeLocale = controlledLocale ?? internalLocale;

  const setLocale = useCallback(
    (next: AppLocale) => {
      if (controlledLocale === undefined) {
        setInternalLocale(next);
      }
      onLocaleChange?.(next);
    },
    [controlledLocale, onLocaleChange]
  );

  const value = useMemo<I18nContextValue>(
    () => ({
      locale: activeLocale,
      setLocale,
      t: (key, params) => translate(activeLocale, key, params),
      formatNumber: (val) => formatNumberForLocale(val, activeLocale),
      formatCompactNumber: (val) => formatCompactNumberForLocale(val, activeLocale),
      formatRelativeTime: (creationMs) => formatRelativeTimeForLocale(creationMs, activeLocale),
    }),
    [activeLocale, setLocale]
  );

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useTranslation(): I18nContextValue {
  const ctx = useContext(I18nContext);
  if (ctx) {
    return ctx;
  }

  // Fallback for standalone component renders/tests without an explicit <I18nProvider>
  const fallbackLocale = getSavedAppLocale() ?? DEFAULT_ONBOARDING_LOCALE;
  return {
    locale: fallbackLocale,
    setLocale: () => {},
    t: (key, params) => translate(fallbackLocale, key, params),
    formatNumber: (val) => formatNumberForLocale(val, fallbackLocale),
    formatCompactNumber: (val) => formatCompactNumberForLocale(val, fallbackLocale),
    formatRelativeTime: (creationMs) => formatRelativeTimeForLocale(creationMs, fallbackLocale),
  };
}
