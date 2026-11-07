//! Module 3: Data Fetching and Win Rate Provider (OP.GG + DataDragon)
//! Handles requests to DataDragon and OP.GG champion analysis (runes, builds, spells, win rates).

use serde::{Deserialize, Serialize};

pub const OPGG_MCP_ENDPOINT: &str = "https://mcp-api.op.gg/mcp";

/// Champion metadata translated from Riot's DataDragon.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChampionMetadata {
    pub champion_id: i32,
    pub key_name: String,
    pub display_name: String,
    pub icon_url: String,
}

/// An individual item group for the shop (e.g. Starter, Core, Situational).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ItemBlock {
    pub title: String,
    pub item_ids: Vec<i32>,
}

/// Complete build recommendations fetched and parsed from OP.GG.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChampionBuild {
    pub champion_id: i32,
    pub champion_name: String,
    pub win_rate: f64,
    pub pick_rate: f64,
    pub primary_style_id: i32,
    pub sub_style_id: i32,
    pub selected_perk_ids: Vec<i32>,
    pub spell1_id: i32,
    pub spell2_id: i32,
    pub item_blocks: Vec<ItemBlock>,
}

/// Parses DataDragon `champion.json` to find champion metadata by numeric ID.
pub fn parse_ddragon_champion_by_id(
    ddragon_json: &serde_json::Value,
    version: &str,
    target_champ_id: i32,
) -> Result<ChampionMetadata, String> {
    let data_map = ddragon_json
        .get("data")
        .and_then(|v| v.as_object())
        .ok_or_else(|| "Missing 'data' object in DataDragon payload".to_string())?;

    let target_key_str = target_champ_id.to_string();

    for (_name_key, champ_data) in data_map {
        let key = champ_data.get("key").and_then(|k| k.as_str()).unwrap_or("");
        if key == target_key_str {
            let id_str = champ_data.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let display_name = champ_data
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or(id_str);
            let image_full = champ_data
                .pointer("/image/full")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let icon_url = format!(
                "https://ddragon.leagueoflegends.com/cdn/{}/img/champion/{}",
                version, image_full
            );

            return Ok(ChampionMetadata {
                champion_id: target_champ_id,
                key_name: id_str.to_string(),
                display_name: display_name.to_string(),
                icon_url,
            });
        }
    }

    Err(format!(
        "Champion ID {} not found in DataDragon data",
        target_champ_id
    ))
}

