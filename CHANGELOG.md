# Changelog

All notable changes to **schumaz.gg** will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.5.1] - 2026-11-26

### English

Patch release fixing vertical clipping of the **Recent form** (`W`/`L`) streak badges on the **Profile** tab at `1280×720` and smaller window heights.

#### Fixed
- **Profile Recent Form Vertical Clipping (`ProfileTab.css`)**: Prevented vertical flex-shrinking on `[data-testid="winloss-chart-card"]`, `.recent-streak-block`, and `.streak-pill`, bounded `.profile-split-grid` to `grid-template-rows: minmax(0, 1fr)` on desktop (resetting to `auto` at `<= 860px`), and refined left-column vertical spacing so all 10 `W`/`L` badges remain 100% visible without clipping while keeping clean column-only scrolling when needed.

### Português (BR)

Release de correção eliminando o corte vertical dos badges `V`/`D` de **Sequência recente** (*Recent form*) na aba **Perfil** em `1280×720` e alturas menores de janela.

#### Corrigido
- **Corte Vertical da Sequência Recente no Perfil (`ProfileTab.css`)**: Impedido o encolhimento vertical em `[data-testid="winloss-chart-card"]`, `.recent-streak-block` e `.streak-pill`, limitado `.profile-split-grid` com `grid-template-rows: minmax(0, 1fr)` no desktop (resetando para `auto` em `<= 860px`) e ajustados os espaçamentos verticais da coluna esquerda para manter todos os 10 badges `V`/`D` 100% visíveis sem corte e com rolagem limpa apenas na coluna quando necessário.

## [0.5.0] - 2026-11-26

### English

Major feature release introducing **rich DataDragon tooltips**, **Skill Order priority & 1–18 leveling grid**, **lane-filtered Matchups & Counters**, **Classic ARAM build explorer & ARAM Mayhem build toggle**, **Swiftplay (`480`) lobby pre-selection & Clash (`700`) support**, and **Profile 2.0** (enriched match rows, incremental pagination, on-demand 5v5 & Arena scoreboard drawer, full mastery catalog modal, and direct Ranked build shortcuts).

#### Added
- **Rich DataDragon Tooltips (`Tooltip`, `BuildTooltips.tsx`, `MatchTooltips.tsx`, `tooltip_parser.rs`)**: Enriched local `.lz4` DataDragon caches (`pt_BR` / `en_US`) with item gold cost, parsed stat lines, and clean descriptions, sanitized rune `shortDesc`, summoner spell descriptions/cooldowns, and champion `Q`/`W`/`E`/`R` ability details, integrated across Runes, Item Blocks, Spells Bar, Skill Order, and Profile match history/scoreboards with viewport-aware positioning and `prefers-reduced-motion` support.
- **Champion Skill Order Priority & 1–18 Grid (`build_provider/skills.rs`, `SkillOrderPopover.tsx`, `SkillOrderCard.tsx`)**: Extracts champion `skill_masteries` / `skills` from REST and MCP payloads enriched with DataDragon ability names and icons, displaying skill priority badges (`Q > E > W`) and an expandable 1–18 leveling matrix in the Runes & Ranked headers.
- **Lane Matchups & Counters (`analytics/matchups.rs`, `LaneMatchupsPopover.tsx`, `LaneMatchupsCard.tsx`)**: Added lane-filtered **Hard** and **Favorable** matchups with real lane Win Rate and sample counts in Summoner's Rift Champ Select and Ranked detail views (clicking any opponent in Ranked opens their build).
- **Classic ARAM Build Explorer & ARAM Mayhem Build Toggle (`AramClassicChampionDetail.tsx`, `AramMayhemBuildPanel.tsx`)**: Made Classic ARAM tier list rows clickable to inspect full read-only builds (`RuneTree`, `SpellsBar`, `SkillOrderPopover`, `ItemBlocks`), and added an **Augments | Build & Spells** segmented toggle on ARAM Mayhem.
- **Top-2 Summoner Spell Pair Selection (`spells/pairs.rs`, `SpellsBar.tsx`)**: Added selectable top-2 summoner spell pairs (such as Flash + Ignite vs. Flash + Teleport) with automatic LCU injection sync and enlarged build selector icons with rich tooltips.
- **Swiftplay (`480`) Lobby Pre-Selection & Clash (`700`) Support (`ws_listener/lobby*.rs`, `lcu_injector/lobby_*.rs`, `LobbySlotsBar.tsx`)**: Listens to `/lol-lobby/v2/lobby` for Swiftplay/Quickplay pre-selection slots (`localMember.playerSlots`), renders selectable champion slots on the **Runes** tab, and applies compatible runes, summoner spells, and shop item sets directly in the lobby; maps Clash (`700`) to `QueueMode::SummonersRift` redirecting cleanly to Ranked (`420`) build data/cache.
- **Profile 2.0 — Enriched Match Rows (`MatchRowSummary.tsx`, `MatchItemsGrid.tsx`, `matches_parser.rs`)**: Displays Keystone + primary/secondary rune styles (or ARAM Mayhem/Arena Augments), dedicated trinket slot (`item6`), neutral **Remake** badge excluded from Win/Loss charts, and Arena placement badges (`1st`–`8th`), all wired with rich tooltips.
- **Profile 2.0 — Incremental Match History Pagination (`useMatchHistoryPagination.ts`, `get_player_match_history_page`)**: Preserves raw LCU cursor offsets (`begIndex`/`endIndex`) across Practice Tool filtering to load +10 matches on demand while dynamically updating Win/Loss and top-champion summaries.
- **Profile 2.0 — On-Demand 5v5 & Arena Scoreboard Drawer (`scoreboard_parser.rs`, `MatchScoreboardDrawer.tsx`, `useMatchScoreboard.ts`)**: Expanding any match fetches `/lol-match-history/v1/games/{gameId}` on demand to render team-grouped 5v5/ARAM scoreboards (`100` vs. `200`, team objectives, Riot ID, KDA + KP%, proportional damage bars, CS/min, vision, and 6 items + trinket) or placement-sorted Arena duos (`1st`–`8th` with augments and items), with graceful offline cache fallback.
- **Profile 2.0 — Full Champion Mastery Catalog & Direct Ranked Shortcuts (`ProfileMasteriesModal.tsx`, `rankedStorage.ts`, `useRanked.ts`)**: Persists `all_masteries` in `.lz4` cache with a searchable/sortable **View all** modal (showing total mastery level, total points, and unlocked champions) and makes champions clickable across Top 3 masteries, the catalog modal, and match scoreboards to jump straight to their Ranked build.

#### Changed
- **Runes Dashboard & Editorial Visual Hierarchy (`RuneTree.tsx`, `RunePagesSelector.tsx`, `tokens-shell.css`)**: Reorganized the Runes dashboard into cleaner sub-cards with squircle spell buttons, sequential item grids, unified `PatchProtectionIndicator`, standardized editorial headings across tabs, and fixed hero title descender clipping and gold augment frames.
- **Patch-Based DataDragon Expiration & Binary Cache Schema (`CACHE_SCHEMA_VERSION = 12`)**: Invalidates DataDragon disk caches cleanly on patch updates (`patch_state.rs`) and bumps `CACHE_SCHEMA_VERSION` to regenerate `.lz4` envelopes with tooltips, skill orders, matchups, and full Profile 2.0 metadata.

