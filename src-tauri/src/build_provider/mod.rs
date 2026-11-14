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

use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FastSpellRecommendation {
    pub champion_id: i32,
    pub spell1_id: i32,
    pub spell2_id: i32,
    pub is_jungle: bool,
}

const BUILD_CACHE_TTL: Duration = Duration::from_secs(600);
const MCP_DEPTH_TIMEOUT: Duration = Duration::from_millis(2500);

static BUILD_CACHE: OnceLock<RwLock<HashMap<String, (Instant, ChampionBuild)>>> = OnceLock::new();

fn get_build_cache() -> &'static RwLock<HashMap<String, (Instant, ChampionBuild)>> {
    BUILD_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

fn extract_fast_spells_from_rest(
    json_res: &serde_json::Value,
    champion_id: i32,
    is_jungle: bool,
) -> Option<FastSpellRecommendation> {
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
    let ((s1, s2), _) = extract_summoner_spell_choices(analysis, is_jungle);
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

pub async fn get_recommended_build_with_filters(
    client: &reqwest::Client,
    champion_id: i32,
    queue_id: i32,
    position: Option<&str>,
    tier: Option<&str>,
    region: Option<&str>,
    patch: Option<&str>,
) -> Result<ChampionBuild, String> {
    get_recommended_build_with_filters_and_fast_spells(
        client,
        champion_id,
        queue_id,
        position,
        tier,
        region,
        patch,
        None,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn get_recommended_build_with_filters_and_fast_spells(
    client: &reqwest::Client,
    champion_id: i32,
    queue_id: i32,
    position: Option<&str>,
    tier: Option<&str>,
    region: Option<&str>,
    patch: Option<&str>,
    fast_spells_tx: Option<tokio::sync::oneshot::Sender<FastSpellRecommendation>>,
) -> Result<ChampionBuild, String> {
    let ddragon_cache = get_or_init_ddragon_cache(client).await?;
    get_recommended_build_with_filters_and_endpoints_internal(
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
        fast_spells_tx,
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
    get_recommended_build_with_filters_and_endpoints_internal(
        client,
        champion_id,
        queue_id,
        position,
        tier,
        region,
        patch,
        ddragon_cache,
        rest_base,
        mcp_endpoint,
        None,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn get_recommended_build_with_filters_and_endpoints_internal(
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
    let pos_cache_key = if is_aram {
        ""
    } else {
        position.unwrap_or("").trim()
    };
    let build_cache_key = format!(
        "{}:{}:{}:{}:{}:{}:{}:{}",
        rest_base,
        mcp_endpoint,
        champion_id,
        queue_id,
        pos_cache_key,
        target_tier,
        target_region,
        patch.unwrap_or("").trim()
    );

    if let Ok(guard) = get_build_cache().read()
        && let Some((ts, cached)) = guard.get(&build_cache_key)
        && ts.elapsed() < BUILD_CACHE_TTL
    {
        let mut build = cached.clone();
        let prefs = crate::lcu_injector::get_user_preferences();
        apply_build_spells_with_preference(&mut build, ddragon_cache, prefs.flash_key);
        if let Some(tx) = fast_spells_tx.take() {
            let _ = tx.send(FastSpellRecommendation {
                champion_id,
                spell1_id: build.spell1_id,
                spell2_id: build.spell2_id,
                is_jungle: build.active_position.eq_ignore_ascii_case("jungle"),
            });
        }
        return Ok(build);
    }

    // 1. Fetch available versions for region (cached in memory after first fetch)
    let available_versions = fetch_opgg_versions_from_base(client, rest_base, target_region).await;

    // 2. Determine initial effective patch
    let (mut effective_patch, mut patch_fallback_applied) = if let Some(p) = patch {
        if !p.trim().is_empty() {
            (p.trim().to_string(), false)
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
    let is_jungle = target_position
        .as_deref()
        .is_some_and(|p| p.eq_ignore_ascii_case("jungle"));

    let now_unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    // 3. Start slow MCP depth item enrichment concurrently with a bounded timeout so it never
    //    blocks fast REST spell resolution or stalls Champ Select updates.
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

    // 4. Fetch REST analysis (single request when latest patch is mature; avoids duplicate fetch)
    let rest_res = if patch.is_none() || patch.unwrap_or("").trim().is_empty() {
        let probe_res = fetch_opgg_rest_champion_analysis_from_base(
            client,
            rest_base,
            target_region,
            mode,
            champion_id,
            target_position.as_deref(),
            target_tier,
            Some(&effective_patch),
        )
        .await;

        match probe_res {
            Ok(probe_json) => {
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

                if fallback {
                    fetch_opgg_rest_champion_analysis_from_base(
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
            target_position.as_deref(),
            target_tier,
            Some(&effective_patch),
        )
        .await
    };

    // Emit fast spell recommendation IMMEDIATELY upon REST completion before awaiting MCP depth items!
    if let Ok(ref json_res) = rest_res
        && let Some(fast) = extract_fast_spells_from_rest(json_res, champion_id, is_jungle)
        && let Some(tx) = fast_spells_tx.take()
    {
        let _ = tx.send(fast);
    }

    let mcp_depth_opt = mcp_depth_handle.await.ok().flatten();

    let mut build = match rest_res {
        Ok(mut json_res) => {
            if let Some(ref pos) = target_position
                && let Some(data_obj) = json_res.get_mut("data").and_then(|d| d.as_object_mut())
            {
                data_obj.insert("position".to_string(), serde_json::json!(pos));
            }
            if let Some((fourth, fifth, sixth)) = mcp_depth_opt
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

    if let Some(tx) = fast_spells_tx.take() {
        let _ = tx.send(FastSpellRecommendation {
            champion_id,
            spell1_id: build.spell1_id,
            spell2_id: build.spell2_id,
            is_jungle,
        });
    }

    if let Ok(mut guard) = get_build_cache().write() {
        guard.insert(build_cache_key, (Instant::now(), build.clone()));
    }

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
            spells: HashMap::new(),
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
                    "summoner_spells": [
                        { "ids": [4, 7], "play": 32000, "win": 17000 },
                        { "ids": [4, 21], "play": 18000, "win": 9500 },
                        { "ids": [4, 32], "play": 5000, "win": 2500 }
                    ],
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
        assert_eq!(
            mayhem_build
                .spell_choices
                .iter()
                .map(|s| s.id)
                .collect::<Vec<_>>(),
            vec![7, 21]
        );

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

    #[tokio::test]
    async fn test_fast_spells_channel_resolves_without_waiting_for_slow_mcp_and_avoids_duplicate_rest()
     {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/global/champions/ranked/versions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": ["16.22", "16.21"]
            })))
            .expect(1)
            .mount(&mock_server)
            .await;

        // REST should only be called ONCE (reusing probe response when patch fallback is not needed)
        Mock::given(method("GET"))
            .and(path("/global/champions/aram/18/none"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {
                    "damage_type": "AD",
                    "summary": { "average_stats": { "win_rate": 0.53, "pick_rate": 0.08, "play": 50000 } },
                    "runes": [{
                        "primary_page_id": 8000,
                        "secondary_page_id": 8100,
                        "primary_rune_ids": [8008, 9111, 9104, 8014],
                        "secondary_rune_ids": [8139, 8135],
                        "stat_mod_ids": [5005, 5008, 5001],
                        "play": 30000,
                        "win": 16000
                    }],
                    "summoner_spells": [
                        { "ids": [4, 7], "play": 35000, "win": 18500 },
                        { "ids": [4, 21], "play": 18000, "win": 9500 }
                    ],
                    "starter_items": [{ "ids": [1038, 2003], "play": 27000, "win": 14400 }],
                    "boots": [{ "ids": [3006], "play": 40000, "win": 21000 }],
                    "core_items": [{ "ids": [3032, 6675, 3031], "play": 18000, "win": 9500 }],
                    "last_items": [{ "ids": [3036], "play": 15000, "win": 8000 }]
                }
            })))
            .expect(1)
            .mount(&mock_server)
            .await;

        // Simulate a slow MCP endpoint (1200ms latency)
        Mock::given(method("POST"))
            .and(path("/mcp"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_delay(Duration::from_millis(1200))
                    .set_body_json(serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": 1,
                        "result": {
                            "content": [{
                                "type": "text",
                                "text": "class LolGetChampionAnalysis: data\nclass Data: fourth_items\nLolGetChampionAnalysis(Data([FourthItem([3036],579,335,0.29)]))"
                            }]
                        }
                    })),
            )
            .mount(&mock_server)
            .await;

        let client = reqwest::Client::new();
        let cache = test_ddragon_cache();
        let mcp_url = format!("{}/mcp", mock_server.uri());
        let base_uri = mock_server.uri();
        let (fast_tx, fast_rx) = tokio::sync::oneshot::channel();

        let started = Instant::now();
        let build_handle = tokio::spawn(async move {
            get_recommended_build_with_filters_and_endpoints_internal(
                &client,
                18,
                crate::ws_listener::QUEUE_ARAM,
                None,
                None,
                None,
                None,
                &cache,
                &base_uri,
                &mcp_url,
                Some(fast_tx),
            )
            .await
        });

        let fast_spells = fast_rx
            .await
            .expect("Fast spell channel should receive spells");
        let fast_elapsed = started.elapsed();

        assert_eq!(fast_spells.champion_id, 18);
        assert_eq!(fast_spells.spell1_id, 4);
        assert_eq!(fast_spells.spell2_id, 7);
        assert!(!fast_spells.is_jungle);
        assert!(
            fast_elapsed < Duration::from_millis(600),
            "Fast spells took {:?}, should NOT block on 1200ms MCP request!",
            fast_elapsed
        );

        let build = build_handle
            .await
            .unwrap()
            .expect("Full build should still complete");
        assert_eq!(build.spell1_id, 4);
        assert_eq!(build.spell2_id, 7);
    }

    #[tokio::test]
    async fn test_tristana_aram_and_mayhem_rest_plus_mcp_merge_builds_exact_opgg_blocks() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/global/champions/ranked/versions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": ["16.22", "16.21"]
            })))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/global/champions/aram/18/none"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {
                    "summary": {
                        "id": 18,
                        "average_stats": { "play": 3627, "win_rate": 0.542046, "pick_rate": 0.091103 },
                        "roles": [{ "name": "MARKSMAN", "stats": { "play": 3271, "win": 1756 } }]
                    },
                    "summoner_spells": [
                        { "ids": [4, 7], "win": 599, "play": 1125, "pick_rate": 0.3358 },
                        { "ids": [4, 21], "win": 404, "play": 695, "pick_rate": 0.2075 },
                        { "ids": [4, 6], "win": 370, "play": 678, "pick_rate": 0.2024 }
                    ],
                    "starter_items": [
                        { "ids": [1038, 2003, 2003], "play": 1397, "win": 745, "pick_rate": 0.415 },
                        { "ids": [1038], "play": 398, "win": 222, "pick_rate": 0.1182 },
                        { "ids": [1042, 3006], "play": 251, "win": 124, "pick_rate": 0.0746 },
                        { "ids": [2031, 3051], "play": 121, "win": 69, "pick_rate": 0.0359 }
                    ],
                    "boots": [
                        { "ids": [3006], "play": 2640, "win": 1415, "pick_rate": 0.8585 },
                        { "ids": [3008], "play": 264, "win": 166, "pick_rate": 0.0859 },
                        { "ids": [3111], "play": 124, "win": 62, "pick_rate": 0.0403 },
                        { "ids": [3047], "play": 42, "win": 26, "pick_rate": 0.0137 }
                    ],
                    "core_items": [
                        { "ids": [3032, 6675, 3031], "play": 598, "win": 329, "pick_rate": 0.2025 },
                        { "ids": [3032, 3031, 6675], "play": 292, "win": 179, "pick_rate": 0.0989 },
                        { "ids": [3032, 3031, 3036], "play": 210, "win": 120, "pick_rate": 0.0711 },
                        { "ids": [3032, 6675, 3036], "play": 167, "win": 90, "pick_rate": 0.0566 },
                        { "ids": [3032, 6676, 3031], "play": 113, "win": 65, "pick_rate": 0.0383 },
                        { "ids": [6676, 6675, 3031], "play": 95, "win": 59, "pick_rate": 0.0322 },
                        { "ids": [3032, 6676, 3036], "play": 52, "win": 28, "pick_rate": 0.0176 },
                        { "ids": [6676, 3031, 3036], "play": 46, "win": 26, "pick_rate": 0.0156 },
                        { "ids": [6676, 3031, 6675], "play": 45, "win": 26, "pick_rate": 0.0152 },
                        { "ids": [3032, 3031, 3033], "play": 41, "win": 25, "pick_rate": 0.0139 },
                        { "ids": [3032, 6676, 6675], "play": 39, "win": 19, "pick_rate": 0.0132 },
                        { "ids": [6672, 6675, 3031], "play": 37, "win": 26, "pick_rate": 0.0125 },
                        { "ids": [3032, 6675, 3033], "play": 37, "win": 17, "pick_rate": 0.0125 },
                        { "ids": [3032, 3031, 3072], "play": 33, "win": 21, "pick_rate": 0.0112 },
                        { "ids": [6676, 6675, 3036], "play": 30, "win": 14, "pick_rate": 0.0102 }
                    ],
                    "last_items": [
                        { "ids": [3031], "play": 2613, "win": 1438 },
                        { "ids": [3032], "play": 2448, "win": 1317 },
                        { "ids": [6675], "play": 2056, "win": 1124 },
                        { "ids": [3036], "play": 1439, "win": 822 },
                        { "ids": [6676], "play": 1007, "win": 551 },
                        { "ids": [3072], "play": 848, "win": 500 },
                        { "ids": [3153], "play": 528, "win": 322 },
                        { "ids": [6672], "play": 429, "win": 239 },
                        { "ids": [1038], "play": 408, "win": 224 },
                        { "ids": [3094], "play": 274, "win": 136 },
                        { "ids": [3033], "play": 245, "win": 125 },
                        { "ids": [2523], "play": 132, "win": 72 },
                        { "ids": [3046], "play": 113, "win": 65 }
                    ],
                    "runes": [{
                        "primary_page_id": 8000,
                        "primary_rune_ids": [8008, 9111, 9104, 8014],
                        "secondary_page_id": 8100,
                        "secondary_rune_ids": [8135, 8139],
                        "stat_mod_ids": [5005, 5008, 5001],
                        "play": 1882,
                        "win": 1010,
                        "pick_rate": 0.5402
                    }]
                }
            })))
            .mount(&mock_server)
            .await;

        // Real MCP response shape for ARAM: Data(fourth_items, fifth_items, counters_meta) without sixth_items
        Mock::given(method("POST"))
            .and(path("/mcp"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": {
                    "content": [{
                        "type": "text",
                        "text": "class LolGetChampionAnalysis: data\nclass Data: fourth_items,fifth_items,counters_meta\nclass FourthItem: ids,play,win,pick_rate\nLolGetChampionAnalysis(Data([FourthItem([3036],593,346,0.29),FourthItem([3072],360,200,0.17),FourthItem([3031],283,159,0.14)],[FourthItem([3072],255,160,0.29),FourthItem([3153],138,87,0.16),FourthItem([3036],95,59,0.11)],[]))"
                    }]
                }
            })))
            .mount(&mock_server)
            .await;

        let client = reqwest::Client::new();
        let cache = test_ddragon_cache();
        let mcp_url = format!("{}/mcp", mock_server.uri());

        for (queue_id, expected_supports_runes) in [
            (crate::ws_listener::QUEUE_ARAM, true),
            (crate::ws_listener::QUEUE_ARAM_MAYHEM, false),
        ] {
            let build = get_recommended_build_with_filters_and_endpoints(
                &client,
                18,
                queue_id,
                None,
                None,
                None,
                Some("16.22"),
                &cache,
                &mock_server.uri(),
                &mcp_url,
            )
            .await
            .expect("Tristana ARAM/Mayhem merge should succeed");

            assert_eq!(build.supports_runes, expected_supports_runes);
            assert_eq!(build.spell1_id, 4);
            assert_eq!(build.spell2_id, 7);
            assert_eq!(
                build.spell_choices.iter().map(|s| s.id).collect::<Vec<_>>(),
                vec![7, 21]
            );
            assert_eq!(build.item_blocks[0].item_ids, vec![1038, 2003]);
            assert_eq!(build.item_blocks[1].item_ids, vec![3032, 6675, 3031]);
            assert_eq!(build.item_blocks[2].item_ids, vec![3006, 3008, 3111]);
            assert_eq!(
                build.item_blocks[3].item_ids,
                vec![3032, 6675, 3031, 3036, 3072, 6676]
            );
            assert_eq!(
                build.item_blocks[4].item_ids,
                vec![3153, 6672, 3033, 3094, 2523, 3046]
            );
            assert_eq!(build.build_options[0].core_item_ids, vec![3032, 6675, 3031]);
            assert_eq!(
                build.build_options[0].final_item_ids,
                vec![3032, 6675, 3031, 3036, 3072, 6676]
            );
            assert_eq!(
                build.build_options[0].situational_item_ids,
                vec![3153, 6672, 3033, 3094, 2523, 3046]
            );
        }
    }
}