/// Pure parser function to transform an OP.GG champion analysis response into `ChampionBuild`.
pub fn parse_opgg_analysis_payload(
    opgg_json: &serde_json::Value,
    champion_id: i32,
    champion_name: &str,
) -> Result<ChampionBuild, String> {
    let analysis = opgg_json
        .get("data")
        .or_else(|| opgg_json.pointer("/result/data"))
        .unwrap_or(opgg_json);

    let win_rate = analysis
        .get("win_rate")
        .or_else(|| analysis.pointer("/stats/win_rate"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    let pick_rate = analysis
        .get("pick_rate")
        .or_else(|| analysis.pointer("/stats/pick_rate"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    // Extract rune styles and perks
    let primary_style_id = analysis
        .pointer("/runes/primary_style_id")
        .or_else(|| analysis.pointer("/runes/primary_page_id"))
        .and_then(|v| v.as_i64())
        .ok_or_else(|| "Missing primary_style_id in OP.GG runes".to_string())?
        as i32;

    let sub_style_id = analysis
        .pointer("/runes/sub_style_id")
        .or_else(|| analysis.pointer("/runes/secondary_page_id"))
        .and_then(|v| v.as_i64())
        .ok_or_else(|| "Missing sub_style_id in OP.GG runes".to_string())?
        as i32;

    let mut selected_perk_ids = Vec::with_capacity(9);

    if let Some(primary_perks) = analysis
        .pointer("/runes/primary_perk_ids")
        .and_then(|v| v.as_array())
    {
        for perk in primary_perks {
            if let Some(id) = perk.as_i64() {
                selected_perk_ids.push(id as i32);
            }
        }
    }

    if let Some(sub_perks) = analysis
        .pointer("/runes/sub_perk_ids")
        .and_then(|v| v.as_array())
    {
        for perk in sub_perks {
            if let Some(id) = perk.as_i64() {
                selected_perk_ids.push(id as i32);
            }
        }
    }

    if let Some(stat_shards) = analysis
        .pointer("/runes/stat_shards")
        .and_then(|v| v.as_array())
    {
        for shard in stat_shards {
            if let Some(id) = shard.as_i64() {
                selected_perk_ids.push(id as i32);
            }
        }
    }

    if selected_perk_ids.len() != 9 {
        return Err(format!(
            "Invalid perk count in OP.GG payload: expected 9 perks, found {}",
            selected_perk_ids.len()
        ));
    }

    // Extract summoner spells
    let spell1_id = analysis
        .pointer("/spells/0")
        .or_else(|| analysis.pointer("/summoner_spells/0"))
        .and_then(|v| v.as_i64())
        .unwrap_or(4) as i32; // Default 4 = Flash

    let spell2_id = analysis
        .pointer("/spells/1")
        .or_else(|| analysis.pointer("/summoner_spells/1"))
        .and_then(|v| v.as_i64())
        .unwrap_or(14) as i32; // Default 14 = Ignite

    // Extract item blocks
    let mut item_blocks = Vec::new();

    if let Some(starters) = analysis
        .pointer("/items/starters")
        .and_then(|v| v.as_array())
    {
        let ids: Vec<i32> = starters
            .iter()
            .filter_map(|v| v.as_i64().map(|id| id as i32))
            .collect();
        if !ids.is_empty() {
            item_blocks.push(ItemBlock {
                title: "Starter Items".to_string(),
                item_ids: ids,
            });
        }
    }

    if let Some(core) = analysis.pointer("/items/core").and_then(|v| v.as_array()) {
        let ids: Vec<i32> = core
            .iter()
            .filter_map(|v| v.as_i64().map(|id| id as i32))
            .collect();
        if !ids.is_empty() {
            item_blocks.push(ItemBlock {
                title: "Core Build".to_string(),
                item_ids: ids,
            });
        }
    }

    if let Some(situational) = analysis
        .pointer("/items/situational")
        .and_then(|v| v.as_array())
    {
        let ids: Vec<i32> = situational
            .iter()
            .filter_map(|v| v.as_i64().map(|id| id as i32))
            .collect();
        if !ids.is_empty() {
            item_blocks.push(ItemBlock {
                title: "Situational Items".to_string(),
                item_ids: ids,
            });
        }
    }

    Ok(ChampionBuild {
        champion_id,
        champion_name: champion_name.to_string(),
        win_rate,
        pick_rate,
        primary_style_id,
        sub_style_id,
        selected_perk_ids,
        spell1_id,
        spell2_id,
        item_blocks,
    })
}

/// Sends a request to the OP.GG MCP endpoint to execute `lol_get_champion_analysis`.
pub async fn fetch_opgg_champion_analysis(
    client: &reqwest::Client,
    endpoint_url: &str,
    champion_name: &str,
    queue_name: &str,
) -> Result<serde_json::Value, String> {
    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "lol_get_champion_analysis",
            "arguments": {
                "champion_name": champion_name,
                "queue": queue_name,
                "desired_output_fields": "data.{stats,runes,spells,items}"
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

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn test_parse_ddragon_champion_by_id_finds_ahri() {
        let ddragon_fixture = serde_json::json!({
            "type": "champion",
            "format": "standAloneComplex",
            "version": "14.23.1",
            "data": {
                "Aatrox": {
                    "version": "14.23.1",
                    "id": "Aatrox",
                    "key": "266",
                    "name": "Aatrox",
                    "image": { "full": "Aatrox.png" }
                },
                "Ahri": {
                    "version": "14.23.1",
                    "id": "Ahri",
                    "key": "103",
                    "name": "Ahri",
                    "image": { "full": "Ahri.png" }
                }
            }
        });

        let result = parse_ddragon_champion_by_id(&ddragon_fixture, "14.23.1", 103);
        assert!(result.is_ok());

        let meta = result.unwrap();
        assert_eq!(meta.champion_id, 103);
        assert_eq!(meta.key_name, "Ahri");
        assert_eq!(meta.display_name, "Ahri");
        assert_eq!(
            meta.icon_url,
            "https://ddragon.leagueoflegends.com/cdn/14.23.1/img/champion/Ahri.png"
        );
    }

    #[test]
    fn test_parse_opgg_analysis_extracts_9_runes_spells_and_items() {
        let opgg_fixture = serde_json::json!({
            "data": {
                "win_rate": 51.85,
                "pick_rate": 8.42,
                "runes": {
                    "primary_style_id": 8100,
                    "sub_style_id": 8200,
                    "primary_perk_ids": [8112, 8126, 8138, 8135],
                    "sub_perk_ids": [8226, 8210],
                    "stat_shards": [5008, 5008, 5001]
                },
                "spells": [4, 14],
                "items": {
                    "starters": [1056, 2003, 2003],
                    "core": [3285, 3020, 3089],
                    "situational": [3157, 3135]
                }
            }
        });

        let result = parse_opgg_analysis_payload(&opgg_fixture, 103, "Ahri");
        assert!(result.is_ok());

        let build = result.unwrap();
        assert_eq!(build.champion_id, 103);
        assert_eq!(build.champion_name, "Ahri");
        assert_eq!(build.primary_style_id, 8100);
        assert_eq!(build.sub_style_id, 8200);
        assert_eq!(build.selected_perk_ids.len(), 9);
        assert_eq!(
            build.selected_perk_ids,
            vec![8112, 8126, 8138, 8135, 8226, 8210, 5008, 5008, 5001]
        );
        assert_eq!(build.spell1_id, 4);
        assert_eq!(build.spell2_id, 14);
        assert_eq!(build.item_blocks.len(), 3);
        assert_eq!(build.item_blocks[0].title, "Starter Items");
        assert_eq!(build.item_blocks[0].item_ids, vec![1056, 2003, 2003]);
        assert_eq!(build.item_blocks[1].title, "Core Build");
        assert_eq!(build.item_blocks[1].item_ids, vec![3285, 3020, 3089]);
    }

    #[tokio::test]
    async fn test_fetch_opgg_champion_analysis_with_wiremock() {
        let mock_server = MockServer::start().await;

        let mock_response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "data": {
                    "win_rate": 52.4,
                    "pick_rate": 11.2,
                    "runes": {
                        "primary_style_id": 8000,
                        "sub_style_id": 8100,
                        "primary_perk_ids": [8005, 9111, 9104, 8014],
                        "sub_perk_ids": [8139, 8135],
                        "stat_shards": [5008, 5008, 5003]
                    },
                    "spells": [4, 7],
                    "items": {
                        "starters": [1055, 2003],
                        "core": [3031, 3006, 3094]
                    }
                }
            }
        });

        Mock::given(method("POST"))
            .and(path("/mcp"))
            .respond_with(ResponseTemplate::new(200).set_body_json(mock_response))
            .mount(&mock_server)
            .await;

        let client = reqwest::Client::new();
        let mcp_url = format!("{}/mcp", mock_server.uri());

        let raw_res = fetch_opgg_champion_analysis(&client, &mcp_url, "Jinx", "ranked")
            .await
            .expect("Mock request should succeed");

        let build = parse_opgg_analysis_payload(&raw_res, 222, "Jinx")
            .expect("Parsing mock response should succeed");

        assert_eq!(build.champion_id, 222);
        assert_eq!(build.champion_name, "Jinx");
        assert_eq!(build.primary_style_id, 8000);
        assert_eq!(build.sub_style_id, 8100);
        assert_eq!(build.selected_perk_ids.len(), 9);
        assert_eq!(build.spell1_id, 4);
        assert_eq!(build.spell2_id, 7);
    }
}
