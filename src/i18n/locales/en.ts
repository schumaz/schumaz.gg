import { TranslationDictionary } from "../types";
import { enBuild } from "./en/build";
import { enCore } from "./en/core";
import { enSettings } from "./en/settings";
import { enTabs } from "./en/tabs";

export const en: TranslationDictionary = {
  ...enCore,
  ...enBuild,
  ...enSettings,
  ...enTabs,
};
