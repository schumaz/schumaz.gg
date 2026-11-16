use serde::{Deserialize, Serialize};
use specta::Type;
use std::sync::Mutex;

const WRY_DEFAULT_DISABLED_FEATURES: &str =
    "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection";

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
}
