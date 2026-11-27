# AGENTS.md — schumaz.gg

Windows 10/11 desktop companion app for League of Legends built with **Tauri v2**, **Rust (edition 2024)** in `src-tauri/` (`tauri-specta`, `irelia`, `tokio`, `reqwest`, `lz4_flex`, `rmp-serde`), and **React 19 + TypeScript 6 (`strict: true`) + Vite 8 + Vitest 5** in `src/`.

## 1. Context Map (Read On Demand)
- **Architecture:** Read [ARCHITECTURE.md](ARCHITECTURE.md) before proposing structural changes, new modules, or cross-layer refactors.
- **Generated IPC Contracts:** `src/types/bindings.ts` is auto-generated from `crates/schumaz-models` (`specta` + `specta-typescript` without linking Tauri/WebView2). Never edit it manually; consume it exclusively from `src/services/ipc/`. Regenerate with `cargo run --bin export_bindings` inside `src-tauri/`.
- **Short References:** `docs/lcu-api.md`, `docs/opgg-api.md`, `docs/tauri-v2-capabilities.md`, `docs/ui-guidelines.md`.
- **Changelog:** Update [CHANGELOG.md](CHANGELOG.md) for notable user-facing features, bugfixes, and releases.
- **Local Backlog:** `todo.md` is gitignored; if present in the local workspace, consult and update it when starting or completing roadmap tasks.

## 2. Non-Negotiable Domain Invariants
- **Anti-Vanguard:** NEVER interact with `League of Legends.exe` or read/write game process memory. Use only the local HTTPS REST and WebSocket APIs of `LeagueClientUx.exe` (LCU).
- **100% Automation:** FORBIDDEN to reintroduce manual injection buttons or commands (e.g., "Apply runes").
- **No Hardcoded Champion Data:** Resolve lanes/roles, runes, summoner spells, and item builds dynamically via OP.GG + DataDragon/CommunityDragon.
- **Client & UI Branding:** Never mention external data sources (e.g., OP.GG) inside the LoL client or in the app UI. Rune pages and item sets must always use `"%Champname% | schumaz.gg"`.

## 3. Code Conventions, File Limits & Layer Boundaries
- **Language & Commits:** Write code comments, docstrings, Conventional Commits (`feat(scope): ...`, `fix(scope): ...`, `perf(scope): ...`), and this file in **English**; keep internal docs (`ARCHITECTURE.md`, `docs/`, `todo.md`) in **Portuguese (BR)**; keep public docs (`README*`, `CHANGELOG.md`) bilingual (EN + PT-BR).
- **Rust Resilience:** Never use `.unwrap()` or `.expect()` in network, LCU, or WebSocket flows. Return `Result` and use retries across client screen transitions.
- **Strict File Line Limits (enforced by `npm run check:lines`):**
  - **Rust (`src-tauri/src/**/*.rs`, `src-tauri/crates/**/*.rs`):** max **350 lines** per file (extract `*_tests.rs` or cohesive submodules whenever exceeded).
  - **Frontend (`src/**/*.ts`, `src/**/*.tsx`):** max **250 lines** per file (sole exception: `src/types/bindings.ts`).
- **Rust 2018+ Module Layout (`src-tauri/` workspace):** Use `module.rs` + `module/` directory across all workspace crates (`src-tauri/src/` and `src-tauri/crates/*/src/`). Never create `mod.rs` files.
- **Dev Build & SSD Protection (`src-tauri/`):**
  - Keep `src-tauri/.cargo/config.toml` configured with `linker = "rust-lld.exe"` under `[target.x86_64-pc-windows-msvc]` for fast Windows linking.
  - Keep `[lib] crate-type = ["rlib"]` and `doctest = false`, plus `test = false` on `[[bin]]` targets (`schumazgg`, `export_bindings`), avoiding wasted test-harness compiles/links (Windows desktop only; never reintroduce `staticlib` or `cdylib`).
  - Preserve lean `[profile.dev]` (`incremental = true`, `debug = "line-tables-only"`, `debug = false` on `package."*"` and `build-override`) strictly in workspace root `src-tauri/Cargo.toml` without altering `[profile.release]` or duplicating `[profile.*]` in member crates.
  - Use `CARGO_BUILD_WARNINGS="deny"` instead of `-- -D warnings` or toggling `RUSTFLAGS`, keeping `tauri dev`, `cargo check`, `cargo clippy`, and `cargo test` on a unified incremental cache.
  - Maintain surgical incremental-orphan pruning via `scripts/sweep-cargo-target.mjs` (keeps the 3 newest session directories per crate in `incremental/` and NEVER deletes active `deps/`; never reintroduce `cargo-sweep --maxsize`).
  - Keep IDE checks isolated in `.vscode/settings.json` (`"rust-analyzer.cargo.targetDir": true`) to avoid file-lock contention and cache invalidation.
