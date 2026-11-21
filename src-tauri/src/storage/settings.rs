pub mod ini_parser;
pub mod ini_serializer;
pub mod store;

#[cfg(test)]
mod ini_parser_tests;

pub use ini_parser::*;
pub use ini_serializer::*;
pub use store::*;

use crate::models::{AppLocale, AramMode, FlashKeyPreference, UserPreferences};
use serde::{Deserialize, Serialize};
use specta::Type;

use super::window_bounds::{DEFAULT_WINDOW_HEIGHT, DEFAULT_WINDOW_WIDTH};

pub const DEFAULT_PROFILE_BLUR_PERCENT: u32 = 10;

pub(super) const SEC_GENERAL: &str = "/Script/SchumazGG.GeneralSettings";
pub(super) const SEC_AUTOMATION: &str = "/Script/SchumazGG.AutomationSettings";
pub(super) const SEC_PROFILE_APPEARANCE: &str = "/Script/SchumazGG.ProfileAppearanceSettings";
pub(super) const SEC_GRAPHICS: &str = "/Script/SchumazGG.GraphicsSettings";
pub(super) const SEC_WINDOW: &str = "/Script/SchumazGG.WindowSettings";
pub(super) const SEC_NAVIGATION: &str = "/Script/SchumazGG.NavigationState";
pub(super) const SEC_UPDATER: &str = "/Script/SchumazGG.UpdaterSettings";

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

pub(super) fn parse_bool(raw: &str) -> Option<bool> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Some(true),
        "false" | "0" | "no" | "off" => Some(false),
        _ => None,
    }
}

pub(super) fn fmt_bool(v: bool) -> &'static str {
    if v { "True" } else { "False" }
}

pub(super) fn parse_locale(raw: &str) -> Option<AppLocale> {
    match raw.trim() {
        "pt_BR" | "pt-BR" | "pt_br" => Some(AppLocale::PtBr),
        "en_US" | "en-US" | "en_us" => Some(AppLocale::EnUs),
        _ => None,
    }
}

pub(super) fn parse_flash_key(raw: &str) -> Option<FlashKeyPreference> {
    match raw.trim().to_ascii_uppercase().as_str() {
        "D" => Some(FlashKeyPreference::D),
        "F" => Some(FlashKeyPreference::F),
        _ => None,
    }
}

pub(super) fn parse_aram_mode(raw: &str) -> Option<AramMode> {
    match raw.trim() {
        "aram" => Some(AramMode::Aram),
        "aram_mayhem" => Some(AramMode::AramMayhem),
        _ => None,
    }
}

pub(super) fn format_aram_mode(mode: AramMode) -> &'static str {
    match mode {
        AramMode::Aram => "aram",
        AramMode::AramMayhem => "aram_mayhem",
    }
}

pub(super) fn clamp_profile_blur(val: u32) -> u32 {
    val.min(100)
}

pub(super) fn validate_ranked_lane(raw: &str) -> String {
    match raw.trim() {
        "all" | "top" | "jungle" | "mid" | "adc" | "support" => raw.trim().to_string(),
        _ => "all".to_string(),
    }
}

pub(super) fn validate_champion_tier_filter(raw: &str) -> String {
    match raw.trim() {
        "all" | "0" | "1" | "2" | "3" | "4" | "5" => raw.trim().to_string(),
        _ => "all".to_string(),
    }
}
