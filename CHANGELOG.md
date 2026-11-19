# Changelog

All notable changes to **schumaz.gg** will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