- **Rust Workspace & Layers (`src-tauri/`):**
  - **`crates/schumaz-models` (`schumaz_models` + `export_bindings` bin):** pure `serde` + `specta::Type` domain/IPC contracts, settings/cache DTOs, and zero-Tauri TypeScript bindings exporter (`ipc_commands*`, `ipc_events`, `ipc_export`).
  - **`crates/schumaz-domain` (`schumaz_domain`):** pure domain services, caches, parsers, LCU injection logic, and updater policies (`build_provider`, `draft_session`, `lcu_injector`, `lcu_parsers`, `logging`, `profile_provider`, `storage`, `updater`) — zero `tauri`, `webview2-com`, `winreg`, `irelia`, `tungstenite`, or `sysinfo` dependencies.
  - **`src-tauri` (`schumazgg` / `schumazgg_lib` shell):** thin Tauri v2 desktop wrapper keeping **IPC Interface (`commands/`)**, **Lifecycle (`app_bootstrap/`)**, `ws_listener/` runtime actors/handlers, `lcu_connector/` (`irelia`/`sysinfo`), trait adapters for `LcuConnection`/`AppHandle` (`lcu_injector`, `profile_provider`, `updater`), `models/events.rs` (`tauri_specta::Event`), and `tests/integration/`.
- **Frontend Layers (`src/`):**
  - **IPC Layer (`src/services/ipc/`):** isolate all Tauri communication (`invoke`, `listen`, `convertFileSrc`, `@tauri-apps/api`, `@tauri-apps/plugin-*` including `plugin-updater` and `plugin-opener`, and `commands`/`events` from `bindings.ts`) in domain-typed adapters. No `.tsx` component, feature hook, or utility may import `@tauri-apps/*` or `bindings.ts` directly.
  - **Domain Features (`src/features/<domain>/`):** co-locate components, hooks, helpers, styles, and tests per domain (`champ-select`, `profile`, `ranked`, `arena`, `aram`, `settings`).
  - **Pure UI (`src/components/ui/`):** reusable visual-only components with zero IPC or domain-logic dependencies.
- **Desktop UI (`docs/ui-guidelines.md`):** Read before editing layout or CSS. Visual refactors must not alter hooks, state, or IPC behavior.

## 4. Commands & Validation (Compressed Output)
### Development & Build (repo root)
- Dev mode (prunes only orphan `incremental/` sessions beyond the 3 newest per crate; keeps `deps/` intact): `npm run tauri dev`
- Frontend typecheck + production bundle: `npm run build`
- Local unsigned Windows builds (NSIS + Portable into `dist-release/`): `npm run build:desktop -- --no-sign`
- Prune orphan incremental sessions (safe, preserves active `deps/`): `npm run tauri:prune`
- Full time/toolchain maintenance (prunes orphan incremental sessions + `cargo sweep --toolchains --time 30`): `npm run tauri:sweep`

### Scoped Incremental Validation Policy
- **Frontend / CSS / Docs-only changes (no `src-tauri/` edits):** FORBIDDEN to run Cargo commands. Run only `npx tsc --noEmit`, `npx vitest run <affected-files> --reporter=dot` (full `vitest` suite only when shared hooks, styles, or cross-feature code changed), and `npm run check:lines`.
- **Minimize Rust recompilation & SSD writes:** Validate only the affected delta; never invalidate `deps/` or trigger full cold rebuilds across features unless those exact files changed.

