pub mod build_parser;
pub mod cache;
pub mod fetch_build;
mod item_options;
pub mod items;
pub mod tier_parser;

pub use build_parser::*;
pub use cache::*;
pub use fetch_build::*;
pub use items::*;
pub use tier_parser::*;

use super::client::{OPGG_REST_API_BASE, fetch_opgg_versions_from_base};
use super::cooldown::send_analytics_request;
use crate::build_provider::cache::build_provider_caches;
use crate::build_provider::models::DataDragonCache;
use crate::build_provider::patch::resolve_effective_patch;
use crate::models::{AppLocale, ArenaTierListResponse};
use serde::Deserialize;
use std::path::Path;
use std::time::Instant;

const ARENA_MIN_PATCH_MATCHES: u64 = 1000;

#[derive(Debug, Deserialize)]
struct ArenaVersionsResponse {
    #[serde(default)]
    data: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct ArenaTierQuery<'a> {
    pub rest_base: &'a str,
    pub patch: Option<&'a str>,
    pub patch_48h_protection: bool,
    pub now_unix: u64,
    pub locale: AppLocale,
    pub arena_dir: Option<&'a Path>,
}

pub(crate) async fn fetch_arena_json(
    client: &reqwest::Client,
    url: &str,
    label: &str,
) -> Result<serde_json::Value, String> {
    let res = send_analytics_request(url, client.get(url), label).await?;
    res.json::<serde_json::Value>()
        .await
        .map_err(|e| format!("Failed to parse {} JSON: {}", label, e))
}

async fn fetch_arena_tier_resp(
    client: &reqwest::Client,
    url: &str,
) -> Result<tier_parser::RestArenaTierListResponse, String> {
    let res = send_analytics_request(url, client.get(url), "Arena tier list").await?;
    res.json::<tier_parser::RestArenaTierListResponse>()
        .await
        .map_err(|e| format!("Failed to parse Arena tier list JSON: {}", e))
}

pub(crate) fn build_arena_url(
    rest_base: &str,
    champion_id: Option<i32>,
    version: Option<&str>,
) -> String {
    let base = rest_base.trim_end_matches('/');
    let mut url = match champion_id {
        Some(id) => format!("{base}/global/champions/arena/{id}"),
        None => format!("{base}/global/champions/arena"),
    };
    if let Some(v) = version.map(str::trim).filter(|s| !s.is_empty()) {
        url.push_str(&format!("?version={v}"));
    }
    url
}

/// Fetches available Arena patch versions from OP.GG with L1 RAM + L2 `.lz4` disk cache (3h TTL),
/// `Singleflight` coalescing, and Stale-While-Error fallback.
pub async fn fetch_arena_versions_with_cache_dir(
    client: &reqwest::Client,
    rest_base: &str,
    arena_dir: Option<&Path>,
) -> Vec<String> {
    let cache = &build_provider_caches().arena_versions;
    let cache_key = rest_base.to_string();
    if let Some(entry) = cache.get_ram(&cache_key, Some(cache::ARENA_VERSIONS_CACHE_TTL)) {
        return entry.payload;
    }

    let res = cache
        .coalesce(cache_key.clone(), || async {
            if let Some(entry) = cache.get_ram(&cache_key, Some(cache::ARENA_VERSIONS_CACHE_TTL)) {
                return Ok(entry.payload);
            }

            let disk_env = arena_dir.and_then(|dir| {
                let path = cache::arena_versions_file_path(dir);
                cache.read_disk(&path)
            });

            if let Some(ref env) = disk_env
                && !env.payload.is_empty()
                && crate::storage::is_envelope_fresh(
                    env.created_at_unix,
                    crate::storage::VERSIONS_DISK_TTL_SECS,
                    crate::storage::now_unix_secs(),
                )
            {
                let latest = env.payload.first().map(String::as_str).unwrap_or("");
                cache.insert_ram(cache_key.clone(), latest, env.payload.clone());
                return Ok(env.payload.clone());
            }

            let url = format!(
                "{}/global/champions/arena/versions",
                rest_base.trim_end_matches('/')
            );
            if let Ok(res) = send_analytics_request(&url, client.get(&url), "Arena versions").await
                && let Ok(parsed) = res.json::<ArenaVersionsResponse>().await
            {
                let vers: Vec<String> = parsed
                    .data
                    .into_iter()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                if !vers.is_empty() {
                    let latest = vers.first().map(String::as_str).unwrap_or("");
                    cache.insert_ram(cache_key.clone(), latest, vers.clone());
                    if let Some(dir) = arena_dir {
                        let path = cache::arena_versions_file_path(dir);
                        let _ = cache.write_disk(&path, latest, &vers);
                    }
                    return Ok(vers);
                }
            }

            if let Some(env) = disk_env
                && !env.payload.is_empty()
            {
                let latest = env.payload.first().map(String::as_str).unwrap_or("");
                cache.insert_ram(cache_key.clone(), latest, env.payload.clone());
                return Ok(env.payload);
            }

            Ok(fetch_opgg_versions_from_base(client, rest_base, "global").await)
        })
        .await;

    res.unwrap_or_default()
}

