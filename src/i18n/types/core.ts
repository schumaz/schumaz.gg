export type AppLocale = "pt_BR" | "en_US";

export const DEFAULT_ONBOARDING_LOCALE: AppLocale = "en_US";

export function isValidAppLocale(value: unknown): value is AppLocale {
  return value === "pt_BR" || value === "en_US";
}

let defaultFallbackLocale: AppLocale | null = null;

export function setDefaultFallbackLocale(locale: AppLocale | null): void {
  defaultFallbackLocale = locale;
}

export function getSavedAppLocale(): AppLocale | null {
  return defaultFallbackLocale;
}

export interface CoreTranslations {
  common: {
    retry: string;
    back: string;
    clearSearch: string;
    loading: string;
    errorPrefix: string;
    gamesSingular: string;
    gamesPlural: string;
    championSingular: string;
    championPlural: string;
    itemAlt: string;
    searchChampionPlaceholder: string;
  };
  startup: {
    ariaLabel: string;
    stages: {
      initializing: string;
      settings: string;
      cache: string;
      client: string;
      assets: string;
      syncing: string;
      ready: string;
    };
  };
  onboarding: {
    badge: string;
    title: string;
    subtitle: string;
    continueBtn: string;
    englishName: string;
    englishRegion: string;
    portugueseName: string;
    portugueseRegion: string;
    ariaGroup: string;
    flashTitle: string;
    flashSubtitle: string;
    flashConfirmBtn: string;
    flashAriaGroup: string;
    flashOptionDTitle: string;
    flashOptionDDesc: string;
    flashOptionFTitle: string;
    flashOptionFDesc: string;
  };
  nav: {
    ariaLabel: string;
    profile: string;
    runes: string;
    ranked: string;
    aram: string;
    settings: string;
  };
  status: {
    connected: string;
    waiting: string;
  };
  idle: {
    runesBadge: string;
    loadingTitle: string;
    loadingDesc: string;
    waitingTitle: string;
    waitingDesc: string;
    fetchFilterError: string;
  };
  filters: {
    eloLabel: string;
    laneLabel: string;
    regionLabel: string;
    patchLabel: string;
    tierLabel: string;
    lockedIn: string;
    hovering: string;
    backToTierListTitle: string;
    pickRate: string;
    protectedVolume: string;
    latestPatch: string;
    patchOption: string;
    remaining1h: string;
    remainingNh: string;
    protectionTooltipBase: string;
    protectionTooltipWithHours1: string;
    protectionTooltipWithHoursN: string;
    queueAramMayhem: string;
    queueAram: string;
    queuePracticeTool: string;
    queueSr: string;
    queueRankedSr: string;
    tiers: {
      all: string;
      challenger: string;
      grandmaster: string;
      master_plus: string;
      diamond_plus: string;
      emerald_plus: string;
      platinum_plus: string;
      gold_plus: string;
      silver: string;
      bronze: string;
      iron: string;
    };
    lanes: {
      top: string;
      topShort: string;
      jungle: string;
      jungleShort: string;
      mid: string;
      midShort: string;
      adc: string;
      adcShort: string;
      support: string;
      supportShort: string;
    };
    regions: {
      globalLabel: string;
      globalTitle: string;
      brTitle: string;
      krTitle: string;
      naTitle: string;
      euwTitle: string;
      euneTitle: string;
      lanTitle: string;
      lasTitle: string;
      oceTitle: string;
      trTitle: string;
      ruTitle: string;
      jpTitle: string;
      meTitle: string;
      sgTitle: string;
      twTitle: string;
      vnTitle: string;
    };
  };
}
