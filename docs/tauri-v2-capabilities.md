# Referência Rápida: Tauri v2 & Capabilities

## Capabilities Aprovadas (`src-tauri/capabilities/default.json`)
```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Capability for the main window",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "core:window:allow-start-dragging",
    "core:window:allow-minimize",
    "core:window:allow-toggle-maximize",
    "core:window:allow-close",
    "opener:default",
    "updater:default"
  ]
}
```

## Regras para Novas Permissões, CSP & Plugins
- Toda permissão nova do frontend para plugins Tauri v2 deve ser declarada explicitamente em `src-tauri/capabilities/default.json` e isolada no frontend via `src/services/ipc/`.
- **Single-Instance (`tauri-plugin-single-instance`):** Registrado como primeiro plugin desktop em `src-tauri/src/lib.rs`, restaurando (se minimizada), exibindo e focando a janela `"main"` via `app_bootstrap::focus_main_window`. Não requer permissão extra em `capabilities/default.json`.
- **Content Security Policy (`app.security.csp` em `src-tauri/tauri.conf.json`):** Política explícita permitindo `'self'`, estilos inline necessários ao shell, IPC do Tauri (`ipc:`, `http://ipc.localhost`) e imagens servidas via protocolo local `schumaz-asset:` (`http://schumaz-asset.localhost`) além de `data:`/`blob:`.
- **Updater (`updater:default`):**
  - `bundle.createUpdaterArtifacts: true`, `bundle.windows.nsis.installMode: "currentUser"` e `plugins.updater.windows.installMode: "passive"` em `src-tauri/tauri.conf.json`.
  - A chave pública (`plugins.updater.pubkey`) fica no `tauri.conf.json`; a chave privada (`TAURI_SIGNING_PRIVATE_KEY` e opcional `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`) **nunca** deve ser versionada no repositório.
  - Builds portáteis usam `--features portable` (ou fallback em runtime sem registro `Uninstall` do NSIS em `HKCU`) para desabilitar `download()`/`install()` e notificar via `opener:default`.
- **Cache/FS:** Manter I/O de disco no backend Rust (`src-tauri/src/storage.rs` + `storage/`) sem expor `fs` ao webview; assets locais são servidos via protocolo `schumaz-asset://`.
- **Janela (`src-tauri/tauri.conf.json`):** `label` implícito `"main"`, `980x720`, `minWidth: 840`, `minHeight: 600`, `center: true`, `resizable: true`, `decorations: false`, `create: false` (exibida após aplicar bounds/preferências no `app_bootstrap.rs`).
- **Contratos IPC:** Comandos e eventos são registrados via `tauri_specta::Builder` em `src-tauri/src/lib.rs` e exportados para `src/types/bindings.ts`.
