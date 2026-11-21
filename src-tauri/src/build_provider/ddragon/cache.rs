use super::super::items::{get_dynamic_item_classifications, set_dynamic_item_classifications};
use super::super::models::{DataDragonCache, ItemClassificationCache};
use super::augment_interpolate::{has_raw_riot_augment_markup, strip_riot_inline_markup};
use super::fetch::{fetch_ddragon_versions, load_ddragon_bundle_for_version};
use crate::build_provider::analytics::aram::{clean_augment_description, is_internal_augment_key};
use crate::models::AppLocale;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock, RwLock};
use std::time::Instant;

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
    let mut bundle = load_ddragon_bundle_from_dir(dir, locale)?;
    let mut healed_dirty_desc = false;
    for aug in bundle.ddragon.arena_augments.values_mut() {
        if has_raw_riot_augment_markup(&aug.desc) {
            let stripped = strip_riot_inline_markup(&aug.desc, Some(locale));
            aug.desc = clean_augment_description(&stripped);
            healed_dirty_desc = true;
        }
    }
    if healed_dirty_desc {
        let _ = save_ddragon_bundle_to_dir(dir, locale, &bundle);
    }
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

pub async fn load_ddragon_bundle_from_base_url_for_locale(
    client: &reqwest::Client,
    base_url: &str,
    locale: AppLocale,
) -> Result<DataDragonDiskBundle, String> {
    let version = fetch_ddragon_versions(client, base_url).await?;
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
/// or if Arena metadata (`arena_augments` / `map30_items` / `ability_slot`) needs backfilling.
/// Falls back to the existing RAM/disk cache on network errors (*Stale-While-Error*).
pub async fn refresh_ddragon_if_outdated_with_base(
    client: &reqwest::Client,
    base_url: &str,
    disk_dir: Option<&Path>,
    locale: AppLocale,
) -> Result<Arc<DataDragonCache>, String> {
    let started = Instant::now();
    let detail = format!("locale={}", locale.as_str());
    let existing = get_ddragon_cache_store()
        .read()
        .ok()
        .and_then(|g| g.get(&locale).cloned())
        .or_else(|| disk_dir.and_then(|d| hydrate_ddragon_from_dir(d, locale)));

    let version = match fetch_ddragon_versions(client, base_url).await {
        Ok(v) => v,
        Err(err) => {
            if let Some(cached) = existing {
                crate::logging::record_data_resolution(
                    "ddragon",
                    "stale_fallback",
                    started.elapsed().as_millis(),
                    &detail,
                );
                return Ok(cached);
            }
            return Err(err);
        }
    };

    let needs_arena_backfill = base_url == DDRAGON_BASE_URL
        && existing.as_ref().is_some_and(|c| {
            c.arena_augments.is_empty()
                || c.arena_augments.values().any(|a| {
                    is_internal_augment_key(&a.name) || has_raw_riot_augment_markup(&a.desc)
                })
                || !c.arena_augments.values().any(|a| a.ability_slot.is_some())
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
            crate::logging::record_data_resolution(
                "ddragon",
                "network",
                started.elapsed().as_millis(),
                &format!("{detail} version={version}"),
            );
            Ok(arc)
        }
        Err(err) => {
            if let Some(cached) = existing {
                crate::logging::record_data_resolution(
                    "ddragon",
                    "stale_fallback",
                    started.elapsed().as_millis(),
                    &detail,
                );
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
    let started = Instant::now();
    let detail = format!("locale={}", locale.as_str());
    if let Ok(guard) = get_ddragon_cache_store().read()
        && let Some(cached) = guard.get(&locale)
    {
        crate::logging::metrics::record_cache_ram_hit();
        return Ok(Arc::clone(cached));
    }

    let _init_guard = get_ddragon_fetch_mutex().lock().await;
    if let Ok(guard) = get_ddragon_cache_store().read()
        && let Some(cached) = guard.get(&locale)
    {
        crate::logging::metrics::record_cache_ram_hit();
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
        crate::logging::record_data_resolution(
            "ddragon",
            "disk_hit",
            started.elapsed().as_millis(),
            &detail,
        );
        return Ok(disk_cached);
    }

    let bundle =
        load_ddragon_bundle_from_base_url_for_locale(client, DDRAGON_BASE_URL, locale).await?;
    if let Some(paths) = crate::storage::global_paths() {
        let _ = save_ddragon_bundle_to_dir(&paths.ddragon_dir, locale, &bundle);
    }
    let version = bundle.ddragon.version.clone();
    let loaded = Arc::new(bundle.ddragon);
    if let Ok(mut guard) = get_ddragon_cache_store().write() {
        guard.insert(locale, Arc::clone(&loaded));
    }
    crate::logging::record_data_resolution(
        "ddragon",
        "network",
        started.elapsed().as_millis(),
        &format!("{detail} version={version}"),
    );
    Ok(loaded)
}

/// Gets the cached DataDragon data for the active user locale (defaults to PT-BR).
pub async fn get_or_init_ddragon_cache(
    client: &reqwest::Client,
) -> Result<Arc<DataDragonCache>, String> {
    let locale = crate::lcu_injector::get_active_locale();
    get_or_init_ddragon_cache_for_locale(client, locale).await
}
