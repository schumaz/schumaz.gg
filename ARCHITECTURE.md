# Arquitetura — schumaz.gg

App desktop **Tauri v2** (backend **Rust** + frontend **React / TypeScript / Vite**) que detecta o Champion Select via LCU, busca builds no OP.GG + DataDragon e injeta runas, feitiços e item sets automaticamente.

## 1. Fluxo de Dados
```text
League Client (LCU: REST + WebSocket)
   ▲                              │
   │ lcu_connector / lcu_injector │ ws_listener
   ▼                              ▼
Backend Rust (src-tauri/src) ◄──► OP.GG & DataDragon (build_provider)
   │
   │ Tauri IPC tipado (tauri-specta → src/types/bindings.ts)
   ▼
Camada IPC Frontend (src/services/ipc/)
   │
   ▼
Frontend React (src/features/<dominio>/ → src/components/ui/)
```

### Cadeia de Resolução e Resiliência de Builds (`build_provider`)
```text
Cache RAM (BUILD_CACHE) + reidratação DataDragon
   │ miss / expirado
   ▼
Cache em disco (.lz4 + MessagePack, validado por schema_version, TTL e patch) + reidratação DataDragon
   │ miss / stale
   ▼
Rede: API REST OP.GG + OP.GG MCP (tools/call) [proteção de novo patch 48h + cooldown anti-flood 45s]
   │ erro / indisponibilidade
   ▼
Stale-While-Error em disco (envelope expirado reidratado via DataDragon)
   │ sem cache em disco
   ▼
Fallbacks (MCP completo / HTML isolado no ARAM Mayhem → build_fallback_champion_build)
   │
   ▼
Modelo canônico ChampionBuild ──► lcu_injector (runas, feitiços, item sets) & IPC tipado (UI)
```

- **Reidratação 100% DataDragon / CommunityDragon:** nomes localizados, ícones e catálogos (campeões, runas, feitiços, itens e augments) são reidratados localmente via `DataDragonCache` (DataDragon + CommunityDragon) antes de entregar o `ChampionBuild` ao `lcu_injector` ou à UI. As allowlists de imagens (`storage/assets.rs` e `src/utils/assets.ts`) aceitam exclusivamente `ddragon.leagueoflegends.com` e `raw.communitydragon.org`.
- **TTLs, anti-flood e ARAM Mayhem:** listas de versões expiram em `3h` no disco (`VERSIONS_DISK_TTL_SECS`), builds/tier lists em `48h` (`OPGG_CACHE_TTL_SECS`) e ARAM Mayhem em `60 min` (RAM) / `72h` (disco, invalidado por patch), com *negative-cache/cooldown* de `45s` (`analytics/cooldown.rs`) após `429`/`5xx`/erros de rede. No ARAM Mayhem, a lista de campeões usa REST JSON (`OPGG_ARAM_REST_URL`) e os augments usam `OP.GG MCP` (`lol_list_aram_augments`) como fonte primária, mantendo `aram/html_extract.rs` isolado para complemento *best-effort* de tiers `tier < 3` e fallback final controlado por `ARAM_MAYHEM_HTML_FALLBACK_ENABLED`.
- **Escala canônica de tiers (`0..=4`):** todos os parsers (Ranqueada, ARAM Clássico/Mayhem, Arena e builds no Champ Select) normalizam os tiers 1-based (`1..=5`) do OP.GG para a escala canônica 0-based `0 = OP`, `1 = Tier 1`, `2 = Tier 2`, `3 = Tier 3` e `4 = Tier 4`, compartilhada na UI via `src/components/ui/championTier.ts`.
- **Conformidade (*Riot Game Integrity*):** o app opera 100% localmente via LCU, sem leitura de memória nem automação em partida. Caso o projeto venha a ser registrado futuramente no *Riot Developer Portal*, revisar a conformidade com políticas específicas por modo (ex.: restrições de visibilidade de *win rates* no modo Arena).

