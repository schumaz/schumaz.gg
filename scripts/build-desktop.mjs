import { createHash } from "node:crypto";
import {
  copyFileSync,
  existsSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const ROOT_DIR = path.resolve(__dirname, "..");

export const DEFAULT_GITHUB_REPO = "schumaz/schumaz.gg";
export const EXCLUDED_CHECKSUM_FILES = new Set(["SHA256SUMS.txt", "latest.json"]);

function printHelp() {
  console.log(`Usage: node scripts/build-desktop.mjs [options]

Builds the Windows NSIS installer and Portable executable into dist-release/.

Options:
  --no-sign            Skip updater/binary signing for local unsigned testing
  --target <triple>    Cargo target triple (defaults to host target)
  --tag <tag>          GitHub release tag used in latest.json URL (default: v<version>)
  --notes <text>       Release notes string embedded in latest.json
  --notes-file <path>  Path to UTF-8 file containing release notes for latest.json
  -h, --help           Show this help message
`);
}

function readOptionValue(argv, index, flagName) {
  const value = argv[index + 1];
  if (!value || value.startsWith("-")) {
    throw new Error(`Missing value for ${flagName}`);
  }
  return value;
}

export function parseCliArgs(argv) {
  let noSign = false;
  let target = null;
  let tag = null;
  let notes = null;
  let notesFile = null;

  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === "--no-sign") {
      noSign = true;
    } else if (arg === "--target") {
      target = readOptionValue(argv, i, "--target");
      i += 1;
    } else if (arg.startsWith("--target=")) {
      target = arg.slice("--target=".length);
      if (!target) {
        throw new Error("Missing value for --target");
      }
    } else if (arg === "--tag") {
      tag = readOptionValue(argv, i, "--tag");
      i += 1;
    } else if (arg.startsWith("--tag=")) {
      tag = arg.slice("--tag=".length);
      if (!tag) {
        throw new Error("Missing value for --tag");
      }
    } else if (arg === "--notes") {
      notes = readOptionValue(argv, i, "--notes");
      i += 1;
    } else if (arg.startsWith("--notes=")) {
      notes = arg.slice("--notes=".length);
      if (!notes) {
        throw new Error("Missing value for --notes");
      }
    } else if (arg === "--notes-file") {
      notesFile = readOptionValue(argv, i, "--notes-file");
      i += 1;
    } else if (arg.startsWith("--notes-file=")) {
      notesFile = arg.slice("--notes-file=".length);
      if (!notesFile) {
        throw new Error("Missing value for --notes-file");
      }
    } else if (arg === "-h" || arg === "--help") {
      return { help: true, noSign, target, tag, notes, notesFile };
    } else {
      throw new Error(`Unknown option: ${arg}`);
    }
  }

  return { help: false, noSign, target, tag, notes, notesFile };
}

function loadEnvIfNeeded() {
  const envPath = path.join(ROOT_DIR, ".env");
  if (existsSync(envPath) && typeof process.loadEnvFile === "function") {
    try {
      process.loadEnvFile(envPath);
    } catch {
      // Ignore if already loaded or unreadable
    }
  }
}

function readPackageMetadata() {
  const tauriConfPath = path.join(ROOT_DIR, "src-tauri", "tauri.conf.json");
  const cargoTomlPath = path.join(ROOT_DIR, "src-tauri", "Cargo.toml");

  const tauriConf = JSON.parse(readFileSync(tauriConfPath, "utf8"));
  const cargoToml = readFileSync(cargoTomlPath, "utf8");
  const nameMatch = cargoToml.match(/^\s*name\s*=\s*"([^"]+)"/m);
  if (!nameMatch) {
    throw new Error("Could not determine Cargo package name from src-tauri/Cargo.toml");
  }

  return {
    productName: tauriConf.productName || nameMatch[1],
    version: tauriConf.version,
    binName: nameMatch[1],
  };
}

function runCommand(command, args, label) {
  console.log(`\n==> ${label}`);
  const result = spawnSync(command, args, {
    cwd: ROOT_DIR,
    stdio: "inherit",
    env: process.env,
  });
  if (result.error) {
    throw result.error;
  }
  if (result.status !== 0) {
    throw new Error(`${label} failed with exit code ${result.status}`);
  }
}

function runNpmBuild() {
  if (process.platform === "win32") {
    runCommand("cmd.exe", ["/d", "/s", "/c", "npm", "run", "build"], "Building frontend (npm run build)");
  } else {
    runCommand("npm", ["run", "build"], "Building frontend (npm run build)");
  }
}

function runTauriBuild(args, label) {
  const tauriCliPath = path.join(ROOT_DIR, "node_modules", "@tauri-apps", "cli", "tauri.js");
  if (!existsSync(tauriCliPath)) {
    throw new Error(`Tauri CLI not found at ${tauriCliPath}. Run npm install first.`);
  }
  runCommand(process.execPath, [tauriCliPath, "build", ...args], label);
}

