pub mod ini_parser;
pub mod ini_serializer;
pub mod store;

#[cfg(test)]
mod ini_parser_tests;

pub use ini_parser::*;
pub use ini_serializer::*;
pub use store::*;

use crate::models::{AppLocale, AramMode, FlashKeyPreference};
pub use crate::models::{
    AppSettings, DEFAULT_PROFILE_BLUR_PERCENT, GeneralSettings, GraphicsSettings, NavigationState,
    NavigationStatePatch, ProfileAppearanceSettings, UpdaterSettings, WindowSettings,
};

pub(super) const SEC_GENERAL: &str = "/Script/SchumazGG.GeneralSettings";
pub(super) const SEC_AUTOMATION: &str = "/Script/SchumazGG.AutomationSettings";
pub(super) const SEC_PROFILE_APPEARANCE: &str = "/Script/SchumazGG.ProfileAppearanceSettings";
pub(super) const SEC_GRAPHICS: &str = "/Script/SchumazGG.GraphicsSettings";
pub(super) const SEC_WINDOW: &str = "/Script/SchumazGG.WindowSettings";
pub(super) const SEC_NAVIGATION: &str = "/Script/SchumazGG.NavigationState";
pub(super) const SEC_UPDATER: &str = "/Script/SchumazGG.UpdaterSettings";

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
        "all" | "0" | "1" | "2" | "3" | "4" => raw.trim().to_string(),
        _ => "all".to_string(),
    }
}
