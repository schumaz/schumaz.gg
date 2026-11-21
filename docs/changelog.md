# Histórico de Entregas — schumaz.gg

Resumo cronológico das entregas concluídas e arquivadas do projeto.

---

## 1. Fundação Desktop, ARAM Mayhem & Correções OP.GG/LCU (`P9.0` – `P9.2`)

- **[P9.0 - Feature] Suporte ao modo ARAM Mayhem** (`ws_listener`, `build_provider`, `FiltersHeader`, `App.tsx`)
  - Detecção da fila `2400` / `gameMode` Mayhem, compartilhamento de builds/feitiços por volume de partidas, bypass de injeção de runas no backend e card informativo no frontend.
- **[P9.0 - Bugfix] Contexto de fila e remoção completa de Rota no ARAM / ARAM Mayhem** (`FiltersHeader.tsx`, `useLcuEvents.ts`, `build_provider`, `analytics/client.rs`, `commands/build.rs`, `commands/injector.rs`)
  - Ocultação do seletor `Rota:` e omissão de `position` no frontend/backend para filas `450` e `2400`, correção de `parse_opgg_mcp_text` / `rest_parser.rs` para builds sem runas e uso do `queue_id` ativo em `inject_item_build`.
- **[P9.1 - UI/UX Desktop] Interface nativa de desktop Windows** (`App.tsx`, `App.css`)
  - Viewport lock na raiz (`100vw × 100vh`, `overflow: hidden`), navegação em dock flutuante (`.floating-dock`) e rolagem estritamente local.
- **[P9.1 - Bugfix & Performance] Otimização de feitiços no Champ Select e nomes PT-BR oficiais** (`ddragon.rs`, `spell_rules.rs`, `injection.rs`)
  - Remoção de espera bloqueante antes de injetar feitiços na troca de campeão, uso dos feitiços mais populares por campeão via `extract_summoner_spell_choices`, aplicação de `apply_flash_key_preference` em todos os modos e nomes oficiais DataDragon (`32 => "Marca"`, `13 => "Clareza"`, `12 => "Teleporte"`) com testes de regressão.
- **[P9.2 - UI/Texto] Padronização dos textos da interface** (`src/components/`, `src/App.tsx`)
  - Refinamento dos textos no topo, `IdleState.tsx`, `SettingsTab.tsx` e seletor `D` / `F` sem referências a terceiros ou executáveis.
- **[P9.2 - Bugfix] Itens iniciais reais e build completa alinhada ao OP.GG no ARAM** (`blocks.rs`)
  - Priorização da combinação inicial mais jogada no ARAM (sem forçar poção nem item exclusivo `2051`), preservação correta de `potion_id` e filtragem precisa de `known_starters` em `build_5_item_blocks_with_depth_lists`, validado com testes em payloads reais.

---

## 2. Internacionalização & Storage/Cache Persistente (`P9.3` – `P9.4`)

- **[P9.3 - Feature] Internacionalização completa (`pt_BR` / `en_US`)** (`src/i18n/`, `SettingsTab.tsx`, `usePreferences.ts`, `ddragon.rs`, `aram.rs`)
  - Dicionários tipados + `useTranslation()`, seletor de idioma em Configurações e parametrização por `locale` no backend (`DDRAGON_CACHE` com `RwLock` por idioma e URLs/cache de augments OP.GG), com testes de integridade das chaves.
- **[P9.4 - Performance/Arquitetura] Cache binário (`.lz4`) e configurações `.ini` em `%LOCALAPPDATA%\schumazgg\`** (`storage/`, `App.tsx`, `ProfileTab.tsx`, `lib.rs`)
  - Diretório unificado organizado em `Config\` (`UserSettings.ini` estilo Unreal com escrita atômica, parse tolerante a falhas e migração única do legado) e `Cache\` (`ddragon\`, `opgg\`, `profile\`, `assets\`).
  - Serialização leve com `MessagePack` + `LZ4` (`CacheEnvelope<T>`), validade de **24h** para OP.GG e Perfil (com *Stale-While-Revalidate* > 5 min e exibição instantânea do último invocador via `LastSummonerId`).
  - Entrega local de imagens via `schumaz-asset://`, *pre-warming* no startup, *fallback offline* (*Stale-While-Error*) e garbage collector em background para arquivos inativos > 7 dias.

