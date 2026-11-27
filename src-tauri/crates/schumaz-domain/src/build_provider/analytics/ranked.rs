mod cache;
mod parser;

pub use cache::*;
pub use parser::*;

use super::client::{
    OPGG_REST_API_BASE, fetch_opgg_rest_champion_analysis_from_base, fetch_opgg_versions_from_base,
    peek_latest_cached_opgg_version,
};
use crate::build_provider::models::DataDragonCache;
use crate::build_provider::patch::{extract_patch_trend_timestamps, resolve_effective_patch};
use crate::models::{AppLocale, RankedTierListResponse};
use std::path::Path;
use std::time::Instant;

const TREND_PROBE_CHAMPION_ID: i32 = 103; // Ahri (high play-rate reference for patch release timestamp)

#[allow(clippy::too_many_arguments)]
pub async fn fetch_ranked_tier_list_with_base_and_dir(
    client: &reqwest::Client,
    rest_base: &str,
    region: Option<&str>,
    tier: Option<&str>,
    patch: Option<&str>,
    ddragon: &DataDragonCache,
    patch_48h_protection: bool,
    now_unix: u64,
    locale: AppLocale,
    ranked_dir: Option<&Path>,
) -> Result<RankedTierListResponse, String> {
    let started = Instant::now();
    let target_region = region
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("global");
    let target_tier = tier
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("emerald_plus");
    let manual_patch = patch.map(str::trim).filter(|s| !s.is_empty());

    let cache_key = format!(
        "{}:{}:{}:{}:{}:{}",
        rest_base,
        target_region,
        target_tier,
        manual_patch.unwrap_or(""),
        patch_48h_protection,
        locale.as_str()
    );

    if let Ok(guard) = cache::get_ranked_cache().read()
        && let Some((ts, cached)) = guard.get(&cache_key)
        && ts.elapsed() < cache::RANKED_CACHE_TTL
    {
        let mut resp = cached.clone();
        cache::rehydrate_ranked_names(&mut resp, ddragon);
        crate::logging::record_data_resolution(
            "ranked_tier_list",
            "ram_hit",
            started.elapsed().as_millis(),
            "",
        );
        return Ok(resp);
    }

    let disk_env = ranked_dir.and_then(|dir| {
        let path = cache::ranked_tier_list_file_path(
            dir,
            target_region,
            target_tier,
            manual_patch,
            patch_48h_protection,
            locale,
        );
        crate::storage::read_lz4_envelope::<RankedTierListResponse>(&path)
    });

    if let Some(ref env) = disk_env
        && !env.payload.champions.is_empty()
        && crate::storage::is_envelope_fresh(
            env.created_at_unix,
            crate::storage::OPGG_CACHE_TTL_SECS,
            now_unix,
        )
    {
        let known_latest = peek_latest_cached_opgg_version(target_region);
        let patch_valid = match (manual_patch, known_latest.as_deref()) {
            (Some(p), _) => env.patch == p,
            (None, Some(latest)) if !env.payload.patch_fallback_applied => env.patch == latest,
            _ => true,
        };
        if patch_valid {
            let mut resp = env.payload.clone();
            cache::rehydrate_ranked_names(&mut resp, ddragon);
            if let Ok(mut guard) = cache::get_ranked_cache().write() {
                guard.insert(cache_key.clone(), (Instant::now(), resp.clone()));
            }
            crate::logging::record_data_resolution(
                "ranked_tier_list",
                "disk_hit",
                started.elapsed().as_millis(),
                "",
            );
            return Ok(resp);
        }
    }

    let available_versions = fetch_opgg_versions_from_base(client, rest_base, target_region).await;

    let mut effective_patch = manual_patch
        .map(str::to_string)
        .unwrap_or_else(|| available_versions.first().cloned().unwrap_or_default());
    let mut patch_fallback_applied = false;
    let mut patch_protection_remaining_hours: Option<u32> = None;

    let fetch_res: Result<serde_json::Value, String> = async {
        if let Some(user_patch) = manual_patch {
            effective_patch = user_patch.to_string();
            cache::fetch_ranked_json(
                client,
                rest_base,
                target_region,
                target_tier,
                Some(user_patch),
            )
            .await
        } else {
            let probe_version = (!effective_patch.is_empty()).then_some(effective_patch.as_str());
            let probe_json = cache::fetch_ranked_json(
                client,
                rest_base,
                target_region,
                target_tier,
                probe_version,
            )
            .await?;

            let (meta_version, match_count) = parser::extract_ranked_meta_info(&probe_json);
            if effective_patch.is_empty()
                && let Some(v) = meta_version
            {
                effective_patch = v;
            }

            if patch_48h_protection
                && available_versions.len() >= 2
                && match_count.unwrap_or(0) < 1000
            {
                let probe_analysis = fetch_opgg_rest_champion_analysis_from_base(
                    client,
                    rest_base,
                    target_region,
                    "ranked",
                    TREND_PROBE_CHAMPION_ID,
                    Some("mid"),
                    target_tier,
                    Some(&effective_patch),
                )
                .await;

                if let Ok(analysis_json) = probe_analysis {
                    let latest_patch = available_versions
                        .first()
                        .map(String::as_str)
                        .unwrap_or(&effective_patch);
                    let prev_patch = available_versions.get(1).map(String::as_str);
                    let (created_at_iso, prev_patch_created_at_iso) =
                        extract_patch_trend_timestamps(&analysis_json, latest_patch, prev_patch);

                    let selection = resolve_effective_patch(
                        &available_versions,
                        created_at_iso,
                        prev_patch_created_at_iso,
                        match_count,
                        now_unix,
                        None,
                        patch_48h_protection,
                    );
                    effective_patch = selection.effective_patch;
                    patch_fallback_applied = selection.fallback_applied;
                    patch_protection_remaining_hours = selection.remaining_hours;

                    if patch_fallback_applied {
                        cache::fetch_ranked_json(
                            client,
                            rest_base,
                            target_region,
                            target_tier,
                            Some(&effective_patch),
                        )
                        .await
                    } else {
                        Ok(probe_json)
                    }
                } else {
                    Ok(probe_json)
                }
            } else {
                Ok(probe_json)
            }
        }
    }
    .await;

    let final_json = match fetch_res {
        Ok(json) => json,
        Err(err) => {
            // Stale-While-Error offline fallback
            if let Some(env) = disk_env
                && !env.payload.champions.is_empty()
            {
                let mut resp = env.payload;
                cache::rehydrate_ranked_names(&mut resp, ddragon);
                crate::logging::record_data_resolution(
                    "ranked_tier_list",
                    "stale_fallback",
                    started.elapsed().as_millis(),
                    "",
                );
                return Ok(resp);
            }
            return Err(err);
        }
    };

    let champions = parser::parse_ranked_tier_list(&final_json, ddragon)?;
    let response = RankedTierListResponse {
        region: target_region.to_string(),
        tier: target_tier.to_string(),
        active_patch: effective_patch.clone(),
        available_patches: available_versions,
        patch_fallback_applied,
        patch_protection_remaining_hours,
        champions,
    };

    if !response.champions.is_empty() {
        let positions_dir = (rest_base == OPGG_REST_API_BASE)
            .then(|| crate::storage::global_paths().map(|p| p.opgg_positions_dir.as_path()))
            .flatten();
        let pos_patch = (!effective_patch.is_empty()).then_some(effective_patch.as_str());
        super::positions::seed_champion_positions_cache(
            rest_base, target_region, target_tier, pos_patch, &final_json, positions_dir,
        );
        if let Some(dir) = ranked_dir {
            let path = cache::ranked_tier_list_file_path(
                dir,
                target_region,
                target_tier,
                manual_patch,
                patch_48h_protection,
                locale,
            );
            let _ = crate::storage::write_lz4_envelope(&path, &effective_patch, &response);
        }
        if let Ok(mut guard) = cache::get_ranked_cache().write() {
            guard.insert(cache_key, (Instant::now(), response.clone()));
        }
    }

    crate::logging::record_data_resolution(
        "ranked_tier_list",
        "network",
        started.elapsed().as_millis(),
        "",
    );
    Ok(response)
}

