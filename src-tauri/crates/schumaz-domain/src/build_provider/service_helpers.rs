use super::analytics::{
    fetch_opgg_rest_champion_analysis_req, get_recommended_build_with_endpoint,
};
use super::ddragon::enrich_build_with_ddragon_for_queue_with_flash_key_and_locale;
use super::models::{ChampionBuild, DataDragonCache};
use super::patch::{extract_patch_trend_timestamps, resolve_effective_patch};
use super::query::{BuildQuery, RestAnalysisRequest};
use super::spells::{ddragon_mode_for_queue, extract_summoner_spell_pairs_for_mode};
use crate::models::queue_supports_spells;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FastSpellRecommendation {
    pub champion_id: i32,
    pub spell1_id: i32,
    pub spell2_id: i32,
    pub is_jungle: bool,
}

pub(super) const MCP_DEPTH_TIMEOUT: Duration = Duration::from_millis(2500);
pub(super) const ARAM_MIN_TIER_SAMPLE: u64 = 100;

pub(super) struct RestFetchOutcome {
    pub(super) result: Result<serde_json::Value, String>,
    pub(super) effective_patch: String,
    pub(super) effective_tier: String,
    pub(super) patch_fallback_applied: bool,
    pub(super) patch_protection_remaining_hours: Option<u32>,
}

pub(super) enum FallbackBuild {
    Stale(ChampionBuild),
    Mcp(ChampionBuild),
}

fn extract_rest_play_count(json: &serde_json::Value) -> Option<u64> {
    json.pointer("/data/summary/average_stats/play")
        .or_else(|| json.pointer("/data/average_stats/play"))
        .and_then(|v| v.as_u64())
}

fn rest_payload_has_items(json: &serde_json::Value) -> bool {
    let analysis = json
        .get("data")
        .or_else(|| json.pointer("/result/data"))
        .unwrap_or(json);
    let has_non_empty = |ptr: &str| {
        analysis
            .pointer(ptr)
            .and_then(|v| v.as_array())
            .is_some_and(|arr| !arr.is_empty())
    };
    has_non_empty("/core_items")
        || has_non_empty("/items/core")
        || has_non_empty("/last_items")
        || has_non_empty("/items/situational")
}

pub(super) fn extract_fast_spells_from_rest(
    json_res: &serde_json::Value,
    champion_id: i32,
    is_jungle: bool,
    queue_id: i32,
    ddragon_cache: &DataDragonCache,
) -> Option<FastSpellRecommendation> {
    if !queue_supports_spells(queue_id) {
        return None;
    }
    let analysis = json_res
        .get("data")
        .or_else(|| json_res.pointer("/result/data"))
        .unwrap_or(json_res);
    let has_spells = analysis
        .get("summoner_spells")
        .or_else(|| analysis.get("spells"))
        .is_some();
    if !has_spells {
        return None;
    }
    let mode = ddragon_mode_for_queue(queue_id);
    let ((s1, s2), _) = extract_summoner_spell_pairs_for_mode(
        analysis,
        is_jungle,
        Some(mode),
        Some(&ddragon_cache.spells),
    );
    if s1 > 0 && s2 > 0 && s1 != s2 {
        Some(FastSpellRecommendation {
            champion_id,
            spell1_id: s1,
            spell2_id: s2,
            is_jungle,
        })
    } else {
        None
    }
}

pub(super) fn send_build_fast_spells(
    fast_spells_tx: &mut Option<tokio::sync::oneshot::Sender<FastSpellRecommendation>>,
    champion_id: i32,
    build: &ChampionBuild,
) {
    if !build.supports_spells {
        let _ = fast_spells_tx.take();
        return;
    }
    if let Some(tx) = fast_spells_tx.take() {
        let _ = tx.send(FastSpellRecommendation {
            champion_id,
            spell1_id: build.spell1_id,
            spell2_id: build.spell2_id,
            is_jungle: build.active_position.eq_ignore_ascii_case("jungle"),
        });
    }
}

pub(super) fn spawn_lane_roster_lookup(
    client: &reqwest::Client,
    query: &BuildQuery<'_>,
    target_position: Option<&String>,
    version: Option<&str>,
) -> Option<tokio::task::JoinHandle<Option<std::collections::HashSet<i32>>>> {
    let lane = target_position?.clone();
    let (c, base, reg, tier, ver) = (
        client.clone(),
        query.rest_base.to_string(),
        query.target_region().to_string(),
        query.target_tier().to_string(),
        version.map(str::to_string),
    );
    let pos_dir = (query.rest_base == super::analytics::OPGG_REST_API_BASE)
        .then(|| crate::storage::global_paths().map(|p| p.opgg_positions_dir.to_path_buf()))
        .flatten();
    Some(tokio::spawn(async move {
        let fut = super::analytics::resolve_lane_champion_roster_with_base_and_dir(
            &c, &base, &lane, &reg, &tier, ver.as_deref(), pos_dir.as_deref(),
        );
        tokio::time::timeout(MCP_DEPTH_TIMEOUT, fut).await.ok().flatten()
    }))
}