---

## 3. Aba de Perfil, Polimento de UI/UX & Qualidade de Vida (`P9.6` – `P9.8`)

- **[P9.6 - Feature] Nova aba de Perfil como tela inicial** (`ProfileTab.tsx`, `NavigationDock.tsx`, `App.tsx`, backend LCU)
  - Aba posicionada como padrão inicial (`Perfil` → `Runas` → `Configurações`), exibindo ícone, nível, papel de parede do perfil (`backgroundSkinId`), Top 3 maestrias, gráfico Win/Loss e últimas 10 partidas filtráveis por modo (excluindo modo Treino), com testes unitários.
- **[P9.6 - UI/UX] Dropdowns customizados com ícones na barra de filtros** (`FiltersHeader.tsx`, `App.css`, `FiltersHeader.test.tsx`)
  - Substituição dos `<select>` nativos por dropdowns acessíveis exibindo ícones oficiais de **Elo**, **Rota** e bandeiras SVG para **Regiões** (com siglas como `BR`, `KR`, `NA`).
- **[P9.6 - Feature] Limpeza automática de missões e objetivos diários** (`SettingsTab.tsx`, `usePreferences.ts`, `lcu_connector/`)
  - Opção desligada por padrão (`autoMarkDailyMissionsRead`) que zera notificações via REST/WebSocket do LCU (`/lol-objectives/v1/...`, `/lol-missions/v1/...`) com varredura pós-conexão (~65s), debounce e proteção anti-loop.
- **[P9.7 - Feature/UX] Proteção de 48h para novos patches** (`FiltersHeader.tsx`, `SettingsTab.tsx`, `usePreferences.ts`, `patch.rs`)
  - Indicação de `(Mais atual)` e ícone `!` com tooltip de horas restantes durante a trava de 48h (ignorada ao escolher patch manualmente), mais toggle em Configurações com modal de aviso de não recomendação e testes.
- **[P9.8 - UI/Texto] Simplificação do badge global de conexão** (`StatusDock.tsx`, `StatusDock.test.tsx`)
  - Remoção de nick, `#tag` e nível do badge verde global (mantidos exclusivos da aba Perfil), exibindo apenas status limpo de conexão com o cliente.
- **[Bug Fix & UI Refactor] Consistência do `StatusDock` e tipografia Display/Branding** (`StatusDock.tsx`, `FiltersHeader.tsx`, `ProfileTab.tsx`, `App.css`)
  - Uniformização de cor e `backdrop-filter: blur` do `.status-badge` entre todas as abas e tipografia de destaque para nomes de campeão e invocador (`.champ-name`, `.profile-summoner-name` + `.profile-tagline`).

---

## 4. Resiliência, Performance Crítica & Refatoração Modular (`P10.0` & Arquitetura)

- **[P10.0 - Feature] Reconciliação final no fim do Champ Select** (`ws_listener`, `lcu_injector`, `useLcuEvents.ts`)
  - Revalidação do `champion_id` ativo via `/lol-champ-select/v1/session` nos instantes finais (`FINALIZATION`) para cobrir trocas rápidas e substituições no banco do ARAM, disparando re-injeção de segurança conforme suporte do modo (com ou sem runas).
