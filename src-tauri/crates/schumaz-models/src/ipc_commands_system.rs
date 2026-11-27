#![allow(clippy::too_many_arguments, unused_variables)]

use crate::ipc_export::CmdResult;
use crate::{
    AppDistributionInfo, AppLocale, AppSettings, AutostartStatus, BootstrapStatus,
    CacheClearScope, DiagnosticSummary, FlashKeyPreference, HardwareAccelerationStatus,
    LocalCacheInfo, NavigationStatePatch, UpdateProbeResult,
};

/// Returns the current backend startup/prewarm bootstrap status.
#[specta::specta]
pub fn get_bootstrap_status() -> BootstrapStatus {
    unreachable!()
}

/// Returns the runtime distribution mode (`isPortable`), current app version, and GitHub Releases URL.
#[specta::specta]
pub async fn get_app_distribution_info() -> AppDistributionInfo {
    unreachable!()
}

/// Evaluates the 24h cooldown and runs a lightweight `HEAD /releases/latest` probe
/// so `latest.json` is only downloaded when a newer version actually exists.
#[specta::specta]
pub async fn probe_for_app_update(manual: bool) -> UpdateProbeResult {
    unreachable!()
}

/// Persists the current timestamp in `UserSettings.ini` after a full updater check completes.
#[specta::specta]
pub async fn record_update_check_completed() -> Option<u64> {
    unreachable!()
}

/// Returns the saved GPU hardware acceleration setting and whether it was active at startup.
#[specta::specta]
pub fn get_hardware_acceleration_status() -> HardwareAccelerationStatus {
    unreachable!()
}

/// Saves the GPU hardware acceleration preference to `UserSettings.ini` for the next startup.
#[specta::specta]
pub async fn set_hardware_acceleration(enabled: bool) -> CmdResult<HardwareAccelerationStatus> {
    unreachable!()
}

/// Updates `[General]` (`bCloseToTray`) in `UserSettings.ini` without touching `[WindowSettings]`.
#[specta::specta]
pub async fn set_close_to_tray(enabled: bool) -> CmdResult<AppSettings> {
    unreachable!()
}

/// Returns the current OS startup launch state. Portable builds report `supported: false`.
#[specta::specta]
pub async fn get_autostart_status() -> AutostartStatus {
    unreachable!()
}

/// Enables or disables OS startup launch (`--autostart`) for installed builds.
/// Rejected on portable builds so volatile paths (e.g. `Downloads`) are never registered.
#[specta::specta]
pub async fn set_autostart_enabled(enabled: bool) -> CmdResult<AutostartStatus> {
    unreachable!()
}

/// Returns the full unified `AppSettings` loaded from `%LOCALAPPDATA%\schumazgg\Config\UserSettings.ini`.
#[specta::specta]
pub fn get_app_settings() -> AppSettings {
    unreachable!()
}

/// Updates `[ProfileAppearanceSettings]` (`ProfileBlurPercent`, `bWallpaperVisible`) in `UserSettings.ini`.
#[specta::specta]
pub async fn update_profile_appearance(
    blur_percent: u32,
    wallpaper_visible: bool,
) -> CmdResult<AppSettings> {
    unreachable!()
}

/// Applies a partial patch to `[NavigationState]` in `UserSettings.ini`.
#[specta::specta]
pub async fn update_navigation_state(patch: NavigationStatePatch) -> CmdResult<AppSettings> {
    unreachable!()
}

/// Marks the first-run language onboarding completed with `locale` and persists to `UserSettings.ini`.
#[specta::specta]
pub async fn complete_language_onboarding(locale: AppLocale) -> CmdResult<AppSettings> {
    unreachable!()
}

/// Marks the first-run Flash key onboarding completed with `flash_key` (`D` | `F`) and persists to `UserSettings.ini`.
#[specta::specta]
pub async fn complete_flash_onboarding(flash_key: FlashKeyPreference) -> CmdResult<AppSettings> {
    unreachable!()
}

/// Returns disk usage statistics for the local cache directory (`Cache/`).
#[specta::specta]
pub async fn get_local_cache_info() -> CmdResult<LocalCacheInfo> {
    unreachable!()
}

/// Deletes cached files under `Cache/` matching `scope` and returns refreshed usage statistics.
#[specta::specta]
pub async fn clear_local_cache(scope: CacheClearScope) -> CmdResult<LocalCacheInfo> {
    unreachable!()
}

/// Builds a sanitized local diagnostic summary without exposing credentials, paths, or PII.
#[specta::specta]
pub async fn get_diagnostic_summary() -> CmdResult<DiagnosticSummary> {
    unreachable!()
}

/// Ensures `%LOCALAPPDATA%\schumazgg\Logs\` exists, flushes buffered logs, and opens it in Explorer
/// directly from the Rust backend without expanding frontend Tauri capabilities.
#[specta::specta]
pub async fn open_logs_folder() -> CmdResult<()> {
    unreachable!()
}

/// Restarts the Tauri application so updated WebView2 GPU flags take effect.
#[specta::specta]
pub async fn restart_app() {
    unreachable!()
}
