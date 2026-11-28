use super::super::client::peek_latest_cached_opgg_version;
use super::html_extract::{
    clean_augment_description, has_unresolved_augment_placeholder, is_internal_augment_key,
    sanitize_augment_placeholders,
};
use super::{OPGG_ARAM_MAYHEM_WEB_URL, OPGG_ARAM_REST_URL};
use crate::build_provider::cache::build_provider_caches;
use crate::build_provider::ddragon::{has_raw_riot_augment_markup, strip_riot_inline_markup};
use crate::build_provider::models::DataDragonCache;
use crate::models::{AppLocale, AramAugmentEntry, AramChampionRank, AramMode};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const ARAM_CACHE_TTL: Duration = Duration::from_secs(900);
/// Extended RAM TTL (60 min) for ARAM Mayhem tier list and augments (still invalidated on patch change).
pub const ARAM_MAYHEM_CACHE_TTL: Duration = Duration::from_secs(60 * 60);

pub(crate) fn aram_tier_cache_key(
    mode: AramMode,
    locale: AppLocale,
    aram_rest_url: &str,
    mayhem_web_url: &str,
) -> String {
    match mode {
        AramMode::Aram => format!("aram:{}:{}", locale.as_str(), aram_rest_url),
        AramMode::AramMayhem => format!(
            "mayhem:{}:{}:{}",
            locale.as_str(),
            aram_rest_url,
            mayhem_web_url
        ),
    }
}

pub(crate) fn aram_augments_cache_key(
    locale: AppLocale,
    mcp_endpoint: &str,
    web_base: &str,
    normalized_champ: &str,
) -> String {
    format!(
        "{}:{}:{}:{}",
        locale.as_str(),
        mcp_endpoint,
        web_base,
        normalized_champ
    )
}

pub(crate) fn persist_aram_augments(
    augments: &[AramAugmentEntry],
    locale: AppLocale,
    normalized: &str,
    aram_dir: Option<&Path>,
    known_latest: Option<&str>,
    cache_key: String,
) {
    if augments.is_empty() {
        return;
    }
    let patch = known_latest.unwrap_or("");
    let cache = &build_provider_caches().aram_augments;
    if let Some(dir) = aram_dir {
        let path = aram_augments_file_path(dir, locale, normalized);
        let _ = cache.write_disk(&path, patch, &augments.to_vec());
    }
    cache.insert_ram(cache_key, patch, augments.to_vec());
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

/// Returns `true` when cached ARAM augments have populated, clean descriptions.
pub(crate) fn has_valid_augment_descriptions(list: &[AramAugmentEntry]) -> bool {
    has_any_augment_description(list)
        && !list.iter().any(|entry| {
            has_unresolved_augment_placeholder(&entry.desc)
                || has_raw_riot_augment_markup(&entry.desc)
                || is_internal_augment_key(&entry.name)
        })
}

/// Best-effort sanitization for offline `stale_fallback` entries.
pub(crate) fn sanitize_stale_augment_entries(list: &mut Vec<AramAugmentEntry>, locale: AppLocale) {
    list.retain_mut(|entry| {
        if is_internal_augment_key(&entry.name) {
            return false;
        }
        if has_raw_riot_augment_markup(&entry.desc) {
            let localized = strip_riot_inline_markup(&entry.desc, Some(locale));
            entry.desc = clean_augment_description(&localized);
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
            if has_raw_riot_augment_markup(&entry.desc) {
                entry.desc = clean_augment_description(&entry.desc);
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
            entry.desc = if has_raw_riot_augment_markup(&meta.desc) {
                clean_augment_description(&meta.desc)
            } else {
                meta.desc.clone()
            };
        } else if has_raw_riot_augment_markup(&entry.desc) {
            entry.desc = clean_augment_description(&entry.desc);
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
    let cache = &build_provider_caches().aram_tiers;

    for mode in [AramMode::Aram, AramMode::AramMayhem] {
        let url_key =
            aram_tier_cache_key(mode, locale, OPGG_ARAM_REST_URL, OPGG_ARAM_MAYHEM_WEB_URL);
        let path = aram_tier_file_path(dir, mode, locale);
        let Some(mut env) = cache.read_disk(&path) else {
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
        cache.insert_ram(url_key, env.patch, env.payload);
        warmed += 1;
    }

    warmed
}

pub fn aram_mayhem_augments_web_base(locale: AppLocale) -> &'static str {
    locale.opgg_aram_mayhem_augments_base()
}
