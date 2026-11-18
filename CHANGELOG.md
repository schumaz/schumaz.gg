# Changelog

All notable changes to **schumaz.gg** will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
