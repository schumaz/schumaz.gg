use schumazgg_lib::build_provider::*;
use std::collections::HashMap;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

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

#[tokio::test]
async fn test_get_recommended_build_uses_fallback_when_opgg_returns_500() {
    let mock_server = MockServer::start().await;

    // Simulate 500 Internal Server Error from OP.GG
    Mock::given(method("POST"))
        .and(path("/mcp"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock_server)
        .await;

    let client = reqwest::Client::new();
    let mcp_url = format!("{}/mcp", mock_server.uri());

    let mut champions_map = HashMap::new();
    champions_map.insert(
        103,
        ChampionMetadata {
            champion_id: 103,
            key_name: "Ahri".to_string(),
            display_name: "Ahri".to_string(),
            icon_url: "https://example.com/ahri.png".to_string(),
        },
    );

    let mut champion_profiles = HashMap::new();
    champion_profiles.insert(
        103,
        ChampionProfile {
            tags: vec!["Mage".to_string(), "Assassin".to_string()],
            partype: "Mana".to_string(),
            attack: 3,
            defense: 4,
            magic: 8,
        },
    );
    let mut cls = ItemClassificationCache::default();
    cls.item_tags.insert(3112, vec!["SpellDamage".to_string(), "ManaRegen".to_string()]);
    cls.item_gold.insert(3112, 950);
    cls.map12_items.insert(3112);
    cls.starters.insert(3112);
    cls.item_tags.insert(3118, vec!["SpellDamage".to_string(), "Mana".to_string()]);
    cls.item_gold.insert(3118, 2800);
    cls.map12_items.insert(3118);
    cls.completed.insert(3118);

    let cache = DataDragonCache {
        version: "14.23.1".to_string(),
        champions: champions_map,
        champion_profiles,
        rune_trees: vec![
            RuneTreeCatalog {
                id: 8200,
                key: "Sorcery".to_string(),
                slots: vec![vec![8229], vec![8226], vec![8210], vec![8237]],
            },
            RuneTreeCatalog {
                id: 8300,
                key: "Inspiration".to_string(),
                slots: vec![vec![8369], vec![8304], vec![8316], vec![8347]],
            },
        ],
        item_classifications: cls,
        runes: HashMap::new(),
        spells: HashMap::new(),
        ..Default::default()
    };

    // Even though OP.GG fails with 500, this function must not fail, returning a resilient fallback!
    let build = get_recommended_build_with_endpoint(&client, &mcp_url, 103, 450, &cache).await;

    assert_eq!(build.champion_id, 103);
    assert_eq!(build.champion_name, "Ahri");
    assert_eq!(build.selected_perk_ids.len(), 9);
    assert_eq!(build.spell1_id, 4);
    assert_eq!(build.spell2_id, 32); // ARAM snowball
}
