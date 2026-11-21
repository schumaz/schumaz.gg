use super::super::client::peek_latest_cached_opgg_version;
use super::{OPGG_ARAM_MAYHEM_WEB_URL, OPGG_ARAM_REST_URL};
use crate::build_provider::models::DataDragonCache;
use crate::models::{AppLocale, AramAugmentEntry, AramChampionRank, AramMode};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{OnceLock, RwLock};
use std::time::{Duration, Instant};

pub const ARAM_CACHE_TTL: Duration = Duration::from_secs(900);

pub type TierCacheMap = HashMap<String, (Instant, Vec<AramChampionRank>)>;
pub type AugmentsCacheMap = HashMap<String, (Instant, Vec<AramAugmentEntry>)>;

static ARAM_TIER_CACHE: OnceLock<RwLock<TierCacheMap>> = OnceLock::new();
static ARAM_AUGMENTS_CACHE: OnceLock<RwLock<AugmentsCacheMap>> = OnceLock::new();

pub(crate) fn get_tier_cache() -> &'static RwLock<TierCacheMap> {
    ARAM_TIER_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

pub(crate) fn get_augments_cache() -> &'static RwLock<AugmentsCacheMap> {
    ARAM_AUGMENTS_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

pub fn aram_tier_file_path(dir: &Path, mode: AramMode, locale: AppLocale) -> PathBuf {
    let mode_key = match mode {
        AramMode::Aram => "aram",
        AramMode::AramMayhem => "aram_mayhem",
    };
    dir.join(format!("tier_{}_{}.lz4", mode_key, locale.as_str()))
}

pub fn aram_augments_file_path(dir: &Path, locale: AppLocale, champ_key: &str) -> PathBuf {
    let clean = crate::storage::sanitize_key_component(champ_key);
    dir.join(format!("augments_{}_{}.lz4", locale.as_str(), clean))
}

pub(crate) fn rehydrate_aram_names(list: &mut [AramChampionRank], ddragon: &DataDragonCache) {
    for entry in list {
        if let Some(meta) = ddragon.champions.get(&entry.id) {
            if !meta.display_name.is_empty() {
                entry.name = meta.display_name.clone();
            }
            entry.icon_url = meta.icon_url.clone();
        } else {
            entry.icon_url.clear();
        }
    }
}

/// Pre-warms valid (< 48h) ARAM and ARAM Mayhem tier lists from `.lz4` disk into RAM on startup.
pub fn prewarm_aram_tier_lists_from_dir(
    dir: &Path,
    locale: AppLocale,
    ddragon: &DataDragonCache,
) -> usize {
    let now = crate::storage::now_unix_secs();
    let known_latest = peek_latest_cached_opgg_version("global");
    let mut warmed = 0;

    for (mode, url_key_prefix) in [
        (
            AramMode::Aram,
            format!("aram:{}:{}", locale.as_str(), OPGG_ARAM_REST_URL),
        ),
        (
            AramMode::AramMayhem,
            format!("mayhem:{}:{}", locale.as_str(), OPGG_ARAM_MAYHEM_WEB_URL),
        ),
    ] {
        let path = aram_tier_file_path(dir, mode, locale);
        let Some(mut env) = crate::storage::read_lz4_envelope::<Vec<AramChampionRank>>(&path)
        else {
            continue;
        };
        if !crate::storage::is_envelope_fresh(
            env.created_at_unix,
            crate::storage::OPGG_CACHE_TTL_SECS,
            now,
        ) {
            continue;
        }
        if let Some(ref latest) = known_latest
            && !env.patch.is_empty()
            && &env.patch != latest
        {
            continue;
        }
        rehydrate_aram_names(&mut env.payload, ddragon);
        if let Ok(mut guard) = get_tier_cache().write() {
            guard.insert(url_key_prefix, (Instant::now(), env.payload));
            warmed += 1;
        }
    }

    warmed
}

pub fn aram_mayhem_augments_web_base(locale: AppLocale) -> &'static str {
    locale.opgg_aram_mayhem_augments_base()
}
