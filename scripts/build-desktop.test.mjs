import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import {
  buildReleaseAssetUrl,
  createUpdaterManifest,
  defaultReleaseNotes,
  defaultReleaseTag,
  derivePlatformKey,
  derivePortableFileName,
  parseCliArgs,
  resolveReleaseNotes,
  writeLatestManifest,
  writeSha256Sums,
} from "./build-desktop.mjs";
import { buildBilingualReleaseNotes } from "./generate-release-notes.mjs";

const tempDirs = [];

function createTempDir() {
  const dir = mkdtempSync(path.join(tmpdir(), "schumazgg-build-test-"));
  tempDirs.push(dir);
  return dir;
}

afterEach(() => {
  while (tempDirs.length > 0) {
    const dir = tempDirs.pop();
    rmSync(dir, { recursive: true, force: true });
  }
});

describe("build-desktop CLI and helpers", () => {
  it("parses --tag, --notes, --notes-file, --target, and --no-sign", () => {
    expect(
      parseCliArgs([
        "--no-sign",
        "--target",
        "x86_64-pc-windows-msvc",
        "--tag",
        "v0.1.0",
        "--notes",
        "Initial release",
      ])
    ).toEqual({
      help: false,
      noSign: true,
      target: "x86_64-pc-windows-msvc",
      tag: "v0.1.0",
      notes: "Initial release",
      notesFile: null,
    });

    expect(
      parseCliArgs(["--tag=v0.2.0", "--notes=Custom notes", "--notes-file=CHANGELOG.md"])
    ).toEqual({
      help: false,
      noSign: false,
      target: null,
      tag: "v0.2.0",
      notes: "Custom notes",
      notesFile: "CHANGELOG.md",
    });
  });

  it("throws when --tag or --notes is missing a value", () => {
    expect(() => parseCliArgs(["--tag"])).toThrow("Missing value for --tag");
    expect(() => parseCliArgs(["--tag="])).toThrow("Missing value for --tag");
    expect(() => parseCliArgs(["--notes", "--no-sign"])).toThrow("Missing value for --notes");
    expect(() => parseCliArgs(["--notes-file="])).toThrow("Missing value for --notes-file");
  });

  it("derives portable file names, platform keys, default tags, and release URLs", () => {
    expect(derivePortableFileName("schumaz.gg_0.1.0_x64-setup.exe", "schumaz.gg", "0.1.0")).toBe(
      "schumaz.gg_0.1.0_x64-portable.exe"
    );
    expect(derivePlatformKey(null, "x64")).toBe("windows-x86_64");
    expect(derivePlatformKey("aarch64-pc-windows-msvc", "x64")).toBe("windows-aarch64");
    expect(defaultReleaseTag("0.1.0")).toBe("v0.1.0");
    expect(defaultReleaseTag("v0.1.0")).toBe("v0.1.0");
    expect(buildReleaseAssetUrl("v0.1.0", "schumaz.gg_0.1.0_x64-setup.exe")).toBe(
      "https://github.com/schumaz/schumaz.gg/releases/download/v0.1.0/schumaz.gg_0.1.0_x64-setup.exe"
    );
  });

  it("resolves release notes from --notes-file, --notes, or default", () => {
    const dir = createTempDir();
    const notesPath = path.join(dir, "notes.txt");
    writeFileSync(notesPath, "  Highlights for 0.1.0  \n", "utf8");

    expect(resolveReleaseNotes({ version: "0.1.0", notesFile: notesPath })).toBe(
      "Highlights for 0.1.0"
    );
    expect(resolveReleaseNotes({ version: "0.1.0", notes: "  Inline release notes  " })).toBe(
      "Inline release notes"
    );
    expect(resolveReleaseNotes({ version: "0.1.0" })).toBe(defaultReleaseNotes("0.1.0"));
  });
});

describe("latest.json and SHA256SUMS.txt generation", () => {
  it("writes a valid Tauri v2 latest.json manifest and excludes latest.json from SHA256SUMS.txt", () => {
    const distDir = createTempDir();
    mkdirSync(distDir, { recursive: true });

    const setupName = "schumaz.gg_0.1.0_x64-setup.exe";
    const sigName = `${setupName}.sig`;
    const portableName = "schumaz.gg_0.1.0_x64-portable.exe";

    writeFileSync(path.join(distDir, setupName), "setup-binary-content", "utf8");
    writeFileSync(path.join(distDir, sigName), "dW50cnVzdGVkLXNpZ25hdHVyZS1saW5lCg==\n", "utf8");
    writeFileSync(path.join(distDir, portableName), "portable-binary-content", "utf8");

    const fixedDate = "2026-11-16T15:00:00.000Z";
    const manifest = writeLatestManifest({
      distReleaseDir: distDir,
      version: "0.1.0",
      tag: "v0.1.0",
      notes: "First public release of schumaz.gg",
      setupFileName: setupName,
      sigFilePath: path.join(distDir, sigName),
      target: "x86_64-pc-windows-msvc",
      pubDate: fixedDate,
    });

    expect(manifest).toEqual({
      version: "0.1.0",
      notes: "First public release of schumaz.gg",
      pub_date: fixedDate,
      platforms: {
        "windows-x86_64": {
          signature: "dW50cnVzdGVkLXNpZ25hdHVyZS1saW5lCg==",
          url: "https://github.com/schumaz/schumaz.gg/releases/download/v0.1.0/schumaz.gg_0.1.0_x64-setup.exe",
        },
      },
    });

    const manifestPath = path.join(distDir, "latest.json");
    expect(existsSync(manifestPath)).toBe(true);
    expect(JSON.parse(readFileSync(manifestPath, "utf8"))).toEqual(manifest);

    const checksummed = writeSha256Sums(distDir);
    expect(checksummed).toEqual([portableName, setupName, sigName]);

    const sumsContent = readFileSync(path.join(distDir, "SHA256SUMS.txt"), "utf8");
    expect(sumsContent).not.toContain("latest.json");
    expect(sumsContent).not.toContain("SHA256SUMS.txt");
    expect(sumsContent).toContain(`  ${setupName}`);
    expect(sumsContent).toContain(`  ${sigName}`);
    expect(sumsContent).toContain(`  ${portableName}`);
    expect(sumsContent.trim().split("\n")).toHaveLength(3);
  });

  it("rejects empty updater signatures", () => {
    expect(() =>
      createUpdaterManifest({
        version: "0.1.0",
        notes: "Release",
        platformKey: "windows-x86_64",
        url: "https://example.com/setup.exe",
        signature: "   \n  ",
      })
    ).toThrow("Updater signature cannot be empty");
  });

  it("generates bilingual release notes with SmartScreen, Installer vs Portable, and SHA256 instructions", () => {
    const body = buildBilingualReleaseNotes({
      tag: "v0.1.0",
      setupName: "schumaz.gg_0.1.0_x64-setup.exe",
      portableName: "schumaz.gg_0.1.0_x64-portable.exe",
    });

    expect(body).toContain("## English");
    expect(body).toContain("## Português (BR)");
    expect(body).toContain("schumaz.gg_0.1.0_x64-setup.exe");
    expect(body).toContain("schumaz.gg_0.1.0_x64-portable.exe");
    expect(body).toContain("Windows SmartScreen");
    expect(body).toContain("SHA256SUMS.txt");
    expect(body).toContain("```powershell");
  });
});
