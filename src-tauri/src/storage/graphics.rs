use serde::{Deserialize, Serialize};
use specta::Type;
use std::sync::atomic::{AtomicBool, Ordering};

const WRY_DEFAULT_DISABLED_FEATURES: &str = "msWebOOUI,msPdfOOUI,msSmartScreenProtection";
const EXTRA_DISABLED_FEATURES: &str = "Translate,TranslateUI,BackForwardCache,Prerender2,SpareRendererForSitePerProcess,MediaRouter,OptimizationHints,AutofillServerCommunication,AudioServiceOutOfProcess";
const ENABLED_FEATURES: &str = "CalculateNativeWinOcclusion";
const SHARED_RUNTIME_ARGS: &str = "--renderer-process-limit=1 --js-flags=--max-old-space-size=128 --disk-cache-size=16777216 --media-cache-size=1048576 --disable-background-networking --disable-component-update --disable-domain-reliability";

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct HardwareAccelerationStatus {
    pub enabled: bool,
    pub active_at_startup: bool,
}

pub struct GraphicsState {
    active_at_startup: AtomicBool,
    current_enabled: AtomicBool,
}

impl GraphicsState {
    pub fn new(active_at_startup: bool) -> Self {
        Self {
            active_at_startup: AtomicBool::new(active_at_startup),
            current_enabled: AtomicBool::new(active_at_startup),
        }
    }

    pub fn mark_webview_created(&self, active_hardware_acceleration: bool) {
        self.active_at_startup
            .store(active_hardware_acceleration, Ordering::SeqCst);
    }

    pub fn status(&self) -> HardwareAccelerationStatus {
        HardwareAccelerationStatus {
            enabled: self.current_enabled.load(Ordering::SeqCst),
            active_at_startup: self.active_at_startup.load(Ordering::SeqCst),
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
    state.current_enabled.store(enabled, Ordering::SeqCst);
    Ok(state.status())
}

pub fn browser_args_for_hardware_acceleration(enabled: bool) -> String {
    let disabled_features = format!("{WRY_DEFAULT_DISABLED_FEATURES},{EXTRA_DISABLED_FEATURES}");
    if enabled {
        format!(
            "--enable-gpu-rasterization --enable-features={ENABLED_FEATURES} --force-gpu-mem-available-mb=64 --gpu-program-cache-size-kb=2048 --disable-features={disabled_features} {SHARED_RUNTIME_ARGS}"
        )
    } else {
        format!(
            "--disable-gpu --disable-gpu-compositing --enable-features={ENABLED_FEATURES} --disable-features={disabled_features} {SHARED_RUNTIME_ARGS}"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_browser_args_preserve_wry_disabled_features_and_cap_gpu_ram() {
        let enabled_args = browser_args_for_hardware_acceleration(true);
        assert!(enabled_args.contains("--enable-gpu-rasterization"));
        assert!(enabled_args.contains("--enable-features=CalculateNativeWinOcclusion"));
        assert!(enabled_args.contains("--force-gpu-mem-available-mb=64"));
        assert!(enabled_args.contains("--gpu-program-cache-size-kb=2048"));
        assert!(enabled_args.contains(WRY_DEFAULT_DISABLED_FEATURES));
        assert!(enabled_args.contains("--renderer-process-limit=1"));
        assert!(enabled_args.contains("--js-flags=--max-old-space-size=128"));
        assert!(!enabled_args.contains("--enable-zero-copy"));
        assert_eq!(enabled_args.matches("--disable-features=").count(), 1);
        assert_eq!(enabled_args.matches("--enable-features=").count(), 1);

        let disabled_args = browser_args_for_hardware_acceleration(false);
        assert!(disabled_args.contains("--disable-gpu"));
        assert!(disabled_args.contains("--disable-gpu-compositing"));
        assert!(disabled_args.contains("--enable-features=CalculateNativeWinOcclusion"));
        assert!(disabled_args.contains(WRY_DEFAULT_DISABLED_FEATURES));
        assert!(!disabled_args.contains("--enable-zero-copy"));
        assert_eq!(disabled_args.matches("--disable-features=").count(), 1);
        assert_eq!(disabled_args.matches("--enable-features=").count(), 1);
    }

    #[test]
    fn test_graphics_state_updates_active_at_startup_on_webview_recreate() {
        let state = GraphicsState::new(true);
        assert_eq!(
            state.status(),
            HardwareAccelerationStatus {
                enabled: true,
                active_at_startup: true,
            }
        );

        state.current_enabled.store(false, Ordering::SeqCst);
        assert_eq!(
            state.status(),
            HardwareAccelerationStatus {
                enabled: false,
                active_at_startup: true,
            }
        );

        state.mark_webview_created(false);
        assert_eq!(
            state.status(),
            HardwareAccelerationStatus {
                enabled: false,
                active_at_startup: false,
            }
        );
    }
}
