import { TranslationDictionary } from "../types";
import { enBuild } from "./en/build";
import { enCore } from "./en/core";
import { enTabs } from "./en/tabs";

export const en: TranslationDictionary = {
  ...enCore,
  ...enBuild,
  ...enTabs,
};
