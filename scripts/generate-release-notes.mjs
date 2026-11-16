import { writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

export function buildBilingualReleaseNotes({ tag, setupName, portableName }) {
  return `## English

### What's New in ${tag}
- **Champion Select Automation**: automatic detection of champion hover/lock-in with final pre-match reconciliation (including late ARAM bench swaps) and independent toggles for **Runes**, **Summoner Spells** (Flash on D or F), and **5-block Shop Item Sets**.
- **Ranked & ARAM / ARAM Mayhem**: browse Ranked tier lists & builds (Tier/Elo, Role, Region, Patch), classic ARAM rankings, and ARAM Mayhem Augments filtered by rarity (*Silver*, *Gold*, *Prismatic*, *Special*).
- **Summoner Profile Analytics**: summoner icon, level, profile wallpaper backdrop, ranked crests, Top 3 masteries, Win/Loss chart, and last 10 matches filterable by queue.
- **48h New Patch Protection & QoL**: holds the previous consolidated patch during the first 48h after a patch drops, bilingual UI (**English** / **Português BR**), optional unread mission badge cleanup, and lightweight Rust + Tauri v2 footprint.

### Which download should I pick?
- **Installer (\`${setupName}\`)** *(Recommended)*: per-user Windows installation (no admin required), Start Menu shortcut, and **automatic background updates** (waits until you leave Champion Select / match before downloading).
- **Portable (\`${portableName}\`)**: standalone single executable with no installation. Does **not** auto-update — it only notifies you inside the app to download new releases manually from GitHub.

> [!IMPORTANT]
> **Windows SmartScreen Notice:** Executables in this release are not Authenticode-signed with an EV/OV certificate yet, so Windows may show *"Windows protected your PC"*. Click **More info** → **Run anyway**.
>
> **Verify file integrity:** Compare the SHA-256 hash against \`SHA256SUMS.txt\` in PowerShell:
> \`\`\`powershell
> Get-FileHash "${setupName}" -Algorithm SHA256
> Get-FileHash "${portableName}" -Algorithm SHA256
> \`\`\`

---

## Português (BR)

### Novidades da versão ${tag}
- **Automação no Champion Select**: detecção automática de *hover* e *lock-in* com reconciliação final antes da partida (incluindo trocas de última hora no banco do ARAM) e ativação independente de **Runas**, **Feitiços de Invocador** (Flash no D ou F) e **Conjuntos de Itens** em 5 blocos na loja.
- **Ranqueada & ARAM / ARAM Desordem (Mayhem)**: tier lists e builds ranqueadas (por Elo, Rota, Região e Patch), ranking do ARAM clássico e **Cartinhas (Augments)** do ARAM Desordem filtráveis por raridade (*Prata*, *Ouro*, *Prismática*, *Especial*).
- **Perfil do Invocador**: ícone, nível, wallpaper do perfil, brasões de elo, Top 3 maestrias, gráfico de Vitórias/Derrotas e últimas 10 partidas filtráveis por modo.
- **Proteção de Novo Patch (48h) & Qualidade de Vida**: mantém o patch anterior consolidado nas primeiras 48h após novo patch, interface bilíngue (**Português BR** / **English**), limpeza automática opcional de missões não lidas no cliente do LoL e baixo consumo com Rust + Tauri v2.

### Qual arquivo escolher?
- **Instalador (\`${setupName}\`)** *(Recomendado)*: instala apenas para o seu usuário (sem precisar de administrador), cria atalho no Menu Iniciar e possui **atualização automática em segundo plano** (aguarda você sair do Champ Select / partida para baixar).
- **Portátil (\`${portableName}\`)**: executável único sem instalação. **Não** aplica atualizações automáticas — apenas avisa dentro do app quando houver uma nova versão para download manual no GitHub.

> [!IMPORTANT]
> **Aviso sobre o Windows SmartScreen:** Os executáveis ainda não possuem assinatura digital Authenticode (EV/OV), então o Windows pode exibir *"O Windows protegeu o computador"*. Clique em **Mais informações** → **Executar assim mesmo**.
>
> **Conferir integridade (SHA-256):** Compare o hash do arquivo baixado com o \`SHA256SUMS.txt\` no PowerShell:
> \`\`\`powershell
> Get-FileHash "${setupName}" -Algorithm SHA256
> Get-FileHash "${portableName}" -Algorithm SHA256
> \`\`\`
`;
}

function parseArgs(argv) {
  let tag = null;
  let setupName = null;
  let portableName = null;
  let outFile = null;

  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === "--tag") {
      tag = argv[++i];
    } else if (arg === "--setup") {
      setupName = argv[++i];
    } else if (arg === "--portable") {
      portableName = argv[++i];
    } else if (arg === "--out") {
      outFile = argv[++i];
    } else {
      throw new Error(`Unknown option: ${arg}`);
    }
  }

  if (!tag || !setupName || !portableName || !outFile) {
    throw new Error("Usage: node scripts/generate-release-notes.mjs --tag <tag> --setup <file> --portable <file> --out <path>");
  }

  return { tag, setupName, portableName, outFile };
}

const isMainModule =
  process.argv[1] && path.resolve(process.argv[1]) === path.resolve(__filename);
if (isMainModule) {
  const { tag, setupName, portableName, outFile } = parseArgs(process.argv.slice(2));
  const content = buildBilingualReleaseNotes({ tag, setupName, portableName });
  writeFileSync(outFile, content, "utf8");
}
