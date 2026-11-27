use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{OnceLock, RwLock};
use std::time::Duration;

use crate::build_provider::DDRAGON_BASE_URL;
use crate::models::{AppLocale, DataDragonCache};

static SKIN_NAME_CACHE: OnceLock<RwLock<HashMap<AppLocale, HashMap<i32, String>>>> =
    OnceLock::new();
static HYDRATED_SKIN_DIRS: OnceLock<RwLock<HashSet<(PathBuf, AppLocale)>>> = OnceLock::new();

pub(crate) fn skin_cache() -> &'static RwLock<HashMap<AppLocale, HashMap<i32, String>>> {
    SKIN_NAME_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

fn hydrated_skin_dirs() -> &'static RwLock<HashSet<(PathBuf, AppLocale)>> {
    HYDRATED_SKIN_DIRS.get_or_init(|| RwLock::new(HashSet::new()))
}

pub fn skins_file_path(dir: &Path, locale: AppLocale) -> PathBuf {
    dir.join(format!("skins_{}.lz4", locale.as_str()))
}

pub fn prewarm_skins_from_dir(dir: &Path, locale: AppLocale) -> usize {
    let path = skins_file_path(dir, locale);
    let loaded =
        crate::storage::read_lz4_envelope::<HashMap<i32, String>>(&path).map(|env| env.payload);
    let count = loaded.as_ref().map_or(0, HashMap::len);
    if let Ok(mut guard) = skin_cache().write() {
        let entry = guard.entry(locale).or_default();
        if let Some(payload) = loaded {
            for (k, v) in payload {
                entry.entry(k).or_insert(v);
            }
        }
    }
    if let Ok(mut dirs) = hydrated_skin_dirs().write() {
        dirs.insert((dir.to_path_buf(), locale));
    }
    count
}

fn is_skins_hydrated_for_dir(dir: &Path, locale: AppLocale) -> bool {
    let cache_has_locale = skin_cache()
        .read()
        .ok()
        .is_some_and(|g| g.contains_key(&locale));
    cache_has_locale
        && hydrated_skin_dirs()
            .read()
            .ok()
            .is_some_and(|s| s.contains(&(dir.to_path_buf(), locale)))
}

/// Hydrates `skins_{locale}.lz4` from `dir` into RAM at most once per `(dir, locale)`
/// while `locale` remains present in `SKIN_NAME_CACHE`.
pub fn ensure_skins_hydrated_from_dir(dir: &Path, locale: AppLocale) {
    if is_skins_hydrated_for_dir(dir, locale) {
        return;
    }
    let _ = prewarm_skins_from_dir(dir, locale);
}

pub fn prewarm_skins_from_disk(locale: AppLocale) -> usize {
    let Some(paths) = crate::storage::global_paths() else {
        return 0;
    };
    prewarm_skins_from_dir(&paths.ddragon_dir, locale)
}

pub fn lookup_cached_skin_name(locale: AppLocale, background_skin_id: i32) -> Option<String> {
    if background_skin_id <= 0 {
        return None;
    }
    if let Ok(guard) = skin_cache().read()
        && let Some(locale_map) = guard.get(&locale)
        && let Some(name) = locale_map.get(&background_skin_id)
    {
        return Some(name.clone());
    }
    None
}

fn save_skins_map_to_dir(dir: &Path, locale: AppLocale, version: &str) {
    let snapshot = skin_cache()
        .read()
        .ok()
        .and_then(|g| g.get(&locale).cloned());
    if let Some(map) = snapshot {
        let path = skins_file_path(dir, locale);
        let _ = crate::storage::write_lz4_envelope(&path, version, &map);
    }
}

/// Extracts `backgroundSkinId` from `/lol-summoner/v1/current-summoner/summoner-profile`.
pub fn extract_background_skin_id(profile_json: &serde_json::Value) -> i32 {
    profile_json
        .get("backgroundSkinId")
        .and_then(|v| v.as_i64())
        .map(|v| v as i32)
        .unwrap_or(0)
}

pub fn resolve_background_splash_urls_for_locale(
    background_skin_id: i32,
    cache: &DataDragonCache,
    locale: AppLocale,
) -> (String, String, String, String, i32) {
    if background_skin_id <= 0 {
        return (
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            0,
        );
    }

    let champion_id = background_skin_id / 1000;
    let skin_num = background_skin_id % 1000;

    if let Some(champ) = cache.champions.get(&champion_id) {
        let splash_url = format!(
            "https://ddragon.leagueoflegends.com/cdn/img/champion/splash/{}_{}.jpg",
            champ.key_name, skin_num
        );
        let fallback_url = format!(
            "https://ddragon.leagueoflegends.com/cdn/img/champion/splash/{}_0.jpg",
            champ.key_name
        );
        let classic_suffix = match locale {
            AppLocale::PtBr => "Clássica",
            AppLocale::EnUs => "Classic",
        };
        let default_label = if skin_num == 0 {
            format!("{} ({})", champ.display_name, classic_suffix)
        } else {
            format!("{} #{}", champ.display_name, skin_num)
        };
        (
            splash_url,
            fallback_url,
            default_label,
            champ.key_name.clone(),
            skin_num,
        )
    } else {
        (
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            skin_num,
        )
    }
}

