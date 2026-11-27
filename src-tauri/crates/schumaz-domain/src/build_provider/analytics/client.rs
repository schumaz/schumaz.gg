use super::super::ddragon::enrich_build_with_ddragon_for_queue;
use super::super::fallback::{
    build_fallback_champion_build_with_ddragon, champion_key_to_opgg_name,
    opgg_queue_param_from_id,
};
use super::super::models::{ChampionBuild, DataDragonCache, ItemStatEntry};
use super::super::spells::ddragon_mode_for_queue;
use super::cooldown::send_analytics_request;
use super::mcp_extractors::extract_mcp_grouped_item_lists;
use super::rest_parser::parse_opgg_analysis_payload_with_ddragon;
pub use super::versions::{
    fetch_opgg_versions, fetch_opgg_versions_from_base, fetch_opgg_versions_with_cache_dir,
    opgg_versions_file_path, peek_latest_cached_opgg_version,
};
use crate::models::{queue_supports_runes, queue_supports_spells};
use std::sync::OnceLock;
use std::time::Duration;

pub const OPGG_MCP_ENDPOINT: &str = "https://mcp-api.op.gg/mcp";
pub const OPGG_REST_API_BASE: &str = "https://lol-api-champion.op.gg/api";

static SHARED_HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
static SHARED_ASSET_HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

/// Returns a shared, connection-pooled `reqwest::Client` with bounded timeouts and HTTP/2 keep-alives.
pub fn shared_http_client() -> &'static reqwest::Client {
    SHARED_HTTP_CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .pool_max_idle_per_host(8)
            .pool_idle_timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(4))
            .timeout(Duration::from_secs(12))
            .tcp_keepalive(Duration::from_secs(20))
            .http2_keep_alive_interval(Duration::from_secs(15))
            .http2_keep_alive_timeout(Duration::from_secs(5))
            .http2_keep_alive_while_idle(true)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new())
    })
}

/// Returns a dedicated HTTP/1.1 `reqwest::Client` for CDN asset downloads to prevent HTTP/2
/// multiplexing head-of-line stalls across concurrent image requests.
pub fn shared_asset_http_client() -> &'static reqwest::Client {
    SHARED_ASSET_HTTP_CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .http1_only()
            .pool_max_idle_per_host(10)
            .pool_idle_timeout(Duration::from_secs(20))
            .connect_timeout(Duration::from_secs(4))
            .timeout(Duration::from_secs(8))
            .tcp_keepalive(Duration::from_secs(20))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new())
    })
}

pub async fn fetch_opgg_champion_analysis(
    client: &reqwest::Client,
    endpoint_url: &str,
    champion_name: &str,
    queue_name: &str,
) -> Result<serde_json::Value, String> {
    let formatted_champ = champion_key_to_opgg_name(champion_name);
    // NOTE: OP.GG MCP `lol_get_champion_analysis` schema strictly requires `position`
    // (`top|mid|jungle|adc|support`) even when `game_mode == "aram"` (omitting it or passing
    // "none" fails with JSON-RPC -32600).
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
                    "data.summary.average_stats.{win_rate,pick_rate,play,tier}",
                    "data.runes[].{primary_page_id,primary_rune_ids[],secondary_page_id,secondary_rune_ids[],stat_mod_ids[],play,win}",
                    "data.summoner_spells.{ids[],play,win}",
                    "data.starter_items.{ids[],play,win,pick_rate}",
                    "data.core_items.{ids[],play,win,pick_rate}",
                    "data.boots[].{ids[],play,win,pick_rate}",
                    "data.last_items[].{ids[],play,win,pick_rate}",
                    "data.fourth_items[].{ids[],play,win,pick_rate}",
                    "data.fifth_items[].{ids[],play,win,pick_rate}",
                    "data.sixth_items[].{ids[],play,win,pick_rate}",
                    "data.skill_masteries[].{ids[],play,win,pick_rate,builds[].{order[],play,win,pick_rate}}",
                    "data.skills[].{order[],play,win,pick_rate}"
                ]
            }
        }
    });

    let cooldown_key = format!("mcp:{endpoint_url}:analysis:{formatted_champ}:{queue_name}");
    let res = send_analytics_request(
        &cooldown_key,
        client.post(endpoint_url).json(&payload),
        "OP.GG MCP analysis",
    )
    .await?;

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
    let spell_mode = ddragon_mode_for_queue(queue_id);
    let supports_runes = queue_supports_runes(queue_id);
    let supports_spells = queue_supports_spells(queue_id);

    let mut build =
        match fetch_opgg_champion_analysis(client, endpoint_url, champ_name, queue_param).await {
            Ok(json_res) => match parse_opgg_analysis_payload_with_ddragon(
                &json_res,
                champion_id,
                champ_name,
                supports_runes,
                Some(spell_mode),
                Some(ddragon_cache),
                Some(&ddragon_cache.spells),
                None,
            ) {
                Ok(build) => build,
                Err(_) => build_fallback_champion_build_with_ddragon(
                    champion_id,
                    champ_name,
                    queue_id,
                    Some(ddragon_cache),
                ),
            },
            Err(_) => build_fallback_champion_build_with_ddragon(
                champion_id,
                champ_name,
                queue_id,
                Some(ddragon_cache),
            ),
        };

    if queue_param == "aram" {
        build.active_position.clear();
    }
    build.supports_runes = supports_runes;
    build.supports_spells = supports_spells;
    enrich_build_with_ddragon_for_queue(&mut build, ddragon_cache, queue_id);
    build
}

