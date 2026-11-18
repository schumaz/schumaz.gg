# Changelog

All notable changes to **schumaz.gg** will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
