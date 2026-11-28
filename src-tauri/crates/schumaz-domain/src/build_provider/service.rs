use super::analytics::{
    RestAnalysisParseOptions, fetch_opgg_mcp_depth_items, fetch_opgg_versions_from_base,
    parse_opgg_analysis_with_options, resolve_champion_most_played_lane,
};
use super::cache::{
    build_provider_caches, load_disk_build_envelope, store_disk_build, store_ram_cached_build,
    try_get_fresh_disk_build, try_get_ram_cached_build,
};
use super::ddragon::enrich_build_with_ddragon_for_queue;
use super::fallback::champion_default_lane;
use super::models::{ChampionBuild, DataDragonCache};
use super::query::BuildQuery;
use super::service_entry_points::resolve_arena_build_or_fallback;
#[cfg(test)]
use super::service_entry_points::*;
pub use super::service_helpers::FastSpellRecommendation;
use super::service_helpers::{
    FallbackBuild, MCP_DEPTH_TIMEOUT, extract_fast_spells_from_rest,
    fetch_rest_with_patch_protection, resolve_build_fallback, send_build_fast_spells,
    spawn_lane_roster_lookup,
};
use super::spells::ddragon_mode_for_queue;
use crate::models::{is_arena_queue, queue_supports_runes, queue_supports_spells};
use std::sync::Mutex;
use std::time::Instant;