- **[Bugfix Crítico - Performance] Redução drástica de CPU e GPU em segundo plano** (`lcu_connector/`, `ws_listener/polling.rs`, CSS modular)
  - **Backend:** eliminação de varreduras completas de processos (`sysinfo` com `exe`/`cmd`/PEB), reuso de `DefaultRequestClient` (`OnceLock`), cache de credenciais LCU e pausa de WebSocket/polling quando o cliente está fechado (polling adaptativo 4s idle / 1.5s Champ Select).
  - **Frontend/GPU:** troca de `offset-distance` + `mask-composite` por `transform: rotate`, pausa de animações em `.inactive-tab` e `.gpu-effects-disabled`, remoção de `blur(0px)` no-op e `content-visibility: auto` em listas longas.
- **[Arquitetura] Refatoração modular orientada a domínio/feature** (`ARCHITECTURE.md`, `.antigravityrules`, `src-tauri/src/`, `src/`)
  - Adoção de limites rígidos de tamanho (Rust <= 350 linhas; TS/TSX <= 250 linhas) e padrão Rust 2018+ (sem `mod.rs`).
  - **Backend:** modularização de `storage/`, `lcu_connector/`, `profile_provider/`, `build_provider/` (`ddragon`, `items`, `analytics`, `runes`), `ws_listener/`, `lcu_injector/`, `commands/` e `app_bootstrap.rs`.
  - **Frontend:** criação da camada `src/services/ipc/`, fatiamento de `i18n/` e `components/ui/`, migração para `src/features/{settings,profile,champ-select,ranked,aram}/` e modularização do CSS e testes.

---

## 5. Auto-Update, Empacotamento Windows & Lançamento `v0.1.0` (`P9.4` – `P9.5`, `P10.1` – `P10.5`)

- **[P9.4 - Release/DevOps] Auto-update em segundo plano estilo VS Code** (`tauri-plugin-updater`, `updater.rs`, `useAppUpdater.ts`, `StatusDock.tsx`)
  - Download silencioso e validação `.sig` em background para instalações NSIS (`currentUser` + `installMode: "passive"`), adiamento automático durante partidas (`ChampSelect`, `GameStart`, `InProgress`, `Reconnect`), cleanup de handle com `update.close()` e detecção confiável de modo **Portable** (feature `portable` + checagem de Registro `HKCU` e `current_exe()`) para apenas notificar e abrir Releases via `opener`.
- **[P9.5 - Release/DevOps] Empacotamento NSIS + Portable e UI de atualizações** (`src-tauri/nsis/installer.nsi`, `tauri.conf.json`, `SettingsMiscSection.tsx`, `scripts/build-desktop.mjs`)
  - Instalador NSIS bilíngue (PT-BR/EN) com destino padrão na raiz de `%LOCALAPPDATA%\schumazgg\` (customizável mantendo `Config/`, `Cache/` e `WebView/` fixos) e opção *"Delete app data"* no desinstalador.
  - Subseção *Atualizações e Versão* nas Configurações (badges *Instalado · Auto-update* vs. *Portable · Manual*, status e checagem manual) e script `npm run build:desktop` gerando `*-setup.exe` (+ `.sig`), `*-portable.exe` e `SHA256SUMS.txt` em `dist-release/`.
- **[P10.1–P10.5 - Release/Launch] Publicação oficial da `v0.1.0`** (`package.json`, `Cargo.toml`, `tauri.conf.json`, `CHANGELOG.md`, `.github/workflows/release.yml`, `scripts/generate-release-notes.mjs`)
  - Alinhamento da versão `0.1.0`, geração automática de `latest.json` (excluído do `SHA256SUMS.txt`) com suporte a `--tag`/`--notes`/`--notes-file` e cobertura em Vitest (`scripts/build-desktop.test.mjs`), além de workflow no GitHub Actions publicando como *Latest* via `gh` CLI com notas bilíngues (EN + PT-BR) e `TAURI_SIGNING_PRIVATE_KEY` nos Secrets.
  - Gatekeeper completo aprovado (`cargo clippy` padrão e `--features portable`, `cargo test`, `tsc`, `vitest`), smoke test real no Windows + League Client concluído e validação em produção confirmada tanto na versão instalada quanto na portable.

---

## 6. Saúde Arquitetural, Hardening, CI/CD & Lançamento `v0.2.0` (`P11.0` – `P11.3`)

- **[P11.0 - CI/DevOps] Workflow de CI em Push/PR + Extração Automática do `CHANGELOG.md`** (`.github/workflows/ci.yml`, `release.yml`, `scripts/generate-release-notes.mjs`, `scripts/check-line-limits.mjs`)
  - Criação de `ci.yml` executando `tsc --noEmit`, `vitest run`, `check-line-limits.mjs` (`350` Rust / `250` TS-TSX), `cargo clippy` (padrão e `--features portable` com `-D warnings`), `cargo test -q` e verificação de sincronismo de `src/types/bindings.ts` via `export_bindings`.
  - Evolução de `generate-release-notes.mjs` e `release.yml` para extrair dinamicamente a seção `## [<versão>]` do `CHANGELOG.md` (com suporte bilíngue EN/PT-BR e resumo limpo para o `latest.json`), eliminando textos fixos da `v0.1.0`.
