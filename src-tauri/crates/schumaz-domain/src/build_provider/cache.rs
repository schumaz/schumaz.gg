use super::analytics::ChampionPositionsMap;
use super::ddragon::enrich_build_with_ddragon_for_queue_with_flash_key_and_locale;
use super::models::{ChampionBuild, DataDragonCache};
use super::peek_latest_cached_opgg_version;
use super::query::BuildQuery;
use crate::models::{
    AramAugmentEntry, AramChampionRank, ArenaTierListResponse, RankedTierListResponse,
};
use crate::storage::{CacheEnvelope, TieredCache};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Duration;

pub(crate) const BUILD_CACHE_TTL: Duration = Duration::from_secs(600);

/// Consolidated L1 RAM + L2 `.lz4` tiered caches + `Singleflight` coordinators for all build/tier-list data.
#[derive(Default)]
pub struct BuildProviderCaches {
    pub builds: TieredCache<String, ChampionBuild>,
    pub ranked_tiers: TieredCache<String, RankedTierListResponse>,
    pub arena_versions: TieredCache<String, Vec<String>>,
    pub arena_tiers: TieredCache<String, ArenaTierListResponse>,
    pub arena_builds: TieredCache<String, ChampionBuild>,
    pub aram_tiers: TieredCache<String, Vec<AramChampionRank>>,
    pub aram_augments: TieredCache<String, Vec<AramAugmentEntry>>,
    pub positions: TieredCache<String, ChampionPositionsMap>,
    pub opgg_versions: TieredCache<String, Vec<String>>,
}

impl BuildProviderCaches {
    /// Evicts every L1 RAM cache across builds, tier lists, augments, positions, and versions.
    pub fn clear_all(&self) {
        self.builds.clear_ram();
        self.ranked_tiers.clear_ram();
        self.arena_versions.clear_ram();
        self.arena_tiers.clear_ram();
        self.arena_builds.clear_ram();
        self.aram_tiers.clear_ram();
        self.aram_augments.clear_ram();
        self.positions.clear_ram();
        self.opgg_versions.clear_ram();
    }
}

static BUILD_PROVIDER_CACHES: OnceLock<BuildProviderCaches> = OnceLock::new();

/// Returns the single global registry of tiered caches for `build_provider`.
pub fn build_provider_caches() -> &'static BuildProviderCaches {
    BUILD_PROVIDER_CACHES.get_or_init(BuildProviderCaches::default)
}

pub fn build_cache_file_path_for(dir: &Path, query: &BuildQuery<'_>) -> PathBuf {
    let pos = query.pos_cache_key();
    let manual_patch = query.manual_patch_trimmed();
    let p_pos = crate::storage::sanitize_key_component(if pos.is_empty() { "auto" } else { pos });
    let p_tier = crate::storage::sanitize_key_component(query.target_tier());
    let p_reg = crate::storage::sanitize_key_component(query.target_region());
    let p_pat = crate::storage::sanitize_key_component(if manual_patch.is_empty() {
        "auto"
    } else {
        manual_patch
    });
    let prot = if query.patch_48h_protection {
        "p1"
    } else {
        "p0"
    };
    dir.join(format!(
        "build_{}_{}_{p_pos}_{p_tier}_{p_reg}_{p_pat}_{prot}_{}.lz4",
        query.champion_id,
        query.canonical_queue_id(),
        query.locale.as_str()
    ))
}

pub(crate) fn try_get_ram_cached_build(
    cache_key: &str,
    query: &BuildQuery<'_>,
    ddragon_cache: &DataDragonCache,
) -> Option<ChampionBuild> {
    let key = cache_key.to_string();
    let entry = build_provider_caches()
        .builds
        .get_ram(&key, Some(BUILD_CACHE_TTL))?;
    let mut build = entry.payload;
    enrich_build_with_ddragon_for_queue_with_flash_key_and_locale(
        &mut build,
        ddragon_cache,
        query.flash_key,
        query.locale,
        Some(query.canonical_queue_id()),
    );
    Some(build)
}

pub(crate) fn load_disk_build_envelope(
    query: &BuildQuery<'_>,
) -> Option<CacheEnvelope<ChampionBuild>> {
    query.builds_dir.and_then(|dir| {
        let path = build_cache_file_path_for(dir, query);
        build_provider_caches().builds.read_disk(&path)
    })
}

pub(crate) fn try_get_fresh_disk_build(
    env: &CacheEnvelope<ChampionBuild>,
    query: &BuildQuery<'_>,
    ddragon_cache: &DataDragonCache,
) -> Option<ChampionBuild> {
    if !crate::storage::is_envelope_fresh(
        env.created_at_unix,
        crate::storage::OPGG_CACHE_TTL_SECS,
        crate::storage::now_unix_secs(),
    ) {
        return None;
    }

    let manual_patch = query.manual_patch_trimmed();
    let known_latest = peek_latest_cached_opgg_version(query.target_region());
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
    enrich_build_with_ddragon_for_queue_with_flash_key_and_locale(
        &mut build,
        ddragon_cache,
        query.flash_key,
        query.locale,
        Some(query.canonical_queue_id()),
    );
    Some(build)
}

pub(crate) fn store_ram_cached_build(cache_key: String, patch: &str, build: &ChampionBuild) {
    build_provider_caches()
        .builds
        .insert_ram(cache_key, patch, build.clone());
}

pub(crate) fn store_disk_build(
    dir: &Path,
    query: &BuildQuery<'_>,
    effective_patch: &str,
    build: &ChampionBuild,
) {
    let cache_file_path = build_cache_file_path_for(dir, query);
    let _ = build_provider_caches()
        .builds
        .write_disk(&cache_file_path, effective_patch, build);
}

#[cfg(test)]
#[path = "cache_tests.rs"]
mod tests;
