use super::client::OPGG_REST_API_BASE;
use super::cooldown::send_analytics_request;
use crate::build_provider::cache::build_provider_caches;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::time::Duration;

const VERSIONS_CACHE_TTL: Duration = Duration::from_secs(600);

#[derive(Debug, Deserialize)]
struct VersionsResponse {
    #[serde(default)]
    data: Vec<String>,
}

pub fn opgg_versions_file_path(dir: &Path, region: &str) -> PathBuf {
    let reg = crate::storage::sanitize_key_component(region);
    dir.join(format!("versions_{reg}.lz4"))
}

/// Derives a canonical `major.minor` patch string from a DataDragon version (e.g. `"16.24.1"` -> `Some("16.24")`).
pub(crate) fn derive_patch_major_minor(ddragon_version: &str) -> Option<String> {
    let mut parts = ddragon_version.trim().split('.');
    let major = parts.next()?.trim();
    let minor = parts.next()?.trim();
    if major.parse::<u32>().is_ok() && minor.parse::<u32>().is_ok() {
        Some(format!("{major}.{minor}"))
    } else {
        None
    }
}

fn emergency_versions_from_ddragon(ddragon_version: Option<&str>) -> Vec<String> {
    ddragon_version
        .and_then(derive_patch_major_minor)
        .map(|patch| vec![patch])
        .unwrap_or_default()
}

/// Non-blocking peek at the latest known OP.GG patch for `region` from RAM or disk `.lz4`.
pub fn peek_latest_cached_opgg_version(region: &str) -> Option<String> {
    let cache_key = format!("{}:{}", OPGG_REST_API_BASE, region);
    if let Some(first) = build_provider_caches()
        .opgg_versions
        .get_ram_with(&cache_key, None, |entry| entry.payload.first().cloned())
    {
        return Some(first);
    }
    if let Some(paths) = crate::storage::global_paths() {
        let path = opgg_versions_file_path(&paths.opgg_versions_dir, region);
        if let Some(env) = build_provider_caches().opgg_versions.read_disk(&path) {
            return env.payload.first().cloned();
        }
    }
    None
}

async fn fetch_opgg_versions_internal(
    client: &reqwest::Client,
    rest_base: &str,
    region: &str,
    versions_dir: Option<&Path>,
    ddragon_version_override: Option<&str>,
) -> Vec<String> {
    let cache_key = format!("{}:{}", rest_base, region);
    let cache = &build_provider_caches().opgg_versions;
    if let Some(cached) = cache.get_ram(&cache_key, Some(VERSIONS_CACHE_TTL)) {
        return cached.payload;
    }

    let res = cache
        .coalesce(cache_key.clone(), || async {
            if let Some(cached) = cache.get_ram(&cache_key, Some(VERSIONS_CACHE_TTL)) {
                return Ok(cached.payload);
            }

            let disk_env = versions_dir.and_then(|dir| {
                let path = opgg_versions_file_path(dir, region);
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

            let url = format!("{}/{}/champions/ranked/versions", rest_base, region);
            if let Ok(res) = send_analytics_request(&url, client.get(&url), "OP.GG versions").await
                && let Ok(parsed) = res.json::<VersionsResponse>().await
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
                    if let Some(dir) = versions_dir {
                        let path = opgg_versions_file_path(dir, region);
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

            if let Some(explicit) = ddragon_version_override {
                return Ok(emergency_versions_from_ddragon(Some(explicit)));
            }
            let dd_version = crate::build_provider::ddragon::peek_cached_ddragon_version(
                crate::storage::get_active_locale(),
            );
            Ok(emergency_versions_from_ddragon(dd_version.as_deref()))
        })
        .await;

    res.unwrap_or_default()
}

/// Fetches available patch versions from the OP.GG REST API with L1 RAM + L2 `.lz4` disk cache (3h TTL)
/// and Stale-While-Error offline fallback.
pub async fn fetch_opgg_versions_with_cache_dir(
    client: &reqwest::Client,
    rest_base: &str,
    region: &str,
    versions_dir: Option<&Path>,
) -> Vec<String> {
    fetch_opgg_versions_internal(client, rest_base, region, versions_dir, None).await
}

/// Fetches available patch versions from the OP.GG REST API with in-memory TTL + disk caching.
pub async fn fetch_opgg_versions_from_base(
    client: &reqwest::Client,
    rest_base: &str,
    region: &str,
) -> Vec<String> {
    let dir = if rest_base == OPGG_REST_API_BASE {
        crate::storage::global_paths().map(|p| p.opgg_versions_dir.as_path())
    } else {
        None
    };
    fetch_opgg_versions_with_cache_dir(client, rest_base, region, dir).await
}

/// Fetches available patch versions from the OP.GG REST API.
pub async fn fetch_opgg_versions(client: &reqwest::Client, region: &str) -> Vec<String> {
    fetch_opgg_versions_from_base(client, OPGG_REST_API_BASE, region).await
}

#[cfg(test)]
#[path = "versions_tests.rs"]
mod tests;