#[allow(clippy::too_many_arguments)]
pub async fn fetch_ranked_tier_list_with_base(
    client: &reqwest::Client,
    rest_base: &str,
    region: Option<&str>,
    tier: Option<&str>,
    patch: Option<&str>,
    ddragon: &DataDragonCache,
    patch_48h_protection: bool,
    now_unix: u64,
) -> Result<RankedTierListResponse, String> {
    let locale = crate::lcu_injector::get_active_locale();
    let dir = if rest_base == OPGG_REST_API_BASE {
        crate::storage::global_paths().map(|p| p.opgg_ranked_dir.as_path())
    } else {
        None
    };
    fetch_ranked_tier_list_with_base_and_dir(
        client,
        rest_base,
        region,
        tier,
        patch,
        ddragon,
        patch_48h_protection,
        now_unix,
        locale,
        dir,
    )
    .await
}

pub async fn fetch_ranked_tier_list(
    client: &reqwest::Client,
    region: Option<&str>,
    tier: Option<&str>,
    patch: Option<&str>,
    ddragon: &DataDragonCache,
) -> Result<RankedTierListResponse, String> {
    let prefs = crate::lcu_injector::get_user_preferences();
    let now_unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    fetch_ranked_tier_list_with_base(
        client,
        OPGG_REST_API_BASE,
        region,
        tier,
        patch,
        ddragon,
        prefs.patch_48h_protection,
        now_unix,
    )
    .await
}

#[cfg(test)]
#[path = "ranked/parser_tests.rs"]
mod parser_tests;

#[cfg(test)]
#[path = "ranked/service_tests.rs"]
mod service_tests;
