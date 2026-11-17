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