### Fast Iteration Loop
- Regenerate IPC bindings (`src-tauri/`): `cargo run --bin export_bindings`
- Rust domain tests without linking WebView2 (`src-tauri/`): `cargo test -p schumaz-domain --lib -q`
- Rust check (`src-tauri/`): `cargo check --lib --message-format=short` *(avoid running `--features portable` unless modifying `src-tauri/src/updater.rs` or `src-tauri/Cargo.toml`)*
- Scoped Rust unit test (`src-tauri/`): `cargo test --lib -q <test_name>`
- Frontend typecheck (root): `npx tsc --noEmit`
- Single Frontend test (root): `npx vitest run <file.test.tsx> --reporter=dot`

### Final Gatekeeper (run only checks matching changed scope; mirrors `.github/workflows/ci.yml`)
**If `src-tauri/` changed:**
1. `src-tauri/`: `$env:CARGO_BUILD_WARNINGS="deny"; cargo clippy --all-targets --message-format=short` *(run `cargo clippy -p schumazgg --all-targets --features portable --message-format=short` locally ONLY if `src-tauri/src/updater.rs`, `src-tauri/crates/schumaz-domain/src/updater/`, or `src-tauri/Cargo.toml` changed; CI always verifies both)*
2. `src-tauri/`: `cargo test --lib -q` *(plus `cargo test --test integration -q` ONLY if network, LCU, cache, updater, or integration boundaries changed)*
3. `src-tauri/` *(ONLY if `commands/` or `models` changed)*: `cargo run --bin export_bindings` and verify `src/types/bindings.ts` is in sync

**If `src/` or `scripts/` changed (or `bindings.ts` regenerated):**
4. Root: `npx tsc --noEmit`
5. Root: `npx vitest run --reporter=dot` *(or `<affected-files>` when changes are isolated to a single feature/script; full suite when shared modules changed)*

**Always:**
6. Root: `npm run check:lines`
7. Root *(if Cargo commands ran)*: `npm run tauri:sweep:auto` *(prunes only orphaned `incremental/` sessions beyond the 3 newest per crate; NEVER deletes active `deps/`)*

## 5. Testing Rules
- **Unit Tests:** Fast, deterministic, zero real I/O.
- **Integration Tests:** Exclusively under `src-tauri/tests/integration/` using `wiremock`.
- **No Duplication:** At most 1 test per logical branch (Solo/Duo `420`, ARAM `450`, Practice Tool `-1`, error / incomplete payload).

## 6. Release & Version Bump Checklist
Before creating and pushing tag `v<version>`:
1. **Bump version in the 3 manifests & sync lockfiles via CLI (never hand-edit lockfiles):**
   - Update `version` in `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, and `package.json`.
   - Sync lockfiles: `npm install --package-lock-only` (root) and `cargo check` (`src-tauri/`).
   - Clean orphaned incremental/toolchain artifacts from the previous version hash without wiping active `deps/` (root): `npm run tauri:sweep -- --allow-missing` *(prunes orphan `incremental/` sessions + uninstalled toolchains + `>30d` stale artifacts)*.
2. **Update `CHANGELOG.md` (single source for release notes):**
   - Add `## [<version>] - YYYY-MM-DD` following Keep a Changelog with bilingual `### English` and `### Português (BR)` subsections.
   - `scripts/generate-release-notes.mjs` and `.github/workflows/release.yml` extract GitHub Release notes and auto-updater `latest.json` notes directly from `CHANGELOG.md`.
3. **Tag & Publish:**
   - Push `git tag v<version> && git push origin v<version>` matching `src-tauri/tauri.conf.json`.
   - GitHub Actions signs artifacts with `secrets.TAURI_SIGNING_PRIVATE_KEY`, generates `latest.json` + `SHA256SUMS.txt`, and publishes the **Latest** release via `gh`.
