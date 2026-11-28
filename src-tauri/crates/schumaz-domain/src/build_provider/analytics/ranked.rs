mod cache;
mod parser;

pub use cache::*;
pub use parser::*;

use super::client::{
    OPGG_REST_API_BASE, fetch_opgg_rest_champion_analysis_req, fetch_opgg_versions_from_base,
    peek_latest_cached_opgg_version,
};
use crate::build_provider::cache::build_provider_caches;
use crate::build_provider::models::DataDragonCache;
use crate::build_provider::patch::{extract_patch_trend_timestamps, resolve_effective_patch};
use crate::build_provider::query::RestAnalysisRequest;
use crate::models::{AppLocale, RankedTierListResponse};
use std::path::Path;
use std::time::Instant;

const TREND_PROBE_CHAMPION_ID: i32 = 103; // Ahri (high play-rate reference for patch release timestamp)

#[derive(Debug, Clone, Copy)]
pub struct RankedTierQuery<'a> {
    pub rest_base: &'a str,
    pub region: Option<&'a str>,
    pub tier: Option<&'a str>,
    pub patch: Option<&'a str>,
    pub patch_48h_protection: bool,
    pub now_unix: u64,
    pub locale: AppLocale,
    pub ranked_dir: Option<&'a Path>,
}

pub async fn fetch_ranked_tier_list_with_query(
    client: &reqwest::Client,
    query: &RankedTierQuery<'_>,
    ddragon: &DataDragonCache,
) -> Result<RankedTierListResponse, String> {
    let started = Instant::now();
    let target_region = query
        .region
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("global");
    let target_tier = query
        .tier
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("emerald_plus");
    let manual_patch = query.patch.map(str::trim).filter(|s| !s.is_empty());

    let cache_key = format!(
        "{}:{}:{}:{}:{}:{}",
        query.rest_base,
        target_region,
        target_tier,
        manual_patch.unwrap_or(""),
        query.patch_48h_protection,
        query.locale.as_str()
    );
    let cache = &build_provider_caches().ranked_tiers;

    if let Some(entry) = cache.get_ram(&cache_key, Some(cache::RANKED_CACHE_TTL)) {
        let mut resp = entry.payload;
        cache::rehydrate_ranked_names(&mut resp, ddragon);
        crate::logging::record_data_resolution(
            "ranked_tier_list",
            "ram_hit",
            started.elapsed().as_millis(),
            "",
        );
        return Ok(resp);
    }

    cache
        .coalesce(cache_key.clone(), || async {
            if let Some(entry) = cache.get_ram(&cache_key, Some(cache::RANKED_CACHE_TTL)) {
                let mut resp = entry.payload;
                cache::rehydrate_ranked_names(&mut resp, ddragon);
                return Ok(resp);
            }

            let disk_env = query.ranked_dir.and_then(|dir| {
                let path = cache::ranked_tier_list_file_path(
                    dir,
                    target_region,
                    target_tier,
                    manual_patch,
                    query.patch_48h_protection,
                    query.locale,
                );
                cache.read_disk(&path)
            });

            if let Some(ref env) = disk_env
                && !env.payload.champions.is_empty()
                && crate::storage::is_envelope_fresh(
                    env.created_at_unix,
                    crate::storage::OPGG_CACHE_TTL_SECS,
                    query.now_unix,
                )
            {
                let known_latest = peek_latest_cached_opgg_version(target_region);
                let patch_valid = match (manual_patch, known_latest.as_deref()) {
                    (Some(p), _) => env.patch == p,
                    (None, Some(latest)) if !env.payload.patch_fallback_applied => {
                        env.patch == latest
                    }
                    _ => true,
                };
                if patch_valid {
                    let mut resp = env.payload.clone();
                    cache::rehydrate_ranked_names(&mut resp, ddragon);
                    cache.insert_ram(cache_key.clone(), env.patch.clone(), resp.clone());
                    crate::logging::record_data_resolution(
                        "ranked_tier_list",
                        "disk_hit",
                        started.elapsed().as_millis(),
                        "",
                    );
                    return Ok(resp);
                }
            }

            let available_versions =
                fetch_opgg_versions_from_base(client, query.rest_base, target_region).await;
            let mut effective_patch = manual_patch
                .map(str::to_string)
                .unwrap_or_else(|| available_versions.first().cloned().unwrap_or_default());
            let mut patch_fallback_applied = false;
            let mut patch_protection_remaining_hours: Option<u32> = None;

            let fetch_res: Result<parser::RestRankedResponse, String> = async {
                if let Some(user_patch) = manual_patch {
                    effective_patch = user_patch.to_string();
                    cache::fetch_ranked_response(
                        client,
                        query.rest_base,
                        target_region,
                        target_tier,
                        Some(user_patch),
                    )
                    .await
                } else {
                    let probe_version =
                        (!effective_patch.is_empty()).then_some(effective_patch.as_str());
                    let probe_resp = cache::fetch_ranked_response(
                        client,
                        query.rest_base,
                        target_region,
                        target_tier,
                        probe_version,
                    )
                    .await?;

                    let (meta_version, match_count) =
                        parser::extract_ranked_meta_from_resp(&probe_resp);
                    if effective_patch.is_empty()
                        && let Some(v) = meta_version
                    {
                        effective_patch = v;
                    }

                    if query.patch_48h_protection
                        && available_versions.len() >= 2
                        && match_count.unwrap_or(0) < 1000
                    {
                        let probe_analysis = fetch_opgg_rest_champion_analysis_req(
                            client,
                            &RestAnalysisRequest {
                                rest_base: query.rest_base,
                                region: target_region,
                                mode: "ranked",
                                champion_id: TREND_PROBE_CHAMPION_ID,
                                position: Some("mid"),
                                tier: target_tier,
                                version: Some(&effective_patch),
                            },
                        )
                        .await;

                        if let Ok(analysis_json) = probe_analysis {
                            let latest_patch = available_versions
                                .first()
                                .map(String::as_str)
                                .unwrap_or(&effective_patch);
                            let prev_patch = available_versions.get(1).map(String::as_str);
                            let (created_at_iso, prev_patch_created_at_iso) =
                                extract_patch_trend_timestamps(
                                    &analysis_json,
                                    latest_patch,
                                    prev_patch,
                                );

                            let selection = resolve_effective_patch(
                                &available_versions,
                                created_at_iso,
                                prev_patch_created_at_iso,
                                match_count,
                                query.now_unix,
                                None,
                                query.patch_48h_protection,
                            );
                            effective_patch = selection.effective_patch;
                            patch_fallback_applied = selection.fallback_applied;
                            patch_protection_remaining_hours = selection.remaining_hours;

                            if patch_fallback_applied {
                                cache::fetch_ranked_response(
                                    client,
                                    query.rest_base,
                                    target_region,
                                    target_tier,
                                    Some(&effective_patch),
                                )
                                .await
                            } else {
                                Ok(probe_resp)
                            }
                        } else {
                            Ok(probe_resp)
                        }
                    } else {
                        Ok(probe_resp)
                    }
                }
            }
            .await;

            let final_resp = match fetch_res {
                Ok(resp) => resp,
                Err(err) => {
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

            let champions = parser::parse_ranked_tier_list_from_resp(&final_resp, ddragon);
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
                let positions_dir = (query.rest_base == OPGG_REST_API_BASE)
                    .then(|| {
                        crate::storage::global_paths().map(|p| p.opgg_positions_dir.as_path())
                    })
                    .flatten();
                let pos_patch =
                    (!effective_patch.is_empty()).then_some(effective_patch.as_str());
                let positions_map = parser::extract_positions_map_from_ranked_resp(&final_resp);
                super::positions::seed_champion_positions_map(
                    query.rest_base,
                    target_region,
                    target_tier,
                    pos_patch,
                    positions_map,
                    positions_dir,
                );
                if let Some(dir) = query.ranked_dir {
                    let path = cache::ranked_tier_list_file_path(
                        dir,
                        target_region,
                        target_tier,
                        manual_patch,
                        query.patch_48h_protection,
                        query.locale,
                    );
                    let _ = cache.write_disk(&path, &effective_patch, &response);
                }
                cache.insert_ram(cache_key.clone(), &effective_patch, response.clone());
            }

            crate::logging::record_data_resolution(
                "ranked_tier_list",
                "network",
                started.elapsed().as_millis(),
                "",
            );
            Ok(response)
        })
        .await
}

pub async fn fetch_ranked_tier_list(
    client: &reqwest::Client,
    region: Option<&str>,
    tier: Option<&str>,
    patch: Option<&str>,
    ddragon: &DataDragonCache,
) -> Result<RankedTierListResponse, String> {
    let prefs = crate::storage::get_user_preferences();
    let dir = crate::storage::global_paths().map(|p| p.opgg_ranked_dir.as_path());
    fetch_ranked_tier_list_with_query(
        client,
        &RankedTierQuery {
            rest_base: OPGG_REST_API_BASE,
            region,
            tier,
            patch,
            patch_48h_protection: prefs.patch_48h_protection,
            now_unix: crate::storage::now_unix_secs(),
            locale: prefs.language,
            ranked_dir: dir,
        },
        ddragon,
    )
    .await
}

#[cfg(test)]
#[path = "ranked/parser_tests.rs"]
mod parser_tests;

#[cfg(test)]
#[path = "ranked/service_tests.rs"]
mod service_tests;
