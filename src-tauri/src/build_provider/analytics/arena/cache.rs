use super::super::client::OPGG_REST_API_BASE;
use crate::build_provider::models::{ChampionBuild, DataDragonCache};
use crate::models::{AppLocale, ArenaTierListResponse};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{OnceLock, RwLock};
use std::time::{Duration, Instant};

pub const ARENA_TIER_CACHE_TTL: Duration = Duration::from_secs(900);
pub const ARENA_BUILD_CACHE_TTL: Duration = Duration::from_secs(600);
pub const ARENA_VERSIONS_CACHE_TTL: Duration = Duration::from_secs(600);

pub type ArenaVersionsCacheMap = HashMap<String, (Instant, Vec<String>)>;
pub type ArenaTierCacheMap = HashMap<String, (Instant, ArenaTierListResponse)>;
pub type ArenaBuildCacheMap = HashMap<String, (Instant, ChampionBuild)>;

static ARENA_VERSIONS_CACHE: OnceLock<RwLock<ArenaVersionsCacheMap>> = OnceLock::new();
static ARENA_TIER_CACHE: OnceLock<RwLock<ArenaTierCacheMap>> = OnceLock::new();
static ARENA_BUILD_CACHE: OnceLock<RwLock<ArenaBuildCacheMap>> = OnceLock::new();

pub(crate) fn get_arena_versions_cache() -> &'static RwLock<ArenaVersionsCacheMap> {
    ARENA_VERSIONS_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

pub(crate) fn get_arena_tier_cache() -> &'static RwLock<ArenaTierCacheMap> {
    ARENA_TIER_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

pub(crate) fn get_arena_build_cache() -> &'static RwLock<ArenaBuildCacheMap> {
    ARENA_BUILD_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

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
    if let Ok(guard) = get_arena_versions_cache().read()
        && let Some((_, vers)) = guard.get(OPGG_REST_API_BASE)
        && let Some(first) = vers.first()
    {
        return Some(first.clone());
    }
    if let Some(paths) = crate::storage::global_paths() {
        let path = arena_versions_file_path(&paths.opgg_arena_dir);
        if let Some(env) = crate::storage::read_lz4_envelope::<Vec<String>>(&path) {
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
    let Some(mut env) = crate::storage::read_lz4_envelope::<ArenaTierListResponse>(&path) else {
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
    if let Ok(mut guard) = get_arena_tier_cache().write() {
        guard.insert(cache_key, (Instant::now(), env.payload));
        return true;
    }
    false
}
