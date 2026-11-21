use super::super::client::peek_latest_cached_opgg_version;
use super::html_extract::{
    has_unresolved_augment_placeholder, is_internal_augment_key, sanitize_augment_placeholders,
};
use super::{OPGG_ARAM_MAYHEM_WEB_URL, OPGG_ARAM_REST_URL};
use crate::build_provider::models::DataDragonCache;
use crate::models::{AppLocale, AramAugmentEntry, AramChampionRank, AramMode};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{OnceLock, RwLock};
use std::time::{Duration, Instant};

pub const ARAM_CACHE_TTL: Duration = Duration::from_secs(900);
/// Extended RAM TTL (60 min) for ARAM Mayhem tier list and augments (still invalidated on patch change).
pub const ARAM_MAYHEM_CACHE_TTL: Duration = Duration::from_secs(60 * 60);

#[derive(Debug, Clone)]
pub struct CachedAramEntry<T> {
    pub cached_at: Instant,
    pub patch: String,
    pub payload: T,
}

pub type TierCacheMap = HashMap<String, CachedAramEntry<Vec<AramChampionRank>>>;
pub type AugmentsCacheMap = HashMap<String, CachedAramEntry<Vec<AramAugmentEntry>>>;

static ARAM_TIER_CACHE: OnceLock<RwLock<TierCacheMap>> = OnceLock::new();
static ARAM_AUGMENTS_CACHE: OnceLock<RwLock<AugmentsCacheMap>> = OnceLock::new();

pub(crate) fn get_tier_cache() -> &'static RwLock<TierCacheMap> {
    ARAM_TIER_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

pub(crate) fn get_augments_cache() -> &'static RwLock<AugmentsCacheMap> {
    ARAM_AUGMENTS_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

pub(crate) fn aram_ram_ttl(mode: AramMode) -> Duration {
    match mode {
        AramMode::Aram => ARAM_CACHE_TTL,
        AramMode::AramMayhem => ARAM_MAYHEM_CACHE_TTL,
    }
}

pub(crate) fn aram_disk_ttl_secs(mode: AramMode) -> u64 {
    match mode {
        AramMode::Aram => crate::storage::OPGG_CACHE_TTL_SECS,
        AramMode::AramMayhem => crate::storage::ARAM_MAYHEM_DISK_TTL_SECS,
    }
}

/// Returns `true` when `cached_patch` is still valid against `known_latest` (invalidating immediately on patch change).
pub(crate) fn is_aram_patch_compatible(cached_patch: &str, known_latest: Option<&str>) -> bool {
    match known_latest {
        Some(latest) if !cached_patch.is_empty() => cached_patch == latest,
        _ => true,
    }
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

pub(crate) fn has_any_augment_description(list: &[AramAugmentEntry]) -> bool {
    list.iter().any(|entry| !entry.desc.trim().is_empty())
}

/// Returns `true` when cached ARAM augments have populated, clean descriptions
/// (no blank-desc cache, no unresolved `?` placeholders, and no raw internal keys).
pub(crate) fn has_valid_augment_descriptions(list: &[AramAugmentEntry]) -> bool {
    has_any_augment_description(list)
        && !list.iter().any(|entry| {
            has_unresolved_augment_placeholder(&entry.desc) || is_internal_augment_key(&entry.name)
        })
}

/// Best-effort sanitization for offline `stale_fallback` entries so raw `?` placeholders
/// and internal Riot keys are cleaned even when network refresh is unavailable.
pub(crate) fn sanitize_stale_augment_entries(list: &mut Vec<AramAugmentEntry>, locale: AppLocale) {
    list.retain_mut(|entry| {
        if is_internal_augment_key(&entry.name) {
            return false;
        }
        if has_unresolved_augment_placeholder(&entry.desc) {
            entry.desc = sanitize_augment_placeholders(&entry.desc, locale);
        }
        true
    });
}

pub(crate) fn rehydrate_aram_augments(list: &mut Vec<AramAugmentEntry>, ddragon: &DataDragonCache) {
    if ddragon.arena_augments.is_empty() {
        list.retain_mut(|entry| {
            if is_internal_augment_key(&entry.name) {
                return false;
            }
            let trimmed = entry.icon_url.trim();
            let is_official = trimmed.starts_with("https://raw.communitydragon.org/")
                || trimmed.starts_with("https://ddragon.leagueoflegends.com/");
            if !is_official {
                entry.icon_url.clear();
            }
            true
        });
        return;
    }

    list.retain_mut(|entry| {
        let Some(meta) = ddragon.arena_augments.get(&entry.id) else {
            return false;
        };
        let name = meta.name.trim();
        if name.is_empty() || is_internal_augment_key(name) {
            return false;
        }
        entry.name = name.to_string();
        if !meta.desc.trim().is_empty() {
            entry.desc = meta.desc.clone();
        }
        entry.icon_url = meta.icon_url.clone();
        entry.rarity = (meta.rarity > 0).then_some(meta.rarity);
        true
    });
}

/// Pre-warms valid (< 48h Classic / < 72h Mayhem, current patch) ARAM tier lists from `.lz4` disk into RAM on startup.
pub fn prewarm_aram_tier_lists_from_dir(
    dir: &Path,
    locale: AppLocale,
    ddragon: &DataDragonCache,
) -> usize {
    let now = crate::storage::now_unix_secs();
    let known_latest = peek_latest_cached_opgg_version("global");
    prewarm_aram_tier_lists_from_dir_at(dir, locale, ddragon, now, known_latest.as_deref())
}

pub(crate) fn prewarm_aram_tier_lists_from_dir_at(
    dir: &Path,
    locale: AppLocale,
    ddragon: &DataDragonCache,
    now_unix: u64,
    known_latest: Option<&str>,
) -> usize {
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
            aram_disk_ttl_secs(mode),
            now_unix,
        ) {
            continue;
        }
        if !is_aram_patch_compatible(&env.patch, known_latest) {
            continue;
        }
        rehydrate_aram_names(&mut env.payload, ddragon);
        if let Ok(mut guard) = get_tier_cache().write() {
            guard.insert(
                url_key_prefix,
                CachedAramEntry {
                    cached_at: Instant::now(),
                    patch: env.patch,
                    payload: env.payload,
                },
            );
            warmed += 1;
        }
    }

    warmed
}

pub fn aram_mayhem_augments_web_base(locale: AppLocale) -> &'static str {
    locale.opgg_aram_mayhem_augments_base()
}
