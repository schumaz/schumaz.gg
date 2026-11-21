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

## 2. Regras de Modularização e Limites
- **Teto de linhas por arquivo:**
  - **Rust (`src-tauri/src/`):** máximo de **350 linhas** por arquivo (extraindo `*_tests.rs` ou submódulos coesos).
  - **Frontend (`src/`):** máximo de **250 linhas** por arquivo `.ts`/`.tsx` (exceto `src/types/bindings.ts`, gerado automaticamente).
- **Padrão de módulos Rust 2018+:** usar `modulo.rs` + diretório `modulo/` em vez de `mod.rs` genéricos.
- **Isolamento IPC no Frontend:** toda chamada a `invoke`, `listen`, `convertFileSrc` e consumo de `src/types/bindings.ts`, `@tauri-apps/api` ou `@tauri-apps/plugin-*` fica encapsulada em `src/services/ipc/`. Componentes `.tsx` nunca importam pacotes `@tauri-apps/*` diretamente.
- **Separação UI vs. Domínio:** `src/features/<dominio>/` concentra componentes, hooks, tipos e estilos do domínio; `src/components/ui/` contém apenas componentes visuais puros.

## 3. Estrutura Principal

```text
├── docs/                          # Referências curtas (LCU, OP.GG, Tauri v2, UI)
├── scripts/                       # Automação de release, gatekeeper de linhas e higiene do target Cargo (run-tauri, sweep-cargo-target)
├── src/
│   ├── services/ipc/              # Adaptadores IPC tipados por domínio (bindings.ts, @tauri-apps/api, plugin-updater, plugin-opener)
│   ├── components/ui/             # Componentes visuais puros (FiltersHeader, FilterDropdown, SearchInput, RegionFlag, RuneTree, SpellsBar, ItemBlocks, Docks)
│   ├── hooks/                     # Hooks transversais do shell (useAppBootstrap, useAppUpdater, useWindowMinimized)
│   ├── features/                  # Domínios isolados (cada qual com componentes, hooks, helpers e CSS locais)
│   │   ├── champ-select/          # Sessão LCU, IdleState e auto-injeção
│   │   ├── profile/               # Perfil do invocador, elos, maestrias e histórico
│   │   ├── ranked/                # Tier list Solo/Duo e explorador de builds
│   │   ├── arena/                 # Arena 3x6 (tier list, augments, sinergias e itens prismáticos)
│   │   ├── aram/                  # ARAM Clássico e ARAM Desordem (Mayhem Augments)
│   │   └── settings/              # Configurações (Geral, Automação & Cliente, Aparência & Desempenho, Sistema & Armazenamento), autostart, bandeja e INI
│   ├── i18n/                      # Contexto de tradução e dicionários fatiados (pt/en core, build, settings & tabs)
│   ├── styles/                    # Tokens, layout shell e estilos compartilhados
│   ├── types/
│   │   ├── bindings.ts            # Gerado automaticamente pelo tauri-specta (uso restrito a src/services/ipc/)
│   │   └── index.ts               # Tipos locais de UI e reexports de tipos puros
│   ├── AppTabsViewport.tsx        # Viewport de abas conectando os domínios do shell
│   └── App.tsx                    # Orquestrador raiz e overlays de inicialização/onboarding
└── src-tauri/
    ├── capabilities/default.json  # Permissões Tauri v2 aprovadas (inclui opener:default e updater:default)
    └── src/                       # Padrão Rust 2018+ (modulo.rs + modulo/, sem mod.rs)
        ├── lib.rs                 # Setup Tauri (single-instance, autostart, opener, updater), RunEvent::ExitRequested e registro tauri-specta
        ├── app_bootstrap.rs + app_bootstrap/ # Inicialização, protocolo schumaz-asset://, tray.rs, window_factory.rs, prewarm.rs e window_events.rs
        ├── commands.rs + commands/        # Handlers IPC finos (#[tauri::command] + #[specta::specta])
        ├── models.rs + models/            # Structs serde/specta e Events IPC
        ├── updater.rs                     # Detecção de modo Portable (feature + Registro HKCU/path) e info de distribuição
        ├── storage.rs + storage/          # Infraestrutura: paths, cache .lz4, GC, assets, graphics, window_bounds, settings/ (INI)
        ├── lcu_connector.rs + lcu_connector/ # Cliente REST LCU, endpoints por domínio, motor de missões e monitor
        ├── profile_provider.rs + profile_provider/ # Cache de perfil/skins e parsers de ranked, maestrias e histórico
        ├── ws_listener.rs + ws_listener/  # WebSocket (subscription.rs) + fallback polling, extração de draft, coordinator e reconciliação
        ├── lcu_injector.rs + lcu_injector/ # Regras de spells/Flash, seleções, payloads e serviço de injeção REST
        └── build_provider.rs + build_provider/ # Serviço/orquestrador de builds, ddragon/, items/, analytics/ (OP.GG), runes, spells, patch
```

