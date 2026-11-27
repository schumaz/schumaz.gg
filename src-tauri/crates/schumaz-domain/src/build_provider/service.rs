use super::analytics::{
    fetch_opgg_mcp_depth_items, fetch_opgg_versions_from_base,
    parse_opgg_analysis_payload_with_ddragon, resolve_champion_most_played_lane,
};
use super::cache::{
    load_disk_build_envelope, make_build_cache_key, store_disk_build, store_ram_cached_build,
    try_get_fresh_disk_build, try_get_ram_cached_build,
};
use super::ddragon::enrich_build_with_ddragon_for_queue;
use super::fallback::champion_default_lane;
use super::models::{ChampionBuild, DataDragonCache};
use super::service_entry_points::resolve_arena_build_or_fallback;
#[cfg(test)]
use super::service_entry_points::*;
pub use super::service_helpers::FastSpellRecommendation;
use super::service_helpers::{
    FallbackBuild, MCP_DEPTH_TIMEOUT, extract_fast_spells_from_rest,
    fetch_rest_with_patch_protection, merge_mcp_depth_into_json, resolve_build_fallback,
    send_build_fast_spells, spawn_lane_roster_lookup,
};
use super::spells::ddragon_mode_for_queue;
use crate::models::{is_arena_queue, queue_supports_runes, queue_supports_spells};
use std::path::Path;
use std::time::Instant;