## 2. Regras de Modularização e Limites
- **Teto de linhas por arquivo:**
  - **Rust (`src-tauri/src/` e `src-tauri/crates/`):** máximo de **350 linhas** por arquivo (extraindo `*_tests.rs` ou submódulos coesos).
  - **Frontend (`src/`):** máximo de **250 linhas** por arquivo `.ts`/`.tsx` (exceto `src/types/bindings.ts`, gerado automaticamente).
- **Padrão de módulos Rust 2018+:** usar `modulo.rs` + diretório `modulo/` em vez de `mod.rs` genéricos.
- **Isolamento IPC no Frontend:** toda chamada a `invoke`, `listen`, `convertFileSrc` e consumo de `src/types/bindings.ts`, `@tauri-apps/api` ou `@tauri-apps/plugin-*` fica encapsulada em `src/services/ipc/`. Componentes `.tsx` nunca importam pacotes `@tauri-apps/*` diretamente.
- **Separação UI vs. Domínio:** `src/features/<dominio>/` concentra componentes, hooks, tipos e estilos do domínio; `src/components/ui/` contém apenas componentes visuais puros.

## 3. Estrutura Principal

```text
├── docs/                          # Referências curtas (LCU, OP.GG, Tauri v2, UI)
├── scripts/                       # Automação de release, gatekeeper de linhas e higiene do target Cargo (run-tauri, sweep-cargo-target)
├── src/
│   ├── services/ipc/              # Adaptadores IPC tipados por domínio (bindings.ts, diagnostics, @tauri-apps/api, plugin-updater, plugin-opener)
│   │   ├── Tooltip, SkillOrderPopover, LaneMatchupsPopover, LobbySlotsBar, RuneTree, SpellsBar, SkillOrderCard, LaneMatchupsCard, ItemBlocks, Docks
│   ├── hooks/                     # Hooks transversais do shell (useAppBootstrap, useAppUpdater, useWindowMinimized)
│   ├── features/                  # Domínios isolados (cada qual com componentes, hooks, helpers e CSS locais)
│   │   ├── champ-select/          # Sessão LCU, pré-seleção de lobby (Swiftplay), IdleState e auto-injeção
│   │   ├── profile/               # Perfil 2.0 (elos, catálogo de maestrias, histórico paginado e placar 5v5/Arena sob demanda)
│   │   ├── ranked/                # Tier list Solo/Duo e explorador de builds
│   │   ├── arena/                 # Arena 3x6 (tier list, augments, sinergias e itens prismáticos)
│   │   ├── aram/                  # ARAM Clássico (explorador de builds) e ARAM Desordem (Mayhem Augments & Builds)
│   │   └── settings/              # Configurações (Geral, Automação & Cliente, Aparência & Desempenho, Sistema & Armazenamento + Suporte e Diagnóstico)
│   ├── i18n/                      # Contexto de tradução e dicionários fatiados (pt/en core, build, settings & tabs)
│   ├── styles/                    # Tokens, layout shell e estilos compartilhados
│   ├── types/
│   │   ├── bindings.ts            # Gerado automaticamente pelo tauri-specta (uso restrito a src/services/ipc/)
│   │   └── index.ts               # Tipos locais de UI e reexports de tipos puros
│   ├── AppTabsViewport.tsx        # Viewport de abas conectando os domínios do shell
│   └── App.tsx                    # Orquestrador raiz e overlays de inicialização/onboarding
└── src-tauri/                       # Cargo Workspace raiz (3 crates)
    ├── capabilities/default.json  # Permissões Tauri v2 aprovadas (inclui opener:default e updater:default)
    ├── crates/
    │   ├── schumaz-models/        # Contratos serde/specta::Type, DTOs de settings/cache e export_bindings sem Tauri/WebView2
    │   └── schumaz-domain/        # Domínio puro (build_provider, draft_session, lcu_parsers, lcu_injector, logging, profile_provider, storage, updater::policy/probe) sem Tauri/WebView2/irelia/sysinfo
    └── src/                       # Casca fina Tauri v2 (padrão Rust 2018+: modulo.rs + modulo/, sem mod.rs)
        ├── lib.rs                 # Setup Tauri (single-instance, autostart, opener, updater), RunEvent::ExitRequested/Exit, re-exports de domínio e paridade tauri-specta
        ├── app_bootstrap.rs + app_bootstrap/ # Inicialização, resolve_app_paths, protocolo schumaz-asset://, tray.rs, window_factory.rs, prewarm.rs e window_events.rs
        ├── commands.rs + commands/        # Handlers IPC finos (#[tauri::command] + #[specta::specta], incluindo diagnostics.rs)
        ├── models.rs + models/            # Re-export de schumaz-models + wrappers Events IPC (tauri_specta::Event)
        ├── updater.rs                     # Adaptador Tauri/winreg para detecção Portable (feature + HKCU/path) e contexto do probe HTTP
        ├── lcu_connector.rs + lcu_connector/ # Cliente REST LCU (irelia/sysinfo), endpoints de rede, motor de missões e monitor
        ├── profile_provider.rs            # Adaptador ProfileLcuClient para LcuConnection e chamadas live de perfil/placar
        ├── ws_listener.rs + ws_listener/  # WebSocket (tungstenite), lobby runtime (lobby*.rs), fallback polling, gameflow runtime e reconciliação
        └── lcu_injector.rs + lcu_injector/ # Adaptadores SafetyInjectionSink/LobbyInjectionSink/ItemSetsStore (LcuConnection) e serviço assíncrono de injeção
```