- **[P11.1 - Refactor/Arquitetura] Teto estrito de linhas e remoção de código residual** (`ws_listener/subscription.rs`, `app_bootstrap/window_events.rs`, `AppTabsViewport.tsx`, `SettingsUpdatesSubsection.tsx`, `src/components/ui/FiltersHeader.tsx`, `ARCHITECTURE.md`)
  - Modularização dos arquivos acima do teto em submódulos e helpers coesos (100% do repositório dentro de `350` Rust / `250` TS-TSX), promoção de `FiltersHeader` / `filterOptions` / `patchUiMeta` para `src/components/ui/`, remoção do comando residual `commands::greet` e atualização de `ARCHITECTURE.md`.
- **[P11.2 - Bugfix/Hardening] Instância única, CSP Tauri v2, i18n no Injector e tipagem de blocos** (`lib.rs`, `tauri.conf.json`, `lcu_injector/injection.rs`, `profile_provider/service.rs`, `models/build.rs`, `ItemBlocks.tsx`, `StatusDock.tsx`)
  - Integração de `tauri-plugin-single-instance` restaurando/desminimizando/focando a janela `"main"`, política CSP explícita permitindo `'self'`, `ipc:` e `schumaz-asset:` (`http://schumaz-asset.localhost`), localização completa de `InjectionReport.message` (`pt_BR` / `en_US`), fallback dinâmico da versão DataDragon salva em disco (`Cache/ddragon/*.lz4`), enum tipado `ItemBlockKind` no contrato IPC e `aria-label` traduzidos nos controles da janela.
