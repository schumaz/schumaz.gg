use std::fs;
use std::path::Path;

use super::ini_parser::parse_user_settings_ini;
use super::store::save_user_settings_to_path;
use super::{
    AppSettings, LegacyLocalStoragePayload, clamp_profile_blur, parse_aram_mode, parse_bool,
    parse_flash_key, parse_locale, validate_ranked_lane,
};

pub fn load_or_migrate_user_settings(
    ini_path: &Path,
    legacy_graphics_json: Option<&Path>,
) -> AppSettings {
    if ini_path.exists()
        && let Ok(raw) = fs::read_to_string(ini_path)
    {
        return parse_user_settings_ini(&raw);
    }

    let mut settings = AppSettings::default();
    let mut migrated_any = false;

    if let Some(legacy_path) = legacy_graphics_json
        && legacy_path.exists()
        && let Ok(raw) = fs::read_to_string(legacy_path)
        && let Ok(json) = serde_json::from_str::<serde_json::Value>(&raw)
        && let Some(enabled) = json.get("hardware_acceleration").and_then(|v| v.as_bool())
    {
        settings.graphics.hardware_acceleration = enabled;
        migrated_any = true;
    }

    if migrated_any {
        let _ = save_user_settings_to_path(ini_path, &settings);
    }

    settings
}

