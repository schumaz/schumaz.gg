use super::ini_parser::parse_user_settings_ini;
use super::ini_serializer::serialize_user_settings_ini;
use super::{
    AppSettings, DEFAULT_PROFILE_BLUR_PERCENT, NavigationState, UpdaterSettings, WindowSettings,
};
use crate::models::{AppLocale, AramMode, FlashKeyPreference};

#[test]
fn test_ini_roundtrip_and_tolerant_manual_edit_fallback() {
    let mut s = AppSettings::default();
    assert!(!s.general.close_to_tray);
    assert_eq!(s.updater.last_check_epoch_secs, None);
    s.general.language = Some(AppLocale::PtBr);
    s.general.language_onboarding_completed = true;
    s.general.flash_onboarding_completed = true;
    s.general.close_to_tray = true;
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
    s.updater = UpdaterSettings {
        last_check_epoch_secs: Some(1_760_000_000),
    };

    let ini = serialize_user_settings_ini(&s);
    assert!(!ini.contains("AramMayhemRarityFilter"));
    assert!(ini.contains("bCloseToTray=True"));
    assert!(ini.contains(r"WindowMonitor=\\.\DISPLAY2"));
    assert!(ini.contains("LastUpdateCheckEpochSecs=1760000000"));
    let parsed = parse_user_settings_ini(&ini);
    assert_eq!(parsed, s);

    // Existing INI without bCloseToTray or UpdaterSettings defaults cleanly
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
    assert!(!parsed_legacy.general.close_to_tray);
    assert_eq!(parsed_legacy.updater.last_check_epoch_secs, None);
    assert_eq!(
        parsed_legacy
            .navigation
            .aram_mayhem_last_champion
            .as_deref(),
        Some("ahri")
    );

    // Corrupt / malformed manual edits fall back per-key without losing valid keys
    let edited = format!(
        "{ini}\n; user comment\n[/Script/SchumazGG.ProfileAppearanceSettings]\nProfileBlurPercent=notanumber\nbWallpaperVisible=True\n[/Script/SchumazGG.AutomationSettings]\nFlashKey=INVALID\n[/Script/SchumazGG.UpdaterSettings]\nLastUpdateCheckEpochSecs=notanumber\n"
    );
    let parsed_edited = parse_user_settings_ini(&edited);
    assert_eq!(parsed_edited.profile_appearance.profile_blur_percent, 45);
    assert!(parsed_edited.profile_appearance.wallpaper_visible);
    assert_eq!(parsed_edited.automation.flash_key, FlashKeyPreference::F);
    assert_eq!(
        parsed_edited.updater.last_check_epoch_secs,
        Some(1_760_000_000)
    );

    let partial_corrupt = "
[/Script/SchumazGG.ProfileAppearanceSettings]
ProfileBlurPercent=abc
bWallpaperVisible=notabool
[/Script/SchumazGG.UpdaterSettings]
LastUpdateCheckEpochSecs=0
";
    let fallback_test = parse_user_settings_ini(partial_corrupt);
    assert_eq!(
        fallback_test.profile_appearance.profile_blur_percent,
        DEFAULT_PROFILE_BLUR_PERCENT
    );
    assert!(fallback_test.profile_appearance.wallpaper_visible);
    assert_eq!(fallback_test.updater.last_check_epoch_secs, None);
}
