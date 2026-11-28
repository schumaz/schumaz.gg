use super::super::client::OPGG_REST_API_BASE;
use crate::build_provider::cache::build_provider_caches;
use crate::build_provider::models::DataDragonCache;
use crate::models::{AppLocale, ArenaTierListResponse};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const ARENA_TIER_CACHE_TTL: Duration = Duration::from_secs(900);
pub const ARENA_BUILD_CACHE_TTL: Duration = Duration::from_secs(600);
pub const ARENA_VERSIONS_CACHE_TTL: Duration = Duration::from_secs(600);

pub fn arena_versions_file_path(dir: &Path) -> PathBuf {
    dir.join("versions.lz4")
}

pub fn arena_tier_list_file_path(
    dir: &Path,
    manual_patch: Option<&str>,
    patch_48h_protection: bool,
    locale: AppLocale,
) -> PathBuf {
    let pat = crate::storage::sanitize_key_component(manual_patch.unwrap_or("auto"));
    let prot = if patch_48h_protection { "p1" } else { "p0" };
    dir.join(format!("tier_{pat}_{prot}_{}.lz4", locale.as_str()))
}

pub fn arena_build_file_path(
    dir: &Path,
    champion_id: i32,
    manual_patch: Option<&str>,
    patch_48h_protection: bool,
    locale: AppLocale,
) -> PathBuf {
    let pat = crate::storage::sanitize_key_component(manual_patch.unwrap_or("auto"));
    let prot = if patch_48h_protection { "p1" } else { "p0" };
    dir.join(format!(
        "champ_{champion_id}_{pat}_{prot}_{}.lz4",
        locale.as_str()
    ))
}

/// Non-blocking peek at the latest known Arena patch from RAM or disk `.lz4`.
pub fn peek_latest_cached_arena_version() -> Option<String> {
    let cache = &build_provider_caches().arena_versions;
    if let Some(first) = cache.get_ram_with(&OPGG_REST_API_BASE.to_string(), None, |entry| {
        entry.payload.first().cloned()
    }) {
        return Some(first);
    }
    if let Some(paths) = crate::storage::global_paths() {
        let path = arena_versions_file_path(&paths.opgg_arena_dir);
        if let Some(env) = cache.read_disk(&path) {
            return env.payload.first().cloned();
        }
    }
    None
}

pub(crate) fn rehydrate_arena_tier_list_names(
    response: &mut ArenaTierListResponse,
    ddragon: &DataDragonCache,
) {
    for champ in &mut response.champions {
        if let Some(meta) = ddragon.champions.get(&champ.id) {
            champ.name = meta.display_name.clone();
            if !meta.icon_url.is_empty() {
                champ.icon_url = meta.icon_url.clone();
            }
        }
    }
}

/// Pre-warms a valid (< 48h) Arena Tier List from `.lz4` disk into RAM on startup.
pub fn prewarm_arena_tier_list_from_dir(
    dir: &Path,
    patch_48h_protection: bool,
    locale: AppLocale,
    ddragon: &DataDragonCache,
) -> bool {
    let path = arena_tier_list_file_path(dir, None, patch_48h_protection, locale);
    let cache = &build_provider_caches().arena_tiers;
    let Some(mut env) = cache.read_disk(&path) else {
        return false;
    };
    let now = crate::storage::now_unix_secs();
    if !crate::storage::is_envelope_fresh(
        env.created_at_unix,
        crate::storage::OPGG_CACHE_TTL_SECS,
        now,
    ) {
        return false;
    }
    if let Some(latest) = peek_latest_cached_arena_version()
        && !env.payload.patch_fallback_applied
        && env.patch != latest
    {
        return false;
    }

    rehydrate_arena_tier_list_names(&mut env.payload, ddragon);
    let cache_key = format!(
        "{}::{}:{}",
        OPGG_REST_API_BASE,
        patch_48h_protection,
        locale.as_str()
    );
    let patch = env.patch.clone();
    cache.insert_ram(cache_key, patch, env.payload);
    true
}
