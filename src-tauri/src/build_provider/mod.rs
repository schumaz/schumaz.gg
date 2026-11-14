pub mod analytics;
pub mod ddragon;
pub mod fallback;
pub mod items;
pub mod models;
pub mod patch;
pub mod runes;
pub mod spells;

pub use analytics::*;
pub use ddragon::*;
pub use fallback::*;
pub use items::*;
pub use models::*;
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
    get_recommended_build_with_filters_and_endpoints(
        client,
        champion_id,
        queue_id,
        position,
        tier,
        region,
        patch,
        ddragon_cache,
        OPGG_REST_API_BASE,
        OPGG_MCP_ENDPOINT,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn get_recommended_build_with_filters_and_endpoints(
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
) -> Result<ChampionBuild, String> {
    let champ_name = ddragon_cache
        .champions
        .get(&champion_id)
        .map(|c| c.key_name.as_str())
        .unwrap_or("Unknown");

    let is_aram = queue_id == crate::ws_listener::QUEUE_ARAM
        || queue_id == crate::ws_listener::QUEUE_ARAM_MAYHEM;
    let mode = if is_aram { "aram" } else { "ranked" };
    let supports_runes = queue_id != crate::ws_listener::QUEUE_ARAM_MAYHEM;
    let target_region = region.unwrap_or("global");
    let target_tier = tier.unwrap_or("emerald_plus");

    // 1. Fetch available versions for region
    let available_versions = fetch_opgg_versions_from_base(client, rest_base, target_region).await;

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

    let target_position: Option<String> = if is_aram {
        // ARAM and ARAM Mayhem have no lanes: ignore any incoming position and skip ranked lane lookup.
        None
    } else if let Some(p) = position
        && !p.trim().is_empty()
        && p != "auto"
    {
        Some(p.to_lowercase())
    } else {
        let resolved = match resolve_champion_most_played_lane(
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
        };
        Some(resolved)
    };

    let now_unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    // If user did not manually specify a patch, inspect the latest patch's created_at
    if (patch.is_none() || patch.unwrap_or("").is_empty())
        && let Ok(probe_json) = fetch_opgg_rest_champion_analysis_from_base(
            client,
            rest_base,
            target_region,
            mode,
            champion_id,
            target_position.as_deref(),
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
        mcp_endpoint,
        champ_name,
        mode,
        target_position.as_deref(),
        target_tier,
    );
    let rest_fut = fetch_opgg_rest_champion_analysis_from_base(
        client,
        rest_base,
        target_region,
        mode,
        champion_id,
        target_position.as_deref(),
        target_tier,
        Some(&effective_patch),
    );
    let (rest_res, mcp_depth_res) = tokio::join!(rest_fut, mcp_depth_fut);

    let mut build = match rest_res {
        Ok(mut json_res) => {
            if let Some(ref pos) = target_position
                && let Some(data_obj) = json_res.get_mut("data").and_then(|d| d.as_object_mut())
            {
                data_obj.insert("position".to_string(), serde_json::json!(pos));
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
            match parse_opgg_analysis_payload_with_options(
                &json_res,
                champion_id,
                champ_name,
                supports_runes,
            ) {
                Ok(b) => b,
                Err(_) => {
                    get_recommended_build_with_endpoint(
                        client,
                        mcp_endpoint,
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
                mcp_endpoint,
                champion_id,
                queue_id,
                ddragon_cache,
            )
            .await
        }
    };

    build.active_position = target_position.unwrap_or_default();
    build.active_tier = target_tier.to_string();
    build.active_region = target_region.to_string();
    build.active_patch = effective_patch;
    build.available_patches = available_versions;
    build.patch_fallback_applied = patch_fallback_applied;
    build.supports_runes = supports_runes;

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn test_ddragon_cache() -> DataDragonCache {
        let mut champions = HashMap::new();
        champions.insert(
            18,
            ChampionMetadata {
                champion_id: 18,
                key_name: "Tristana".to_string(),
                display_name: "Tristana".to_string(),
                icon_url: "https://example.com/tristana.png".to_string(),
            },
        );
        champions.insert(
            103,
            ChampionMetadata {
                champion_id: 103,
                key_name: "Ahri".to_string(),
                display_name: "Ahri".to_string(),
                icon_url: "https://example.com/ahri.png".to_string(),
            },
        );
        DataDragonCache {
            version: "16.22.1".to_string(),
            champions,
            runes: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_aram_and_aram_mayhem_ignore_position_and_return_empty_active_position() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/global/champions/ranked/versions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": ["16.22", "16.21"]
            })))
            .mount(&mock_server)
            .await;

        // Ranked positions endpoint MUST NOT be called for ARAM or ARAM Mayhem
        Mock::given(method("GET"))
            .and(path("/global/champions/ranked"))
            .respond_with(ResponseTemplate::new(500))
            .expect(0)
            .mount(&mock_server)
            .await;

        // Any /aram/*/{lane} request MUST NOT happen
        Mock::given(method("GET"))
            .and(path("/global/champions/aram/18/mid"))
            .respond_with(ResponseTemplate::new(422))
            .expect(0)
            .mount(&mock_server)
            .await;
        Mock::given(method("GET"))
            .and(path("/global/champions/aram/18/top"))
            .respond_with(ResponseTemplate::new(422))
            .expect(0)
            .mount(&mock_server)
            .await;
        Mock::given(method("GET"))
            .and(path("/global/champions/aram/103/mid"))
            .respond_with(ResponseTemplate::new(422))
            .expect(0)
            .mount(&mock_server)
            .await;

        // ARAM Mayhem (18) REST must hit /aram/18/none; omit runes to verify ARAM Mayhem works without runes
        Mock::given(method("GET"))
            .and(path("/global/champions/aram/18/none"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {
                    "damage_type": "AD",
                    "summary": { "average_stats": { "win_rate": 0.53, "pick_rate": 0.08, "play": 50000 } },
                    "runes": [],
                    "summoner_spells": [{ "ids": [4, 7], "play": 32000, "win": 17000 }],
                    "starter_items": [{ "ids": [1038, 2003], "play": 27000, "win": 14400 }],
                    "boots": [{ "ids": [3006], "play": 40000, "win": 21000 }],
                    "core_items": [{ "ids": [3032, 6675, 3031], "play": 18000, "win": 9500 }],
                    "last_items": [{ "ids": [3036], "play": 15000, "win": 8000 }]
                }
            })))
            .mount(&mock_server)
            .await;

        // Classic ARAM (103) REST must hit /aram/103/none with runes present
        Mock::given(method("GET"))
            .and(path("/global/champions/aram/103/none"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {
                    "damage_type": "AP",
                    "summary": { "average_stats": { "win_rate": 0.52, "pick_rate": 0.09, "play": 48000 } },
                    "runes": [{
                        "primary_page_id": 8100,
                        "secondary_page_id": 8200,
                        "primary_rune_ids": [8128, 8126, 8138, 8106],
                        "secondary_rune_ids": [8226, 8210],
                        "stat_mod_ids": [5008, 5008, 5001],
                        "play": 30000,
                        "win": 16000,
                        "pick_rate": 0.62
                    }],
                    "summoner_spells": [{ "ids": [4, 32], "play": 31000, "win": 16500 }],
                    "starter_items": [{ "ids": [3802, 2003], "play": 25000, "win": 13500 }],
                    "boots": [{ "ids": [3020], "play": 39000, "win": 20500 }],
                    "core_items": [{ "ids": [6655, 4645, 3089], "play": 17000, "win": 9000 }],
                    "last_items": [{ "ids": [3157], "play": 14000, "win": 7600 }]
                }
            })))
            .mount(&mock_server)
            .await;

        Mock::given(method("POST"))
            .and(path("/mcp"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": {
                    "content": [{
                        "type": "text",
                        "text": "class LolGetChampionAnalysis: data\nclass Data: fourth_items,fifth_items\nLolGetChampionAnalysis(Data([FourthItem([3036],579,335,0.29)],[FourthItem([3072],248,155,0.29)]))"
                    }]
                }
            })))
            .mount(&mock_server)
            .await;

        let client = reqwest::Client::new();
        let cache = test_ddragon_cache();
        let mcp_url = format!("{}/mcp", mock_server.uri());

        // ARAM Mayhem (2400): even with incoming position Some("top"), it is ignored, runes are optional, active_position == ""
        let mayhem_build = get_recommended_build_with_filters_and_endpoints(
            &client,
            18,
            crate::ws_listener::QUEUE_ARAM_MAYHEM,
            Some("top"),
            None,
            None,
            Some("16.22"),
            &cache,
            &mock_server.uri(),
            &mcp_url,
        )
        .await
        .expect("ARAM Mayhem build should succeed");

        assert_eq!(mayhem_build.active_position, "");
        assert!(!mayhem_build.supports_runes);
        assert_eq!(mayhem_build.spell1_id, 4);
        assert_eq!(mayhem_build.spell2_id, 7);

        // Classic ARAM (450): even with incoming position Some("mid"), it is ignored, runes are supported, active_position == ""
        let aram_build = get_recommended_build_with_filters_and_endpoints(
            &client,
            103,
            crate::ws_listener::QUEUE_ARAM,
            Some("mid"),
            None,
            None,
            Some("16.22"),
            &cache,
            &mock_server.uri(),
            &mcp_url,
        )
        .await
        .expect("Classic ARAM build should succeed");

        assert_eq!(aram_build.active_position, "");
        assert!(aram_build.supports_runes);
        assert_eq!(aram_build.primary_style_id, 8100);
        assert_eq!(aram_build.sub_style_id, 8200);
    }
}
