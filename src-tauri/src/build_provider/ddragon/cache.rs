use super::super::items::{get_dynamic_item_classifications, set_dynamic_item_classifications};
use super::super::models::{DataDragonCache, ItemClassificationCache};
use super::augments::fetch_arena_augments_catalog;
use super::parsers::{
    parse_all_ddragon_champions, parse_ddragon_item_classifications, parse_ddragon_runes,
    parse_ddragon_summoners, parse_latest_ddragon_version,
};
use crate::models::AppLocale;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock, RwLock};

pub const DDRAGON_BASE_URL: &str = "https://ddragon.leagueoflegends.com";

pub static DDRAGON_CACHE: OnceLock<RwLock<HashMap<AppLocale, Arc<DataDragonCache>>>> =
    OnceLock::new();
static DDRAGON_FETCH_MUTEX: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
static DDRAGON_BG_REFRESHED: OnceLock<RwLock<HashSet<AppLocale>>> = OnceLock::new();

pub fn get_ddragon_cache_store() -> &'static RwLock<HashMap<AppLocale, Arc<DataDragonCache>>> {
    DDRAGON_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

fn get_ddragon_fetch_mutex() -> &'static tokio::sync::Mutex<()> {
    DDRAGON_FETCH_MUTEX.get_or_init(|| tokio::sync::Mutex::new(()))
}

fn get_bg_refreshed_locales() -> &'static RwLock<HashSet<AppLocale>> {
    DDRAGON_BG_REFRESHED.get_or_init(|| RwLock::new(HashSet::new()))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataDragonDiskBundle {
    pub ddragon: DataDragonCache,
    #[serde(default)]
    pub item_classifications: Option<ItemClassificationCache>,
}

pub fn ddragon_metadata_file_path(dir: &Path, locale: AppLocale) -> PathBuf {
    dir.join(format!("metadata_{}.lz4", locale.as_str()))
}

pub fn save_ddragon_bundle_to_dir(
    dir: &Path,
    locale: AppLocale,
    bundle: &DataDragonDiskBundle,
) -> Result<(), String> {
    let path = ddragon_metadata_file_path(dir, locale);
    crate::storage::write_lz4_envelope(&path, &bundle.ddragon.version, bundle)
}

pub fn load_ddragon_bundle_from_dir(dir: &Path, locale: AppLocale) -> Option<DataDragonDiskBundle> {
    let path = ddragon_metadata_file_path(dir, locale);
    crate::storage::read_lz4_envelope::<DataDragonDiskBundle>(&path).map(|env| env.payload)
}

pub fn hydrate_ddragon_from_dir(dir: &Path, locale: AppLocale) -> Option<Arc<DataDragonCache>> {
    let bundle = load_ddragon_bundle_from_dir(dir, locale)?;
    if let Some(classifications) = bundle.item_classifications {
        set_dynamic_item_classifications(classifications);
    }
    let arc = Arc::new(bundle.ddragon);
    if let Ok(mut guard) = get_ddragon_cache_store().write() {
        guard.insert(locale, Arc::clone(&arc));
    }
    Some(arc)
}

pub fn prewarm_ddragon_from_disk(locale: AppLocale) -> Option<Arc<DataDragonCache>> {
    let paths = crate::storage::global_paths()?;
    hydrate_ddragon_from_dir(&paths.ddragon_dir, locale)
}