pub async fn fetch_arena_tier_list_with_query(
    client: &reqwest::Client,
    query: &ArenaTierQuery<'_>,
    ddragon: &DataDragonCache,
) -> Result<ArenaTierListResponse, String> {
    let started = Instant::now();
    let manual_patch = query.patch.map(str::trim).filter(|s| !s.is_empty());
    let cache_key = format!(
        "{}:{}:{}:{}",
        query.rest_base,
        manual_patch.unwrap_or(""),
        query.patch_48h_protection,
        query.locale.as_str()
    );
    let cache = &build_provider_caches().arena_tiers;

    if let Some(entry) = cache.get_ram(&cache_key, Some(cache::ARENA_TIER_CACHE_TTL)) {
        let mut resp = entry.payload;
        cache::rehydrate_arena_tier_list_names(&mut resp, ddragon);
        crate::logging::record_data_resolution("arena_tier_list", "ram_hit", started.elapsed().as_millis(), "");
        return Ok(resp);
    }

    cache
        .coalesce(cache_key.clone(), || async {
            if let Some(entry) = cache.get_ram(&cache_key, Some(cache::ARENA_TIER_CACHE_TTL)) {
                let mut resp = entry.payload;
                cache::rehydrate_arena_tier_list_names(&mut resp, ddragon);
                return Ok(resp);
            }

            let disk_env = query.arena_dir.and_then(|dir| {
                let path = cache::arena_tier_list_file_path(dir, manual_patch, query.patch_48h_protection, query.locale);
                cache.read_disk(&path)
            });

            if let Some(ref env) = disk_env
                && !env.payload.champions.is_empty()
                && crate::storage::is_envelope_fresh(env.created_at_unix, crate::storage::OPGG_CACHE_TTL_SECS, query.now_unix)
            {
                let known_latest = cache::peek_latest_cached_arena_version();
                let patch_valid = match (manual_patch, known_latest.as_deref()) {
                    (Some(p), _) => env.patch == p,
                    (None, Some(latest)) if !env.payload.patch_fallback_applied => env.patch == latest,
                    _ => true,
                };
                if patch_valid {
                    let mut resp = env.payload.clone();
                    cache::rehydrate_arena_tier_list_names(&mut resp, ddragon);
                    cache.insert_ram(cache_key.clone(), env.patch.clone(), resp.clone());
                    crate::logging::record_data_resolution("arena_tier_list", "disk_hit", started.elapsed().as_millis(), "");
                    return Ok(resp);
                }
            }

            let available_versions = fetch_arena_versions_with_cache_dir(client, query.rest_base, query.arena_dir).await;
            let mut effective_patch = manual_patch
                .map(str::to_string)
                .unwrap_or_else(|| available_versions.first().cloned().unwrap_or_default());
            let mut patch_fallback_applied = false;
            let mut patch_protection_remaining_hours: Option<u32> = None;

            let fetch_res: Result<tier_parser::RestArenaTierListResponse, String> = async {
                let probe_version = (!effective_patch.is_empty()).then_some(effective_patch.as_str());
                let probe_url = build_arena_url(query.rest_base, None, probe_version);
                let probe_resp = fetch_arena_tier_resp(client, &probe_url).await?;

                let (meta_version, match_count) = tier_parser::extract_arena_meta_from_resp(&probe_resp);
                if effective_patch.is_empty()
                    && let Some(v) = meta_version
                {
                    effective_patch = v;
                }

                if manual_patch.is_none()
                    && query.patch_48h_protection
                    && available_versions.len() >= 2
                    && match_count.unwrap_or(0) < ARENA_MIN_PATCH_MATCHES
                {
                    let selection = resolve_effective_patch(
                        &available_versions, None, None, match_count, query.now_unix, None, query.patch_48h_protection,
                    );
                    effective_patch = selection.effective_patch;
                    patch_fallback_applied = selection.fallback_applied;
                    patch_protection_remaining_hours = selection.remaining_hours;

                    if patch_fallback_applied {
                        let fallback_url = build_arena_url(query.rest_base, None, Some(&effective_patch));
                        return fetch_arena_tier_resp(client, &fallback_url).await;
                    }
                }
                Ok(probe_resp)
            }
            .await;

            let final_resp = match fetch_res {
                Ok(resp) => resp,
                Err(err) => {
                    if let Some(env) = disk_env
                        && !env.payload.champions.is_empty()
                    {
                        let mut resp = env.payload;
                        cache::rehydrate_arena_tier_list_names(&mut resp, ddragon);
                        crate::logging::record_data_resolution("arena_tier_list", "stale_fallback", started.elapsed().as_millis(), "");
                        return Ok(resp);
                    }
                    return Err(err);
                }
            };

            let champions = tier_parser::parse_arena_tier_list_from_resp(&final_resp, ddragon);
            let response = ArenaTierListResponse {
                active_patch: effective_patch.clone(),
                available_patches: available_versions,
                patch_fallback_applied,
                patch_protection_remaining_hours,
                champions,
            };

            if !response.champions.is_empty() {
                if let Some(dir) = query.arena_dir {
                    let path = cache::arena_tier_list_file_path(dir, manual_patch, query.patch_48h_protection, query.locale);
                    let _ = cache.write_disk(&path, &effective_patch, &response);
                }
                cache.insert_ram(cache_key.clone(), &effective_patch, response.clone());
            }

            crate::logging::record_data_resolution("arena_tier_list", "network", started.elapsed().as_millis(), "");
            Ok(response)
        })
        .await
}

pub async fn fetch_arena_tier_list(
    client: &reqwest::Client,
    patch: Option<&str>,
    ddragon: &DataDragonCache,
) -> Result<ArenaTierListResponse, String> {
    let prefs = crate::storage::get_user_preferences();
    let dir = crate::storage::global_paths().map(|p| p.opgg_arena_dir.as_path());
    fetch_arena_tier_list_with_query(
        client,
        &ArenaTierQuery {
            rest_base: OPGG_REST_API_BASE,
            patch,
            patch_48h_protection: prefs.patch_48h_protection,
            now_unix: crate::storage::now_unix_secs(),
            locale: prefs.language,
            arena_dir: dir,
        },
        ddragon,
    )
    .await
}

#[cfg(test)]
#[path = "arena/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "arena/items_tests.rs"]
mod items_tests;

#[cfg(test)]
#[path = "arena/service_tests.rs"]
mod service_tests;
