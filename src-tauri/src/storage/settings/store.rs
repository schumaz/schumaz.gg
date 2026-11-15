use std::path::{Path, PathBuf};
use std::sync::{OnceLock, RwLock};

use super::super::paths::atomic_write;
use super::ini_serializer::serialize_user_settings_ini;
use super::{
    AppSettings, NavigationState, NavigationStatePatch, validate_champion_tier_filter,
    validate_ranked_lane, validate_rarity_filter,
};

pub fn save_user_settings_to_path(path: &Path, settings: &AppSettings) -> Result<(), String> {
    let serialized = serialize_user_settings_ini(settings);
    atomic_write(path, serialized.as_bytes())
}

pub fn apply_navigation_patch(nav: &mut NavigationState, patch: NavigationStatePatch) {
    if let Some(mode) = patch.saved_aram_mode {
        nav.saved_aram_mode = Some(mode);
    }
    if let Some(mode_filter) = patch.profile_match_mode_filter {
        let trimmed = mode_filter.trim();
        nav.profile_match_mode_filter = if trimmed.is_empty() {
            "all".to_string()
        } else {
            trimmed.to_string()
        };
    }
    if let Some(rarity) = patch.aram_mayhem_rarity_filter {
        nav.aram_mayhem_rarity_filter = validate_rarity_filter(&rarity);
    }
    if let Some(champ) = patch.aram_mayhem_last_champion {
        let trimmed = champ.trim().to_ascii_lowercase();
        nav.aram_mayhem_last_champion = (!trimmed.is_empty()).then_some(trimmed);
    }
    if let Some(region) = patch.ranked_filter_region {
        let trimmed = region.trim();
        if !trimmed.is_empty() {
            nav.ranked_filter_region = trimmed.to_string();
        }
    }
    if let Some(tier) = patch.ranked_filter_tier {
        let trimmed = tier.trim();
        if !trimmed.is_empty() {
            nav.ranked_filter_tier = trimmed.to_string();
        }
    }
    if let Some(lane) = patch.ranked_filter_lane {
        nav.ranked_filter_lane = validate_ranked_lane(&lane);
    }
    if let Some(p) = patch.ranked_filter_patch {
        let trimmed = p.trim();
        nav.ranked_filter_patch = (!trimmed.is_empty()).then(|| trimmed.to_string());
    }
    if let Some(ct) = patch.ranked_champion_tier_filter {
        nav.ranked_champion_tier_filter = validate_champion_tier_filter(&ct);
    }
    if let Some(id) = patch.last_summoner_id
        && id > 0
    {
        nav.last_summoner_id = Some(id);
    }
}

#[derive(Default)]
struct SettingsStore {
    settings: AppSettings,
    ini_path: Option<PathBuf>,
}

static SETTINGS_STORE: OnceLock<RwLock<SettingsStore>> = OnceLock::new();

fn settings_lock() -> &'static RwLock<SettingsStore> {
    SETTINGS_STORE.get_or_init(|| RwLock::new(SettingsStore::default()))
}

pub fn init_global_settings(ini_path: PathBuf, settings: AppSettings) {
    if let Ok(mut guard) = settings_lock().write() {
        guard.ini_path = Some(ini_path);
        guard.settings = settings;
    }
    if let Some(lang) = get_global_settings().general.language {
        let mut prefs = get_global_settings().automation;
        prefs.language = lang;
        crate::lcu_injector::set_user_preferences(prefs);
    } else {
        crate::lcu_injector::set_user_preferences(get_global_settings().automation);
    }
}

pub fn get_global_settings() -> AppSettings {
    settings_lock()
        .read()
        .map(|g| g.settings.clone())
        .unwrap_or_default()
}

pub fn update_global_settings<F>(mutator: F) -> Result<AppSettings, String>
where
    F: FnOnce(&mut AppSettings),
{
    let (updated, path_opt) = {
        let mut guard = settings_lock()
            .write()
            .map_err(|_| "Settings lock poisoned".to_string())?;
        mutator(&mut guard.settings);
        (guard.settings.clone(), guard.ini_path.clone())
    };

    if let Some(path) = path_opt {
        save_user_settings_to_path(&path, &updated)?;
    }

    Ok(updated)
}
