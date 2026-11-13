use super::super::ddragon::enrich_build_with_ddragon;
use super::super::fallback::{
    build_fallback_champion_build, champion_key_to_opgg_name, opgg_queue_param_from_id,
};
use super::super::models::{ChampionBuild, DataDragonCache, ItemStatEntry};
use super::mcp_extractors::extract_mcp_grouped_item_lists;
use super::rest_parser::parse_opgg_analysis_payload;

pub const OPGG_MCP_ENDPOINT: &str = "https://mcp-api.op.gg/mcp";
pub const OPGG_REST_API_BASE: &str = "https://lol-api-champion.op.gg/api";

pub async fn fetch_opgg_champion_analysis(
    client: &reqwest::Client,
    endpoint_url: &str,
    champion_name: &str,
    queue_name: &str,
) -> Result<serde_json::Value, String> {
    let formatted_champ = champion_key_to_opgg_name(champion_name);
    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "lol_get_champion_analysis",
            "arguments": {
                "game_mode": queue_name,
                "champion": formatted_champ,
                "position": "mid",
                "desired_output_fields": [
                    "data.damage_type",
                    "data.summary.average_stats.{win_rate,pick_rate,play}",
                    "data.runes[].{primary_page_id,primary_rune_ids[],secondary_page_id,secondary_rune_ids[],stat_mod_ids[],play,win}",
                    "data.summoner_spells.{ids[],play,win}",
                    "data.starter_items.{ids[],play,win,pick_rate}",
                    "data.core_items.{ids[],play,win,pick_rate}",
                    "data.boots[].{ids[],play,win,pick_rate}",
                    "data.last_items[].{ids[],play,win,pick_rate}",
                    "data.fourth_items[].{ids[],play,win,pick_rate}",
                    "data.fifth_items[].{ids[],play,win,pick_rate}",
                    "data.sixth_items[].{ids[],play,win,pick_rate}"
                ]
            }
        }
    });

    let res = client
        .post(endpoint_url)
        .json(&payload)
        .send()
        .await
        .map_err(|err| format!("HTTP request to OP.GG failed: {}", err))?;

    if !res.status().is_success() {
        return Err(format!("OP.GG responded with HTTP error: {}", res.status()));
    }

    let json_body: serde_json::Value = res
        .json()
        .await
        .map_err(|err| format!("Failed to parse JSON response from OP.GG: {}", err))?;

    Ok(json_body)
}

/// Helper function to fetch champion analysis and return JSON body directly.
pub async fn fetch_opgg_analysis_json(
    client: &reqwest::Client,
    champion_name: &str,
    queue: &str,
) -> Result<serde_json::Value, String> {
    let json_body =
        fetch_opgg_champion_analysis(client, OPGG_MCP_ENDPOINT, champion_name, queue).await?;
    Ok(json_body)
}

/// High-level function that fetches the recommended build for a champion with automatic resilient fallback.
pub async fn get_recommended_build_with_endpoint(
    client: &reqwest::Client,
    endpoint_url: &str,
    champion_id: i32,
    queue_id: i32,
    ddragon_cache: &DataDragonCache,
) -> ChampionBuild {
    let champ_name = ddragon_cache
        .champions
        .get(&champion_id)
        .map(|c| c.key_name.as_str())
        .unwrap_or("Unknown");

    let queue_param = opgg_queue_param_from_id(queue_id);

    let mut build =
        match fetch_opgg_champion_analysis(client, endpoint_url, champ_name, queue_param).await {
            Ok(json_res) => match parse_opgg_analysis_payload(&json_res, champion_id, champ_name) {
                Ok(build) => build,
                Err(_) => build_fallback_champion_build(champion_id, champ_name, queue_id),
            },
            Err(_) => build_fallback_champion_build(champion_id, champ_name, queue_id),
        };

    build.supports_runes = queue_id != crate::ws_listener::QUEUE_ARAM_MAYHEM;
    enrich_build_with_ddragon(&mut build, ddragon_cache);
    build
}

/// Fetches available patch versions from the OP.GG REST API.
pub async fn fetch_opgg_versions(client: &reqwest::Client, region: &str) -> Vec<String> {
    let url = format!(
        "{}/{}/champions/ranked/versions",
        OPGG_REST_API_BASE, region
    );
    if let Ok(res) = client.get(&url).send().await
        && let Ok(json) = res.json::<serde_json::Value>().await
        && let Some(arr) = json.get("data").and_then(|d| d.as_array())
    {
        let vers: Vec<String> = arr
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();
        if !vers.is_empty() {
            return vers;
        }
    }
    vec!["16.22".to_string(), "16.21".to_string()]
}

/// Fetches champion build analysis from the OP.GG REST API with full filter parameters.
pub async fn fetch_opgg_rest_champion_analysis(
    client: &reqwest::Client,
    region: &str,
    mode: &str,
    champion_id: i32,
    position: &str,
    tier: &str,
    version: Option<&str>,
) -> Result<serde_json::Value, String> {
    let mut url = format!(
        "{}/{}/champions/{}/{}/{}?tier={}",
        OPGG_REST_API_BASE, region, mode, champion_id, position, tier
    );
    if let Some(v) = version
        && !v.trim().is_empty()
    {
        url.push_str(&format!("&version={}", v.trim()));
    }

    let res = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("REST request to OP.GG failed: {}", e))?;

    if !res.status().is_success() {
        return Err(format!(
            "OP.GG REST responded with HTTP error: {}",
            res.status()
        ));
    }

    let json_body: serde_json::Value = res
        .json()
        .await
        .map_err(|e| format!("Failed to parse JSON response from OP.GG REST: {}", e))?;

    Ok(json_body)
}

/// Fetches separate `fourth_items`, `fifth_items`, and `sixth_items` lists from OP.GG MCP.
pub async fn fetch_opgg_mcp_depth_items(
    client: &reqwest::Client,
    endpoint_url: &str,
    champion_name: &str,
    game_mode: &str,
    position: &str,
    tier: &str,
) -> Result<(Vec<ItemStatEntry>, Vec<ItemStatEntry>, Vec<ItemStatEntry>), String> {
    let formatted_champ = champion_key_to_opgg_name(champion_name);
    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "lol_get_champion_analysis",
            "arguments": {
                "game_mode": game_mode,
                "champion": formatted_champ,
                "position": position,
                "tier": tier,
                "desired_output_fields": [
                    "data.fourth_items[].{ids[],play,win,pick_rate}",
                    "data.fifth_items[].{ids[],play,win,pick_rate}",
                    "data.sixth_items[].{ids[],play,win,pick_rate}"
                ]
            }
        }
    });

    let res = client
        .post(endpoint_url)
        .json(&payload)
        .send()
        .await
        .map_err(|err| format!("MCP depth items request failed: {}", err))?;

    if !res.status().is_success() {
        return Err(format!("MCP depth items HTTP error: {}", res.status()));
    }

    let json_body: serde_json::Value = res
        .json()
        .await
        .map_err(|err| format!("Failed to parse MCP depth items JSON: {}", err))?;

    let text = json_body
        .pointer("/result/content/0/text")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing text in MCP depth items response".to_string())?;

    let grouped = extract_mcp_grouped_item_lists(text);
    let mut fourth = Vec::new();
    let mut fifth = Vec::new();
    let mut sixth = Vec::new();
    for (name, items) in grouped {
        match name.as_str() {
            "fourth_items" => fourth = items,
            "fifth_items" => fifth = items,
            "sixth_items" => sixth = items,
            _ => {}
        }
    }
    Ok((fourth, fifth, sixth))
}
