use super::{
    AppSettings, SEC_AUTOMATION, SEC_GENERAL, SEC_GRAPHICS, SEC_NAVIGATION, SEC_PROFILE_APPEARANCE,
    SEC_WINDOW, parse_aram_mode, parse_bool, parse_flash_key, parse_locale,
    validate_champion_tier_filter, validate_ranked_lane,
};

pub fn parse_user_settings_ini(raw: &str) -> AppSettings {
    let mut settings = AppSettings::default();
    let mut current_section: &str = "";

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with(';') || trimmed.starts_with('#') {
            continue;
        }

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            current_section = &trimmed[1..trimmed.len() - 1];
            continue;
        }

        let Some((key_raw, val_raw)) = trimmed.split_once('=') else {
            continue;
        };
        let key = key_raw.trim();
        let val = val_raw.trim();

        match current_section {
            SEC_GENERAL => match key {
                "Language" => {
                    if let Some(loc) = parse_locale(val) {
                        settings.general.language = Some(loc);
                    }
                }
                "bLanguageOnboardingCompleted" => {
                    if let Some(b) = parse_bool(val) {
                        settings.general.language_onboarding_completed = b;
                    }
                }
                "bFlashOnboardingCompleted" => {
                    if let Some(b) = parse_bool(val) {
                        settings.general.flash_onboarding_completed = b;
                    }
                }
                _ => {}
            },
            SEC_AUTOMATION => match key {
                "bAutoInjectRunes" => {
                    if let Some(b) = parse_bool(val) {
                        settings.automation.auto_inject_runes = b;
                    }
                }
                "bAutoInjectSpells" => {
                    if let Some(b) = parse_bool(val) {
                        settings.automation.auto_inject_spells = b;
                    }
                }
                "bAutoInjectItems" => {
                    if let Some(b) = parse_bool(val) {
                        settings.automation.auto_inject_items = b;
                    }
                }
                "FlashKey" => {
                    if let Some(fk) = parse_flash_key(val) {
                        settings.automation.flash_key = fk;
                    }
                }
                "bPatch48hProtection" => {
                    if let Some(b) = parse_bool(val) {
                        settings.automation.patch_48h_protection = b;
                    }
                }
                "bAutoMarkDailyMissionsRead" => {
                    if let Some(b) = parse_bool(val) {
                        settings.automation.auto_mark_daily_missions_read = b;
                    }
                }
                "Language" => {
                    if let Some(loc) = parse_locale(val) {
                        settings.automation.language = loc;
                    }
                }
                _ => {}
            },
            SEC_PROFILE_APPEARANCE => match key {
                "ProfileBlurPercent" => {
                    if let Ok(v) = val.parse::<u32>() {
                        settings.profile_appearance.profile_blur_percent = v.min(100);
                    }
                }
                "bWallpaperVisible" => {
                    if let Some(b) = parse_bool(val) {
                        settings.profile_appearance.wallpaper_visible = b;
                    }
                }
                _ => {}
            },
            SEC_GRAPHICS => {
                if key == "bHardwareAcceleration"
                    && let Some(b) = parse_bool(val)
                {
                    settings.graphics.hardware_acceleration = b;
                }
            }
            SEC_WINDOW => match key {
                "WindowWidth" => {
                    if let Ok(v) = val.parse::<f64>()
                        && v > 0.0
                    {
                        settings.window.window_width = v;
                    }
                }
                "WindowHeight" => {
                    if let Ok(v) = val.parse::<f64>()
                        && v > 0.0
                    {
                        settings.window.window_height = v;
                    }
                }
                "WindowX" => {
                    if let Ok(v) = val.parse::<f64>() {
                        settings.window.window_x = Some(v);
                    }
                }
                "WindowY" => {
                    if let Ok(v) = val.parse::<f64>() {
                        settings.window.window_y = Some(v);
                    }
                }
                "bWindowMaximized" => {
                    if let Some(b) = parse_bool(val) {
                        settings.window.window_maximized = b;
                    }
                }
                "WindowMonitor" => {
                    settings.window.window_monitor = if val.is_empty() {
                        None
                    } else {
                        Some(val.to_string())
                    };
                }
                _ => {}
            },
            SEC_NAVIGATION => match key {
                "SavedAramMode" => {
                    settings.navigation.saved_aram_mode = parse_aram_mode(val);
                }
                "ProfileMatchModeFilter" => {
                    if !val.is_empty() {
                        settings.navigation.profile_match_mode_filter = val.to_string();
                    }
                }
                "AramMayhemLastChampion" => {
                    settings.navigation.aram_mayhem_last_champion = if val.is_empty() {
                        None
                    } else {
                        Some(val.to_ascii_lowercase())
                    };
                }
                "RankedFilterRegion" => {
                    if !val.is_empty() {
                        settings.navigation.ranked_filter_region = val.to_string();
                    }
                }
                "RankedFilterTier" => {
                    if !val.is_empty() {
                        settings.navigation.ranked_filter_tier = val.to_string();
                    }
                }
                "RankedFilterLane" => {
                    settings.navigation.ranked_filter_lane = validate_ranked_lane(val);
                }
                "RankedFilterPatch" => {
                    settings.navigation.ranked_filter_patch = if val.is_empty() {
                        None
                    } else {
                        Some(val.to_string())
                    };
                }
                "RankedChampionTierFilter" => {
                    settings.navigation.ranked_champion_tier_filter =
                        validate_champion_tier_filter(val);
                }
                "LastSummonerId" => {
                    if let Ok(id) = val.parse::<u64>()
                        && id > 0
                    {
                        settings.navigation.last_summoner_id = Some(id);
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }

    // Consistency: If general.language is set, ensure automation.language mirrors it
    if let Some(loc) = settings.general.language {
        settings.automation.language = loc;
    }

    settings
}

#[cfg(test)]
mod tests {
    use super::super::ini_serializer::serialize_user_settings_ini;
    use super::super::{DEFAULT_PROFILE_BLUR_PERCENT, NavigationState, WindowSettings};
    use super::*;
    use crate::models::{AppLocale, AramMode, FlashKeyPreference};

    #[test]
    fn test_ini_roundtrip_and_tolerant_manual_edit_fallback() {
        let mut s = AppSettings::default();
        s.general.language = Some(AppLocale::PtBr);
        s.general.language_onboarding_completed = true;
        s.general.flash_onboarding_completed = true;
        s.automation.auto_inject_runes = false;
        s.automation.flash_key = FlashKeyPreference::F;
        s.automation.auto_mark_daily_missions_read = true;
        s.automation.language = AppLocale::PtBr;
        s.profile_appearance.profile_blur_percent = 45;
        s.profile_appearance.wallpaper_visible = false;
        s.graphics.hardware_acceleration = false;
        s.window = WindowSettings {
            window_width: 1280.0,
            window_height: 720.0,
            window_x: Some(2050.0),
            window_y: Some(80.0),
            window_maximized: true,
            window_monitor: Some(r"\\.\DISPLAY2".to_string()),
        };
        s.navigation = NavigationState {
            saved_aram_mode: Some(AramMode::AramMayhem),
            profile_match_mode_filter: "ranked_solo".to_string(),
            aram_mayhem_last_champion: Some("ahri".to_string()),
            ranked_filter_region: "kr".to_string(),
            ranked_filter_tier: "master_plus".to_string(),
            ranked_filter_lane: "mid".to_string(),
            ranked_filter_patch: Some("16.21".to_string()),
            ranked_champion_tier_filter: "1".to_string(),
            last_summoner_id: Some(987654321),
        };

        let ini = serialize_user_settings_ini(&s);
        assert!(!ini.contains("AramMayhemRarityFilter"));
        assert!(ini.contains(r"WindowMonitor=\\.\DISPLAY2"));
        let parsed = parse_user_settings_ini(&ini);
        assert_eq!(parsed, s);

        // Existing INI with unknown/removed keys ignores them gracefully
        let legacy_ini = "
[/Script/SchumazGG.GeneralSettings]
Language=pt_BR
bLanguageOnboardingCompleted=True
bLegacyStorageMigrated=True
[/Script/SchumazGG.NavigationState]
AramMayhemRarityFilter=8
AramMayhemLastChampion=Ahri
";
        let parsed_legacy = parse_user_settings_ini(legacy_ini);
        assert!(parsed_legacy.general.language_onboarding_completed);
        assert!(!parsed_legacy.general.flash_onboarding_completed);
        assert_eq!(
            parsed_legacy
                .navigation
                .aram_mayhem_last_champion
                .as_deref(),
            Some("ahri")
        );

        // Corrupt / malformed manual edits fall back per-key without losing valid keys
        let edited = format!(
            "{ini}\n; user comment\n[/Script/SchumazGG.ProfileAppearanceSettings]\nProfileBlurPercent=notanumber\nbWallpaperVisible=True\n[/Script/SchumazGG.AutomationSettings]\nFlashKey=INVALID\n"
        );
        let parsed_edited = parse_user_settings_ini(&edited);
        assert_eq!(parsed_edited.profile_appearance.profile_blur_percent, 45);
        assert!(parsed_edited.profile_appearance.wallpaper_visible);
        assert_eq!(parsed_edited.automation.flash_key, FlashKeyPreference::F);

        let partial_corrupt = "
[/Script/SchumazGG.ProfileAppearanceSettings]
ProfileBlurPercent=abc
bWallpaperVisible=notabool
";
        let fallback_test = parse_user_settings_ini(partial_corrupt);
        assert_eq!(
            fallback_test.profile_appearance.profile_blur_percent,
            DEFAULT_PROFILE_BLUR_PERCENT
        );
        assert!(fallback_test.profile_appearance.wallpaper_visible);
    }
}
