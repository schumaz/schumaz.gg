use serde::{Deserialize, Serialize};
use specta::Type;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

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

    pub fn status(&self) -> HardwareAccelerationStatus {
        let enabled = self
            .current_enabled
            .lock()
            .map(|g| *g)
            .unwrap_or(self.active_at_startup);
        HardwareAccelerationStatus {
            enabled,
            active_at_startup: self.active_at_startup,
        }
    }
}

pub fn update_hardware_acceleration_setting(
    enabled: bool,
    state: &GraphicsState,
) -> Result<HardwareAccelerationStatus, String> {
    super::settings::update_global_settings(|s| {
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
            .map_err(|e| format!("Failed to create graphics settings directory: {e}"))?;
    }
    let payload = GraphicsSettingsFile {
        hardware_acceleration: enabled,
    };
    let serialized = serde_json::to_string_pretty(&payload)
        .map_err(|e| format!("Failed to serialize graphics settings: {e}"))?;
    fs::write(path, serialized).map_err(|e| format!("Failed to save graphics settings: {e}"))
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
