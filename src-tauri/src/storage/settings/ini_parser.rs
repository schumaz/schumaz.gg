use super::{
    AppSettings, SEC_AUTOMATION, SEC_GENERAL, SEC_GRAPHICS, SEC_NAVIGATION, SEC_PROFILE_APPEARANCE,
    SEC_UPDATER, SEC_WINDOW, parse_aram_mode, parse_bool, parse_flash_key, parse_locale,
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
                "bCloseToTray" => {
                    if let Some(b) = parse_bool(val) {
                        settings.general.close_to_tray = b;
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
            SEC_UPDATER if key == "LastUpdateCheckEpochSecs" => {
                if val.is_empty() {
                    settings.updater.last_check_epoch_secs = None;
                } else if let Ok(ts) = val.parse::<u64>()
                    && ts > 0
                {
                    settings.updater.last_check_epoch_secs = Some(ts);
                }
            }
            _ => {}
        }
    }

    // Consistency: If general.language is set, ensure automation.language mirrors it
    if let Some(loc) = settings.general.language {
        settings.automation.language = loc;
    }

    settings
}