## 4. Onde Cada Integração Reside
| Integração / Domínio | Arquivos Principais | Referência Curta |
| :--- | :--- | :--- |
| **LCU REST & Auth** | `src-tauri/src/lcu_connector.rs` (`lcu_connector/`) + `crates/schumaz-domain/src/lcu_parsers/` | `docs/lcu-api.md` |
| **LCU WebSocket / Draft** | `src-tauri/src/ws_listener/` (`actor.rs`, `subscription.rs`, `polling.rs`, `reconcile.rs`) + `crates/schumaz-domain/src/draft_session/` | `docs/lcu-api.md` |
| **LCU Injeção (Runas/Spells/Itens)** | `src-tauri/src/lcu_injector/` (`live_injection.rs`, `live_lobby.rs`) + `crates/schumaz-domain/src/lcu_injector/` (`payloads.rs`, `injection.rs`, `spell_rules.rs`) | `docs/lcu-api.md` |
| **OP.GG Analytics (REST & MCP)** | `crates/schumaz-domain/src/build_provider/analytics/` (`client.rs`, `cooldown.rs`, `versions.rs`, `positions.rs`, `matchups.rs`, `rest_extractors.rs`, `rest_parser.rs`, `mcp_{parser,extractors,stats}.rs`, `ranked.rs`, `arena/`, `aram.rs`, `aram/{augments,mcp_augments,parsers,cache,html_extract}.rs`) | `docs/opgg-api.md` |
| **DataDragon Cache & Assets** | `crates/schumaz-domain/src/build_provider/ddragon.rs` (`ddragon/`), `crates/schumaz-domain/src/storage/assets.rs`, `src/utils/assets.ts` | — |
| **Lógica de Itens (5 blocos / builds)** | `crates/schumaz-domain/src/build_provider/items/` (`blocks.rs`, `options.rs`, `classification.rs`, `affinity.rs`) | — |
| **System Tray, Autostart & WebView Sob Demanda** | `src-tauri/src/app_bootstrap/` (`tray.rs`, `window_factory.rs`, `prewarm.rs`, `window_events.rs`), `src-tauri/src/lib.rs`, `src-tauri/src/commands/system.rs` | `docs/tauri-v2-capabilities.md` |
| **Logs Rotativos & Diagnóstico Local** | `crates/schumaz-domain/src/logging.rs` (`logging/{writer,sanitizer,metrics,diagnostics}.rs`), `commands/diagnostics.rs`, `crates/schumaz-models/src/diagnostics.rs`, `src/services/ipc/diagnostics.ipc.ts`, `src/features/settings/{useDiagnostics.ts,SettingsDiagnosticsSubsection.tsx}` | `docs/tauri-v2-capabilities.md` |
| **Persistência & Preferências** | `src/features/settings/` + `crates/schumaz-domain/src/storage/settings/` + `crates/schumaz-domain/src/lcu_injector/` | — |
| **Auto-Update & Portable (NSIS)** | `src-tauri/src/updater.rs`, `crates/schumaz-domain/src/updater/`, `crates/schumaz-models/src/updater.rs`, `src/services/ipc/updater.ipc.ts`, `src/hooks/useAppUpdater.ts`, `src/components/ui/StatusDock.tsx` | `docs/tauri-v2-capabilities.md` |
| **Contrato IPC (Backend -> Frontend)** | `crates/schumaz-models/src/` + `src-tauri/src/{commands,models}/` → `src/types/bindings.ts` → `src/services/ipc/` | — |
| **Capabilities, CSP & Janela Tauri v2** | `src-tauri/capabilities/default.json`, `src-tauri/tauri.conf.json` | `docs/tauri-v2-capabilities.md` |
| **Diretrizes Visuais Desktop** | `src/styles/`, `src/features/*/*.css`, `src/components/ui/` | `docs/ui-guidelines.md` |

