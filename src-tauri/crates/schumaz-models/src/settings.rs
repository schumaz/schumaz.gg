use serde::{Deserialize, Serialize};
use specta::Type;

use super::{AppLocale, AramMode, UserPreferences};

pub const MIN_WINDOW_WIDTH: f64 = 840.0;
pub const MIN_WINDOW_HEIGHT: f64 = 600.0;
pub const DEFAULT_WINDOW_WIDTH: f64 = 1280.0;
pub const DEFAULT_WINDOW_HEIGHT: f64 = 720.0;
pub const DEFAULT_PROFILE_BLUR_PERCENT: u32 = 10;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, Type)]
#[serde(rename_all = "camelCase")]
pub struct GeneralSettings {
    pub language: Option<AppLocale>,
    pub language_onboarding_completed: bool,
    pub flash_onboarding_completed: bool,
    pub close_to_tray: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Type)]
#[serde(rename_all = "camelCase")]
pub struct ProfileAppearanceSettings {
    pub profile_blur_percent: u32,
    pub wallpaper_visible: bool,
}

impl Default for ProfileAppearanceSettings {
    fn default() -> Self {
        Self {
            profile_blur_percent: DEFAULT_PROFILE_BLUR_PERCENT,
            wallpaper_visible: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Type)]
#[serde(rename_all = "camelCase")]
pub struct GraphicsSettings {
    pub hardware_acceleration: bool,
}

impl Default for GraphicsSettings {
    fn default() -> Self {
        Self {
            hardware_acceleration: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Type)]
#[serde(rename_all = "camelCase")]
pub struct WindowSettings {
    pub window_width: f64,
    pub window_height: f64,
    pub window_x: Option<f64>,
    pub window_y: Option<f64>,
    pub window_maximized: bool,
    pub window_monitor: Option<String>,
}

impl Default for WindowSettings {
    fn default() -> Self {
        Self {
            window_width: DEFAULT_WINDOW_WIDTH,
            window_height: DEFAULT_WINDOW_HEIGHT,
            window_x: None,
            window_y: None,
            window_maximized: false,
            window_monitor: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Type)]
#[serde(rename_all = "camelCase")]
pub struct NavigationState {
    pub saved_aram_mode: Option<AramMode>,
    pub profile_match_mode_filter: String,
    pub aram_mayhem_last_champion: Option<String>,
    pub ranked_filter_region: String,
    pub ranked_filter_tier: String,
    pub ranked_filter_lane: String,
    pub ranked_filter_patch: Option<String>,
    pub ranked_champion_tier_filter: String,
    pub last_summoner_id: Option<u64>,
}

impl Default for NavigationState {
    fn default() -> Self {
        Self {
            saved_aram_mode: None,
            profile_match_mode_filter: "all".to_string(),
            aram_mayhem_last_champion: None,
            ranked_filter_region: "global".to_string(),
            ranked_filter_tier: "emerald_plus".to_string(),
            ranked_filter_lane: "all".to_string(),
            ranked_filter_patch: None,
            ranked_champion_tier_filter: "all".to_string(),
            last_summoner_id: None,
        }
    }
}

/// Persisted auto-updater state stored in `UserSettings.ini`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, Type)]
#[serde(rename_all = "camelCase")]
pub struct UpdaterSettings {
    pub last_check_epoch_secs: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Type)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub general: GeneralSettings,
    pub automation: UserPreferences,
    pub profile_appearance: ProfileAppearanceSettings,
    pub graphics: GraphicsSettings,
    pub window: WindowSettings,
    pub navigation: NavigationState,
    #[serde(default)]
    pub updater: UpdaterSettings,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct NavigationStatePatch {
    pub saved_aram_mode: Option<AramMode>,
    pub profile_match_mode_filter: Option<String>,
    /// Empty string clears `aram_mayhem_last_champion` to `None`.
    pub aram_mayhem_last_champion: Option<String>,
    pub ranked_filter_region: Option<String>,
    pub ranked_filter_tier: Option<String>,
    pub ranked_filter_lane: Option<String>,
    /// Empty string clears `ranked_filter_patch` to `None`.
    pub ranked_filter_patch: Option<String>,
    pub ranked_champion_tier_filter: Option<String>,
    pub last_summoner_id: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct HardwareAccelerationStatus {
    pub enabled: bool,
    pub active_at_startup: bool,
}

/// Disk usage summary for `%LOCALAPPDATA%\schumazgg\Cache\`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LocalCacheInfo {
    pub total_bytes: u64,
    pub assets_bytes: u64,
    pub data_bytes: u64,
    pub file_count: u32,
}

/// Target subset when clearing local cache from Settings.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CacheClearScope {
    Builds,
    Images,
    All,
}
