import { readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const DEFAULT_CHANGELOG_PATH = path.resolve(__dirname, "..", "CHANGELOG.md");

const KEEP_A_CHANGELOG_PT_MAP = new Map([
  ["added", "Adicionado"],
  ["changed", "Alterado"],
  ["deprecated", "Obsoleto"],
  ["removed", "Removido"],
  ["fixed", "Corrigido"],
  ["security", "Segurança"],
  ["performance", "Desempenho"],
  ["documentation", "Documentação"],
  ["docs", "Documentação"],
  ["internal", "Interno"],
  ["chore", "Manutenção"],
  ["ci/cd", "CI/CD"],
]);

export function normalizeVersion(tagOrVersion) {
  const cleaned = String(tagOrVersion ?? "")
    .trim()
    .replace(/^v/i, "");
  if (!cleaned) {
    throw new Error("Release tag/version cannot be empty.");
  }
  return cleaned;
}

export function normalizeTag(tagOrVersion) {
  const version = normalizeVersion(tagOrVersion);
  return `v${version}`;
}

export function extractChangelogSection(changelogContent, tagOrVersion) {
  const targetVersion = normalizeVersion(tagOrVersion);
  const lines = changelogContent.replace(/\r\n/g, "\n").split("\n");

  const sectionHeaderRegex = /^##\s+\[([^\]]+)\](?:\s+-\s+([^\s]+))?.*$/;
  let startIndex = -1;
  let date = null;

  for (let i = 0; i < lines.length; i += 1) {
    const match = lines[i].match(sectionHeaderRegex);
    if (!match) {
      continue;
    }
    const headingVersion = match[1].trim().replace(/^v/i, "");
    if (headingVersion.toLowerCase() === targetVersion.toLowerCase()) {
      startIndex = i + 1;
      date = match[2]?.trim() || null;
      break;
    }
  }

  if (startIndex === -1) {
    throw new Error(
      `Could not find section '## [${targetVersion}]' in CHANGELOG.md.`
    );
  }

  const collected = [];
  for (let i = startIndex; i < lines.length; i += 1) {
    if (/^##\s+/.test(lines[i])) {
      break;
    }
    collected.push(lines[i]);
  }

  const body = collected.join("\n").trim();
  if (!body) {
    throw new Error(
      `Section '## [${targetVersion}]' in CHANGELOG.md is empty.`
    );
  }

  return {
    version: targetVersion,
    tag: normalizeTag(targetVersion),
    date,
    body,
  };
}

function shiftHeadingsToH4(markdown) {
  return markdown
    .split("\n")
    .map((line) => (line.startsWith("### ") ? `#${line}` : line))
    .join("\n")
    .trim();
}

export function localizeKeepAChangelogHeadings(markdown) {
  return markdown
    .split("\n")
    .map((line) => {
      const match = line.match(/^(#{3,6})\s+(.+?)\s*$/);
      if (!match) {
        return line;
      }
      const [, hashes, title] = match;
      const localized = KEEP_A_CHANGELOG_PT_MAP.get(title.toLowerCase());
      return localized ? `${hashes} ${localized}` : line;
    })
    .join("\n")
    .trim();
}

export function splitBilingualSections(sectionBody) {
  const lines = sectionBody.replace(/\r\n/g, "\n").split("\n");
  const enHeadingRegex = /^#{3,4}\s+(?:English|EN)\s*$/i;
  const ptHeadingRegex = /^#{3,4}\s+(?:Portugu[êe]s(?:\s*\(BR\))?|PT-BR)\s*$/i;

  const preamble = [];
  const enLines = [];
  const ptLines = [];
  let activeLang = null;

  for (const line of lines) {
    if (enHeadingRegex.test(line)) {
      activeLang = "en";
      continue;
    }
    if (ptHeadingRegex.test(line)) {
      activeLang = "pt";
      continue;
    }
    if (activeLang === "en") {
      enLines.push(line);
    } else if (activeLang === "pt") {
      ptLines.push(line);
    } else {
      preamble.push(line);
    }
  }

  const rawEn = enLines.join("\n").trim();
  const rawPt = ptLines.join("\n").trim();
  const rawPreamble = preamble.join("\n").trim();

  if (!rawEn && !rawPt) {
    const normalized = shiftHeadingsToH4(sectionBody);
    return {
      enRaw: sectionBody.trim(),
      en: normalized,
      pt: localizeKeepAChangelogHeadings(normalized),
    };
  }

  const withPreamble = (langBody) =>
    [rawPreamble, langBody].filter(Boolean).join("\n\n").trim();

  const effectiveEnRaw = rawEn ? withPreamble(rawEn) : withPreamble(rawPt);
  const enNormalized = shiftHeadingsToH4(effectiveEnRaw);
  const ptNormalized = rawPt
    ? shiftHeadingsToH4(withPreamble(rawPt))
    : localizeKeepAChangelogHeadings(enNormalized);

  return {
    enRaw: effectiveEnRaw,
    en: enNormalized,
    pt: ptNormalized,
  };
}

function stripInlineMarkdown(text) {
  return text
    .replace(/\[([^\]]+)\]\([^)]+\)/g, "$1")
    .replace(/\*\*([^*]+)\*\*/g, "$1")
    .replace(/__([^_]+)__/g, "$1")
    .replace(/\*([^*]+)\*/g, "$1")
    .replace(/`([^`]+)`/g, "$1")
    .replace(/\s+/g, " ")
    .trim();
}

function condenseBullet(text) {
  const cleaned = stripInlineMarkdown(text).replace(/[.;:]+$/, "");
  const colonMatch = cleaned.match(/^([^:]{3,55}):\s+(.+)$/);
  return colonMatch ? colonMatch[1].trim() : cleaned;
}

function truncateAtWordBoundary(text, maxLength) {
  if (text.length <= maxLength) {
    return text;
  }
  const slice = text.slice(0, Math.max(0, maxLength - 3));
  const lastSpace = slice.lastIndexOf(" ");
  const cut = lastSpace > maxLength * 0.5 ? slice.slice(0, lastSpace) : slice;
  return `${cut.replace(/[;,:\s]+$/, "")}...`;
}

export function buildUpdaterSummary({
  tag,
  changelogContent,
  changelogPath = DEFAULT_CHANGELOG_PATH,
  maxLength = 380,
}) {
  const resolvedContent =
    typeof changelogContent === "string"
      ? changelogContent
      : readFileSync(changelogPath, "utf8");
  const section = extractChangelogSection(resolvedContent, tag);
  const { enRaw } = splitBilingualSections(section.body);

  const lines = enRaw.split("\n");
  const introLines = [];
  const bullets = [];
  let seenHeadingOrList = false;

  for (const line of lines) {
    const trimmed = line.trim();
    if (!trimmed) {
      continue;
    }
    if (/^#{1,6}\s+/.test(trimmed)) {
      seenHeadingOrList = true;
      continue;
    }
    if (/^[-*]\s+/.test(line)) {
      seenHeadingOrList = true;
      bullets.push(line.replace(/^[-*]\s+/, "").trim());
      continue;
    }
    if (/^\s+[-*]\s+/.test(line)) {
      seenHeadingOrList = true;
      continue;
    }
    if (!seenHeadingOrList) {
      introLines.push(stripInlineMarkdown(trimmed));
    }
  }

  const intro = introLines.join(" ").trim();
  const fullBullets = bullets.map((item) =>
    stripInlineMarkdown(item).replace(/[.;:]+$/, "")
  );

  const prefix = `Release ${section.tag} — `;
  const fullBulletJoined = fullBullets.length > 0 ? `${fullBullets.join("; ")}.` : "";

  let details = "";
  if (intro && fullBullets.length === 0) {
    details = intro;
  } else if (!intro && fullBulletJoined) {
    details =
      prefix.length + fullBulletJoined.length <= maxLength
        ? fullBulletJoined
        : `${bullets.map(condenseBullet).join("; ")}.`;
  } else if (intro && fullBulletJoined) {
    const introWithDot = /[.!?]$/.test(intro) ? intro : `${intro}.`;
    const candidateFull = `${introWithDot} ${fullBulletJoined}`;
    if (prefix.length + candidateFull.length <= maxLength) {
      details = candidateFull;
    } else {
      details = `${introWithDot} ${bullets.map(condenseBullet).join("; ")}.`;
    }
  } else {
    details = stripInlineMarkdown(enRaw);
  }

  return truncateAtWordBoundary(`${prefix}${details}`, maxLength);
}

export function buildBilingualReleaseNotes({
  tag,
  setupName,
  portableName,
  changelogContent,
  changelogPath = DEFAULT_CHANGELOG_PATH,
}) {
  const resolvedContent =
    typeof changelogContent === "string"
      ? changelogContent
      : readFileSync(changelogPath, "utf8");
  const section = extractChangelogSection(resolvedContent, tag);
  const { en, pt } = splitBilingualSections(section.body);

  return `## English

### What's New in ${section.tag}
${en}

### Which download should I pick?
- **Installer (\`${setupName}\`)** *(Recommended)*: per-user Windows installation (no admin required), Start Menu shortcut, and **automatic background updates** (waits until you leave Champion Select / match before downloading).
- **Portable (\`${portableName}\`)**: standalone single executable with no installation. Does **not** auto-update — it only notifies you inside the app to download new releases manually from GitHub.

<!-- UNSIGNED NOTICE -->
> [!IMPORTANT]
> **Windows SmartScreen Notice:** Executables in this release are not Authenticode-signed with an EV/OV certificate yet, so Windows may show *"Windows protected your PC"*. Click **More info** → **Run anyway**.
>
> **Verify file integrity:** Compare the SHA-256 hash against \`SHA256SUMS.txt\` in PowerShell:
> \`\`\`powershell
> Get-FileHash "${setupName}" -Algorithm SHA256
> Get-FileHash "${portableName}" -Algorithm SHA256
> \`\`\`
<!-- END UNSIGNED NOTICE -->

---

## Português (BR)

### Novidades da versão ${section.tag}
${pt}

### Qual arquivo escolher?
- **Instalador (\`${setupName}\`)** *(Recomendado)*: instala apenas para o seu usuário (sem precisar de administrador), cria atalho no Menu Iniciar e possui **atualização automática em segundo plano** (aguarda você sair do Champ Select / partida para baixar).
- **Portátil (\`${portableName}\`)**: executável único sem instalação. **Não** aplica atualizações automáticas — apenas avisa dentro do app quando houver uma nova versão para download manual no GitHub.

<!-- UNSIGNED NOTICE -->
> [!IMPORTANT]
> **Aviso sobre o Windows SmartScreen:** Os executáveis ainda não possuem assinatura digital Authenticode (EV/OV), então o Windows pode exibir *"O Windows protegeu o computador"*. Clique em **Mais informações** → **Executar assim mesmo**.
>
> **Conferir integridade (SHA-256):** Compare o hash do arquivo baixado com o \`SHA256SUMS.txt\` no PowerShell:
> \`\`\`powershell
> Get-FileHash "${setupName}" -Algorithm SHA256
> Get-FileHash "${portableName}" -Algorithm SHA256
> \`\`\`
<!-- END UNSIGNED NOTICE -->
`;
}

function readArgValue(argv, index, flagName) {
  const value = argv[index + 1];
  if (!value || value.startsWith("-")) {
    throw new Error(`Missing value for ${flagName}`);
  }
  return value;
}

export function parseArgs(argv) {
  let tag = null;
  let setupName = null;
  let portableName = null;
  let outFile = null;
  let updaterOutFile = null;
  let changelogPath = DEFAULT_CHANGELOG_PATH;

  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === "--tag") {
      tag = readArgValue(argv, i, "--tag");
      i += 1;
    } else if (arg.startsWith("--tag=")) {
      tag = arg.slice("--tag=".length);
    } else if (arg === "--setup") {
      setupName = readArgValue(argv, i, "--setup");
      i += 1;
    } else if (arg.startsWith("--setup=")) {
      setupName = arg.slice("--setup=".length);
    } else if (arg === "--portable") {
      portableName = readArgValue(argv, i, "--portable");
      i += 1;
    } else if (arg.startsWith("--portable=")) {
      portableName = arg.slice("--portable=".length);
    } else if (arg === "--out") {
      outFile = readArgValue(argv, i, "--out");
      i += 1;
    } else if (arg.startsWith("--out=")) {
      outFile = arg.slice("--out=".length);
    } else if (arg === "--updater-out") {
      updaterOutFile = readArgValue(argv, i, "--updater-out");
      i += 1;
    } else if (arg.startsWith("--updater-out=")) {
      updaterOutFile = arg.slice("--updater-out=".length);
    } else if (arg === "--changelog") {
      changelogPath = readArgValue(argv, i, "--changelog");
      i += 1;
    } else if (arg.startsWith("--changelog=")) {
      changelogPath = arg.slice("--changelog=".length);
    } else {
      throw new Error(`Unknown option: ${arg}`);
    }
  }

  if (!tag || (!outFile && !updaterOutFile)) {
    throw new Error(
      "Usage: node scripts/generate-release-notes.mjs --tag <tag> [--setup <file> --portable <file> --out <path>] [--updater-out <path>] [--changelog <path>]"
    );
  }

  if (outFile && (!setupName || !portableName)) {
    throw new Error("--setup and --portable are required when --out is specified.");
  }

  return { tag, setupName, portableName, outFile, updaterOutFile, changelogPath };
}

const isMainModule =
  process.argv[1] && path.resolve(process.argv[1]) === path.resolve(__filename);
if (isMainModule) {
  const { tag, setupName, portableName, outFile, updaterOutFile, changelogPath } =
    parseArgs(process.argv.slice(2));

  if (outFile) {
    const content = buildBilingualReleaseNotes({
      tag,
      setupName,
      portableName,
      changelogPath,
    });
    writeFileSync(outFile, content, "utf8");
  }

  if (updaterOutFile) {
    const summary = buildUpdaterSummary({ tag, changelogPath });
    writeFileSync(updaterOutFile, `${summary}\n`, "utf8");
  }
}