/// Fetches champion build analysis from the OP.GG REST API with full filter parameters.
/// When `mode == "aram"` or `position` is `None`/empty, uses `"none"` because the OP.GG REST
/// route `/{region}/champions/{mode}/{champion_id}/{position}` requires a path segment.
pub async fn fetch_opgg_rest_champion_analysis(
    client: &reqwest::Client,
    region: &str,
    mode: &str,
    champion_id: i32,
    position: Option<&str>,
    tier: &str,
    version: Option<&str>,
) -> Result<serde_json::Value, String> {
    fetch_opgg_rest_champion_analysis_from_base(
        client,
        OPGG_REST_API_BASE,
        region,
        mode,
        champion_id,
        position,
        tier,
        version,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn fetch_opgg_rest_champion_analysis_from_base(
    client: &reqwest::Client,
    rest_base: &str,
    region: &str,
    mode: &str,
    champion_id: i32,
    position: Option<&str>,
    tier: &str,
    version: Option<&str>,
) -> Result<serde_json::Value, String> {
    let pos_segment = if mode.eq_ignore_ascii_case("aram") {
        "none"
    } else {
        match position {
            Some(p) if !p.trim().is_empty() => p.trim(),
            _ => "none",
        }
    };

    let mut url = format!(
        "{}/{}/champions/{}/{}/{}?tier={}",
        rest_base, region, mode, champion_id, pos_segment, tier
    );
    if let Some(v) = version
        && !v.trim().is_empty()
    {
        url.push_str(&format!("&version={}", v.trim()));
    }

    let res = send_analytics_request(&url, client.get(&url), "OP.GG REST analysis").await?;

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
    position: Option<&str>,
    tier: &str,
) -> Result<(Vec<ItemStatEntry>, Vec<ItemStatEntry>, Vec<ItemStatEntry>), String> {
    let formatted_champ = champion_key_to_opgg_name(champion_name);
    // OP.GG MCP `lol_get_champion_analysis` strictly requires `position` in its schema
    // (`top|mid|jungle|adc|support`) even for `game_mode == "aram"` (omitting it or passing
    // "none" fails with JSON-RPC -32600, while any valid enum value returns global ARAM data).
    let mcp_position = if game_mode.eq_ignore_ascii_case("aram") {
        "mid"
    } else {
        match position {
            Some(p) if !p.trim().is_empty() => p.trim(),
            _ => "mid",
        }
    };
    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "lol_get_champion_analysis",
            "arguments": {
                "game_mode": game_mode,
                "champion": formatted_champ,
                "position": mcp_position,
                "tier": tier,
                "desired_output_fields": [
                    "data.fourth_items[].{ids[],play,win,pick_rate}",
                    "data.fifth_items[].{ids[],play,win,pick_rate}",
                    "data.sixth_items[].{ids[],play,win,pick_rate}",
                    "data.skill_masteries[].{ids[],play,win,pick_rate,builds[].{order[],play,win,pick_rate}}",
                    "data.skills[].{order[],play,win,pick_rate}"
                ]
            }
        }
    });

    let cooldown_key =
        format!("mcp:{endpoint_url}:depth:{formatted_champ}:{game_mode}:{mcp_position}:{tier}");
    let res = send_analytics_request(
        &cooldown_key,
        client.post(endpoint_url).json(&payload),
        "MCP depth items",
    )
    .await?;

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

#[cfg(test)]
#[path = "client_tests.rs"]
mod tests;
