use super::client::OPGG_REST_API_BASE;
use super::cooldown::send_analytics_request;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{OnceLock, RwLock};
use std::time::{Duration, Instant};

type VersionsCacheMap = HashMap<String, (Instant, Vec<String>)>;

static OPGG_VERSIONS_CACHE: OnceLock<RwLock<VersionsCacheMap>> = OnceLock::new();
const VERSIONS_CACHE_TTL: Duration = Duration::from_secs(600);

fn get_versions_cache() -> &'static RwLock<VersionsCacheMap> {
    OPGG_VERSIONS_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
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
    if let Ok(guard) = get_versions_cache().read()
        && let Some((_, vers)) = guard.get(&cache_key)
        && let Some(first) = vers.first()
    {
        return Some(first.clone());
    }
    if let Some(paths) = crate::storage::global_paths() {
        let path = opgg_versions_file_path(&paths.opgg_versions_dir, region);
        if let Some(env) = crate::storage::read_lz4_envelope::<Vec<String>>(&path) {
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
    if let Ok(guard) = get_versions_cache().read()
        && let Some((ts, cached)) = guard.get(&cache_key)
        && ts.elapsed() < VERSIONS_CACHE_TTL
    {
        return cached.clone();
    }

    let disk_env = versions_dir.and_then(|dir| {
        let path = opgg_versions_file_path(dir, region);
        crate::storage::read_lz4_envelope::<Vec<String>>(&path)
    });

    if let Some(ref env) = disk_env
        && !env.payload.is_empty()
        && crate::storage::is_envelope_fresh(
            env.created_at_unix,
            crate::storage::VERSIONS_DISK_TTL_SECS,
            crate::storage::now_unix_secs(),
        )
    {
        if let Ok(mut guard) = get_versions_cache().write() {
            guard.insert(cache_key.clone(), (Instant::now(), env.payload.clone()));
        }
        return env.payload.clone();
    }

    let url = format!("{}/{}/champions/ranked/versions", rest_base, region);
    if let Ok(res) = send_analytics_request(&url, client.get(&url), "OP.GG versions").await
        && let Ok(json) = res.json::<serde_json::Value>().await
        && let Some(arr) = json.get("data").and_then(|d| d.as_array())
    {
        let vers: Vec<String> = arr
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();
        if !vers.is_empty() {
            if let Ok(mut guard) = get_versions_cache().write() {
                guard.insert(cache_key, (Instant::now(), vers.clone()));
            }
            if let Some(dir) = versions_dir {
                let path = opgg_versions_file_path(dir, region);
                let latest = vers.first().map(String::as_str).unwrap_or("");
                let _ = crate::storage::write_lz4_envelope(&path, latest, &vers);
            }
            return vers;
        }
    }

    // Stale-While-Error offline fallback: return expired `.lz4` versions if present
    if let Some(env) = disk_env
        && !env.payload.is_empty()
    {
        if let Ok(mut guard) = get_versions_cache().write() {
            guard.insert(cache_key, (Instant::now(), env.payload.clone()));
        }
        return env.payload;
    }

    // Dynamic emergency fallback: derive `major.minor` from cached DataDragon version (no hardcoded patches)
    if let Some(explicit) = ddragon_version_override {
        return emergency_versions_from_ddragon(Some(explicit));
    }
    let dd_version = crate::build_provider::ddragon::peek_cached_ddragon_version(
        crate::lcu_injector::get_active_locale(),
    );
    emergency_versions_from_ddragon(dd_version.as_deref())
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