## 4. Onde Cada Integração Reside
| Integração / Domínio | Arquivos Principais | Referência Curta |
| :--- | :--- | :--- |
| **LCU REST & Auth** | `src-tauri/src/lcu_connector.rs` (`lcu_connector/`) | `docs/lcu-api.md` |
| **LCU WebSocket / Draft** | `src-tauri/src/ws_listener/` (`extract.rs`, `subscription.rs`, `polling.rs`, `reconcile.rs`) | `docs/lcu-api.md` |
| **LCU Injeção (Runas/Spells/Itens)** | `src-tauri/src/lcu_injector/` (`payloads.rs`, `injection.rs`, `spell_rules.rs`) | `docs/lcu-api.md` |
| **OP.GG Analytics (REST & MCP)** | `src-tauri/src/build_provider/analytics/` (`client.rs`, `positions.rs`, `rest_parser.rs`, `mcp_parser.rs`, `ranked.rs`, `arena/`, `aram.rs`) | `docs/opgg-api.md` |
| **DataDragon Cache & Assets** | `src-tauri/src/build_provider/ddragon.rs` (`ddragon/`) | — |
| **Lógica de Itens (5 blocos / builds)** | `src-tauri/src/build_provider/items/` (`blocks.rs`, `options.rs`, `classification.rs`, `affinity.rs`) | — |
| **System Tray, Autostart & WebView Sob Demanda** | `src-tauri/src/app_bootstrap/` (`tray.rs`, `window_factory.rs`, `prewarm.rs`, `window_events.rs`), `src-tauri/src/lib.rs`, `src-tauri/src/commands/system.rs` | `docs/tauri-v2-capabilities.md` |
| **Persistência & Preferências** | `src/features/settings/` + `src-tauri/src/storage/settings/` + `lcu_injector/` | — |
| **Auto-Update & Portable (NSIS)** | `src-tauri/src/updater.rs`, `models/updater.rs`, `src/services/ipc/updater.ipc.ts`, `src/hooks/useAppUpdater.ts`, `src/components/ui/StatusDock.tsx` | `docs/tauri-v2-capabilities.md` |
| **Contrato IPC (Backend -> Frontend)** | `src-tauri/src/{commands,models}/` → `src/types/bindings.ts` → `src/services/ipc/` | — |
| **Capabilities, CSP & Janela Tauri v2** | `src-tauri/capabilities/default.json`, `src-tauri/tauri.conf.json` | `docs/tauri-v2-capabilities.md` |
| **Diretrizes Visuais Desktop** | `src/styles/`, `src/features/*/*.css`, `src/components/ui/` | `docs/ui-guidelines.md` |

## 5. Otimização de Build Dev & Higiene do `src-tauri/target`
Para manter `npm run tauri dev` ágil sem inflar o SSD com dezenas de GB de artefatos órfãos:
- **Alvo Desktop puro (`src-tauri/Cargo.toml`):** `[lib]` usa apenas `crate-type = ["rlib"]` (sem `staticlib`/`cdylib`, usados só em mobile).
- **Perfil `[profile.dev]` enxuto:** mantém `incremental = true` com `debug = "line-tables-only"` no crate principal e `debug = false` em `package."*"` e `build-override` (sem alterar `[profile.release]`).
- **Limpeza automática via `cargo-sweep` (`scripts/sweep-cargo-target.mjs` e `scripts/run-tauri.mjs`):**
  - Ao rodar `npm run tauri dev`, o wrapper verifica o tamanho de `src-tauri/target` antes de iniciar o Cargo; se passar de **6 GiB**, executa automaticamente `cargo sweep --maxsize 4GB src-tauri` (pula silenciosamente se abaixo de 6 GiB ou se `cargo-sweep` não estiver instalado; pode ser ignorado com `SCHUMAZ_SKIP_CARGO_SWEEP=1`).
  - Scripts disponíveis: `npm run tauri:sweep` (limpeza direta até 4 GB) e `npm run tauri:sweep:auto` (acionado no gatekeeper final e após bumps de versão conforme `.antigravityrules`).