#### Fixed
- **ARAM Mayhem Asset Resilience & Augment Descriptions (`storage/assets/fetcher.rs`, `augments.rs`, `tooltip_clean.rs`)**: Hardened asset download retries/error handling and restored missing ARAM Mayhem augment descriptions.
- **Full Champion Masteries LCU Fallback (`lcu_connector/endpoints/summoner.rs`, `masteries_parser.rs`)**: Prioritized complete player mastery endpoints before falling back to `top?limit=3` and fixed localized champion name fallbacks.

### Português (BR)

Grande release de funcionalidades trazendo **tooltips ricos do DataDragon**, **prioridade de habilidades e matriz `1–18` (`Skill Order`)**, **confrontos e counters por rota**, **explorador de builds no ARAM Clássico e alternância Build/Augments no ARAM Desordem**, **pré-seleção no lobby do Jogo Rápido / Swiftplay (`480`) e suporte ao Clash (`700`)**, além do **Perfil 2.0** (linhas de partida enriquecidas, paginação incremental, placar completo 5v5 e Arena sob demanda, catálogo completo de maestrias e atalhos diretos para builds na Ranqueada).

#### Adicionado
- **Tooltips Ricos do DataDragon (`Tooltip`, `BuildTooltips.tsx`, `MatchTooltips.tsx`, `tooltip_parser.rs`)**: Enriquecimento do cache local `.lz4` (`pt_BR` / `en_US`) com custo em ouro, atributos limpos e descrição dos itens, `shortDesc` sanitizado de runas, descrição/tempo de recarga de feitiços e detalhes das habilidades (`Q`/`W`/`E`/`R`), aplicados em Runas, Blocos de Itens, Feitiços, Ordem de Habilidades e no histórico/placar do Perfil com posicionamento inteligente e suporte a `prefers-reduced-motion`.
- **Ordem de Habilidades (`Skill Order`: Prioridade `Q > E > W` e Grade `1–18`) (`build_provider/skills.rs`, `SkillOrderPopover.tsx`, `SkillOrderCard.tsx`)**: Extrai `skill_masteries` / `skills` dos payloads REST e MCP com nomes e ícones do DataDragon, exibindo badges de prioridade e matriz expansível de níveis `1–18` nos cabeçalhos das abas Runas e Ranqueada.
- **Confrontos & Counters por Rota (`analytics/matchups.rs`, `LaneMatchupsPopover.tsx`, `LaneMatchupsCard.tsx`)**: Exibe confrontos **Difíceis** e **Favoráveis** filtrados por rota com taxa de vitória real e amostragem na Ranqueada e no Champ Select de Summoner's Rift (clicável na Ranqueada para abrir a build do adversário).
- **Explorador de Builds no ARAM Clássico & Alternância no ARAM Desordem (`AramClassicChampionDetail.tsx`, `AramMayhemBuildPanel.tsx`)**: Linhas clicáveis na Tier List do ARAM Clássico para consultar a build completa em modo leitura (`RuneTree`, `SpellsBar`, `SkillOrderPopover`, `ItemBlocks`), além de controle segmentado (*Aprimoramentos* | *Build & Feitiços*) no ARAM Desordem.
- **Seleção dos Top-2 Pares de Feitiços (`spells/pairs.rs`, `SpellsBar.tsx`)**: Permite alternar entre os 2 principais pares de feitiços (ex.: Flash + Incendiar vs. Flash + Teleporte) com sincronização automática de injeção no LCU, ícones ampliados no seletor de builds com tooltips e indicador unificado de proteção de patch (`PatchProtectionIndicator`).
- **Suporte ao Swiftplay (`480`) Direto no Lobby & Clash (`700`) (`ws_listener/lobby*.rs`, `lcu_injector/lobby_*.rs`, `LobbySlotsBar.tsx`)**: Escuta `/lol-lobby/v2/lobby` para pré-seleção de campeões no Swiftplay/Quickplay (`localMember.playerSlots`), exibe slots selecionáveis na aba **Runas** e aplica runas, feitiços e conjuntos de itens compatíveis direto no lobby; mapeia Clash (`700`) para `QueueMode::SummonersRift` redirecionando para os dados/cache da Ranqueada (`420`).
- **Perfil 2.0 — Linha da Partida Completa (`MatchRowSummary.tsx`, `MatchItemsGrid.tsx`, `matches_parser.rs`)**: Exibe Keystone + árvore primária/secundária (ou Aprimoramentos em Arena/ARAM Desordem), slot dedicado para amuleto (`item6`), badge neutro de **Remake** excluído do cálculo de Vitórias/Derrotas e colocação da Arena (`1º`–`8º`), com tooltips ricos em toda a linha.
- **Perfil 2.0 — Paginação Incremental do Histórico (`useMatchHistoryPagination.ts`, `get_player_match_history_page`)**: Preserva o cursor bruto do LCU (`begIndex`/`endIndex`) ao filtrar partidas de Treino para carregar +10 jogos sob demanda, recalculando estatísticas de Vitórias/Derrotas e Campeão Destaque dinamicamente.
- **Perfil 2.0 — Placar Completo Sob Demanda 5v5 & Arena (`scoreboard_parser.rs`, `MatchScoreboardDrawer.tsx`, `useMatchScoreboard.ts`)**: Ao expandir uma partida, consulta `/lol-match-history/v1/games/{gameId}` exibindo placar 5v5/ARAM agrupado por `teamId` (`100` vs. `200`, objetivos da equipe, Riot ID, KDA + KP%, barra proporcional de dano, CS/min, visão e build `6 + amuleto`) ou ranking da Arena por duplas (`1º` ao `8º` com aprimoramentos e itens), com fallback gracioso offline via cache.
- **Perfil 2.0 — Catálogo Completo de Maestrias & Atalhos para Ranqueada (`ProfileMasteriesModal.tsx`, `rankedStorage.ts`, `useRanked.ts`)**: Persiste `all_masteries` no cache `.lz4` alimentando o modal pesquisável e ordenável **Ver todas** (com Nível Total, Pontos Totais e campeões habilitados), e torna os campeões clicáveis no Top 3, no catálogo e no placar expandido para abrir diretamente sua build na aba Ranqueada.

#### Alterado
- **Redesign do Painel de Runas & Hierarquia Tipográfica (`RuneTree.tsx`, `RunePagesSelector.tsx`, `tokens-shell.css`)**: Sub-cards organizados para páginas de runas/fragmentos, feitiços em *squircle*, grade sequencial de itens, hierarquia editorial padronizada entre abas e correção de corte em descendentes de títulos hero e molduras de aprimoramentos ouro.
- **Expiração DataDragon por Patch & Esquema de Cache (`CACHE_SCHEMA_VERSION = 12`)**: Expiração automática dos bundles DataDragon na troca de patch (`patch_state.rs`) e incremento de `CACHE_SCHEMA_VERSION` para regenerar caches `.lz4` com tooltips, ordem de habilidades, confrontos e metadados do Perfil 2.0.

