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
use crate::build_provider::models::DataDragonCache;
use crate::build_provider::patch::resolve_effective_patch;
use crate::models::{AppLocale, ArenaTierListResponse};
use std::path::Path;
use std::time::Instant;

const ARENA_MIN_PATCH_MATCHES: u64 = 1000;

pub(crate) async fn fetch_arena_json(
    client: &reqwest::Client,
    url: &str,
    label: &str,
) -> Result<serde_json::Value, String> {
    let res = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch {}: {}", label, e))?;

    if !res.status().is_success() {
        return Err(format!("{} HTTP error: {}", label, res.status()));
    }

    res.json::<serde_json::Value>()
        .await
        .map_err(|e| format!("Failed to parse {} JSON: {}", label, e))
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

/// Fetches available Arena patch versions from OP.GG with L1 RAM + L2 `.lz4` disk cache
/// and Stale-While-Error fallback.
pub async fn fetch_arena_versions_with_cache_dir(
    client: &reqwest::Client,
    rest_base: &str,
    arena_dir: Option<&Path>,
) -> Vec<String> {
    let cache_key = rest_base.to_string();
    if let Ok(guard) = cache::get_arena_versions_cache().read()
        && let Some((ts, cached)) = guard.get(&cache_key)
        && ts.elapsed() < cache::ARENA_VERSIONS_CACHE_TTL
    {
        return cached.clone();
    }

    let disk_env = arena_dir.and_then(|dir| {
        let path = cache::arena_versions_file_path(dir);
        crate::storage::read_lz4_envelope::<Vec<String>>(&path)
    });

    if let Some(ref env) = disk_env
        && !env.payload.is_empty()
        && crate::storage::is_envelope_fresh(
            env.created_at_unix,
            crate::storage::OPGG_CACHE_TTL_SECS,
            crate::storage::now_unix_secs(),
        )
    {
        if let Ok(mut guard) = cache::get_arena_versions_cache().write() {
            guard.insert(cache_key, (Instant::now(), env.payload.clone()));
        }
        return env.payload.clone();
    }

    let url = format!(
        "{}/global/champions/arena/versions",
        rest_base.trim_end_matches('/')
    );
    if let Ok(json) = fetch_arena_json(client, &url, "Arena versions").await
        && let Some(arr) = json.get("data").and_then(|d| d.as_array())
    {
        let vers: Vec<String> = arr
            .iter()
            .filter_map(|v| v.as_str().map(str::trim).filter(|s| !s.is_empty()))
            .map(str::to_string)
            .collect();
        if !vers.is_empty() {
            if let Ok(mut guard) = cache::get_arena_versions_cache().write() {
                guard.insert(cache_key, (Instant::now(), vers.clone()));
            }
            if let Some(dir) = arena_dir {
                let path = cache::arena_versions_file_path(dir);
                let latest = vers.first().map(String::as_str).unwrap_or("");
                let _ = crate::storage::write_lz4_envelope(&path, latest, &vers);
            }
            return vers;
        }
    }

    if let Some(env) = disk_env
        && !env.payload.is_empty()
    {
        if let Ok(mut guard) = cache::get_arena_versions_cache().write() {
            guard.insert(cache_key, (Instant::now(), env.payload.clone()));
        }
        return env.payload;
    }

    fetch_opgg_versions_from_base(client, rest_base, "global").await
}

#[allow(clippy::too_many_arguments)]
pub async fn fetch_arena_tier_list_with_base_and_dir(
    client: &reqwest::Client,
    rest_base: &str,
    patch: Option<&str>,
    ddragon: &DataDragonCache,
    patch_48h_protection: bool,
    now_unix: u64,
    locale: AppLocale,
    arena_dir: Option<&Path>,
) -> Result<ArenaTierListResponse, String> {
    let manual_patch = patch.map(str::trim).filter(|s| !s.is_empty());
    let cache_key = format!(
        "{}:{}:{}:{}",
        rest_base,
        manual_patch.unwrap_or(""),
        patch_48h_protection,
        locale.as_str()
    );

    if let Ok(guard) = cache::get_arena_tier_cache().read()
        && let Some((ts, cached)) = guard.get(&cache_key)
        && ts.elapsed() < cache::ARENA_TIER_CACHE_TTL
    {
        let mut resp = cached.clone();
        cache::rehydrate_arena_tier_list_names(&mut resp, ddragon);
        return Ok(resp);
    }

    let disk_env = arena_dir.and_then(|dir| {
        let path =
            cache::arena_tier_list_file_path(dir, manual_patch, patch_48h_protection, locale);
        crate::storage::read_lz4_envelope::<ArenaTierListResponse>(&path)
    });

    if let Some(ref env) = disk_env
        && !env.payload.champions.is_empty()
        && crate::storage::is_envelope_fresh(
            env.created_at_unix,
            crate::storage::OPGG_CACHE_TTL_SECS,
            now_unix,
        )
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
            if let Ok(mut guard) = cache::get_arena_tier_cache().write() {
                guard.insert(cache_key, (Instant::now(), resp.clone()));
            }
            return Ok(resp);
        }
    }

    let available_versions =
        fetch_arena_versions_with_cache_dir(client, rest_base, arena_dir).await;
    let mut effective_patch = manual_patch
        .map(str::to_string)
        .unwrap_or_else(|| available_versions.first().cloned().unwrap_or_default());
    let mut patch_fallback_applied = false;
    let mut patch_protection_remaining_hours: Option<u32> = None;

    let fetch_res: Result<serde_json::Value, String> = async {
        let probe_version = (!effective_patch.is_empty()).then_some(effective_patch.as_str());
        let probe_url = build_arena_url(rest_base, None, probe_version);
        let probe_json = fetch_arena_json(client, &probe_url, "Arena tier list").await?;

        let (meta_version, match_count) = tier_parser::extract_arena_meta_info(&probe_json);
        if effective_patch.is_empty()
            && let Some(v) = meta_version
        {
            effective_patch = v;
        }

        if manual_patch.is_none()
            && patch_48h_protection
            && available_versions.len() >= 2
            && match_count.unwrap_or(0) < ARENA_MIN_PATCH_MATCHES
        {
            let selection = resolve_effective_patch(
                &available_versions,
                None,
                None,
                match_count,
                now_unix,
                None,
                patch_48h_protection,
            );
            effective_patch = selection.effective_patch;
            patch_fallback_applied = selection.fallback_applied;
            patch_protection_remaining_hours = selection.remaining_hours;

            if patch_fallback_applied {
                let fallback_url = build_arena_url(rest_base, None, Some(&effective_patch));
                return fetch_arena_json(client, &fallback_url, "Arena tier list").await;
            }
        }
        Ok(probe_json)
    }
    .await;

    let final_json = match fetch_res {
        Ok(json) => json,
        Err(err) => {
            if let Some(env) = disk_env
                && !env.payload.champions.is_empty()
            {
                let mut resp = env.payload;
                cache::rehydrate_arena_tier_list_names(&mut resp, ddragon);
                return Ok(resp);
            }
            return Err(err);
        }
    };

    let champions = tier_parser::parse_arena_tier_list(&final_json, ddragon)?;
    let response = ArenaTierListResponse {
        active_patch: effective_patch.clone(),
        available_patches: available_versions,
        patch_fallback_applied,
        patch_protection_remaining_hours,
        champions,
    };

    if !response.champions.is_empty() {
        if let Some(dir) = arena_dir {
            let path =
                cache::arena_tier_list_file_path(dir, manual_patch, patch_48h_protection, locale);
            let _ = crate::storage::write_lz4_envelope(&path, &effective_patch, &response);
        }
        if let Ok(mut guard) = cache::get_arena_tier_cache().write() {
            guard.insert(cache_key, (Instant::now(), response.clone()));
        }
    }

    Ok(response)
}

pub async fn fetch_arena_tier_list(
    client: &reqwest::Client,
    patch: Option<&str>,
    ddragon: &DataDragonCache,
) -> Result<ArenaTierListResponse, String> {
    let prefs = crate::lcu_injector::get_user_preferences();
    let dir = crate::storage::global_paths().map(|p| p.opgg_arena_dir.as_path());
    fetch_arena_tier_list_with_base_and_dir(
        client,
        OPGG_REST_API_BASE,
        patch,
        ddragon,
        prefs.patch_48h_protection,
        crate::storage::now_unix_secs(),
        prefs.language,
        dir,
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
