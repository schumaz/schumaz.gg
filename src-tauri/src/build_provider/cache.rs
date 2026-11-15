use super::ddragon::enrich_build_with_ddragon_with_flash_key_and_locale;
use super::models::{ChampionBuild, DataDragonCache};
use super::peek_latest_cached_opgg_version;
use crate::models::{AppLocale, FlashKeyPreference};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{OnceLock, RwLock};
use std::time::{Duration, Instant};

pub(crate) const BUILD_CACHE_TTL: Duration = Duration::from_secs(600);

type BuildCacheMap = HashMap<String, (Instant, ChampionBuild)>;

static BUILD_CACHE: OnceLock<RwLock<BuildCacheMap>> = OnceLock::new();

pub(crate) fn get_build_cache() -> &'static RwLock<BuildCacheMap> {
    BUILD_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

#[allow(clippy::too_many_arguments)]
pub fn build_cache_file_path(
    dir: &Path,
    champion_id: i32,
    queue_id: i32,
    pos: &str,
    tier: &str,
    region: &str,
    manual_patch: &str,
    patch_48h_protection: bool,
    locale: AppLocale,
) -> PathBuf {
    let p_pos = crate::storage::sanitize_key_component(if pos.is_empty() { "auto" } else { pos });
    let p_tier = crate::storage::sanitize_key_component(tier);
    let p_reg = crate::storage::sanitize_key_component(region);
    let p_pat = crate::storage::sanitize_key_component(if manual_patch.is_empty() {
        "auto"
    } else {
        manual_patch
    });
    let prot = if patch_48h_protection { "p1" } else { "p0" };
    dir.join(format!(
        "build_{champion_id}_{queue_id}_{p_pos}_{p_tier}_{p_reg}_{p_pat}_{prot}_{}.lz4",
        locale.as_str()
    ))
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn make_build_cache_key(
    locale: AppLocale,
    rest_base: &str,
    mcp_endpoint: &str,
    champion_id: i32,
    queue_id: i32,
    pos_cache_key: &str,
    target_tier: &str,
    target_region: &str,
    manual_patch: &str,
    patch_48h_protection: bool,
) -> String {
    format!(
        "{}:{}:{}:{}:{}:{}:{}:{}:{}:{}",
        locale.as_str(),
        rest_base,
        mcp_endpoint,
        champion_id,
        queue_id,
        pos_cache_key,
        target_tier,
        target_region,
        manual_patch,
        patch_48h_protection
    )
}

pub(crate) fn try_get_ram_cached_build(
    cache_key: &str,
    ddragon_cache: &DataDragonCache,
    flash_key: FlashKeyPreference,
    locale: AppLocale,
) -> Option<ChampionBuild> {
    let guard = get_build_cache().read().ok()?;
    let (ts, cached) = guard.get(cache_key)?;
    if ts.elapsed() >= BUILD_CACHE_TTL {
        return None;
    }
    let mut build = cached.clone();
    enrich_build_with_ddragon_with_flash_key_and_locale(
        &mut build,
        ddragon_cache,
        flash_key,
        locale,
    );
    Some(build)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn load_disk_build_envelope(
    builds_dir: Option<&Path>,
    champion_id: i32,
    queue_id: i32,
    pos_cache_key: &str,
    target_tier: &str,
    target_region: &str,
    manual_patch: &str,
    patch_48h_protection: bool,
    locale: AppLocale,
) -> Option<crate::storage::CacheEnvelope<ChampionBuild>> {
    builds_dir.and_then(|dir| {
        let path = build_cache_file_path(
            dir,
            champion_id,
            queue_id,
            pos_cache_key,
            target_tier,
            target_region,
            manual_patch,
            patch_48h_protection,
            locale,
        );
        crate::storage::read_lz4_envelope::<ChampionBuild>(&path)
    })
}

pub(crate) fn try_get_fresh_disk_build(
    env: &crate::storage::CacheEnvelope<ChampionBuild>,
    target_region: &str,
    manual_patch: &str,
    ddragon_cache: &DataDragonCache,
    flash_key: FlashKeyPreference,
    locale: AppLocale,
) -> Option<ChampionBuild> {
    if !crate::storage::is_envelope_fresh(
        env.created_at_unix,
        crate::storage::OPGG_CACHE_TTL_SECS,
        crate::storage::now_unix_secs(),
    ) {
        return None;
    }

    let known_latest = peek_latest_cached_opgg_version(target_region);
    let patch_valid = if !manual_patch.is_empty() {
        env.patch == manual_patch
    } else if let Some(ref latest) = known_latest {
        env.payload.patch_fallback_applied || env.patch == *latest
    } else {
        true
    };
    if !patch_valid {
        return None;
    }

    let mut build = env.payload.clone();
    enrich_build_with_ddragon_with_flash_key_and_locale(
        &mut build,
        ddragon_cache,
        flash_key,
        locale,
    );
    Some(build)
}

pub(crate) fn store_ram_cached_build(cache_key: String, build: &ChampionBuild) {
    if let Ok(mut guard) = get_build_cache().write() {
        guard.insert(cache_key, (Instant::now(), build.clone()));
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn store_disk_build(
    builds_dir: &Path,
    champion_id: i32,
    queue_id: i32,
    pos_cache_key: &str,
    target_tier: &str,
    target_region: &str,
    manual_patch: &str,
    patch_48h_protection: bool,
    locale: AppLocale,
    effective_patch: &str,
    build: &ChampionBuild,
) {
    let cache_file_path = build_cache_file_path(
        builds_dir,
        champion_id,
        queue_id,
        pos_cache_key,
        target_tier,
        target_region,
        manual_patch,
        patch_48h_protection,
        locale,
    );
    let _ = crate::storage::write_lz4_envelope(&cache_file_path, effective_patch, build);
}

#[cfg(test)]
#[path = "cache_tests.rs"]
mod tests;