function findNsisSetupFile(nsisDir, version) {
  if (!existsSync(nsisDir)) {
    throw new Error(`NSIS bundle directory not found: ${nsisDir}`);
  }

  const candidates = readdirSync(nsisDir)
    .filter((name) => name.toLowerCase().endsWith("-setup.exe"))
    .map((name) => ({
      name,
      fullPath: path.join(nsisDir, name),
      mtimeMs: statSync(path.join(nsisDir, name)).mtimeMs,
    }))
    .sort((a, b) => b.mtimeMs - a.mtimeMs);

  const versionMatches = version
    ? candidates.filter((entry) => entry.name.includes(version))
    : candidates;

  const chosen = versionMatches[0] ?? candidates[0];
  if (!chosen) {
    throw new Error(`No *-setup.exe installer found in ${nsisDir}`);
  }
  return chosen;
}

export function derivePortableFileName(setupFileName, productName, version, target) {
  if (/-setup\.exe$/i.test(setupFileName)) {
    return setupFileName.replace(/-setup\.exe$/i, "-portable.exe");
  }
  const arch = target?.includes("aarch64") || process.arch === "arm64" ? "arm64" : "x64";
  return `${productName}_${version}_${arch}-portable.exe`;
}

export function derivePlatformKey(target, hostArch = process.arch) {
  if (target?.includes("aarch64") || (!target && hostArch === "arm64")) {
    return "windows-aarch64";
  }
  return "windows-x86_64";
}

export function defaultReleaseTag(version) {
  return version.startsWith("v") ? version : `v${version}`;
}

export function defaultReleaseNotes(version) {
  return `Release v${version} — Automated Champion Select runes, summoner spells & item sets, Ranked & ARAM/ARAM Mayhem builds, and Summoner Profile analytics.`;
}

export function buildReleaseAssetUrl(tag, assetFileName, repoSlug = DEFAULT_GITHUB_REPO) {
  const cleanRepo = repoSlug.replace(/^\/+|\/+$/g, "");
  return `https://github.com/${cleanRepo}/releases/download/${encodeURIComponent(tag)}/${encodeURIComponent(assetFileName)}`;
}

export function createUpdaterManifest({
  version,
  notes,
  pubDate = new Date().toISOString(),
  platformKey,
  url,
  signature,
}) {
  const trimmedSignature = signature.trim();
  if (!trimmedSignature) {
    throw new Error("Updater signature cannot be empty when generating latest.json");
  }
  return {
    version,
    notes: notes ?? defaultReleaseNotes(version),
    pub_date: pubDate,
    platforms: {
      [platformKey]: {
        signature: trimmedSignature,
        url,
      },
    },
  };
}

export function resolveReleaseNotes({ version, notes, notesFile, cwd = ROOT_DIR }) {
  if (notesFile) {
    const resolvedPath = path.isAbsolute(notesFile) ? notesFile : path.resolve(cwd, notesFile);
    if (!existsSync(resolvedPath)) {
      throw new Error(`Notes file not found: ${resolvedPath}`);
    }
    const content = readFileSync(resolvedPath, "utf8").trim();
    if (!content) {
      throw new Error(`Notes file is empty: ${resolvedPath}`);
    }
    return content;
  }
  if (typeof notes === "string" && notes.trim().length > 0) {
    return notes.trim();
  }
  return defaultReleaseNotes(version);
}

export function sha256File(filePath) {
  const hash = createHash("sha256");
  hash.update(readFileSync(filePath));
  return hash.digest("hex");
}

export function getChecksumArtifactNames(distReleaseDir) {
  return readdirSync(distReleaseDir)
    .filter((name) => !EXCLUDED_CHECKSUM_FILES.has(name))
    .sort((a, b) => a.localeCompare(b));
}

export function writeSha256Sums(distReleaseDir) {
  const artifactNames = getChecksumArtifactNames(distReleaseDir);
  const checksumLines = artifactNames.map((name) => {
    const fullPath = path.join(distReleaseDir, name);
    return `${sha256File(fullPath)}  ${name}`;
  });
  writeFileSync(
    path.join(distReleaseDir, "SHA256SUMS.txt"),
    `${checksumLines.join("\n")}\n`,
    "utf8"
  );
  return artifactNames;
}