/// Core build resolution pipeline backed by `BuildQuery`, `TieredCache`, and `Singleflight`.
pub async fn resolve_recommended_build(
    client: &reqwest::Client,
    query: &BuildQuery<'_>,
    ddragon_cache: &DataDragonCache,
    mut fast_spells_tx: Option<tokio::sync::oneshot::Sender<FastSpellRecommendation>>,
) -> Result<ChampionBuild, String> {
    let queue_id = query.canonical_queue_id();
    let started = Instant::now();
    let detail = format!("champ_id={} queue_id={queue_id}", query.champion_id);
    let champ_name = ddragon_cache
        .champions
        .get(&query.champion_id)
        .map(|c| c.key_name.as_str())
        .unwrap_or("Unknown");

    if is_arena_queue(queue_id) {
        let _ = fast_spells_tx.take();
        return Ok(resolve_arena_build_or_fallback(
            client,
            query,
            champ_name,
            ddragon_cache,
            started,
            &detail,
        )
        .await);
    }

    let cache_key = query.cache_key();
    if let Some(build) = try_get_ram_cached_build(&cache_key, query, ddragon_cache) {
        send_build_fast_spells(&mut fast_spells_tx, query.champion_id, &build);
        crate::logging::record_data_resolution(
            "build",
            "ram_hit",
            started.elapsed().as_millis(),
            &detail,
        );
        return Ok(build);
    }

    let fast_tx_slot = Mutex::new(fast_spells_tx);
    let result = build_provider_caches()
        .builds
        .coalesce(cache_key.clone(), || async {
            if let Some(build) = try_get_ram_cached_build(&cache_key, query, ddragon_cache) {
                return Ok(build);
            }

            let mut leader_fast_tx = fast_tx_slot.lock().ok().and_then(|mut g| g.take());
            let is_aram = query.is_aram();
            let mode = super::fallback::analytics_rest_mode_for_queue(queue_id);
            let spell_mode = ddragon_mode_for_queue(queue_id);
            let supports_runes = queue_supports_runes(queue_id);
            let supports_spells = queue_supports_spells(queue_id);
            let target_region = query.target_region();
            let target_tier = query.target_tier();
            let manual_patch_trimmed = query.manual_patch_trimmed();

            let disk_env = load_disk_build_envelope(query);
            if let Some(ref env) = disk_env
                && let Some(build) = try_get_fresh_disk_build(env, query, ddragon_cache)
            {
                send_build_fast_spells(&mut leader_fast_tx, query.champion_id, &build);
                store_ram_cached_build(cache_key.clone(), &build.active_patch, &build);
                crate::logging::record_data_resolution(
                    "build",
                    "disk_hit",
                    started.elapsed().as_millis(),
                    &detail,
                );
                return Ok(build);
            }

            let available_versions =
                fetch_opgg_versions_from_base(client, query.rest_base, target_region).await;
            let initial_patch = if !manual_patch_trimmed.is_empty() {
                manual_patch_trimmed.to_string()
            } else {
                available_versions.first().cloned().unwrap_or_default()
            };

            let target_position: Option<String> = if is_aram {
                None
            } else if let Some(p) = query.position
                && !p.trim().is_empty()
                && p != "auto"
            {
                Some(p.to_lowercase())
            } else {
                let initial_patch_opt =
                    (!initial_patch.is_empty()).then_some(initial_patch.as_str());
                let resolved = resolve_champion_most_played_lane(
                    client,
                    query.champion_id,
                    target_region,
                    target_tier,
                    initial_patch_opt,
                )
                .await
                .unwrap_or_else(|_| champion_default_lane(champ_name).to_string());
                Some(resolved)
            };
            let is_jungle = target_position
                .as_deref()
                .is_some_and(|p| p.eq_ignore_ascii_case("jungle"));

            let mcp_client = client.clone();
            let mcp_url = query.mcp_endpoint.to_string();
            let mcp_champ = champ_name.to_string();
            let mcp_mode = mode.to_string();
            let mcp_pos = target_position.clone();
            let mcp_tier = target_tier.to_string();
            let mcp_depth_handle = tokio::spawn(async move {
                tokio::time::timeout(
                    MCP_DEPTH_TIMEOUT,
                    fetch_opgg_mcp_depth_items(
                        &mcp_client,
                        &mcp_url,
                        &mcp_champ,
                        &mcp_mode,
                        mcp_pos.as_deref(),
                        &mcp_tier,
                    ),
                )
                .await
                .ok()
                .and_then(|res| res.ok())
            });

            let lane_roster_handle = spawn_lane_roster_lookup(
                client,
                query,
                target_position.as_ref(),
                (!initial_patch.is_empty()).then_some(initial_patch.as_str()),
            );
            let fetch_outcome = fetch_rest_with_patch_protection(
                client,
                query,
                mode,
                target_position.as_deref(),
                &available_versions,
                initial_patch,
            )
            .await;

            if supports_spells
                && let Ok(ref json_res) = fetch_outcome.result
                && let Some(fast) = extract_fast_spells_from_rest(
                    json_res,
                    query.champion_id,
                    is_jungle,
                    queue_id,
                    ddragon_cache,
                )
                && let Some(tx) = leader_fast_tx.take()
            {
                let _ = tx.send(fast);
            }

            let mcp_depth_opt = mcp_depth_handle.await.ok().flatten();
            let lane_roster_opt = match lane_roster_handle {
                Some(h) => h.await.ok().flatten(),
                None => None,
            };
            let mut fetched_ok = false;
            let parsed_build = match fetch_outcome.result {
                Ok(ref json_res) => parse_opgg_analysis_with_options(
                    json_res,
                    RestAnalysisParseOptions {
                        champion_id: query.champion_id,
                        champion_name: champ_name,
                        supports_runes,
                        spell_mode: Some(spell_mode),
                        ddragon: Some(ddragon_cache),
                        spells_catalog: Some(&ddragon_cache.spells),
                        allowed_lane_champions: lane_roster_opt.as_ref(),
                        position_override: target_position.as_deref(),
                        mcp_depth_override: mcp_depth_opt,
                    },
                )
                .ok(),
                Err(_) => None,
            };

            let mut build = if let Some(b) = parsed_build {
                fetched_ok = true;
                b
            } else {
                match resolve_build_fallback(
                    client,
                    query,
                    ddragon_cache,
                    disk_env,
                    &mut leader_fast_tx,
                )
                .await
                {
                    FallbackBuild::Stale(stale) => {
                        crate::logging::record_data_resolution(
                            "build",
                            "stale_fallback",
                            started.elapsed().as_millis(),
                            &detail,
                        );
                        return Ok(stale);
                    }
                    FallbackBuild::Mcp(mcp_fallback) => mcp_fallback,
                }
            };

            build.active_position = target_position.unwrap_or_default();
            build.active_tier = fetch_outcome.effective_tier;
            build.active_region = target_region.to_string();
            build.active_patch = fetch_outcome.effective_patch.clone();
            build.available_patches = available_versions;
            build.patch_fallback_applied = fetch_outcome.patch_fallback_applied;
            build.patch_protection_remaining_hours = fetch_outcome.patch_protection_remaining_hours;
            build.supports_runes = supports_runes;
            build.supports_spells = supports_spells;

            enrich_build_with_ddragon_for_queue(&mut build, ddragon_cache, queue_id);
            if supports_spells && let Some(tx) = leader_fast_tx.take() {
                let _ = tx.send(FastSpellRecommendation {
                    champion_id: query.champion_id,
                    spell1_id: build.spell1_id,
                    spell2_id: build.spell2_id,
                    is_jungle,
                });
            }

            if fetched_ok && let Some(dir) = query.builds_dir {
                store_disk_build(dir, query, &fetch_outcome.effective_patch, &build);
            }
            store_ram_cached_build(cache_key.clone(), &fetch_outcome.effective_patch, &build);
            let source = if fetched_ok {
                "network"
            } else {
                "stale_fallback"
            };
            crate::logging::record_data_resolution(
                "build",
                source,
                started.elapsed().as_millis(),
                &detail,
            );
            Ok(build)
        })
        .await;

    if let Ok(ref build) = result
        && let Ok(mut guard) = fast_tx_slot.lock()
        && guard.is_some()
    {
        send_build_fast_spells(&mut guard, query.champion_id, build);
    }

    result
}

#[cfg(test)]
#[path = "service_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "service_patch_tests.rs"]
mod patch_tests;

#[cfg(test)]
#[path = "service_spells_tests.rs"]
mod spells_tests;

#[cfg(test)]
#[path = "service_irelia_tests.rs"]
mod irelia_tests;
