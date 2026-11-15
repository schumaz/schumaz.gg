import { TranslationDictionary } from "../types";
import { ptBuild } from "./pt/build";
import { ptCore } from "./pt/core";
import { ptTabs } from "./pt/tabs";

export const pt: TranslationDictionary = {
  ...ptCore,
  ...ptBuild,
  ...ptTabs,
};
