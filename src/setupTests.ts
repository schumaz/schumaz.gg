import "@testing-library/jest-dom/vitest";
import { beforeEach } from "vitest";
import { LANGUAGE_STORAGE_KEY } from "./i18n";

beforeEach(() => {
  localStorage.setItem(LANGUAGE_STORAGE_KEY, "pt_BR");
});
