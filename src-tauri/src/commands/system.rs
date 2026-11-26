use crate::app_bootstrap::{self, BootstrapState};
use crate::lcu_injector;
use crate::logging;
use crate::models::{
    AppDistributionInfo, AppLocale, AutostartStatus, BootstrapStatus, FlashKeyPreference,
    UpdateProbeResult,
};
use crate::storage::{
    self, AppSettings, CacheClearScope, GraphicsState, HardwareAccelerationStatus, LocalCacheInfo,
    NavigationStatePatch,
};
use crate::updater;
use tauri::{AppHandle, Manager, State};

/// Returns the current backend startup/prewarm bootstrap status.
#[tauri::command]
#[specta::specta]
pub fn get_bootstrap_status(state: State<'_, BootstrapState>) -> BootstrapStatus {
    state.snapshot()
}

/// Returns the runtime distribution mode (`isPortable`), current app version, and GitHub Releases URL.
#[tauri::command]
#[specta::specta]
pub async fn get_app_distribution_info(app_handle: AppHandle) -> AppDistributionInfo {
    let fallback = app_handle.clone();
    tokio::task::spawn_blocking(move || updater::get_distribution_info(&app_handle))
        .await
        .unwrap_or_else(|_| AppDistributionInfo {
            is_portable: cfg!(feature = "portable"),
            current_version: fallback.package_info().version.to_string(),
            releases_url: updater::GITHUB_RELEASES_LATEST_URL.to_string(),
            last_check_epoch_secs: storage::get_last_update_check_epoch_secs(),
            check_cooldown_secs: updater::AUTO_UPDATE_CHECK_COOLDOWN_SECS,
        })
}

/// Evaluates the 24h cooldown and runs a lightweight `HEAD /releases/latest` probe
/// so `latest.json` is only downloaded when a newer version actually exists.
#[tauri::command]
#[specta::specta]
pub async fn probe_for_app_update(app_handle: AppHandle, manual: bool) -> UpdateProbeResult {
    updater::probe_for_app_update(&app_handle, manual).await
}

/// Persists the current timestamp in `UserSettings.ini` after a full updater check completes.
#[tauri::command]
#[specta::specta]
pub async fn record_update_check_completed() -> Option<u64> {
    tokio::task::spawn_blocking(updater::record_update_check_completed)
        .await
        .ok()
        .flatten()
}

/// Returns the saved GPU hardware acceleration setting and whether it was active at startup.
#[tauri::command]
#[specta::specta]
pub fn get_hardware_acceleration_status(
    state: State<'_, GraphicsState>,
) -> HardwareAccelerationStatus {
    state.status()
}

/// Saves the GPU hardware acceleration preference to `UserSettings.ini` for the next startup.
#[tauri::command]
#[specta::specta]
pub async fn set_hardware_acceleration(
    app_handle: AppHandle,
    enabled: bool,
) -> Result<HardwareAccelerationStatus, String> {
    tokio::task::spawn_blocking(move || {
        let state = app_handle.state::<GraphicsState>();
        storage::update_hardware_acceleration_setting(enabled, &state)
    })
    .await
    .map_err(|e| format!("Failed to update hardware acceleration: {e}"))?
}

/// Updates `[General]` (`bCloseToTray`) in `UserSettings.ini` without touching `[WindowSettings]`.
#[tauri::command]
#[specta::specta]
pub async fn set_close_to_tray(enabled: bool) -> Result<AppSettings, String> {
    tokio::task::spawn_blocking(move || storage::update_close_to_tray_setting(enabled))
        .await
        .map_err(|e| format!("Failed to update close-to-tray setting: {e}"))?
}

fn build_autostart_status(is_portable: bool, enabled: bool) -> AutostartStatus {
    if is_portable {
        AutostartStatus {
            enabled: false,
            supported: false,
            is_portable: true,
        }
    } else {
        AutostartStatus {
            enabled,
            supported: cfg!(desktop),
            is_portable: false,
        }
    }
}

/// Returns the current OS startup launch state. Portable builds report `supported: false`.
#[tauri::command]
#[specta::specta]
pub async fn get_autostart_status(app_handle: AppHandle) -> AutostartStatus {
    tokio::task::spawn_blocking(move || {
        let is_portable = updater::detect_is_portable(&app_handle);
        if is_portable {
            return build_autostart_status(true, false);
        }

        #[cfg(desktop)]
        {
            use tauri_plugin_autostart::ManagerExt;
            let enabled = app_handle.autolaunch().is_enabled().unwrap_or(false);
            build_autostart_status(false, enabled)
        }

        #[cfg(not(desktop))]
        {
            build_autostart_status(false, false)
        }
    })
    .await
    .unwrap_or_else(|_| build_autostart_status(cfg!(feature = "portable"), false))
}

