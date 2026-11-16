import "@testing-library/jest-dom/vitest";
import { beforeEach } from "vitest";
import { LANGUAGE_STORAGE_KEY } from "./i18n";
import { FLASH_ONBOARDING_STORAGE_KEY } from "./types";

beforeEach(() => {
  localStorage.setItem(LANGUAGE_STORAGE_KEY, "pt_BR");
  localStorage.setItem(FLASH_ONBOARDING_STORAGE_KEY, "true");
});
