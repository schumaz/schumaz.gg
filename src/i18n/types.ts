import type { CoreTranslations } from "./types/core";
import type { BuildTranslations } from "./types/build";
import type { SettingsTranslations } from "./types/settings";
import type { TabTranslations } from "./types/tabs";

export * from "./types/core";
export * from "./types/build";
export * from "./types/settings";
export * from "./types/tabs";

export interface TranslationDictionary
  extends CoreTranslations,
    BuildTranslations,
    SettingsTranslations,
    TabTranslations {}