export function writeLatestManifest({
  distReleaseDir,
  version,
  tag,
  notes,
  setupFileName,
  sigFilePath,
  target,
  repoSlug = process.env.GITHUB_REPOSITORY || DEFAULT_GITHUB_REPO,
  pubDate = new Date().toISOString(),
}) {
  if (!existsSync(sigFilePath)) {
    throw new Error(`Expected updater signature not found: ${sigFilePath}`);
  }
  const signature = readFileSync(sigFilePath, "utf8");
  const resolvedTag = tag || defaultReleaseTag(version);
  const platformKey = derivePlatformKey(target);
  const manifest = createUpdaterManifest({
    version,
    notes,
    pubDate,
    platformKey,
    url: buildReleaseAssetUrl(resolvedTag, setupFileName, repoSlug),
    signature,
  });

  const manifestPath = path.join(distReleaseDir, "latest.json");
  writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`, "utf8");
  return manifest;
}

function formatSizeMb(bytes) {
  return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
}

function main() {
  const options = parseCliArgs(process.argv.slice(2));
  if (options.help) {
    printHelp();
    return;
  }

  if (process.platform !== "win32") {
    throw new Error("scripts/build-desktop.mjs targets Windows NSIS & Portable builds.");
  }

  loadEnvIfNeeded();

  if (!options.noSign && !process.env.TAURI_SIGNING_PRIVATE_KEY) {
    throw new Error(
      "TAURI_SIGNING_PRIVATE_KEY is not set. Set it in .env / environment for signed release artifacts, or pass --no-sign for local unsigned builds."
    );
  }

  const { productName, version, binName } = readPackageMetadata();
  const releaseTag = options.tag || defaultReleaseTag(version);
  const releaseNotes = resolveReleaseNotes({
    version,
    notes: options.notes,
    notesFile: options.notesFile,
  });

  const distReleaseDir = path.join(ROOT_DIR, "dist-release");
  const targetReleaseDir = options.target
    ? path.join(ROOT_DIR, "src-tauri", "target", options.target, "release")
    : path.join(ROOT_DIR, "src-tauri", "target", "release");

  rmSync(distReleaseDir, { recursive: true, force: true });
  mkdirSync(distReleaseDir, { recursive: true });

  const overrideConfigPath = path.join(
    tmpdir(),
    `schumazgg-tauri-build-${process.pid}-${Date.now()}.json`
  );
  const overrideConfig = {
    build: {
      beforeBuildCommand: "",
    },
    ...(options.noSign
      ? {
          bundle: {
            createUpdaterArtifacts: false,
          },
        }
      : {}),
  };
  writeFileSync(overrideConfigPath, JSON.stringify(overrideConfig, null, 2), "utf8");

  try {
    // 1. Run frontend build once
    runNpmBuild();

    // 2. Build NSIS installer and extract *-setup.exe (+ .sig) BEFORE portable overwrites schumazgg.exe
    const commonTauriArgs = ["--config", overrideConfigPath];
    if (options.target) {
      commonTauriArgs.push("--target", options.target);
    }
    if (options.noSign) {
      commonTauriArgs.push("--no-sign");
    }

    runTauriBuild(
      ["--bundles", "nsis", ...commonTauriArgs],
      `Building Windows NSIS installer (${options.noSign ? "unsigned" : "signed"})`
    );

    const nsisDir = path.join(targetReleaseDir, "bundle", "nsis");
    const setupEntry = findNsisSetupFile(nsisDir, version);
    const destSetupPath = path.join(distReleaseDir, setupEntry.name);
    copyFileSync(setupEntry.fullPath, destSetupPath);

    const sigFileName = `${setupEntry.name}.sig`;
    const srcSigPath = path.join(nsisDir, sigFileName);
    const destSigPath = path.join(distReleaseDir, sigFileName);
    if (existsSync(srcSigPath)) {
      copyFileSync(srcSigPath, destSigPath);
    } else if (!options.noSign) {
      throw new Error(`Expected updater signature not found: ${srcSigPath}`);
    }

    // 3. Build Portable standalone executable (--features portable --no-bundle)
    runTauriBuild(
      ["--features", "portable", "--no-bundle", ...commonTauriArgs],
      "Building Windows Portable executable (--features portable)"
    );

    const portableSrcPath = path.join(targetReleaseDir, `${binName}.exe`);
    if (!existsSync(portableSrcPath)) {
      throw new Error(`Portable binary not found at ${portableSrcPath}`);
    }

    const portableFileName = derivePortableFileName(
      setupEntry.name,
      productName,
      version,
      options.target
    );
    const destPortablePath = path.join(distReleaseDir, portableFileName);
    copyFileSync(portableSrcPath, destPortablePath);

    // 4. Generate latest.json for signed builds (must remain excluded from SHA256SUMS.txt)
    const generatedExtraFiles = [];
    if (!options.noSign) {
      writeLatestManifest({
        distReleaseDir,
        version,
        tag: releaseTag,
        notes: releaseNotes,
        setupFileName: setupEntry.name,
        sigFilePath: destSigPath,
        target: options.target,
      });
      generatedExtraFiles.push("latest.json");
    }

    // 5. Generate SHA256SUMS.txt (excluding SHA256SUMS.txt and latest.json)
    const checksummedArtifacts = writeSha256Sums(distReleaseDir);
    generatedExtraFiles.push("SHA256SUMS.txt");

    console.log("\nRelease artifacts generated in dist-release/:");
    for (const name of [...checksummedArtifacts, ...generatedExtraFiles]) {
      const fullPath = path.join(distReleaseDir, name);
      const size = statSync(fullPath).size;
      console.log(`  - ${name} (${formatSizeMb(size)})`);
    }
  } finally {
    rmSync(overrideConfigPath, { force: true });
  }
}

const isMainModule =
  process.argv[1] && path.resolve(process.argv[1]) === path.resolve(__filename);
if (isMainModule) {
  try {
    main();
  } catch (err) {
    console.error(`\n[build-desktop] Error: ${err instanceof Error ? err.message : String(err)}`);
    process.exit(1);
  }
}
