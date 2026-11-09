pub mod ddragon;
pub mod fallback;
pub mod items;
pub mod models;
pub mod analytics;
pub mod patch;
pub mod runes;
pub mod spells;

pub use ddragon::*;
pub use fallback::*;
pub use items::*;
pub use models::*;
pub use analytics::*;
pub use patch::*;
pub use runes::*;
pub use spells::*;

pub async fn get_recommended_build_with_filters(
    client: &reqwest::Client,
    champion_id: i32,
    queue_id: i32,
    position: Option<&str>,
    tier: Option<&str>,
    region: Option<&str>,
    patch: Option<&str>,
) -> Result<ChampionBuild, String> {
    let ddragon_cache = get_or_init_ddragon_cache(client).await?;
    let champ_name = ddragon_cache
        .champions
        .get(&champion_id)
        .map(|c| c.key_name.as_str())
        .unwrap_or("Unknown");

    let is_aram = queue_id == 450;
    let mode = if is_aram { "aram" } else { "ranked" };
    let target_region = region.unwrap_or("global");
    let target_tier = tier.unwrap_or("emerald_plus");

    // 1. Fetch available versions for region
    let available_versions = fetch_opgg_versions(client, target_region).await;

    // 2. Determine effective patch (2-day volume protection rule)
    let (mut effective_patch, mut patch_fallback_applied) = if let Some(p) = patch {
        if !p.trim().is_empty() {
            (p.to_string(), false)
        } else {
            (
                available_versions.first().cloned().unwrap_or_default(),
                false,
            )
        }
    } else {
        (
            available_versions.first().cloned().unwrap_or_default(),
            false,
        )
    };

    let target_position = if is_aram {
        "mid".to_string()
    } else if let Some(p) = position
        && !p.trim().is_empty()
        && p != "auto"
    {
        p.to_lowercase()
    } else {
        match resolve_champion_most_played_lane(
            client,
            champion_id,
            target_region,
            target_tier,
            Some(&effective_patch),
        )
        .await
        {
            Ok(lane) => lane,
            Err(_) => champion_default_lane(champ_name).to_string(),
        }
    };

    let now_unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    // If user did not manually specify a patch, inspect the latest patch's created_at
    if (patch.is_none() || patch.unwrap_or("").is_empty())
        && let Ok(probe_json) = fetch_opgg_rest_champion_analysis(
            client,
            target_region,
            mode,
            champion_id,
            &target_position,
            target_tier,
            Some(&effective_patch),
        )
        .await
    {
        let created_at_iso = probe_json
            .pointer("/data/trends/win/0/created_at")
            .and_then(|v| v.as_str());
        let prev_patch_created_at_iso = probe_json
            .pointer("/data/trends/win/1/created_at")
            .and_then(|v| v.as_str());
        let play_count = probe_json
            .pointer("/data/summary/average_stats/play")
            .or_else(|| probe_json.pointer("/data/average_stats/play"))
            .and_then(|v| v.as_u64());

        let (chosen_patch, fallback) = select_effective_patch_with_volume(
            &available_versions,
            created_at_iso,
            prev_patch_created_at_iso,
            play_count,
            now_unix,
            None,
        );
        effective_patch = chosen_patch;
        patch_fallback_applied = fallback;
    }

    // 3. Fetch build with effective patch (and enrich with MCP fourth/fifth/sixth item lists)
    let mcp_depth_fut = fetch_opgg_mcp_depth_items(
        client,
        OPGG_MCP_ENDPOINT,
        champ_name,
        mode,
        &target_position,
        target_tier,
    );
    let rest_fut = fetch_opgg_rest_champion_analysis(
        client,
        target_region,
        mode,
        champion_id,
        &target_position,
        target_tier,
        Some(&effective_patch),
    );
    let (rest_res, mcp_depth_res) = tokio::join!(rest_fut, mcp_depth_fut);

    let mut build = match rest_res {
        Ok(mut json_res) => {
            if let Some(data_obj) = json_res.get_mut("data").and_then(|d| d.as_object_mut()) {
                data_obj.insert("position".to_string(), serde_json::json!(target_position));
            }
            if let Ok((fourth, fifth, sixth)) = mcp_depth_res
                && let Some(data_obj) = json_res.get_mut("data").and_then(|d| d.as_object_mut())
            {
                let to_json_arr = |entries: Vec<ItemStatEntry>| -> serde_json::Value {
                    serde_json::Value::Array(
                        entries
                            .into_iter()
                            .map(|e| {
                                serde_json::json!({
                                    "ids": e.ids,
                                    "play": e.play,
                                    "win": e.win,
                                    "pick_rate": e.pick_rate
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
            match parse_opgg_analysis_payload(&json_res, champion_id, champ_name) {
                Ok(b) => b,
                Err(_) => {
                    get_recommended_build_with_endpoint(
                        client,
                        OPGG_MCP_ENDPOINT,
                        champion_id,
                        queue_id,
                        ddragon_cache,
                    )
                    .await
                }
            }
        }
        Err(_) => {
            get_recommended_build_with_endpoint(
                client,
                OPGG_MCP_ENDPOINT,
                champion_id,
                queue_id,
                ddragon_cache,
            )
            .await
        }
    };

    build.active_position = target_position;
    build.active_tier = target_tier.to_string();
    build.active_region = target_region.to_string();
    build.active_patch = effective_patch;
    build.available_patches = available_versions;
    build.patch_fallback_applied = patch_fallback_applied;

    enrich_build_with_ddragon(&mut build, ddragon_cache);
    Ok(build)
}

/// Production entry point to obtain recommended build for champion.
pub async fn get_recommended_build(
    client: &reqwest::Client,
    champion_id: i32,
    queue_id: i32,
) -> Result<ChampionBuild, String> {
    get_recommended_build_with_filters(client, champion_id, queue_id, None, None, None, None).await
}

