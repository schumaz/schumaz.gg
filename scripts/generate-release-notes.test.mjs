import { describe, expect, it } from "vitest";
import {
  buildBilingualReleaseNotes,
  buildUpdaterSummary,
  extractChangelogSection,
  parseArgs,
  splitBilingualSections,
} from "./generate-release-notes.mjs";

const SAMPLE_CHANGELOG = `# Changelog

## [0.2.0] - 2026-12-01

### English
#### Added
- **System Tray**: minimize to tray when closing the window.
#### Fixed
- Fix duplicate instance launch on Windows.

### Português (BR)
#### Adicionado
- **Bandeja do Sistema**: minimizar para a bandeja ao fechar a janela.
#### Corrigido
- Corrige abertura duplicada no Windows.

## [0.1.1] - 2026-11-20

### Fixed
- Enforce single app instance on Windows via \`tauri-plugin-single-instance\`.
- Configure explicit CSP in \`tauri.conf.json\`.

### Changed
- Localize \`InjectionReport\` messages by \`AppLocale\`.

## [0.1.0] - 2026-11-16

First public release of **schumaz.gg**.

### Added
- **Champion Select Automation**: automatic detection.
`;

describe("generate-release-notes", () => {
  it("extracts a specific version section without bleeding into older versions", () => {
    const section = extractChangelogSection(SAMPLE_CHANGELOG, "v0.1.1");
    expect(section.version).toBe("0.1.1");
    expect(section.tag).toBe("v0.1.1");
    expect(section.date).toBe("2026-11-20");
    expect(section.body).toContain("tauri-plugin-single-instance");
    expect(section.body).not.toContain("Champion Select Automation");
    expect(section.body).not.toContain("System Tray");
  });

  it("throws when the target version section is missing from CHANGELOG.md", () => {
    expect(() => extractChangelogSection(SAMPLE_CHANGELOG, "v9.9.9")).toThrow(
      "Could not find section '## [9.9.9]' in CHANGELOG.md"
    );
  });

  it("supports explicit EN and PT-BR subsections inside a version section", () => {
    const body = buildBilingualReleaseNotes({
      tag: "v0.2.0",
      setupName: "schumaz.gg_0.2.0_x64-setup.exe",
      portableName: "schumaz.gg_0.2.0_x64-portable.exe",
      changelogContent: SAMPLE_CHANGELOG,
    });

    expect(body).toContain("### What's New in v0.2.0");
    expect(body).toContain("**System Tray**: minimize to tray");
    expect(body).toContain("### Novidades da versão v0.2.0");
    expect(body).toContain("**Bandeja do Sistema**: minimizar para a bandeja");
    expect(body).not.toContain("v0.1.0");
  });

  it("falls back to Keep-a-Changelog headings with localized PT-BR titles when only English is written", () => {
    const { en, pt } = splitBilingualSections(
      extractChangelogSection(SAMPLE_CHANGELOG, "v0.1.1").body
    );

    expect(en).toContain("#### Fixed");
    expect(en).toContain("#### Changed");
    expect(pt).toContain("#### Corrigido");
    expect(pt).toContain("#### Alterado");
    expect(pt).toContain("tauri-plugin-single-instance");
  });

  it("builds a clean plain-text summary for latest.json without markdown syntax", () => {
    const patchSummary = buildUpdaterSummary({
      tag: "v0.1.1",
      changelogContent: SAMPLE_CHANGELOG,
    });
    expect(patchSummary).toBe(
      "Release v0.1.1 — Enforce single app instance on Windows via tauri-plugin-single-instance; Configure explicit CSP in tauri.conf.json; Localize InjectionReport messages by AppLocale."
    );
    expect(patchSummary).not.toContain("`");
    expect(patchSummary).not.toContain("**");
    expect(patchSummary).not.toContain("###");

    const realV010Summary = buildUpdaterSummary({ tag: "v0.1.0" });
    expect(realV010Summary.startsWith("Release v0.1.0 — First public release of schumaz.gg")).toBe(
      true
    );
    expect(realV010Summary).not.toContain("**");
    expect(realV010Summary).not.toContain("`");
    expect(realV010Summary).not.toContain(":.");
    expect(realV010Summary.length).toBeLessThanOrEqual(380);
  });

  it("parses CLI options for both GitHub release notes and updater summary outputs", () => {
    expect(
      parseArgs(["--tag", "v0.1.1", "--updater-out", "tmp/updater.txt"])
    ).toMatchObject({
      tag: "v0.1.1",
      updaterOutFile: "tmp/updater.txt",
      outFile: null,
    });

    expect(() =>
      parseArgs(["--tag=v0.1.1", "--out=tmp/notes.md"])
    ).toThrow("--setup and --portable are required when --out is specified.");
  });
});
