use super::super::window_bounds::{MIN_WINDOW_HEIGHT, MIN_WINDOW_WIDTH};
use super::{
    AppSettings, SEC_AUTOMATION, SEC_GENERAL, SEC_GRAPHICS, SEC_NAVIGATION, SEC_PROFILE_APPEARANCE,
    SEC_WINDOW, clamp_profile_blur, parse_aram_mode, parse_bool, parse_flash_key, parse_locale,
    validate_champion_tier_filter, validate_ranked_lane, validate_rarity_filter,
};

/// Parses `UserSettings.ini` tolerantly: unknown sections/keys are ignored,
/// and malformed values fall back per-key to defaults.
pub fn parse_user_settings_ini(content: &str) -> AppSettings {
    let mut settings = AppSettings::default();
    let mut current_section = String::new();
    let mut saw_explicit_onboarding = false;

    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') && line.len() >= 2 {
            current_section = line[1..line.len() - 1].trim().to_string();
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let key = k.trim();
        let val = v.trim();

        match current_section.as_str() {
            SEC_GENERAL => match key {
                "Language" => {
                    if let Some(loc) = parse_locale(val) {
                        settings.general.language = Some(loc);
                        settings.automation.language = loc;
                    }
                }
                "bLanguageOnboardingCompleted" => {
                    if let Some(b) = parse_bool(val) {
                        settings.general.language_onboarding_completed = b;
                        saw_explicit_onboarding = true;
                    }
                }
                "bFlashOnboardingCompleted" => {
                    if let Some(b) = parse_bool(val) {
                        settings.general.flash_onboarding_completed = b;
                    }
                }
                "bLegacyStorageMigrated" => {
                    if let Some(b) = parse_bool(val) {
                        settings.general.legacy_storage_migrated = b;
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
                _ => {}
            },
            SEC_PROFILE_APPEARANCE => match key {
                "ProfileBlurPercent" => {
                    if let Ok(n) = val.parse::<u32>() {
                        settings.profile_appearance.profile_blur_percent = clamp_profile_blur(n);
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
                    if let Ok(w) = val.parse::<f64>()
                        && w.is_finite()
                        && w >= MIN_WINDOW_WIDTH
                    {
                        settings.window.window_width = w;
                    }
                }
                "WindowHeight" => {
                    if let Ok(h) = val.parse::<f64>()
                        && h.is_finite()
                        && h >= MIN_WINDOW_HEIGHT
                    {
                        settings.window.window_height = h;
                    }
                }
                "WindowX" => {
                    if let Ok(x) = val.parse::<f64>()
                        && x.is_finite()
                    {
                        settings.window.window_x = Some(x);
                    }
                }
                "WindowY" => {
                    if let Ok(y) = val.parse::<f64>()
                        && y.is_finite()
                    {
                        settings.window.window_y = Some(y);
                    }
                }
                "bWindowMaximized" => {
                    if let Some(b) = parse_bool(val) {
                        settings.window.window_maximized = b;
                    }
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
                "AramMayhemRarityFilter" => {
                    settings.navigation.aram_mayhem_rarity_filter = validate_rarity_filter(val);
                }
                "AramMayhemLastChampion" => {
                    if !val.is_empty() {
                        settings.navigation.aram_mayhem_last_champion =
                            Some(val.to_ascii_lowercase());
                    }
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
                    if !val.is_empty() {
                        settings.navigation.ranked_filter_patch = Some(val.to_string());
                    }
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

    if !saw_explicit_onboarding && settings.general.language.is_some() {
        settings.general.language_onboarding_completed = true;
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
        s.general.legacy_storage_migrated = true;
        s.automation.auto_inject_runes = false;
        s.automation.flash_key = FlashKeyPreference::F;
        s.automation.auto_mark_daily_missions_read = true;
        s.automation.language = AppLocale::PtBr;
        s.profile_appearance.profile_blur_percent = 45;
        s.profile_appearance.wallpaper_visible = false;
        s.graphics.hardware_acceleration = false;
        s.window = WindowSettings {
            window_width: 1280.0,
            window_height: 800.0,
            window_x: Some(150.0),
            window_y: Some(80.0),
            window_maximized: true,
        };
        s.navigation = NavigationState {
            saved_aram_mode: Some(AramMode::AramMayhem),
            profile_match_mode_filter: "ranked_solo".to_string(),
            aram_mayhem_rarity_filter: "8".to_string(),
            aram_mayhem_last_champion: Some("ahri".to_string()),
            ranked_filter_region: "kr".to_string(),
            ranked_filter_tier: "master_plus".to_string(),
            ranked_filter_lane: "mid".to_string(),
            ranked_filter_patch: Some("16.21".to_string()),
            ranked_champion_tier_filter: "1".to_string(),
            last_summoner_id: Some(987654321),
        };

        let ini = serialize_user_settings_ini(&s);
        let parsed = parse_user_settings_ini(&ini);
        assert_eq!(parsed, s);

        // Existing INI without bFlashOnboardingCompleted keeps flash_onboarding_completed = false
        // so existing users still see the Flash onboarding modal once.
        let legacy_ini = "
[/Script/SchumazGG.GeneralSettings]
Language=pt_BR
bLanguageOnboardingCompleted=True
bLegacyStorageMigrated=True
";
        let parsed_legacy = parse_user_settings_ini(legacy_ini);
        assert!(parsed_legacy.general.language_onboarding_completed);
        assert!(!parsed_legacy.general.flash_onboarding_completed);

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
bWallpaperVisible=False
";
        let fallback_test = parse_user_settings_ini(partial_corrupt);
        assert_eq!(
            fallback_test.profile_appearance.profile_blur_percent,
            DEFAULT_PROFILE_BLUR_PERCENT
        );
        assert!(!fallback_test.profile_appearance.wallpaper_visible);
    }
}
