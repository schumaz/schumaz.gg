use crate::models::AppLocale;
use crate::storage::{
    self, AppSettings, GraphicsState, HardwareAccelerationStatus, LegacyLocalStoragePayload,
    NavigationStatePatch,
};
use tauri::{AppHandle, State};

/// Returns the saved GPU hardware acceleration setting and whether it was active at startup.
#[tauri::command]
#[specta::specta]
pub fn get_hardware_acceleration_status(
    state: State<'_, GraphicsState>,
) -> HardwareAccelerationStatus {
    let enabled = state
        .current_enabled
        .lock()
        .map(|g| *g)
        .unwrap_or(state.active_at_startup);
    HardwareAccelerationStatus {
        enabled,
        active_at_startup: state.active_at_startup,
    }
}

/// Saves the GPU hardware acceleration preference to `UserSettings.ini` for the next startup.
#[tauri::command]
#[specta::specta]
pub fn set_hardware_acceleration(
    enabled: bool,
    state: State<'_, GraphicsState>,
) -> Result<HardwareAccelerationStatus, String> {
    storage::update_global_settings(|s| {
        s.graphics.hardware_acceleration = enabled;
    })?;
    if let Ok(mut guard) = state.current_enabled.lock() {
        *guard = enabled;
    }
    Ok(HardwareAccelerationStatus {
        enabled,
        active_at_startup: state.active_at_startup,
    })
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
pub fn update_profile_appearance(
    blur_percent: u32,
    wallpaper_visible: bool,
) -> Result<AppSettings, String> {
    storage::update_global_settings(|s| {
        s.profile_appearance.profile_blur_percent = blur_percent.min(100);
        s.profile_appearance.wallpaper_visible = wallpaper_visible;
    })
}

/// Applies a partial patch to `[NavigationState]` in `UserSettings.ini`.
#[tauri::command]
#[specta::specta]
pub fn update_navigation_state(patch: NavigationStatePatch) -> Result<AppSettings, String> {
    storage::update_global_settings(|s| {
        storage::apply_navigation_patch(&mut s.navigation, patch);
    })
}

/// Marks the first-run language onboarding completed with `locale` and persists to `UserSettings.ini`.
#[tauri::command]
#[specta::specta]
pub async fn complete_language_onboarding(locale: AppLocale) -> Result<AppSettings, String> {
    crate::lcu_injector::set_active_locale(locale);
    let updated = storage::update_global_settings(|s| {
        s.general.language = Some(locale);
        s.general.language_onboarding_completed = true;
        s.automation.language = locale;
    })?;

    let client = crate::build_provider::shared_http_client().clone();
    tauri::async_runtime::spawn(async move {
        let _ = crate::build_provider::get_or_init_ddragon_cache_for_locale(&client, locale).await;
    });

    Ok(updated)
}

/// Performs one-time migration of the 6 legacy `localStorage` keys into `UserSettings.ini`.
#[tauri::command]
#[specta::specta]
pub fn migrate_legacy_local_storage(
    payload: LegacyLocalStoragePayload,
) -> Result<AppSettings, String> {
    let updated = storage::update_global_settings(|s| {
        let _ = storage::apply_legacy_local_storage_migration(s, &payload);
    })?;

    crate::lcu_injector::set_user_preferences(updated.automation.clone());
    Ok(updated)
}

/// Restarts the Tauri application so updated WebView2 GPU flags take effect.
#[tauri::command]
#[specta::specta]
pub fn restart_app(app_handle: AppHandle) {
    app_handle.restart();
}
