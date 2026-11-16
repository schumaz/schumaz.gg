import "@testing-library/jest-dom/vitest";
import { beforeEach } from "vitest";
import { setDefaultFallbackLocale } from "./i18n";
import { resetMockAppSettings } from "./__tests__/appTestFixtures";

beforeEach(() => {
  localStorage.clear();
  setDefaultFallbackLocale("pt_BR");
  resetMockAppSettings();
});