pub(super) async fn fetch_rest_with_patch_protection(
    client: &reqwest::Client,
    query: &BuildQuery<'_>,
    mode: &str,
    target_position: Option<&str>,
    available_versions: &[String],
    mut effective_patch: String,
) -> RestFetchOutcome {
    let target_region = query.target_region();
    let mut effective_tier = query.target_tier().to_string();
    let manual_patch_trimmed = query.manual_patch_trimmed();
    let mut patch_fallback_applied = false;
    let mut patch_protection_remaining_hours = None;
    let now_unix = crate::storage::now_unix_secs();

    let mut result = if manual_patch_trimmed.is_empty() {
        let probe_res = fetch_opgg_rest_champion_analysis_req(
            client,
            &RestAnalysisRequest {
                rest_base: query.rest_base,
                region: target_region,
                mode,
                champion_id: query.champion_id,
                position: target_position,
                tier: &effective_tier,
                version: Some(&effective_patch),
            },
        )
        .await;
        match probe_res {
            Ok(probe_json) => {
                let latest_patch = available_versions
                    .first()
                    .map(String::as_str)
                    .unwrap_or(&effective_patch);
                let prev_patch = available_versions.get(1).map(String::as_str);
                let (created_at_iso, prev_patch_created_at_iso) =
                    extract_patch_trend_timestamps(&probe_json, latest_patch, prev_patch);
                let play_count = extract_rest_play_count(&probe_json);
                let selection = resolve_effective_patch(
                    available_versions,
                    created_at_iso,
                    prev_patch_created_at_iso,
                    play_count,
                    now_unix,
                    None,
                    query.patch_48h_protection,
                );
                effective_patch = selection.effective_patch;
                patch_fallback_applied = selection.fallback_applied;
                patch_protection_remaining_hours = selection.remaining_hours;
                if patch_fallback_applied {
                    fetch_opgg_rest_champion_analysis_req(
                        client,
                        &RestAnalysisRequest {
                            rest_base: query.rest_base,
                            region: target_region,
                            mode,
                            champion_id: query.champion_id,
                            position: target_position,
                            tier: &effective_tier,
                            version: Some(&effective_patch),
                        },
                    )
                    .await
                } else {
                    Ok(probe_json)
                }
            }
            Err(e) => Err(e),
        }
    } else {
        fetch_opgg_rest_champion_analysis_req(
            client,
            &RestAnalysisRequest {
                rest_base: query.rest_base,
                region: target_region,
                mode,
                champion_id: query.champion_id,
                position: target_position,
                tier: &effective_tier,
                version: Some(&effective_patch),
            },
        )
        .await
    };

    // Automatic ARAM tier fallback (`emerald_plus` -> `all`) when explicit tier sample is too small
    if mode.eq_ignore_ascii_case("aram") && !effective_tier.eq_ignore_ascii_case("all") {
        let (needs_broader_sample, prev_play, prev_has_items) = match &result {
            Ok(json) => {
                let play = extract_rest_play_count(json).unwrap_or(0);
                let has_items = rest_payload_has_items(json);
                (play < ARAM_MIN_TIER_SAMPLE || !has_items, play, has_items)
            }
            Err(_) => (true, 0, false),
        };

        if needs_broader_sample
            && let Ok(all_json) = fetch_opgg_rest_champion_analysis_req(
                client,
                &RestAnalysisRequest {
                    rest_base: query.rest_base,
                    region: target_region,
                    mode,
                    champion_id: query.champion_id,
                    position: target_position,
                    tier: "all",
                    version: Some(&effective_patch),
                },
            )
            .await
        {
            let all_has_items = rest_payload_has_items(&all_json);
            let all_play = extract_rest_play_count(&all_json).unwrap_or(0);
            if all_has_items && (!prev_has_items || all_play > prev_play) {
                result = Ok(all_json);
                effective_tier = "all".to_string();
            }
        }
    }

    RestFetchOutcome {
        result,
        effective_patch,
        effective_tier,
        patch_fallback_applied,
        patch_protection_remaining_hours,
    }
}

pub(super) async fn resolve_build_fallback(
    client: &reqwest::Client,
    query: &BuildQuery<'_>,
    ddragon_cache: &DataDragonCache,
    disk_env: Option<crate::storage::CacheEnvelope<ChampionBuild>>,
    fast_spells_tx: &mut Option<tokio::sync::oneshot::Sender<FastSpellRecommendation>>,
) -> FallbackBuild {
    let queue_id = query.canonical_queue_id();
    if let Some(env) = disk_env {
        let mut stale = env.payload;
        enrich_build_with_ddragon_for_queue_with_flash_key_and_locale(
            &mut stale,
            ddragon_cache,
            query.flash_key,
            query.locale,
            Some(queue_id),
        );
        send_build_fast_spells(fast_spells_tx, query.champion_id, &stale);
        return FallbackBuild::Stale(stale);
    }
    FallbackBuild::Mcp(
        get_recommended_build_with_endpoint(
            client,
            query.mcp_endpoint,
            query.champion_id,
            queue_id,
            ddragon_cache,
        )
        .await,
    )
}