#### Corrigido
- **Resiliência de Assets & Descrições no ARAM Desordem (`storage/assets/fetcher.rs`, `augments.rs`, `tooltip_clean.rs`)**: Endurecimento do download de assets e restauração das descrições faltantes de aprimoramentos no ARAM Desordem.
- **Fallback Completo de Maestrias no LCU (`lcu_connector/endpoints/summoner.rs`, `masteries_parser.rs`)**: Priorização dos endpoints completos de maestria antes de `top?limit=3` e correção de fallback localizado de nomes de campeões.

## [0.4.0] - 2026-11-21

### English

Consolidated public release since `v0.3.1`, delivering a native **System Tray** with **on-demand WebView2 unloading**, optional **Start with Windows**, a reorganized 4-section **Settings** experience, **rotating local diagnostic logs**, **100% Riot DataDragon & CommunityDragon assets** with MCP/REST-first **ARAM Mayhem** resolution, a **Champion Meta Tier badge** (`OP`, `Tier 1`–`4`) on the **Runes / Champ Select** header, and critical **LCU mid-draft detection** & **multi-monitor window** fixes.

#### Added
- **Reactive System Tray (`app_bootstrap/tray.rs`)**: Added a native Tauri v2 tray icon with localized context menu (**Open schumaz.gg**, read-only **LCU status** indicator, and **Quit**). Left-clicking the tray icon restores or recreates the main window; LCU status updates reactively via `lcu_connector::subscribe_lcu_connection()` (zero polling timers) and menu labels synchronize immediately on language changes.
- **Close to System Tray with Full WebView2 Unload (`bCloseToTray=False` default)**: Added the **Close to system tray** toggle under **General** settings (`UserSettings.ini` `[General]`). When enabled, clicking `X` saves window bounds and destroys the `"main"` WebView2 window while keeping the lightweight Rust LCU/WebSocket/auto-injection engine alive in the tray via `RunEvent::ExitRequested` (`prevent_exit()`), freeing 100% of WebView2 RAM/VRAM while playing League of Legends.
- **Start with Windows (`tauri-plugin-autostart`)**: Added backend-orchestrated Windows startup launch passing `--autostart` (`get_autostart_status` / `set_autostart_enabled`), disabled by default on NSIS installations and explicitly locked with an explanation on **Portable** builds to prevent broken entries in volatile folders (such as `Downloads`). The NSIS uninstaller removes the `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` registry entry cleanly on uninstall.
- **Silent `--autostart` Boot & Warm Tray Reopen Fast-Path**: Launching via `--autostart` skips creating the `"main"` WebView2 window and defers heavy UI prewarming until the first window open (prewarming only DataDragon from disk once LCU connects). Reopening the window from the tray skips `MIN_SPLASH_DURATION_MS` whenever disk caches are already ready.
- **Rotating Sanitized Local Logs & Diagnostic Export (`logging/`, `commands/diagnostics.rs`)**: Added zero-extra-dependency rotating local logs under `%LOCALAPPDATA%\schumazgg\Logs\` (`1 MiB` × 2 files, `logfmt`, asynchronous dedicated writer thread, panic hook, and mandatory sanitization stripping LCU tokens, ports, lockfiles, local paths, and summoner PII) plus a **Support & Diagnostics** subsection in Settings with **Copy diagnostic summary** and **Open logs folder**.
- **Champion Meta Tier Badge in Champ Select (`FiltersHeader.tsx`, `models/build.rs`)**: Added canonical meta tier (`0 = OP`, `1 = Tier 1`–`4 = Tier 4`) to `ChampionBuild` across REST (per-position in Ranked, omitting off-meta lanes and `is_rip` entries), MCP fallback, ARAM, and Arena parsers, rendering the color-coded tier pill next to the champion name on the **Runes** tab with graceful omission when unavailable.
- **Updater 24h Check Cooldown & `HEAD` Pre-Check (`updater.rs`, `useAppUpdater.ts`)**: Added a 24-hour cooldown for automatic update checks alongside a lightweight `HEAD` release pre-check before downloading the full updater manifest (manual checks in Settings bypass the cooldown).

#### Changed
- **Industry-Standard Settings Reorganization (`src/features/settings/`)**: Preserved the existing visual aesthetic (`SettingsTab.css`) while replacing the generic *Miscellaneous* group with 4 semantic sections: **General** (Language, Start with Windows, Close to System Tray), **Automation & Client** (Flash key, Auto-Inject, 48h Patch Protection, Auto-mark daily missions read), **Appearance & Performance** (Profile blur/wallpaper, GPU Hardware Acceleration), and **System & Storage** (Local Cache, Updates & Version, Support & Diagnostics).
- **100% DataDragon & CommunityDragon Assets (`storage/assets.rs`, `src/utils/assets.ts`)**: Removed `opgg-static.akamaized.net` from backend and frontend allowlists (keeping only `ddragon.leagueoflegends.com` and `raw.communitydragon.org`), enforced DataDragon champion icons/names and CommunityDragon augment metadata across ARAM Mayhem, and derived emergency patch fallback dynamically from the cached DataDragon version instead of hardcoded patch numbers.
- **ARAM Mayhem MCP/REST Primary Architecture (`analytics/aram/`)**: Switched ARAM Mayhem augments to query the official OP.GG MCP (`lol_list_aram_augments`) first and champion rankings to use REST JSON + DataDragon, isolating HTML extraction strictly for best-effort top-tier supplement (`tier < 3`) and last-resort fallback.
- **Canonical Tier Scale (`OP`, `Tier 1`–`4`) & Classic ARAM Normalization**: Normalized Classic ARAM tiers (`1..=5 → 0..=4`) to match Ranked, ARAM Mayhem, Arena, and Champ Select, and removed all nonexistent `Tier 5` references across UI filters, CSS classes, storage validators, parsers, and documentation.
- **Cache TTL Tuning & Anti-Flood Cooldown (`analytics/cooldown.rs`, `storage/lz4_cache.rs`)**: Tuned patch-versions disk TTL from `48h` to `3h` and ARAM Mayhem TTL to `60 min` RAM / `72h` disk (invalidated on patch changes), and added a `45s` negative-cache cooldown (`FailureCooldownStore`) after HTTP `429`/`5xx` or network failures.
- **Default `1280×720` Centered Window & Multi-Monitor Persistence**: Updated default first-launch dimensions to `1280×720` centered on the primary display and persisted `WindowMonitor` alongside physical virtual-desktop positioning and mixed-DPI restoration (with safe centered fallback when a saved monitor is disconnected).
- **Open-Source Legal & Developer Documentation (`README.md`, `README.pt-BR.md`, `ARCHITECTURE.md`, `docs/`)**: Clarified non-commercial open-source scope, legal disclaimers/trademarks, and high-level data flow; optimized Cargo `[profile.dev]` (`rlib`, lean debug tables) and automated `cargo-sweep` target hygiene (`npm run tauri:sweep:auto`).

#### Fixed
- **Immediate Mid-ChampSelect Recognition on Startup**: Eliminated the initial `4000ms` sleep on LCU connection (`tokio::join!` parallel session + queue probe), added the typed `get_active_champ_select_snapshot` bootstrap IPC command, registered frontend listeners before hydration to prevent dropped events or duplicate fetches, and coalesced `FINALIZATION` startup with live reconciliation.
- **ARAM Mayhem Augment Descriptions & Self-Healing Cache**: Restored augment descriptions with CDragon-first fallback and automatic `.lz4` cache backfill/auto-healing, stripped raw Riot inline markup tokens (`<OnHit>`, `<scale*>`, `@Spell.*@`, etc.), interpolated CDragon keybind/range tokens, and filtered internal/uncatalogued augment keys.
- **Ephemeral ARAM Mayhem Rarity Filter**: Removed persistence of the ARAM Mayhem rarity filter so the tab always opens on **All** instead of staying stuck on the previous session's filter.
- **Settings Section Header Truncation**: Prevented Settings section titles from truncating with ellipsis across narrower window widths.

### Português (BR)

Release pública consolidada desde a `v0.3.1`, trazendo **Bandeja do Sistema (*System Tray*)** reativa com **descarga completa do WebView2 sob demanda**, opção **Iniciar com o Windows**, aba de **Configurações** reorganizada em 4 seções semânticas, **logs locais rotativos e exportação de diagnóstico**, **100% assets DataDragon & CommunityDragon** com **ARAM Desordem** orientado a MCP/REST, **badge de Tier do Campeão** (`OP`, `Tier 1`–`4`) no cabeçalho de **Runas / Champ Select**, além das correções de **detecção imediata no meio do draft** e **restauração multi-monitor**.

#### Adicionado
- **Bandeja do Sistema Reativa (`app_bootstrap/tray.rs`)**: Ícone nativo na bandeja do Windows com menu localizado (**Abrir schumaz.gg**, indicador de status do **LCU** somente leitura e **Encerrar**). Um clique esquerdo restaura ou recria a janela principal; o status do LCU atualiza reativamente via `lcu_connector::subscribe_lcu_connection()` (zero polling extra) e os textos sincronizam imediatamente ao trocar o idioma.
- **Fechar para a Bandeja com Descarga Completa do WebView2 (`bCloseToTray=False` por padrão)**: Nova opção **Fechar para a bandeja do sistema** em **Geral** (`UserSettings.ini` `[General]`). Quando ativada, clicar no `X` salva os bounds da janela e destrói o WebView2 (`"main"`) mantendo o motor Rust de monitoramento LCU e injeção automática ativo perto do relógio via `RunEvent::ExitRequested` (`prevent_exit()`), liberando 100% da RAM/VRAM do WebView2 durante as partidas.
- **Iniciar com o Windows (`tauri-plugin-autostart`)**: Inicialização automática orquestrada 100% no backend passando `--autostart` (`get_autostart_status` / `set_autostart_enabled`), desligada por padrão em instalações NSIS e explicitamente bloqueada com explicação no modo **Portable** para evitar registros quebrados em pastas temporárias (como `Downloads`). O desinstalador NSIS limpa automaticamente a chave em `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`.
- **Boot Silencioso via `--autostart` e Reabertura Rápida da Bandeja**: Ao iniciar pelo `--autostart`, o app não cria a janela WebView2 e adia o prewarm pesado da interface até a primeira abertura (aquecendo apenas DataDragon sob demanda quando o LCU conectar). Ao reabrir a janela pela bandeja, o splash pula `MIN_SPLASH_DURATION_MS` quando o cache em disco já está pronto.
- **Logs Locais Rotativos Sanitizados & Exportação de Diagnóstico (`logging/`, `commands/diagnostics.rs`)**: Adicionado pipeline de logs locais sem dependências extras em `%LOCALAPPDATA%\schumazgg\Logs\` (`1 MiB` × 2 arquivos, formato `logfmt`, thread dedicada não-bloqueante, panic hook e sanitização obrigatória removendo tokens LCU, portas, lockfiles, caminhos locais e PII de invocador), além da subseção **Suporte e Diagnóstico** em Configurações com **Copiar resumo de diagnóstico** e **Abrir pasta de logs**.
- **Badge de Tier do Campeão no Champ Select (`FiltersHeader.tsx`, `models/build.rs`)**: Adicionado o tier canônico do meta (`0 = OP`, `1 = Tier 1`–`4 = Tier 4`) em `ChampionBuild` nos parsers REST (por rota na Ranqueada, omitindo rotas off-meta e campeões `is_rip`), fallback MCP, ARAM e Arena, exibindo o pill estilizado ao lado do nome do campeão na aba **Runas** com ocultação graciosa quando indisponível.
- **Cooldown de 24h e Pré-Checagem `HEAD` no Updater (`updater.rs`, `useAppUpdater.ts`)**: Checagens automáticas de atualização agora respeitam cooldown de `24h` com pré-checagem leve `HEAD` da release antes de baixar o manifesto completo (checagens manuais em Configurações ignoram o cooldown).

#### Alterado
- **Reorganização das Configurações em 4 Seções Semânticas (`src/features/settings/`)**: Mantendo intacta a estética visual (`SettingsTab.css`), o grupo genérico *Diversos* foi substituído por 4 seções organizadas: **Geral** (Idioma, Iniciar com o Windows, Fechar para a bandeja do sistema), **Automação e Cliente** (Tecla do Flash, Injeção Automática, Proteção de Patch 48h, Marcar missões diárias como lidas), **Aparência e Desempenho** (Perfil com desfoque/fundo e Aceleração de Hardware GPU) e **Sistema e Armazenamento** (Cache Local, Atualizações e Versão, Suporte e Diagnóstico).
- **100% Assets DataDragon & CommunityDragon (`storage/assets.rs`, `src/utils/assets.ts`)**: Removido `opgg-static.akamaized.net` das allowlists do backend e frontend (mantendo apenas `ddragon.leagueoflegends.com` e `raw.communitydragon.org`), padronizados ícones/nomes de campeões via DataDragon e metadados de augments via CommunityDragon no ARAM Desordem, e substituído o fallback de patch hardcoded pela derivação dinâmica a partir da versão do DataDragon.
- **Arquitetura MCP/REST Primária no ARAM Desordem (`analytics/aram/`)**: A busca de augments passou a consultar primeiro o OP.GG MCP oficial (`lol_list_aram_augments`) e o ranking de campeões a usar REST JSON + DataDragon, mantendo a extração HTML isolada apenas para complemento *best-effort* de tiers superiores (`tier < 3`) e último recurso.
- **Padronização Canônica de Tiers (`OP`, `Tier 1`–`4`) e Normalização do ARAM Clássico**: Normalizados os tiers do ARAM Clássico (`1..=5 → 0..=4`) para alinhar com Ranqueada, ARAM Desordem, Arena e Champ Select, removendo todas as referências ao inexistente `Tier 5` em filtros de UI, classes CSS, validadores de persistência, parsers e documentação.
- **Refinamento de TTLs de Cache & Cooldown Anti-Flood (`analytics/cooldown.rs`, `storage/lz4_cache.rs`)**: Reduzido o TTL de disco das listas de versões de `48h` para `3h`, ajustado o TTL do ARAM Desordem para `60 min` em RAM / `72h` em disco (invalidado na troca de patch) e adicionado cooldown negativo de `45s` (`FailureCooldownStore`) após HTTP `429`/`5xx` ou falhas de rede.
- **Janela Padrão `1280×720` Centralizada e Persistência Multi-Monitor**: Atualizado o tamanho padrão de primeira abertura para `1280×720` centralizado e adicionada persistência de `WindowMonitor` com restauração fiel em múltiplos monitores e DPIs distintos (com fallback centralizado seguro se o monitor salvo estiver desconectado).
- **Documentação Legal Open-Source & Higiene de Build Dev (`README.md`, `README.pt-BR.md`, `ARCHITECTURE.md`, `docs/`)**: Reforço dos avisos legais, escopo não-comercial e fluxo de dados; otimização do perfil `[profile.dev]` do Cargo (`rlib`, debug enxuto) e limpeza automática do `target` via `cargo-sweep` (`npm run tauri:sweep:auto`).

#### Corrigido
- **Reconhecimento Imediato de Champ Select Já em Andamento ao Abrir o App**: Eliminada a espera inicial de `4000ms` ao conectar ao LCU (`tokio::join!` paralelo para sessão + fila), adicionado o comando IPC tipado `get_active_champ_select_snapshot`, sincronizado o registro paralelo de listeners antes da hidratação (sem fetch duplicado) e coalescida a abertura nos segundos finais da `FINALIZATION`.
- **Descrições de Augments no ARAM Desordem & Auto-Cura de Cache**: Restauradas as descrições com fallback CDragon-first e backfill/auto-cura automática no cache `.lz4`, removidas tags/tokens Riot inline (`<OnHit>`, `<scale*>`, `@Spell.*@`, etc.), interpolados atalhos/alcances e filtradas chaves internas não catalogadas.
- **Reset do Filtro de Raridade no ARAM Desordem**: Removida a persistência do filtro de raridade para que a aba sempre abra em **Todos** ao iniciar o aplicativo.
- **Truncamento de Títulos nas Configurações**: Corrigido o truncamento com reticências nos títulos das seções de Configurações em larguras de janela menores.

## [0.3.3] - 2026-11-20

### English

Critical LCU detection bugfix ensuring **schumaz.gg** immediately recognizes an already-running Champion Select session when launched mid-draft across Summoner's Rift, ARAM, ARAM Mayhem, and Arena.

#### Fixed
- **Immediate Mid-ChampSelect Recognition on Startup**: Eliminated the initial `4000ms` sleep on LCU connection (`needs_immediate_poll` + `poll_lcu_session_once`), querying `/lol-champ-select/v1/session` and `/lol-gameflow/v1/session` in parallel (`tokio::join!`) and syncing `last_gameflow_phase` to immediately activate the `1500ms` adaptive polling cadence.
- **Consolidated LCU Bootstrap Snapshot (`get_active_champ_select_snapshot`)**: Added a typed `ActiveChampSelectSnapshot` IPC command returning `gameflow_phase`, `champ_selected`, and any compatible in-memory `build` in 0ms (or probing LCU on demand if not yet polled), eliminating lost `champ-selected` / `recommended-build` events emitted before frontend listeners mounted.
- **Race-Free Frontend Hydration & `FINALIZATION` Coalescing**: Registered all LCU event listeners in parallel before fetching the initial snapshot alongside `getSummonerStatus`, hydrating state and auto-navigating once to the **Runes** tab without duplicate build fetches, and coalescing startup during the final seconds of `FINALIZATION` so `spawn_latest` and `trigger_live_reconciliation` never run concurrent build fetches or LCU injections.

### Português (BR)

Correção crítica na detecção do LCU garantindo que o **schumaz.gg** reconheça imediatamente uma Seleção de Campeões já em andamento ao ser aberto no meio do draft em Summoner's Rift, ARAM, ARAM Desordem e Arena.

#### Corrigido
- **Reconhecimento Imediato de Champ Select Já em Andamento ao Abrir o App**: Eliminou a espera inicial de `4000ms` ao conectar ao LCU (`needs_immediate_poll` + `poll_lcu_session_once`), consultando `/lol-champ-select/v1/session` e `/lol-gameflow/v1/session` em paralelo (`tokio::join!`) e sincronizando `last_gameflow_phase` para ativar imediatamente o intervalo adaptativo de `1500ms`.
- **Snapshot IPC Consolidado de Bootstrap LCU (`get_active_champ_select_snapshot`)**: Adicionou comando tipado retornando `ActiveChampSelectSnapshot` (`gameflow_phase`, `champ_selected` e `build` compatível em 0ms quando já em memória, ou sondando o LCU sob demanda com disparo automático de injeção e timer de finalização), eliminando eventos `champ-selected` / `recommended-build` perdidos antes do mount do frontend.
- **Hidratação Sem Duplo Fetch e Coalescência na `FINALIZATION`**: Passou a aguardar o registro paralelo de todos os listeners LCU antes do snapshot inicial em paralelo a `getSummonerStatus`, hidratando fase/campeão/build e auto-navegando uma única vez para a aba **Runas** sem fetch duplicado, e coalescendo aberturas nos segundos finais da `FINALIZATION` para evitar fetch/injeção concorrentes entre `spawn_latest` e `trigger_live_reconciliation`.

## [0.3.2] - 2026-11-20

### English

UX polish release updating the default window layout to centered `1280×720`, adding accurate multi-monitor and mixed-DPI window restoration, and making the ARAM Mayhem augment rarity filter ephemeral per session.

#### Changed
- **Default `1280×720` Centered Window & Monitor Persistence**: Updated default first-launch window dimensions to `1280×720` centered on the primary display (`tauri.conf.json` and `window_bounds.rs`), and added `WindowMonitor` persistence in `UserSettings.ini` (`window.current_monitor()`) alongside physical virtual-desktop positioning and per-monitor DPI scaling. Reopening the app now restores the exact position, size, monitor, and maximized state chosen by the user across multi-monitor and mixed-DPI setups, with a safe centered `1280×720` clamped fallback whenever a saved monitor is disconnected.

#### Fixed
- **Ephemeral ARAM Mayhem Rarity Filter**: Removed `AramMayhemRarityFilter` / `aramMayhemRarityFilter` persistence from `UserSettings.ini`, IPC bindings, and the ARAM tab so the augment rarity filter always opens on **All** on startup instead of remaining stuck on the last selected rarity from a previous session.

### Português (BR)

Release de polimento de UX atualizando a janela padrão para `1280×720` centralizada, adicionando restauração fiel em setups multi-monitor com DPIs distintos e tornando o filtro de raridade do ARAM Desordem efêmero por sessão.

#### Alterado
- **Janela Padrão `1280×720` Centralizada e Persistência de Monitor**: Atualizou o tamanho padrão de primeira abertura para `1280×720` centralizado no monitor principal (`tauri.conf.json` e `window_bounds.rs`), e passou a salvar `WindowMonitor` em `UserSettings.ini` (`window.current_monitor()`) junto ao posicionamento físico no desktop virtual e escala DPI por monitor. Ao reabrir o aplicativo, a posição, o tamanho, o monitor e o estado maximizado escolhidos pelo usuário são restaurados com precisão inclusive em múltiplos monitores com DPIs diferentes, com fallback centralizado em `1280×720` e *clamping* seguro caso o monitor salvo esteja desconectado.

#### Corrigido
- **Reset do Filtro de Raridade no ARAM Desordem**: Removeu completamente a persistência de `AramMayhemRarityFilter` / `aramMayhemRarityFilter` do `UserSettings.ini`, dos contratos IPC e da aba ARAM, garantindo que o filtro de raridade das cartinhas sempre abra na opção **Todos** ao iniciar o aplicativo em vez de ficar preso na última raridade selecionada em sessões anteriores.

## [0.3.1] - 2026-11-19

### English

Consolidated public release since `v0.2.0` featuring full end-to-end **Arena (`3x6`)** support, build/spell/Tear accuracy fixes across Ranked & ARAM, and a refined `IdleIsland` loading animation.

#### Added
- **Dedicated `Arena` Tab**: Added the **Arena** tab between **Ranked** and **ARAM**, featuring a searchable **Arena (`3x6`)** Tier List (`OP`, `1`–`5`) with Top 3 / Win Rate, 1st Place Rate, and Pick/Ban metrics, plus a Champion Detail view with rarity-filterable **Augments** (*Silver*, *Gold*, *Prismatic*), **Best Synergies**, and 5-block Arena items (**Prismatic Items**, **Core 3**, **Boots**, **Final Build**, and **Situational**).
- **Live Arena (`3x6`) Support in Champion Select**: Automatically detects Arena queues (`1700`, `1710`, `1750`, `CHERRY` / `ARENA` / `TGR`, or Map `30`), injects only the **Map `30` Shop Item Set** (safely skipping disabled runes and summoner spells with a localized status notice), and displays Arena Augments, Best Synergies, Prismatic Items, and selectable Core 1/2/3 build options on the **Runes** tab.
- **Map `30` Arena Build Engine**: Separates `2750g` **Prismatic Items** without blocking `2500g` Arena legendaries or 4-digit Arena exclusives, filters anvils/consumables and low-sample troll boots, safely normalizes Arena Tear evolutions (`223121 → 223119`, `223042 → 223004`, `223040 → 223003`, `222530 → 222526`), and preserves AP/AD archetype cohesion.

#### Fixed
- **`IdleIsland` Loading Ring Animation**: Replaced the rotating rounded-square border on *Loading Profile* and *Loading Build* with a concentric SVG rounded-perimeter light trail aligned to the icon container's rounded geometry, eliminating protruding rotating corners while preserving automatic pausing in inactive tabs, minimized windows, GPU-effects-disabled mode, and `prefers-reduced-motion`.
- **Single-Game Troll Build Elimination**: Stopped inflating `play == 1` core combinations over consolidated builds (`play >= 2`), prevented empty build payloads from fabricating default items, and improved weighted AD/AP damage-type inference.
- **48h New-Patch Protection & ARAM Sample Quality**: Fixed 48h new-patch fallback priority against frozen timestamps, validated trend patch versions, and defaulted ARAM (`450`) and ARAM Mayhem (`2400`) build queries to `tier = "all"` (with automatic fallback whenever narrower tiers have fewer than `100` games).
- **Tear of the Goddess Item Ordering**: Preserved real legendary completion order in Core and Final builds so early Tear purchases no longer force *Manamune*, *Archangel's Staff*, *Whispering Circlet*, or *Winter's Approach* into slot 1.
- **Dynamic Mode-Based Summoner Spell Filtering**: Filtered recommended and injected summoner spells by DataDragon queue `modes` (`CLASSIC`, `ARAM`, `KIWI`), preventing disabled spells such as **Exhaust** in ARAM Mayhem and **Teleport** in ARAM/Mayhem.

#### Changed
- **Binary Cache Schema (`v4`)**: Automatically invalidates legacy local `.lz4` build and DataDragon caches on startup so Map `30` item classifications, localized Arena Augments, spell mode filters, and cleaned build samples regenerate seamlessly.

### Português (BR)

Release pública consolidada desde a `v0.2.0` trazendo suporte completo ao modo **Arena (`3x6`)**, correções de precisão para builds/feitiços/Lágrima em Ranqueada e ARAM, e uma animação de carregamento redesenhada na `IdleIsland`.

#### Adicionado
- **Aba dedicada `Arena`**: Nova aba **Arena** entre **Ranqueada** e **ARAM**, com Tier List pesquisável de **Arena (`3x6`)** (`OP`, `1`–`5`) exibindo Top 3 / Taxa de Vitória, Taxa de 1º Lugar e Escolha/Banimento, além da tela de detalhes do campeão com **Aprimoramentos (Augments)** filtráveis por raridade (*Prata*, *Ouro*, *Prismático*), **Melhores Sinergias** e itens em 5 blocos (**Itens Prismáticos**, **Core 3**, **Botas**, **Build Final** e **Situacionais**).
- **Suporte ao vivo para Arena (`3x6`) no Champion Select**: Detecta automaticamente filas de Arena (`1700`, `1710`, `1750`, `CHERRY` / `ARENA` / `TGR` ou Mapa `30`), injeta apenas o **Conjunto de Itens da Loja (Mapa `30`)** — pulando com segurança runas e feitiços desativados com aviso localizado — e exibe Aprimoramentos, Melhores Sinergias, Itens Prismáticos e opções selecionáveis de Core 1/2/3 na aba **Runas**.
- **Motor de Builds do Mapa `30` (Arena)**: Separa **Itens Prismáticos** (`2750g`) sem bloquear lendários compráveis de `2500g` ou exclusivos da Arena, filtra bigornas/consumíveis e botas sem amostra, normaliza evoluções da Lágrima no Mapa `30` (`223121 → 223119`, `223042 → 223004`, `223040 → 223003`, `222530 → 222526`) e preserva a coesão de arquétipo AP/AD.

#### Corrigido
- **Animação de Carregamento da `IdleIsland`**: Substituiu a borda giratória em *Carregando Perfil* e *Carregando Build* por uma trilha luminosa SVG concêntrica alinhada ao contorno arredondado do ícone, eliminando cantos giratórios salientes e mantendo pausa automática em abas inativas, janelas minimizadas, modo sem efeitos de GPU e `prefers-reduced-motion`.
- **Eliminação de Builds Troll de 1 Partida**: Removeu a inflação de combinações `play == 1` sobre builds consolidadas (`play >= 2`), impediu a criação de itens artificiais em payloads vazios e aprimorou a inferência ponderada de dano AD/AP.
- **Proteção de Novo Patch (48h) e Amostragem no ARAM**: Corrigiu a prioridade do fallback de 48h contra timestamps congelados, validou a versão nas tendências de patch e definiu `tier = "all"` por padrão no ARAM (`450`) e ARAM Desordem (`2400`), com fallback automático sempre que tiers mais restritos tiverem menos de `100` jogos.
- **Ordem de Itens da Lágrima da Deusa**: Preservou a ordem real de conclusão dos lendários nas builds Core e Final para que a compra antecipada da Lágrima não force *Manamune*, *Cajado do Arcanjo*, *Diadema Sussurrante* ou *Aproximação Invernal* no 1º slot.
- **Filtragem Dinâmica de Feitiços por Modo de Fila**: Passou a filtrar feitiços recomendados e injetados pelos `modes` do DataDragon (`CLASSIC`, `ARAM`, `KIWI`), evitando feitiços desativados como **Exaustão** no ARAM Desordem e **Teleporte** no ARAM/Desordem.

#### Alterado
- **Esquema de Cache Binário (`v4`)**: Invalida automaticamente caches `.lz4` antigos na inicialização para regenerar classificações de itens do Mapa `30`, Aprimoramentos localizados da Arena, filtros de feitiços por modo e amostras limpas de builds.

## [0.3.0] - 2026-11-19

Full end-to-end **Arena (`CHERRY` / `3x6`)** support: live Champion Select automation on the Runes tab, dedicated Arena tab between Ranked and ARAM, Map `30`-safe Arena build engine, and real-payload regression suite.

### Added
- **Dedicated `Arena` Tab (`src/features/arena/` & `NavigationDock`)**: Added the **Arena** tab between **Ranked** and **ARAM**, featuring a searchable **Arena (`3x6`)** Tier List grouped by Tier (`OP`, `1`–`5`) with Top 3 / Win Rate, 1st Place Rate, and Pick/Ban metrics, plus a Champion Detail view with rarity-filterable **Augments** (*Silver*, *Gold*, *Prismatic*), **Best Synergies**, and 5-block Arena items (**Prismatic Items**, **Core 3**, **Boots**, **Final Build**, and **Situational**).
- **Live Arena (`3x6`) Support on the `Runes` Tab (`src/features/champ-select/`)**: Automatically detects Arena queues (`1700`, `1710`, `1750`, `gameMode == "CHERRY" | "ARENA" | "TGR"`, or `map.id == 30`), injects only the **Map `30` Shop Item Set** (safely skipping disabled runes and summoner spells with a localized status notice), and renders Arena Augments, Best Synergies, Prismatic Items, and selectable Core 1/2/3 build options in Champ Select.
- **Arena Build Engine & Metadata (`build_provider/analytics/arena/` & `build_provider/ddragon/`)**: Added dedicated Arena Tier List and Champion Build endpoints (`get_arena_tier_list`, `get_arena_champion_build`) with localized `pt_BR` / `en_US` Augment metadata (`cherry-augments.json`) and Map `30` item classification:
  - Separates real `2750g` **Prismatic Items** into their own block without blocking purchasable `2500g` `44xxxx` legendaries (such as `447111` *Overlord's Bloodmail*) or 4-digit Arena exclusives (`3430`, `4010`–`4017` such as `4017` *Hellfire Hatchet*).
  - Filters Arena anvils and non-purchasable Shardholder/consumable items (`220000..=220012`), troll low-sample boots (`< 1.5%` pick rate), and invalid SR starters (`1055`/`1056`/`2003`).
  - Safely normalizes Arena Tear evolutions (`223042 → 223004`, `223040 → 223003`, `222530 → 222526`, and Riot's broken `223121 → 223119` instead of non-Map-30 `3119`), deduplicates permuted Core 3 trios, and preserves AP/AD archetype cohesion.
- **Real-Payload Arena Contract Test Suite (`src-tauri/tests/integration/arena_real_payloads.rs`)**: Added end-to-end integration tests running real Arena payloads for **Yuumi**, **Master Yi**, **Rakan**, **Volibear**, **Shaco**, and **Ezreal** alongside real DataDragon Map `30` metadata.

### Changed
- **Binary Cache Schema (`CACHE_SCHEMA_VERSION = 4`)**: Incremented `CACHE_SCHEMA_VERSION` in `src-tauri/src/storage/lz4_cache.rs` to safely regenerate cached DataDragon bundles with Map `30` item classifications (`map30_items`, `arena_prismatics`, `arena_consumables`) and localized Arena augments.

## [0.2.3] - 2026-11-19

Eliminated single-game troll builds (`play == 1`), fixed 48h new-patch protection priority, improved ARAM sample quality (`tier=all`), and hardened damage-type inference.

### Fixed
- **Single-Game Troll Build Inflation (`rest_parser.rs`)**: Stopped inflating real `play == 1` core combinations to synthetic weights (`10000 - idx`) that caused 1-game troll builds (such as AP Irelia `[6655, 3118, 4646]` in ARAM Mayhem) to overtake real consolidated builds (`[3153, 6610, 6333]`). Synthetic descending weights are now applied only when the entire array lacks explicit `"play"` statistics, while 1-game outliers are filtered out whenever consolidated combinations (`play >= 2`) exist.
- **48h New-Patch Protection & Trend Version Validation (`patch.rs`, `service_helpers.rs`, `analytics/ranked.rs`)**: Prioritized `latest_created_at_iso < 48h` (with `< 1000` games) over frozen timestamps from `prev_patch_last_created_at_iso`, matched each `data.trends.win` entry by `version`, and triggered fallback to the previous patch when a newly released patch has fewer than `MIN_MATURE_SAMPLE` (`30`) recorded games.
- **Empty Payload Validation & Damage-Type Inference (`rest_parser.rs`, `rest_extractors.rs`, `ddragon/parsers.rs`)**: Returned an error when both `core_items` and `last_items` are empty (even when `supports_runes == false`) so resilient fallback triggers instead of fabricating default items; aggregated weighted AD/AP tags across `core_items` + `last_items` (supplemented from DataDragon `stats`), mapping `FIGHTER`, `ASSASSIN`, and `MARKSMAN` to `"AD"` when no `"SpellDamage"` tags are present.
- **ARAM & ARAM Mayhem Sample Volume (`service.rs`, `service_helpers.rs`)**: Defaulted ARAM (`450`) and ARAM Mayhem (`2400`) build queries to `tier = "all"`, with automatic fallback from narrower tiers (such as `emerald_plus`) to `"all"` whenever sample volume is under `100` games.

### Changed
- **Binary Cache Schema (`CACHE_SCHEMA_VERSION = 3`)**: Incremented `CACHE_SCHEMA_VERSION` in `src-tauri/src/storage/lz4_cache.rs` to automatically purge previously cached builds affected by the `play == 1` inflation and 48h protection bugs.

## [0.2.2] - 2026-11-19

Accurate Tear of the Goddess (`3070`) item ordering across Core and Final builds.

### Fixed
- **Tear of the Goddess (`3070` / `323070` / `773070`) Core & Final Build Ordering**: Refactored `resolve_tear_item_ids`, `resolve_tear_to_completed_item`, and Tear component filtering across `classification.rs`, `blocks.rs`, `core_final_options.rs`, `options.rs`, `rest_parser.rs`, and `mcp_parser.rs` so an early Tear purchase no longer forces its legendary evolution (`Manamune`, `Archangel's Staff`, `Whispering Circlet`, `Winter's Approach`) into slot 1.
  - Drops raw Tear components when the combination already contains a completed/evolved Tear legendary, preserving real completion order (e.g. *Trinity Force → Manamune* on Ezreal, *Rod of Ages → Archangel's Staff* on Veigar, *Moonstone → Whispering Circlet* on Lulu, and Jayce's build order).
  - Drops raw Tear without inventing an upgrade or displacing the 3rd legendary when 3 completed items are already built or when no Tear evolution exists in the payload.
  - Appends a confirmed Tear evolution **after** completed legendaries only when fewer than 3 completed items are present alongside raw Tear (e.g. Seraphine / Lulu short core builds).

## [0.2.1] - 2026-11-19

Dynamic mode-based summoner spell filtering for ARAM Mayhem and Classic ARAM.

### Fixed
- **Dynamic Summoner Spell Filtering by Queue Mode**: Persisted DataDragon `summoner.json` `modes` metadata in `SpellMetadata` and dynamically filtered spell combinations, `spell_choices`, fast REST spells, and enrichment fallbacks by active queue mode (`"KIWI"` for ARAM Mayhem `2400`, `"ARAM"` for Classic ARAM `450`, and `"CLASSIC"` for Summoner's Rift), preventing disabled spells such as **Exhaust** in ARAM Mayhem and **Teleport** in ARAM/ARAM Mayhem from being recommended or injected.
- **ARAM/Mayhem Spell Fallbacks**: Replaced static `Exhaust` (`3`) and `Teleport` (`12`) fallbacks with mode-aware prioritization favoring **Mark / Snowball** (`32`), **Heal** (`7`), **Barrier** (`21`), **Ghost** (`6`), **Ignite** (`14`), and **Cleanse** (`1`).

### Changed
- **Binary Cache Schema (`CACHE_SCHEMA_VERSION = 2`)**: Incremented `CACHE_SCHEMA_VERSION` in `src-tauri/src/storage/lz4_cache.rs` to safely invalidate legacy `.lz4` caches missing spell `modes` metadata or containing disallowed ARAM Mayhem spells.

## [0.2.0] - 2026-11-18

Asset singleflight request deduplication, disk cache quota enforcement, and Settings local cache controls.

### Added
- **Local Cache Settings Control**: Added a **Local Cache** subsection in Settings (*Miscellaneous*) displaying total disk usage and an images vs. builds/data breakdown, along with a **Clear cache** modal to selectively clear **builds & data** (recommended), **images** (not recommended), or **both** via `get_local_cache_info` and `clear_local_cache` IPC commands.

### Performance
- **Singleflight `schumaz-asset://` Deduplication**: Coalesced concurrent cold-cache requests for the same asset URL in `src-tauri/src/storage/assets.rs` so simultaneous `<img>` tags share a single HTTP download and atomic disk write.
- **Asset Cache LRU Disk Quota (`1 GiB`)**: Added a `1 GiB` maximum disk quota for `Cache\assets\` in `src-tauri/src/storage/gc.rs`, evicting the least recently used files by `mtime` after the 7-day inactivity pass.

## [0.1.2] - 2026-11-18

Stability, security, localization, and accessibility hardening release.

### Fixed
- **Single-Instance Guard (`tauri-plugin-single-instance`)**: Prevented opening duplicate app windows that could contend over `WebView/`, `UserSettings.ini`, or trigger duplicate LCU injections; launching a second instance now restores (if minimized), shows, and focuses the existing `"main"` window.
- **Localized LCU Injection Reports**: `InjectionReport.message` in `src-tauri/src/lcu_injector/injection.rs` now respects the active `AppLocale` (`en_US` and `pt_BR`) instead of emitting fixed Portuguese strings.
- **Dynamic DataDragon Disk Fallback**: Replaced the hardcoded `"16.22.1"` fallback in `src-tauri/src/profile_provider/service.rs` with lookup of the latest cached DataDragon version saved on disk (`Cache/ddragon/*.lz4`).
- **Window Control Accessibility**: Added translated `aria-label` attributes (`Minimize` / `Maximize` / `Restore` / `Close`) to the custom window controls in `StatusDock.tsx`.

### Security
- **Explicit Tauri v2 Content Security Policy**: Replaced `"csp": null` in `src-tauri/tauri.conf.json` with a strict policy restricting scripts and connections to `'self'` and Tauri IPC, while permitting bundled styles and cached image assets via the `schumaz-asset:` (`http://schumaz-asset.localhost`) protocol.

### Changed
- **Typed Shop Block Kinds (`ItemBlockKind`)**: Added `ItemBlockKind` (`starter`, `core`, `boots`, `final`, `situational`, `other`) to the Rust/TypeScript IPC contract (`src-tauri/src/models/build.rs` → `src/types/bindings.ts`), migrating both backend item-set injection and `ItemBlocks.tsx` away from localized title substring matching.

## [0.1.1] - 2026-11-18

Internal architectural refactoring and codebase cleanup without user-facing feature changes.

### Changed
- **Modularization & Line Limits**: Split oversized Rust (`ws_listener/subscription.rs`, `app_bootstrap/window_events.rs`) and frontend (`AppTabsViewport.tsx`, `SettingsUpdatesSubsection.tsx`, `appBootstrapHelpers.ts`, `appUpdaterHelpers.ts`, and test suites) modules to strictly comply with the 350-line Rust and 250-line TS/TSX limits.
- **Shared UI Boundaries**: Promoted `FiltersHeader`, `filterOptions`, and `patchUiMeta` to `src/components/ui/`, removing cross-feature coupling between `ranked` and `champ-select`.
- **Architecture Documentation**: Updated `ARCHITECTURE.md` to reflect the current module layout and remove obsolete migration notes.

### Removed
- **Residual Template Command**: Removed unused `commands::greet` from `src-tauri/src/commands/lcu.rs` and `src-tauri/src/lib.rs`, regenerating `src/types/bindings.ts`.

## [0.1.0] - 2026-11-16

First public release of **schumaz.gg** for Windows 10/11 (64-bit).

### Added
- **Champion Select Automation**: automatic detection of champion hover and lock-in via the local LCU REST & WebSocket APIs, with final pre-match safety reconciliation (including late ARAM bench swaps).
- **Configurable Auto-Injection**: independent toggles for automatic **Runes**, **Summoner Spells** (with **Flash on D or F** preference), and **5-block Shop Item Sets**.
- **Ranked Tier List & Build Explorer**: champion rankings and detailed builds filtered by **Tier/Elo**, **Role**, **Region** (15 servers + Global), and **Patch**.
- **ARAM & ARAM Mayhem**: dedicated mode selector with **ARAM** champion rankings and **ARAM Mayhem** champion builds + **Augments** filtered by rarity (*Silver*, *Gold*, *Prismatic*, *Special*).
- **Summoner Profile Analytics**: summoner icon, level, client profile wallpaper backdrop (configurable blur), ranked emblems, Top 3 champion masteries, Win/Loss chart, and last 10 matches filterable by queue.
- **48h New Patch Protection**: automatically holds the previous consolidated patch during the first 48 hours of a new patch to avoid low-sample noise (can be overridden per filter or toggled in Settings).
- **Quality of Life & Performance**: bilingual UI (**English** and **Português BR**), optional automatic clearing of unread LoL client mission/objective badges, compressed local binary cache (LZ4 + MessagePack), adaptive idle polling, and GPU acceleration toggle.
- **Windows Packaging & Auto-Update**:
  - `currentUser` NSIS installer (`*-setup.exe`) with PT-BR/EN wizard, custom install path support, optional *"Delete app data"* uninstall cleanup, and silent background auto-updates (`tauri-plugin-updater`, deferred while in Champ Select or in-game).
  - Standalone Portable executable (`*-portable.exe`) with GitHub update notifications for manual download.