- **[P11.3 - Performance/Storage] Deduplicação *Singleflight*, cota LRU em disco e controle de cache local** (`storage/assets/singleflight.rs`, `storage/gc.rs`, `commands/system.rs`, `SettingsCacheSubsection.tsx`, `ClearCacheModal.tsx`)
  - Coalescência *singleflight* para downloads frios simultâneos no protocolo `schumaz-asset://`, cota máxima de `1 GiB` com despejo LRU por `mtime` em `Cache\assets\`, comandos IPC `get_local_cache_info` / `clear_local_cache` e nova subseção *Cache Local* nas Configurações com modal de confirmação e seleção de escopo (*Builds & dados*, *Imagens* ou *Ambos*).
- **Estabilização de testes & publicação oficial da `v0.2.0`** (`CHANGELOG.md`, `versions_tests.rs`, `ranked/service_tests.rs`, `cache_tests.rs`, `service_tests.rs`)
  - Isolamento de prefixos de URL nos mocks de testes para eliminar colisões por reuso de porta efêmera no `OPGG_VERSIONS_CACHE`, CI verde na `main`, release `v0.2.0` assinada e publicada no GitHub e **primeira atualização automática em produção (`v0.1.0` → `v0.2.0` via NSIS) validada com sucesso**.

---

## 7. Correções de Builds/Feitiços, Modo Arena (`3x6`), Polimento UX & Detecção LCU (`P11.4` – `P11.10` · `v0.2.1` – `v0.3.3`)

- **[P11.4 - Bugfix Feitiços/ARAM Mayhem] Filtragem dinâmica de feitiços por modo (`v0.2.1`)** (`ddragon/parsers.rs`, `models/build.rs`, `spells/`, `service_helpers.rs`, `ddragon/enrichment.rs`, `lz4_cache.rs`)
  - Persistência do campo `modes: Vec<String>` em `SpellMetadata` (incrementando `CACHE_SCHEMA_VERSION = 2`) e filtragem dinâmica de combinações, `spell_choices`, feitiços REST e fallbacks conforme a fila ativa (`"KIWI"` para ARAM Mayhem `2400`, `"ARAM"` para ARAM Clássico `450` e `"CLASSIC"` para SR), bloqueando *Exaustão* (`3`) no ARAM Mayhem e *Teleporte* (`12`) no ARAM/Mayhem e priorizando *Marca/Bola de Neve* (`32`) nos fallbacks.
- **[P11.5 - Bugfix Itens] Ordenação precisa da Gota da Deusa (`3070`) em Core e Build Final (`v0.2.2`)** (`items/classification.rs`, `items/blocks.rs`, `items/core_final_options.rs`, `items/options.rs`, `rest_parser.rs`, `mcp_parser.rs`)
  - Refatoração de `resolve_tear_item_ids` e `resolve_tear_to_completed_item` para não forçar evoluções da Gota (`Manamune`, `Cajado do Arcanjo`, `Whispering Circlet`, `Aproximação Invernal`) no 1º slot: descarta o componente `3070` quando o lendário evoluído ou 3 itens completos já estão presentes e só anexa a evolução após os lendários já fechados em cores curtos, coberto com testes para Ezreal, Lulu, Veigar, Jayce e Seraphine.
- **[P11.6 - Bugfix Crítico Builds/Patch] Eliminação de builds troll (`play == 1`), proteção de 48h e inferência de dano (`v0.2.3`)** (`rest_parser.rs`, `rest_extractors.rs`, `patch.rs`, `service.rs`, `service_helpers.rs`, `analytics/ranked.rs`, `lz4_cache.rs`)
  - Remoção da inflação sintética de entradas `play == 1` (evitando builds troll de 1 jogo como Irelia AP no ARAM Mayhem), priorização de `latest_created_at_iso < 48h` (com validação de `version` em `data.trends.win` e fallback abaixo de `MIN_MATURE_SAMPLE = 30`), padrão `tier = "all"` (e fallback `< 100` jogos) no ARAM/Mayhem, erro explícito em payloads vazios, inferência agregada de dano AD/AP e `CACHE_SCHEMA_VERSION = 3`.
- **[P11.7 - Feature Novo Modo] Suporte completo ao Modo Arena (`CHERRY` / `3x6`) na aba `Runas` + nova página `Arena` (`v0.3.0`)** (`models/arena.rs`, `ws_listener/extract.rs`, `lcu_injector/`, `build_provider/analytics/arena/`, `build_provider/ddragon/augments.rs`, `src/features/arena/`, `src/components/ui/Arena*.tsx`, `src/features/champ-select/`, `arena_real_payloads.rs`)
  - Detecção automática da Arena (`1700`, `1710`, `1750`, `CHERRY`/`ARENA`/`TGR`, `map.id == 30`), injeção segura apenas do Item Set do Mapa `30` (pulando runas e feitiços desativados com aviso localizado) e motor de builds Arena com classificação segura de Prismáticos (`2750g`) sem bloquear lendários `44xxxx` compráveis (`447111`, `2500g`) nem exclusivos de 4 dígitos (`4017`), filtro de bigornas (`220000..=220012`) e botas troll, normalização segura de Gota (`223121 → 223119`), `CACHE_SCHEMA_VERSION = 4` e suite de contrato com JSONs reais (Yuumi, Master Yi, Rakan, Volibear, Shaco e Ezreal).
  - Nova aba **Arena** entre **Ranqueada** e **ARAM** no `NavigationDock` com Tier List (`3x6`, 6 trios / 18 jogadores), busca, filtro de Tier (`OP` / `1`–`5`), detalhe do campeão com **Aprimoramentos** filtráveis por raridade (*Prata*, *Ouro*, *Prismático*), **Melhores Sinergias** e 5 blocos de itens da Arena, além de suporte completo ao vivo na aba `Runas`.
- **[P11.8 - Bugfix UI/Animação] Substituição do quadrado girando em `IdleIsland` (`.idle-spinner-ring`) (`v0.3.1`)** (`IdleState.tsx`, `ChampSelect.css`, `loading-skeleton.css`, `tokens-shell.css`, `IdleState.test.tsx`)
  - Substituição da rotação de borda quadrada (`animation: spin`) em `.idle-icon-box.is-loading .idle-spinner-ring` (*Carregando Perfil* e *Carregando Build*) por um `<svg><rect pathLength="100" /></svg>` concêntrico com trilha de luz perimetral (`idleSpinnerTrailOrbit` / `idleSpinnerBeamOrbit`) alinhada ao `border-radius` do container, mantendo pausa/ocultação em `.inactive-tab`, `.app-minimized`, `.gpu-effects-disabled` e `prefers-reduced-motion`.
- **[P11.9 - UX / Polimento] Janela padrão `1280×720` centralizada, persistência de monitor & reset do filtro de raridade no ARAM Mayhem (`v0.3.2`)** (`tauri.conf.json`, `storage/window_bounds.rs`, `storage/settings/`, `app_bootstrap.rs`, `app_bootstrap/window_events.rs`, `AppTabsViewport.tsx`, `AramTab.tsx`)
  - Remoção completa da persistência de `AramMayhemRarityFilter` / `aramMayhemRarityFilter` (em `UserSettings.ini`, contratos IPC/`bindings.ts` e frontend), mantendo o filtro de raridade das cartinhas do ARAM Mayhem como estado local efêmero sempre iniciado em **Todos (`All`)**.
  - Atualização da janela padrão para `1280×720` centralizada na primeira abertura e persistência de `WindowMonitor` (`window.current_monitor()`) com restauração física multi-monitor/DPI distinto (incluindo janela maximizada e fallback centralizado em `1280×720` com *clamping* seguro quando o monitor salvo está desconectado).
- **[P11.10 - Bugfix Crítico LCU] Reconhecimento imediato de Champ Select já em andamento ao abrir o app (`v0.3.3`)** (`ws_listener/polling.rs`, `ws_listener/session_handler.rs`, `ws_listener/reconcile_state.rs`, `ws_listener/reconcile.rs`, `commands/lcu.rs`, `models/ws.rs`, `src/services/ipc/lcu.ipc.ts`, `src/features/champ-select/`)
  - Sondagem imediata (`0ms` via `needs_immediate_poll` + `poll_lcu_session_once`) e paralela (`tokio::join!` para sessão + `queue_id`) ao conectar ao LCU, sincronizando `last_gameflow_phase` para ativar o intervalo adaptativo de `1500ms` e coalescendo a abertura nos segundos finais de `FINALIZATION` para evitar fetch/injeção concorrentes entre `spawn_latest` e `trigger_live_reconciliation`.
  - Novo comando IPC tipado `get_active_champ_select_snapshot` (`ActiveChampSelectSnapshot`) com build em `0ms` quando já em memória ou sondagem sob demanda, acoplado ao registro paralelo de listeners antes do snapshot no frontend (`lcuEventListeners.ts`, `lcuSnapshotHydration.ts`, `useLcuEvents.ts`) sem fetch duplicado e com suíte de regressão Rust + Vitest (SR, ARAM, ARAM Mayhem, Arena e `FINALIZATION`).