async fn load_ddragon_bundle_for_version(
    client: &reqwest::Client,
    base_url: &str,
    version: &str,
    locale: AppLocale,
) -> Result<DataDragonDiskBundle, String> {
    let dd_locale = locale.ddragon_code();

    let champions_url = format!(
        "{}/cdn/{}/data/{}/champion.json",
        base_url, version, dd_locale
    );
    let runes_url = format!(
        "{}/cdn/{}/data/{}/runesReforged.json",
        base_url, version, dd_locale
    );
    let items_url = format!("{}/cdn/{}/data/{}/item.json", base_url, version, dd_locale);
    let summoners_url = format!(
        "{}/cdn/{}/data/{}/summoner.json",
        base_url, version, dd_locale
    );

    let champions_fut = async {
        client
            .get(&champions_url)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch champion.json: {}", e))?
            .json::<serde_json::Value>()
            .await
            .map_err(|e| format!("Failed to parse champion.json: {}", e))
    };

    let runes_fut = async {
        client
            .get(&runes_url)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch runesReforged.json: {}", e))?
            .json::<serde_json::Value>()
            .await
            .map_err(|e| format!("Failed to parse runesReforged.json: {}", e))
    };

    let items_fut = async {
        if let Ok(res) = client.get(&items_url).send().await
            && res.status().is_success()
            && let Ok(items_res) = res.json::<serde_json::Value>().await
        {
            let classification = parse_ddragon_item_classifications(&items_res);
            set_dynamic_item_classifications(classification.clone());
            return Some(classification);
        }
        None
    };

    let summoners_fut = async {
        if let Ok(res) = client.get(&summoners_url).send().await
            && res.status().is_success()
            && let Ok(summoners_res) = res.json::<serde_json::Value>().await
            && let Ok(parsed_spells) = parse_ddragon_summoners(&summoners_res, version)
        {
            return parsed_spells;
        }
        HashMap::new()
    };

    let arena_augments_fut = fetch_arena_augments_catalog(client, base_url, locale);

    let (champions_res, runes_res, item_classifications, spells, arena_augments) = tokio::join!(
        champions_fut,
        runes_fut,
        items_fut,
        summoners_fut,
        arena_augments_fut
    );

    let champions = parse_all_ddragon_champions(&champions_res?, version)?;
    let runes = parse_ddragon_runes(&runes_res?)?;

    Ok(DataDragonDiskBundle {
        ddragon: DataDragonCache {
            version: version.to_string(),
            champions,
            runes,
            spells,
            arena_augments,
        },
        item_classifications,
    })
}