/// Resolves DataDragon splash URLs and default label from a `backgroundSkinId` (`championId * 1000 + skinNum`).
pub fn resolve_background_splash_urls(
    background_skin_id: i32,
    cache: &DataDragonCache,
) -> (String, String, String, String, i32) {
    resolve_background_splash_urls_for_locale(
        background_skin_id,
        cache,
        crate::lcu_injector::get_active_locale(),
    )
}

#[allow(clippy::too_many_arguments)]
pub async fn fetch_skin_name_for_locale_with_base(
    client: &reqwest::Client,
    base_url: &str,
    version: &str,
    key_name: &str,
    skin_num: i32,
    background_skin_id: i32,
    locale: AppLocale,
    disk_dir: Option<&Path>,
) -> Option<String> {
    if version.is_empty() || key_name.is_empty() || background_skin_id <= 0 {
        return None;
    }

    if let Some(cached) = lookup_cached_skin_name(locale, background_skin_id) {
        return Some(cached);
    }

    if let Some(dir) = disk_dir {
        if !is_skins_hydrated_for_dir(dir, locale) {
            let dir_buf = dir.to_path_buf();
            let _ = tokio::task::spawn_blocking(move || {
                ensure_skins_hydrated_from_dir(&dir_buf, locale);
            })
            .await;
        }
        if let Some(cached) = lookup_cached_skin_name(locale, background_skin_id) {
            return Some(cached);
        }
    }

    let url = format!(
        "{}/cdn/{}/data/{}/champion/{}.json",
        base_url.trim_end_matches('/'),
        version,
        locale.ddragon_code(),
        key_name
    );

    let res = tokio::time::timeout(Duration::from_millis(1800), client.get(&url).send())
        .await
        .ok()?
        .ok()?;
    if !res.status().is_success() {
        return None;
    }

    let json = res.json::<serde_json::Value>().await.ok()?;
    let skins = json
        .pointer(&format!("/data/{}/skins", key_name))
        .and_then(|v| v.as_array())?;

    let matched = skins.iter().find(|skin| {
        let num = skin.get("num").and_then(|v| v.as_i64()).map(|n| n as i32);
        let id = skin.get("id").and_then(|v| v.as_i64()).map(|n| n as i32);
        num == Some(skin_num) || id == Some(background_skin_id)
    })?;

    let raw_name = matched.get("name").and_then(|v| v.as_str())?;
    let champ_name = json
        .pointer(&format!("/data/{}/name", key_name))
        .and_then(|v| v.as_str())
        .unwrap_or(key_name);

    let formatted = if raw_name.eq_ignore_ascii_case("default") {
        let classic = match locale {
            AppLocale::PtBr => "Clássica",
            AppLocale::EnUs => "Classic",
        };
        format!("{} ({})", champ_name, classic)
    } else {
        raw_name.to_string()
    };

    if let Ok(mut guard) = skin_cache().write() {
        guard
            .entry(locale)
            .or_default()
            .insert(background_skin_id, formatted.clone());
    }
    if let Some(dir) = disk_dir {
        let dir_buf = dir.to_path_buf();
        let ver = version.to_string();
        let _ = tokio::task::spawn_blocking(move || {
            save_skins_map_to_dir(&dir_buf, locale, &ver);
        })
        .await;
    }

    Some(formatted)
}

/// Fetches the localized skin name from DataDragon `champion/{key_name}.json` with caching and short timeout.
pub async fn fetch_skin_name_for_locale(
    client: &reqwest::Client,
    version: &str,
    key_name: &str,
    skin_num: i32,
    background_skin_id: i32,
    locale: AppLocale,
) -> Option<String> {
    let dir = crate::storage::global_paths().map(|p| p.ddragon_dir.as_path());
    fetch_skin_name_for_locale_with_base(
        client,
        DDRAGON_BASE_URL,
        version,
        key_name,
        skin_num,
        background_skin_id,
        locale,
        dir,
    )
    .await
}

pub async fn fetch_skin_name(
    client: &reqwest::Client,
    version: &str,
    key_name: &str,
    skin_num: i32,
    background_skin_id: i32,
) -> Option<String> {
    fetch_skin_name_for_locale(
        client,
        version,
        key_name,
        skin_num,
        background_skin_id,
        crate::lcu_injector::get_active_locale(),
    )
    .await
}

#[cfg(test)]
#[path = "skins_tests.rs"]
mod tests;