#[allow(clippy::too_many_arguments)]
pub async fn get_recommended_build_with_filters_and_endpoints_with_dir(
    client: &reqwest::Client,
    champion_id: i32,
    queue_id: i32,
    position: Option<&str>,
    tier: Option<&str>,
    region: Option<&str>,
    patch: Option<&str>,
    ddragon_cache: &DataDragonCache,
    rest_base: &str,
    mcp_endpoint: &str,
    mut fast_spells_tx: Option<tokio::sync::oneshot::Sender<FastSpellRecommendation>>,
    patch_48h_protection: bool,
    builds_dir: Option<&Path>,
) -> Result<ChampionBuild, String> {
    let queue_id = crate::models::canonical_build_queue_id(queue_id);
    let started = Instant::now();
    let detail = format!("champ_id={champion_id} queue_id={queue_id}");
    let champ_name = ddragon_cache
        .champions
        .get(&champion_id)
        .map(|c| c.key_name.as_str())
        .unwrap_or("Unknown");
    let prefs = crate::lcu_injector::get_user_preferences();
    let active_locale = prefs.language;

    if is_arena_queue(queue_id) {
        let _ = fast_spells_tx.take();
        return Ok(resolve_arena_build_or_fallback(
            client,
            rest_base,
            champion_id,
            champ_name,
            queue_id,
            patch,
            ddragon_cache,
            patch_48h_protection,
            active_locale,
            builds_dir,
            started,
            &detail,
        )
        .await);
    }

    let is_aram = queue_id == crate::models::QUEUE_ARAM
        || queue_id == crate::models::QUEUE_ARAM_MAYHEM;
    let mode = super::fallback::analytics_rest_mode_for_queue(queue_id);
    let spell_mode = ddragon_mode_for_queue(queue_id);
    let supports_runes = queue_supports_runes(queue_id);
    let supports_spells = queue_supports_spells(queue_id);
    let target_region = region.unwrap_or("global");
    let target_tier = match tier.map(str::trim).filter(|t| !t.is_empty()) {
        Some(explicit) => explicit,
        None if is_aram => "all",
        None => "emerald_plus",
    };
    let pos_cache_key = if is_aram {
        ""
    } else {
        position.unwrap_or("").trim()
    };
    let manual_patch_trimmed = patch.unwrap_or("").trim();

    let build_cache_key = make_build_cache_key(
        active_locale,
        rest_base,
        mcp_endpoint,
        champion_id,
        queue_id,
        pos_cache_key,
        target_tier,
        target_region,
        manual_patch_trimmed,
        patch_48h_protection,
    );

    if let Some(build) = try_get_ram_cached_build(
        &build_cache_key,
        queue_id,
        ddragon_cache,
        prefs.flash_key,
        active_locale,
    ) {
        send_build_fast_spells(&mut fast_spells_tx, champion_id, &build);
        crate::logging::record_data_resolution(
            "build",
            "ram_hit",
            started.elapsed().as_millis(),
            &detail,
        );
        return Ok(build);
    }

    let disk_env = load_disk_build_envelope(
        builds_dir,
        champion_id,
        queue_id,
        pos_cache_key,
        target_tier,
        target_region,
        manual_patch_trimmed,
        patch_48h_protection,
        active_locale,
    );

    if let Some(ref env) = disk_env
        && let Some(build) = try_get_fresh_disk_build(
            env,
            queue_id,
            target_region,
            manual_patch_trimmed,
            ddragon_cache,
            prefs.flash_key,
            active_locale,
        )
    {
        send_build_fast_spells(&mut fast_spells_tx, champion_id, &build);
        store_ram_cached_build(build_cache_key, &build);
        crate::logging::record_data_resolution(
            "build",
            "disk_hit",
            started.elapsed().as_millis(),
            &detail,
        );
        return Ok(build);
    }

    let available_versions = fetch_opgg_versions_from_base(client, rest_base, target_region).await;
    let initial_patch = if !manual_patch_trimmed.is_empty() {
        manual_patch_trimmed.to_string()
    } else {
        available_versions.first().cloned().unwrap_or_default()
    };

    let target_position: Option<String> = if is_aram {
        None
    } else if let Some(p) = position
        && !p.trim().is_empty()
        && p != "auto"
    {
        Some(p.to_lowercase())
    } else {
        let initial_patch_opt = (!initial_patch.is_empty()).then_some(initial_patch.as_str());
        let resolved = resolve_champion_most_played_lane(
            client,
            champion_id,
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
    let mcp_url = mcp_endpoint.to_string();
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

    let lane_roster_handle = spawn_lane_roster_lookup(client, rest_base, target_position.as_ref(), target_region, target_tier, (!initial_patch.is_empty()).then_some(initial_patch.as_str()));
    let fetch_outcome = fetch_rest_with_patch_protection(
        client,
        rest_base,
        target_region,
        mode,
        champion_id,
        target_position.as_deref(),
        target_tier,
        manual_patch_trimmed,
        &available_versions,
        initial_patch,
        patch_48h_protection,
    )
    .await;
    let effective_patch = fetch_outcome.effective_patch;
    let effective_tier = fetch_outcome.effective_tier;
    let patch_fallback_applied = fetch_outcome.patch_fallback_applied;
    let patch_protection_remaining_hours = fetch_outcome.patch_protection_remaining_hours;

    if supports_spells
        && let Ok(ref json_res) = fetch_outcome.result
        && let Some(fast) =
            extract_fast_spells_from_rest(json_res, champion_id, is_jungle, queue_id, ddragon_cache)
        && let Some(tx) = fast_spells_tx.take()
    {
        let _ = tx.send(fast);
    }

    let mcp_depth_opt = mcp_depth_handle.await.ok().flatten();
    let lane_roster_opt = match lane_roster_handle { Some(h) => h.await.ok().flatten(), None => None };
    let mut fetched_ok = false;
    let parsed_build = match fetch_outcome.result {
        Ok(mut json_res) => {
            merge_mcp_depth_into_json(&mut json_res, target_position.as_ref(), mcp_depth_opt);
            parse_opgg_analysis_payload_with_ddragon(
                &json_res,
                champion_id,
                champ_name,
                supports_runes,
                Some(spell_mode),
                Some(ddragon_cache),
                Some(&ddragon_cache.spells),
                lane_roster_opt.as_ref(),
            )
            .ok()
        }
        Err(_) => None,
    };

    let mut build = if let Some(b) = parsed_build {
        fetched_ok = true;
        b
    } else {
        match resolve_build_fallback(
            client,
            mcp_endpoint,
            champion_id,
            queue_id,
            ddragon_cache,
            prefs.flash_key,
            active_locale,
            disk_env,
            &mut fast_spells_tx,
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
    build.active_tier = effective_tier;
    build.active_region = target_region.to_string();
    build.active_patch = effective_patch.clone();
    build.available_patches = available_versions;
    build.patch_fallback_applied = patch_fallback_applied;
    build.patch_protection_remaining_hours = patch_protection_remaining_hours;
    build.supports_runes = supports_runes;
    build.supports_spells = supports_spells;

    enrich_build_with_ddragon_for_queue(&mut build, ddragon_cache, queue_id);
    if supports_spells && let Some(tx) = fast_spells_tx.take() {
        let _ = tx.send(FastSpellRecommendation {
            champion_id,
            spell1_id: build.spell1_id,
            spell2_id: build.spell2_id,
            is_jungle,
        });
    }

    if fetched_ok && let Some(dir) = builds_dir {
        store_disk_build(
            dir,
            champion_id,
            queue_id,
            pos_cache_key,
            target_tier,
            target_region,
            manual_patch_trimmed,
            patch_48h_protection,
            active_locale,
            &effective_patch,
            &build,
        );
    }
    store_ram_cached_build(build_cache_key, &build);
    let source = if fetched_ok {
        "network"
    } else {
        "stale_fallback"
    };
    crate::logging::record_data_resolution("build", source, started.elapsed().as_millis(), &detail);
    Ok(build)
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
