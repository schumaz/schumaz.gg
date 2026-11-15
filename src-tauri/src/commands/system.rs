use crate::models::AppLocale;
use crate::storage::{self, AppSettings, LegacyLocalStoragePayload, NavigationStatePatch};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{AppHandle, Manager, State};

const GRAPHICS_SETTINGS_FILENAME: &str = "graphics_settings.json";
const WRY_DEFAULT_DISABLED_FEATURES: &str =
    "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GraphicsSettingsFile {
    #[serde(default = "default_hardware_acceleration")]
    hardware_acceleration: bool,
}

fn default_hardware_acceleration() -> bool {
    true
}

impl Default for GraphicsSettingsFile {
    fn default() -> Self {
        Self {
            hardware_acceleration: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct HardwareAccelerationStatus {
    pub enabled: bool,
    pub active_at_startup: bool,
}

pub struct GraphicsState {
    pub active_at_startup: bool,
    pub current_enabled: Mutex<bool>,
}

impl GraphicsState {
    pub fn new(active_at_startup: bool) -> Self {
        Self {
            active_at_startup,
            current_enabled: Mutex::new(active_at_startup),
        }
    }
}

pub fn browser_args_for_hardware_acceleration(enabled: bool) -> String {
    if enabled {
        format!("--enable-gpu-rasterization --enable-zero-copy {WRY_DEFAULT_DISABLED_FEATURES}")
    } else {
        format!("--disable-gpu --disable-gpu-compositing {WRY_DEFAULT_DISABLED_FEATURES}")
    }
}

pub fn resolve_legacy_graphics_config_path(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .app_config_dir()
        .ok()
        .map(|dir| dir.join(GRAPHICS_SETTINGS_FILENAME))
}

pub fn load_hardware_acceleration_from_path(path: &Path) -> bool {
    let Ok(raw) = fs::read_to_string(path) else {
        return true;
    };
    serde_json::from_str::<GraphicsSettingsFile>(&raw)
        .map(|cfg| cfg.hardware_acceleration)
        .unwrap_or(true)
}

pub fn save_hardware_acceleration_to_path(path: &Path, enabled: bool) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Falha ao criar pasta de configurações gráficas: {e}"))?;
    }
    let payload = GraphicsSettingsFile {
        hardware_acceleration: enabled,
    };
    let serialized = serde_json::to_string_pretty(&payload)
        .map_err(|e| format!("Falha ao serializar configurações gráficas: {e}"))?;
    fs::write(path, serialized).map_err(|e| format!("Falha ao salvar configurações gráficas: {e}"))
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_browser_args_preserve_wry_disabled_features() {
        let enabled_args = browser_args_for_hardware_acceleration(true);
        assert!(enabled_args.contains("--enable-gpu-rasterization"));
        assert!(enabled_args.contains("--enable-zero-copy"));
        assert!(enabled_args.contains(WRY_DEFAULT_DISABLED_FEATURES));

        let disabled_args = browser_args_for_hardware_acceleration(false);
        assert!(disabled_args.contains("--disable-gpu"));
        assert!(disabled_args.contains("--disable-gpu-compositing"));
        assert!(disabled_args.contains(WRY_DEFAULT_DISABLED_FEATURES));
    }

    #[test]
    fn test_load_and_save_hardware_acceleration_defaults_to_true() {
        let temp_dir =
            std::env::temp_dir().join(format!("schumazgg_gpu_test_{}", std::process::id()));
        let config_path = temp_dir.join(GRAPHICS_SETTINGS_FILENAME);
        let _ = fs::remove_dir_all(&temp_dir);

        assert!(load_hardware_acceleration_from_path(&config_path));

        save_hardware_acceleration_to_path(&config_path, false).expect("save false");
        assert!(!load_hardware_acceleration_from_path(&config_path));

        save_hardware_acceleration_to_path(&config_path, true).expect("save true");
        assert!(load_hardware_acceleration_from_path(&config_path));

        fs::write(&config_path, "{invalid json").expect("write invalid json");
        assert!(load_hardware_acceleration_from_path(&config_path));

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