pub async fn load_ddragon_bundle_from_base_url_for_locale(
    client: &reqwest::Client,
    base_url: &str,
    locale: AppLocale,
) -> Result<DataDragonDiskBundle, String> {
    let versions_url = format!("{}/api/versions.json", base_url);
    let versions_res: serde_json::Value = client
        .get(&versions_url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch versions.json: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Failed to parse versions.json: {}", e))?;

    let version = parse_latest_ddragon_version(&versions_res)?;
    load_ddragon_bundle_for_version(client, base_url, &version, locale).await
}

/// Loads the DataDragonCache for a specific `locale` from a base URL (mockable in tests).
pub async fn load_ddragon_cache_from_base_url_for_locale(
    client: &reqwest::Client,
    base_url: &str,
    locale: AppLocale,
) -> Result<DataDragonCache, String> {
    let bundle = load_ddragon_bundle_from_base_url_for_locale(client, base_url, locale).await?;
    Ok(bundle.ddragon)
}

/// Loads the DataDragonCache from a specific base URL (PT-BR default for backwards compatibility).
pub async fn load_ddragon_cache_from_base_url(
    client: &reqwest::Client,
    base_url: &str,
) -> Result<DataDragonCache, String> {
    load_ddragon_cache_from_base_url_for_locale(client, base_url, AppLocale::PtBr).await
}

/// Checks `/api/versions.json` and refreshes both RAM and `metadata_{locale}.lz4` if a newer version exists
/// or if Arena metadata (`arena_augments` / `map30_items`) is absent in an older cached bundle.
/// Falls back to the existing RAM/disk cache on network errors (*Stale-While-Error*).
pub async fn refresh_ddragon_if_outdated_with_base(
    client: &reqwest::Client,
    base_url: &str,
    disk_dir: Option<&Path>,
    locale: AppLocale,
) -> Result<Arc<DataDragonCache>, String> {
    let existing = get_ddragon_cache_store()
        .read()
        .ok()
        .and_then(|g| g.get(&locale).cloned())
        .or_else(|| disk_dir.and_then(|d| hydrate_ddragon_from_dir(d, locale)));

    let versions_url = format!("{}/api/versions.json", base_url);
    let latest_version = match client.get(&versions_url).send().await {
        Ok(res) => match res.json::<serde_json::Value>().await {
            Ok(json) => parse_latest_ddragon_version(&json),
            Err(e) => Err(format!("Failed to parse versions.json: {e}")),
        },
        Err(e) => Err(format!("Failed to fetch versions.json: {e}")),
    };

    let version = match latest_version {
        Ok(v) => v,
        Err(err) => {
            if let Some(cached) = existing {
                return Ok(cached);
            }
            return Err(err);
        }
    };

    let needs_arena_backfill = base_url == DDRAGON_BASE_URL
        && existing.as_ref().is_some_and(|c| {
            c.arena_augments.is_empty()
                || get_dynamic_item_classifications().is_some_and(|cls| cls.map30_items.is_empty())
        });

    if let Some(ref cached) = existing
        && cached.version == version
        && !needs_arena_backfill
    {
        return Ok(Arc::clone(cached));
    }

    match load_ddragon_bundle_for_version(client, base_url, &version, locale).await {
        Ok(bundle) => {
            if let Some(dir) = disk_dir {
                let _ = save_ddragon_bundle_to_dir(dir, locale, &bundle);
            }
            let arc = Arc::new(bundle.ddragon);
            if let Ok(mut guard) = get_ddragon_cache_store().write() {
                guard.insert(locale, Arc::clone(&arc));
            }
            Ok(arc)
        }
        Err(err) => {
            if let Some(cached) = existing {
                Ok(cached)
            } else {
                Err(err)
            }
        }
    }
}

pub async fn refresh_ddragon_if_outdated(
    client: &reqwest::Client,
    locale: AppLocale,
) -> Result<Arc<DataDragonCache>, String> {
    let dir = crate::storage::global_paths().map(|p| p.ddragon_dir.as_path());
    refresh_ddragon_if_outdated_with_base(client, DDRAGON_BASE_URL, dir, locale).await
}

/// Gets the cached DataDragon data for `locale` (RAM -> `.lz4` disk -> Riot CDN).
/// When hydrated from disk, triggers a single background version check without blocking UI/Champ Select.
pub async fn get_or_init_ddragon_cache_for_locale(
    client: &reqwest::Client,
    locale: AppLocale,
) -> Result<Arc<DataDragonCache>, String> {
    if let Ok(guard) = get_ddragon_cache_store().read()
        && let Some(cached) = guard.get(&locale)
    {
        return Ok(Arc::clone(cached));
    }

    let _init_guard = get_ddragon_fetch_mutex().lock().await;
    if let Ok(guard) = get_ddragon_cache_store().read()
        && let Some(cached) = guard.get(&locale)
    {
        return Ok(Arc::clone(cached));
    }

    if let Some(disk_cached) = prewarm_ddragon_from_disk(locale) {
        let should_bg_check = get_bg_refreshed_locales()
            .write()
            .map(|mut set| set.insert(locale))
            .unwrap_or(false);
        if should_bg_check {
            let bg_client = client.clone();
            tokio::spawn(async move {
                let _ = refresh_ddragon_if_outdated(&bg_client, locale).await;
            });
        }
        return Ok(disk_cached);
    }

    let bundle =
        load_ddragon_bundle_from_base_url_for_locale(client, DDRAGON_BASE_URL, locale).await?;
    if let Some(paths) = crate::storage::global_paths() {
        let _ = save_ddragon_bundle_to_dir(&paths.ddragon_dir, locale, &bundle);
    }
    let loaded = Arc::new(bundle.ddragon);
    if let Ok(mut guard) = get_ddragon_cache_store().write() {
        guard.insert(locale, Arc::clone(&loaded));
    }
    Ok(loaded)
}

/// Gets the cached DataDragon data for the currently active locale or initializes it if not yet loaded.
pub async fn get_or_init_ddragon_cache(
    client: &reqwest::Client,
) -> Result<Arc<DataDragonCache>, String> {
    let locale = crate::lcu_injector::get_active_locale();
    get_or_init_ddragon_cache_for_locale(client, locale).await
}
