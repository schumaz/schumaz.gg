use super::analytics::{
    fetch_opgg_rest_champion_analysis_from_base, get_recommended_build_with_endpoint,
};
use super::ddragon::enrich_build_with_ddragon_for_queue_with_flash_key_and_locale;
use super::models::{ChampionBuild, DataDragonCache, ItemStatEntry};
use super::patch::{extract_patch_trend_timestamps, resolve_effective_patch};
use super::spells::{ddragon_mode_for_queue, extract_summoner_spell_choices_for_mode};
use crate::models::{AppLocale, FlashKeyPreference, queue_supports_spells};
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
    let ((s1, s2), _) = extract_summoner_spell_choices_for_mode(
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

pub(super) fn merge_mcp_depth_into_json(
    json_res: &mut serde_json::Value,
    target_position: Option<&String>,
    mcp_depth_opt: Option<(Vec<ItemStatEntry>, Vec<ItemStatEntry>, Vec<ItemStatEntry>)>,
) {
    let Some(data_obj) = json_res.get_mut("data").and_then(|d| d.as_object_mut()) else {
        return;
    };
    if let Some(pos) = target_position {
        data_obj.insert("position".to_string(), serde_json::json!(pos));
    }
    let Some((fourth, fifth, sixth)) = mcp_depth_opt else {
        return;
    };
    let to_json_arr = |entries: Vec<ItemStatEntry>| -> serde_json::Value {
        serde_json::Value::Array(
            entries
                .into_iter()
                .map(|e| {
                    serde_json::json!({
                        "ids": e.ids, "play": e.play, "win": e.win, "pick_rate": e.pick_rate
                    })
                })
                .collect(),
        )
    };
    if !fourth.is_empty() {
        data_obj.insert("fourth_items".to_string(), to_json_arr(fourth));
    }
    if !fifth.is_empty() {
        data_obj.insert("fifth_items".to_string(), to_json_arr(fifth));
    }
    if !sixth.is_empty() {
        data_obj.insert("sixth_items".to_string(), to_json_arr(sixth));
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn fetch_rest_with_patch_protection(
    client: &reqwest::Client,
    rest_base: &str,
    target_region: &str,
    mode: &str,
    champion_id: i32,
    target_position: Option<&str>,
    target_tier: &str,
    manual_patch_trimmed: &str,
    available_versions: &[String],
    mut effective_patch: String,
    patch_48h_protection: bool,
) -> RestFetchOutcome {
    let mut effective_tier = target_tier.to_string();
    let mut patch_fallback_applied = false;
    let mut patch_protection_remaining_hours = None;
    let now_unix = crate::storage::now_unix_secs();

    let mut result = if manual_patch_trimmed.is_empty() {
        let probe_res = fetch_opgg_rest_champion_analysis_from_base(
            client,
            rest_base,
            target_region,
            mode,
            champion_id,
            target_position,
            &effective_tier,
            Some(&effective_patch),
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
                    patch_48h_protection,
                );
                effective_patch = selection.effective_patch;
                patch_fallback_applied = selection.fallback_applied;
                patch_protection_remaining_hours = selection.remaining_hours;
                if patch_fallback_applied {
                    fetch_opgg_rest_champion_analysis_from_base(
                        client,
                        rest_base,
                        target_region,
                        mode,
                        champion_id,
                        target_position,
                        &effective_tier,
                        Some(&effective_patch),
                    )
                    .await
                } else {
                    Ok(probe_json)
                }
            }
            Err(e) => Err(e),
        }
    } else {
        fetch_opgg_rest_champion_analysis_from_base(
            client,
            rest_base,
            target_region,
            mode,
            champion_id,
            target_position,
            &effective_tier,
            Some(&effective_patch),
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
            && let Ok(all_json) = fetch_opgg_rest_champion_analysis_from_base(
                client,
                rest_base,
                target_region,
                mode,
                champion_id,
                target_position,
                "all",
                Some(&effective_patch),
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

#[allow(clippy::too_many_arguments)]
pub(super) async fn resolve_build_fallback(
    client: &reqwest::Client,
    mcp_endpoint: &str,
    champion_id: i32,
    queue_id: i32,
    ddragon_cache: &DataDragonCache,
    flash_key: FlashKeyPreference,
    active_locale: AppLocale,
    disk_env: Option<crate::storage::CacheEnvelope<ChampionBuild>>,
    fast_spells_tx: &mut Option<tokio::sync::oneshot::Sender<FastSpellRecommendation>>,
) -> FallbackBuild {
    if let Some(env) = disk_env {
        let mut stale = env.payload;
        enrich_build_with_ddragon_for_queue_with_flash_key_and_locale(
            &mut stale,
            ddragon_cache,
            flash_key,
            active_locale,
            Some(queue_id),
        );
        send_build_fast_spells(fast_spells_tx, champion_id, &stale);
        return FallbackBuild::Stale(stale);
    }
    FallbackBuild::Mcp(
        get_recommended_build_with_endpoint(
            client,
            mcp_endpoint,
            champion_id,
            queue_id,
            ddragon_cache,
        )
        .await,
    )
}