/// Enables or disables OS startup launch (`--autostart`) for installed builds.
/// Rejected on portable builds so volatile paths (e.g. `Downloads`) are never registered.
#[tauri::command]
#[specta::specta]
pub async fn set_autostart_enabled(
    app_handle: AppHandle,
    enabled: bool,
) -> Result<AutostartStatus, String> {
    tokio::task::spawn_blocking(move || {
        if updater::detect_is_portable(&app_handle) {
            return Err("Autostart is not supported in Portable mode".to_string());
        }

        #[cfg(desktop)]
        {
            use tauri_plugin_autostart::ManagerExt;
            let manager = app_handle.autolaunch();
            let operation = if enabled {
                manager.enable()
            } else {
                manager.disable()
            };
            operation.map_err(|err| format!("Failed to update autostart setting: {err}"))?;
            let actual_enabled = manager.is_enabled().unwrap_or(enabled);
            Ok(build_autostart_status(false, actual_enabled))
        }

        #[cfg(not(desktop))]
        {
            let _ = enabled;
            Err("Autostart is not supported on this platform".to_string())
        }
    })
    .await
    .map_err(|e| format!("Failed to update autostart setting: {e}"))?
}

/// Returns the full unified `AppSettings` loaded from `%LOCALAPPDATA%\schumazgg\Config\UserSettings.ini`.
#[tauri::command]
#[specta::specta]
pub fn get_app_settings() -> AppSettings {
    storage::get_global_settings()
}

/// Updates `[ProfileAppearanceSettings]` (`ProfileBlurPercent`, `bWallpaperVisible`) in `UserSettings.ini`.
#[tauri::command]
#[specta::specta]
pub async fn update_profile_appearance(
    blur_percent: u32,
    wallpaper_visible: bool,
) -> Result<AppSettings, String> {
    tokio::task::spawn_blocking(move || {
        storage::update_profile_appearance_settings(blur_percent, wallpaper_visible)
    })
    .await
    .map_err(|e| format!("Failed to update profile appearance: {e}"))?
}

/// Applies a partial patch to `[NavigationState]` in `UserSettings.ini`.
#[tauri::command]
#[specta::specta]
pub async fn update_navigation_state(
    patch: NavigationStatePatch,
) -> Result<AppSettings, String> {
    tokio::task::spawn_blocking(move || storage::update_navigation_settings(patch))
        .await
        .map_err(|e| format!("Failed to update navigation state: {e}"))?
}

/// Marks the first-run language onboarding completed with `locale` and persists to `UserSettings.ini`.
#[tauri::command]
#[specta::specta]
pub async fn complete_language_onboarding(
    app_handle: AppHandle,
    locale: AppLocale,
) -> Result<AppSettings, String> {
    let settings =
        tokio::task::spawn_blocking(move || lcu_injector::complete_language_onboarding_service(locale))
            .await
            .map_err(|e| format!("Failed to complete language onboarding: {e}"))??;
    app_bootstrap::refresh_tray_locale(&app_handle, locale);
    Ok(settings)
}

/// Marks the first-run Flash key onboarding completed with `flash_key` (`D` | `F`) and persists to `UserSettings.ini`.
#[tauri::command]
#[specta::specta]
pub async fn complete_flash_onboarding(
    flash_key: FlashKeyPreference,
) -> Result<AppSettings, String> {
    tokio::task::spawn_blocking(move || lcu_injector::complete_flash_onboarding_service(flash_key))
        .await
        .map_err(|e| format!("Failed to complete Flash onboarding: {e}"))?
}

/// Returns disk usage statistics for the local cache directory (`Cache/`).
#[tauri::command]
#[specta::specta]
pub async fn get_local_cache_info() -> Result<LocalCacheInfo, String> {
    let Some(paths) = storage::global_paths().cloned() else {
        return Ok(LocalCacheInfo::default());
    };
    tokio::task::spawn_blocking(move || storage::inspect_local_cache(&paths))
        .await
        .map_err(|e| format!("Failed to inspect local cache: {e}"))
}

/// Deletes cached files under `Cache/` matching `scope` and returns refreshed usage statistics.
#[tauri::command]
#[specta::specta]
pub async fn clear_local_cache(scope: CacheClearScope) -> Result<LocalCacheInfo, String> {
    let Some(paths) = storage::global_paths().cloned() else {
        return Ok(LocalCacheInfo::default());
    };
    tokio::task::spawn_blocking(move || storage::clear_local_cache_at(&paths, scope))
        .await
        .map_err(|e| format!("Failed to clear local cache: {e}"))?
}

/// Restarts the Tauri application so updated WebView2 GPU flags take effect.
#[tauri::command]
#[specta::specta]
pub async fn restart_app(app_handle: AppHandle) {
    logging::log_info("app.lifecycle", "event=restart_requested");
    let _ = tokio::task::spawn_blocking(logging::flush_logs).await;
    app_bootstrap::allow_app_exit(&app_handle);
    app_handle.restart();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_autostart_status_portable_blocks_support() {
        let status = build_autostart_status(true, true);
        assert!(!status.enabled);
        assert!(!status.supported);
        assert!(status.is_portable);
    }

    #[test]
    fn test_build_autostart_status_installed_reflects_state() {
        let enabled_status = build_autostart_status(false, true);
        assert!(enabled_status.enabled);
        assert_eq!(enabled_status.supported, cfg!(desktop));
        assert!(!enabled_status.is_portable);

        let disabled_status = build_autostart_status(false, false);
        assert!(!disabled_status.enabled);
        assert_eq!(disabled_status.supported, cfg!(desktop));
        assert!(!disabled_status.is_portable);
    }
}