## 5. Otimização de Build Dev & Higiene do `src-tauri/target`
Para manter `npm run tauri dev`, `cargo clippy` e `cargo test` incrementais, evitando recompilações completas e desgaste prematuro do SSD:
- **Linker LLVM (`src-tauri/.cargo/config.toml`):** usa `linker = "rust-lld.exe"` em `x86_64-pc-windows-msvc`, acelerando a linkagem de `.exe`/`.pdb` no Windows sem alterar o ABI MSVC.
- **Cargo Workspace desacoplado (`src-tauri/Cargo.toml` + `crates/`):** `schumaz-models` gera `src/types/bindings.ts` e compila sem Tauri/WebView2, e `schumaz-domain` executa os testes unitários puros (~1–2s) sem linkar WebView2/Windows GUI. A casca `schumazgg` concentra `commands/`, `app_bootstrap/`, `ws_listener/` e `lcu_connector/`.
- **Alvo Desktop puro e sem harnesses vazios:** `[lib]` usa apenas `crate-type = ["rlib"]` e `doctest = false`, enquanto os binários `[[bin]]` (`schumazgg` na raiz e `export_bindings` em `crates/schumaz-models`) usam `test = false` para não compilar/linkar executáveis de teste sem testes.
- **Perfil `[profile.dev]` enxuto e flags unificadas:** mantém `incremental = true` com `debug = "line-tables-only"` no workspace raiz (`src-tauri/Cargo.toml`) e `debug = false` em `package."*"` e `build-override` (sem alterar `[profile.release]` nem duplicar perfis nos subcrates). A severidade de warnings usa `CARGO_BUILD_WARNINGS="deny"` (em vez de `-- -D warnings` ou alternância de `RUSTFLAGS`), permitindo que `tauri dev`, `cargo check`, `cargo clippy` e `cargo test` compartilhem o mesmo cache.
- **Poda cirúrgica de sessões órfãs em `incremental/` (`scripts/sweep-cargo-target.mjs` e `scripts/run-tauri.mjs`):**
  - Ao rodar `npm run tauri dev` ou `npm run tauri:sweep:auto`, a manutenção inspeciona apenas os diretórios `target/*/incremental/`, mantendo as **3 sessões mais recentes** por crate (`dev`, `test`, `clippy`) e removendo apenas hashes órfãos antigos — **sem jamais apagar `deps/` ativos** (substituindo o antigo `cargo-sweep --maxsize`, que causava *cache thrashing*). Pode ser ignorado via `SCHUMAZ_SKIP_CARGO_SWEEP=1`.
  - Scripts disponíveis: `npm run tauri:prune` (poda apenas `incremental/` órfãos), `npm run tauri:sweep` (poda + `cargo sweep --toolchains --time 30`) e `npm run tauri:sweep:auto` (poda silenciosa no gatekeeper final).
- **Isolamento do `rust-analyzer` (`.vscode/settings.json`):** define `"rust-analyzer.cargo.targetDir": true` para isolar o check do IDE em `target/rust-analyzer` e evitar disputa de *file lock* e invalidação cruzada no Windows.
