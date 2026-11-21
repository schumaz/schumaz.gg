use crate::models::FlashKeyPreference;

use super::{
    AppSettings, SEC_AUTOMATION, SEC_GENERAL, SEC_GRAPHICS, SEC_NAVIGATION, SEC_PROFILE_APPEARANCE,
    SEC_UPDATER, SEC_WINDOW, clamp_profile_blur, fmt_bool, format_aram_mode,
};

pub fn serialize_user_settings_ini(s: &AppSettings) -> String {
    let mut out = String::with_capacity(1024);

    out.push_str(&format!("[{SEC_GENERAL}]\n"));
    if let Some(lang) = s.general.language {
        out.push_str(&format!("Language={}\n", lang.as_str()));
    } else {
        out.push_str("Language=\n");
    }
    out.push_str(&format!(
        "bLanguageOnboardingCompleted={}\n",
        fmt_bool(s.general.language_onboarding_completed)
    ));
    out.push_str(&format!(
        "bFlashOnboardingCompleted={}\n",
        fmt_bool(s.general.flash_onboarding_completed)
    ));
    out.push_str(&format!(
        "bCloseToTray={}\n\n",
        fmt_bool(s.general.close_to_tray)
    ));

    out.push_str(&format!("[{SEC_AUTOMATION}]\n"));
    out.push_str(&format!(
        "bAutoInjectRunes={}\n",
        fmt_bool(s.automation.auto_inject_runes)
    ));
    out.push_str(&format!(
        "bAutoInjectSpells={}\n",
        fmt_bool(s.automation.auto_inject_spells)
    ));
    out.push_str(&format!(
        "bAutoInjectItems={}\n",
        fmt_bool(s.automation.auto_inject_items)
    ));
    out.push_str(&format!(
        "FlashKey={}\n",
        match s.automation.flash_key {
            FlashKeyPreference::D => "D",
            FlashKeyPreference::F => "F",
        }
    ));
    out.push_str(&format!(
        "bPatch48hProtection={}\n",
        fmt_bool(s.automation.patch_48h_protection)
    ));
    out.push_str(&format!(
        "bAutoMarkDailyMissionsRead={}\n\n",
        fmt_bool(s.automation.auto_mark_daily_missions_read)
    ));

    out.push_str(&format!("[{SEC_PROFILE_APPEARANCE}]\n"));
    out.push_str(&format!(
        "ProfileBlurPercent={}\n",
        clamp_profile_blur(s.profile_appearance.profile_blur_percent)
    ));
    out.push_str(&format!(
        "bWallpaperVisible={}\n\n",
        fmt_bool(s.profile_appearance.wallpaper_visible)
    ));

    out.push_str(&format!("[{SEC_GRAPHICS}]\n"));
    out.push_str(&format!(
        "bHardwareAcceleration={}\n\n",
        fmt_bool(s.graphics.hardware_acceleration)
    ));

    out.push_str(&format!("[{SEC_WINDOW}]\n"));
    out.push_str(&format!(
        "WindowWidth={}\n",
        s.window.window_width.round() as i64
    ));
    out.push_str(&format!(
        "WindowHeight={}\n",
        s.window.window_height.round() as i64
    ));
    match s.window.window_x {
        Some(x) => out.push_str(&format!("WindowX={}\n", x.round() as i64)),
        None => out.push_str("WindowX=\n"),
    }
    match s.window.window_y {
        Some(y) => out.push_str(&format!("WindowY={}\n", y.round() as i64)),
        None => out.push_str("WindowY=\n"),
    }
    out.push_str(&format!(
        "bWindowMaximized={}\n",
        fmt_bool(s.window.window_maximized)
    ));
    match s.window.window_monitor.as_deref().map(str::trim) {
        Some(monitor) if !monitor.is_empty() => {
            out.push_str(&format!("WindowMonitor={monitor}\n\n"))
        }
        _ => out.push_str("WindowMonitor=\n\n"),
    }

    out.push_str(&format!("[{SEC_NAVIGATION}]\n"));
    match s.navigation.saved_aram_mode {
        Some(m) => out.push_str(&format!("SavedAramMode={}\n", format_aram_mode(m))),
        None => out.push_str("SavedAramMode=\n"),
    }
    out.push_str(&format!(
        "ProfileMatchModeFilter={}\n",
        s.navigation.profile_match_mode_filter
    ));
    match &s.navigation.aram_mayhem_last_champion {
        Some(c) => out.push_str(&format!("AramMayhemLastChampion={c}\n")),
        None => out.push_str("AramMayhemLastChampion=\n"),
    }
    out.push_str(&format!(
        "RankedFilterRegion={}\n",
        s.navigation.ranked_filter_region
    ));
    out.push_str(&format!(
        "RankedFilterTier={}\n",
        s.navigation.ranked_filter_tier
    ));
    out.push_str(&format!(
        "RankedFilterLane={}\n",
        s.navigation.ranked_filter_lane
    ));
    match &s.navigation.ranked_filter_patch {
        Some(p) => out.push_str(&format!("RankedFilterPatch={p}\n")),
        None => out.push_str("RankedFilterPatch=\n"),
    }
    out.push_str(&format!(
        "RankedChampionTierFilter={}\n",
        s.navigation.ranked_champion_tier_filter
    ));
    match s.navigation.last_summoner_id {
        Some(id) => out.push_str(&format!("LastSummonerId={id}\n\n")),
        None => out.push_str("LastSummonerId=\n\n"),
    }

    out.push_str(&format!("[{SEC_UPDATER}]\n"));
    match s.updater.last_check_epoch_secs {
        Some(ts) if ts > 0 => out.push_str(&format!("LastUpdateCheckEpochSecs={ts}\n")),
        _ => out.push_str("LastUpdateCheckEpochSecs=\n"),
    }

    out
}