pub fn apply_legacy_local_storage_migration(
    settings: &mut AppSettings,
    payload: &LegacyLocalStoragePayload,
) -> bool {
    if settings.general.legacy_storage_migrated {
        return false;
    }

    let mut changed = false;

    if let Some(raw_lang) = payload.schumaz_language.as_deref()
        && let Some(loc) = parse_locale(raw_lang)
    {
        settings.general.language = Some(loc);
        settings.general.language_onboarding_completed = true;
        settings.automation.language = loc;
        changed = true;
    }

    if let Some(raw_prefs) = payload.schumaz_user_preferences.as_deref()
        && let Ok(parsed) = serde_json::from_str::<serde_json::Value>(raw_prefs)
    {
        if let Some(v) = parsed.get("autoInjectRunes").and_then(|x| x.as_bool()) {
            settings.automation.auto_inject_runes = v;
        }
        if let Some(v) = parsed.get("autoInjectSpells").and_then(|x| x.as_bool()) {
            settings.automation.auto_inject_spells = v;
        }
        if let Some(v) = parsed.get("autoInjectItems").and_then(|x| x.as_bool()) {
            settings.automation.auto_inject_items = v;
        }
        if let Some(fk) = parsed
            .get("flashKey")
            .and_then(|x| x.as_str())
            .and_then(parse_flash_key)
        {
            settings.automation.flash_key = fk;
        }
        if let Some(v) = parsed.get("patch48hProtection").and_then(|x| x.as_bool()) {
            settings.automation.patch_48h_protection = v;
        }
        if let Some(v) = parsed
            .get("autoMarkDailyMissionsRead")
            .and_then(|x| x.as_bool())
        {
            settings.automation.auto_mark_daily_missions_read = v;
        }
        if let Some(loc) = parsed
            .get("language")
            .and_then(|x| x.as_str())
            .and_then(parse_locale)
            && settings.general.language.is_none()
        {
            settings.general.language = Some(loc);
            settings.automation.language = loc;
        }
        changed = true;
    }

    if let Some(raw_blur) = payload.schumaz_profile_ui_blur.as_deref()
        && let Ok(val) = raw_blur.trim().parse::<u32>()
    {
        settings.profile_appearance.profile_blur_percent = clamp_profile_blur(val);
        changed = true;
    }

    if let Some(raw_vis) = payload.schumaz_profile_wallpaper_visible.as_deref()
        && let Some(b) = parse_bool(raw_vis)
    {
        settings.profile_appearance.wallpaper_visible = b;
        changed = true;
    }

    if let Some(raw_aram) = payload.schumaz_aram_submode.as_deref()
        && let Some(mode) = parse_aram_mode(raw_aram)
    {
        settings.navigation.saved_aram_mode = Some(mode);
        changed = true;
    }

    if let Some(raw_ranked) = payload.schumaz_ranked_filters.as_deref()
        && let Ok(parsed) = serde_json::from_str::<serde_json::Value>(raw_ranked)
    {
        if let Some(r) = parsed.get("region").and_then(|x| x.as_str()).map(str::trim)
            && !r.is_empty()
        {
            settings.navigation.ranked_filter_region = r.to_string();
        }
        if let Some(t) = parsed.get("tier").and_then(|x| x.as_str()).map(str::trim)
            && !t.is_empty()
        {
            settings.navigation.ranked_filter_tier = t.to_string();
        }
        if let Some(l) = parsed.get("lane").and_then(|x| x.as_str()) {
            settings.navigation.ranked_filter_lane = validate_ranked_lane(l);
        }
        if let Some(p) = parsed.get("patch").and_then(|x| x.as_str()).map(str::trim)
            && !p.is_empty()
        {
            settings.navigation.ranked_filter_patch = Some(p.to_string());
        }
        changed = true;
    }

    settings.general.legacy_storage_migrated = true;
    changed || true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AppLocale, AramMode, FlashKeyPreference};

    #[test]
    fn test_legacy_graphics_json_and_local_storage_migration() {
        let dir =
            std::env::temp_dir().join(format!("schumazgg_ini_migrate_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let ini_path = dir.join("UserSettings.ini");
        let legacy_gpu_path = dir.join("graphics_settings.json");
        fs::write(&legacy_gpu_path, r#"{ "hardware_acceleration": false }"#).unwrap();

        let mut settings = load_or_migrate_user_settings(&ini_path, Some(&legacy_gpu_path));
        assert!(!settings.graphics.hardware_acceleration);
        assert!(ini_path.exists());

        let legacy_ls = LegacyLocalStoragePayload {
            schumaz_user_preferences: Some(
                r#"{"autoInjectRunes":false,"autoInjectSpells":true,"autoInjectItems":false,"flashKey":"F","patch48hProtection":false,"autoMarkDailyMissionsRead":true,"language":"pt_BR"}"#
                    .to_string(),
            ),
            schumaz_language: Some("pt_BR".to_string()),
            schumaz_profile_ui_blur: Some("75".to_string()),
            schumaz_profile_wallpaper_visible: Some("false".to_string()),
            schumaz_aram_submode: Some("aram_mayhem".to_string()),
            schumaz_ranked_filters: Some(
                r#"{"region":"br","tier":"diamond_plus","lane":"jungle","patch":"16.21"}"#
                    .to_string(),
            ),
        };

        assert!(apply_legacy_local_storage_migration(
            &mut settings,
            &legacy_ls
        ));
        assert_eq!(settings.general.language, Some(AppLocale::PtBr));
        assert!(settings.general.language_onboarding_completed);
        assert!(settings.general.legacy_storage_migrated);
        assert!(!settings.automation.auto_inject_runes);
        assert_eq!(settings.automation.flash_key, FlashKeyPreference::F);
        assert!(!settings.automation.patch_48h_protection);
        assert!(settings.automation.auto_mark_daily_missions_read);
        assert_eq!(settings.profile_appearance.profile_blur_percent, 75);
        assert!(!settings.profile_appearance.wallpaper_visible);
        assert_eq!(
            settings.navigation.saved_aram_mode,
            Some(AramMode::AramMayhem)
        );
        assert_eq!(settings.navigation.ranked_filter_region, "br");
        assert_eq!(settings.navigation.ranked_filter_tier, "diamond_plus");
        assert_eq!(settings.navigation.ranked_filter_lane, "jungle");
        assert_eq!(
            settings.navigation.ranked_filter_patch.as_deref(),
            Some("16.21")
        );

        // Second migration call is a no-op
        assert!(!apply_legacy_local_storage_migration(
            &mut settings,
            &legacy_ls
        ));

        let _ = fs::remove_dir_all(&dir);
    }
}
