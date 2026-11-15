use crate::lcu_injector;
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
    state.status()
}

/// Saves the GPU hardware acceleration preference to `UserSettings.ini` for the next startup.
#[tauri::command]
#[specta::specta]
pub fn set_hardware_acceleration(
    enabled: bool,
    state: State<'_, GraphicsState>,
) -> Result<HardwareAccelerationStatus, String> {
    storage::update_hardware_acceleration_setting(enabled, &state)
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
    storage::update_profile_appearance_settings(blur_percent, wallpaper_visible)
}

/// Applies a partial patch to `[NavigationState]` in `UserSettings.ini`.
#[tauri::command]
#[specta::specta]
pub fn update_navigation_state(patch: NavigationStatePatch) -> Result<AppSettings, String> {
    storage::update_navigation_settings(patch)
}

/// Marks the first-run language onboarding completed with `locale` and persists to `UserSettings.ini`.
#[tauri::command]
#[specta::specta]
pub async fn complete_language_onboarding(locale: AppLocale) -> Result<AppSettings, String> {
    lcu_injector::complete_language_onboarding_service(locale)
}

/// Performs one-time migration of the 6 legacy `localStorage` keys into `UserSettings.ini`.
#[tauri::command]
#[specta::specta]
pub fn migrate_legacy_local_storage(
    payload: LegacyLocalStoragePayload,
) -> Result<AppSettings, String> {
    lcu_injector::migrate_legacy_local_storage_service(payload)
}

/// Restarts the Tauri application so updated WebView2 GPU flags take effect.
#[tauri::command]
#[specta::specta]
pub fn restart_app(app_handle: AppHandle) {
    app_handle.restart();
}
