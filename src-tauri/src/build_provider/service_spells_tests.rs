use super::*;
use crate::models::{ChampionMetadata, QUEUE_ARAM, QUEUE_ARAM_MAYHEM, SpellMetadata};
use std::collections::HashMap;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn ddragon_cache_with_real_spell_modes() -> DataDragonCache {
    let mut champions = HashMap::new();
    champions.insert(
        117,
        ChampionMetadata {
            champion_id: 117,
            key_name: "Lulu".to_string(),
            display_name: "Lulu".to_string(),
            icon_url: "https://example.com/lulu.png".to_string(),
        },
    );
    let spell_entries: &[(i32, &str, &[&str])] = &[
        (4, "Flash", &["CLASSIC", "ARAM", "KIWI"]),
        (3, "Exaustão", &["CLASSIC", "ARAM"]),
        (12, "Teleporte", &["CLASSIC"]),
        (32, "Marca", &["ARAM", "KIWI"]),
        (7, "Curar", &["CLASSIC", "ARAM", "KIWI"]),
        (21, "Barreira", &["CLASSIC", "ARAM", "KIWI"]),
    ];
    let spells = spell_entries
        .iter()
        .map(|&(id, name, modes)| {
            (
                id,
                SpellMetadata {
                    id,
                    name: name.to_string(),
                    icon_url: format!("https://example.com/{id}.png"),
                    modes: modes.iter().map(|m| (*m).to_string()).collect(),
                },
            )
        })
        .collect();

    DataDragonCache {
        version: "16.22.1".to_string(),
        champions,
        runes: HashMap::new(),
        spells,

        ..Default::default()
    }
}

#[tokio::test]
async fn test_lulu_aram_mayhem_filters_exhaust_in_fast_spells_and_full_build_while_classic_aram_keeps_it()
 {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/mayhem_spells/global/champions/ranked/versions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": ["16.22"]
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/mayhem_spells/global/champions/aram/117/none"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {
                "damage_type": "AP",
                "summary": { "average_stats": { "win_rate": 0.52, "pick_rate": 0.06, "play": 25000 } },
                "runes": [{
                    "primary_page_id": 8200,
                    "secondary_page_id": 8300,
                    "primary_rune_ids": [8214, 8226, 8210, 8237],
                    "secondary_rune_ids": [8345, 8347],
                    "stat_mod_ids": [5007, 5008, 5001],
                    "play": 18000,
                    "win": 9500
                }],
                "summoner_spells": [
                    { "ids": [3, 4], "play": 19000, "win": 10000 },
                    { "ids": [4, 7], "play": 9000, "win": 4700 },
                    { "ids": [4, 21], "play": 5500, "win": 2800 }
                ],
                "starter_items": [{ "ids": [3802, 2003], "play": 14000, "win": 7300 }],
                "boots": [{ "ids": [3158], "play": 21000, "win": 11000 }],
                "core_items": [{ "ids": [6617, 3504, 3107], "play": 12000, "win": 6400 }],
                "last_items": [{ "ids": [3222], "play": 9000, "win": 4800 }]
            }
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/mcp"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock_server)
        .await;

    let client = reqwest::Client::new();
    let cache = ddragon_cache_with_real_spell_modes();
    let rest_base = format!("{}/mayhem_spells", mock_server.uri());
    let mcp_url = format!("{}/mcp", mock_server.uri());

    // 1. ARAM Mayhem (2400 / "KIWI"): fast spells and full build must drop Exhaust (3)
    let (fast_tx, fast_rx) = tokio::sync::oneshot::channel();
    let mayhem_build = get_recommended_build_with_filters_and_endpoints_internal(
        &client,
        117,
        QUEUE_ARAM_MAYHEM,
        None,
        None,
        None,
        Some("16.22"),
        &cache,
        &rest_base,
        &mcp_url,
        Some(fast_tx),
    )
    .await
    .expect("Lulu ARAM Mayhem build should succeed");

    let fast_mayhem = fast_rx.await.expect("Fast spells should be emitted");
    assert_eq!((fast_mayhem.spell1_id, fast_mayhem.spell2_id), (4, 7));
    assert_ne!(mayhem_build.spell1_id, 3);
    assert_ne!(mayhem_build.spell2_id, 3);
    assert_eq!(
        mayhem_build
            .spell_choices
            .iter()
            .map(|s| s.id)
            .collect::<Vec<_>>(),
        vec![7, 21]
    );

    // 2. Classic ARAM (450 / "ARAM"): Exhaust (3) remains allowed while listed in DataDragon "ARAM"
    let (fast_aram_tx, fast_aram_rx) = tokio::sync::oneshot::channel();
    let aram_build = get_recommended_build_with_filters_and_endpoints_internal(
        &client,
        117,
        QUEUE_ARAM,
        None,
        None,
        None,
        Some("16.22"),
        &cache,
        &rest_base,
        &mcp_url,
        Some(fast_aram_tx),
    )
    .await
    .expect("Lulu Classic ARAM build should succeed");

    let fast_aram = fast_aram_rx.await.expect("Fast spells should be emitted");
    assert_eq!((fast_aram.spell1_id, fast_aram.spell2_id), (4, 3));
    assert_eq!(
        aram_build
            .spell_choices
            .iter()
            .map(|s| s.id)
            .collect::<Vec<_>>(),
        vec![3, 7]
    );
}
