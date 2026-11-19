use std::collections::HashMap;

use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::super::skins::{
    fetch_skin_name_for_locale_with_base, lookup_cached_skin_name, prewarm_skins_from_dir,
    skin_cache, skins_file_path,
};
use super::*;
use crate::models::{ChampionMetadata, RankedQueueStats, SummonerInfo};

fn sample_cache() -> DataDragonCache {
    let mut champions = HashMap::new();
    champions.insert(
        103,
        ChampionMetadata {
            champion_id: 103,
            key_name: "Ahri".to_string(),
            display_name: "Ahri".to_string(),
            icon_url: "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/champion/Ahri.png"
                .to_string(),
        },
    );

    DataDragonCache {
        version: "16.22.1".to_string(),
        champions,
        runes: HashMap::new(),
        spells: HashMap::new(),
        ..Default::default()
    }
}

#[tokio::test]
async fn test_skin_cache_lz4_incremental_and_profile_persistence_relocalization() {
    let dir = std::env::temp_dir().join(format!(
        "schumazgg_profile_cache_test_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    let ddragon_dir = dir.join("ddragon");
    let profile_dir = dir.join("profile");
    std::fs::create_dir_all(&ddragon_dir).unwrap();
    std::fs::create_dir_all(&profile_dir).unwrap();

    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/cdn/16.22.1/data/pt_BR/champion/Ahri.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {
                "Ahri": {
                    "name": "Ahri",
                    "skins": [
                        { "id": 103027, "num": 27, "name": "Ahri Florescer Espiritual" }
                    ]
                }
            }
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = reqwest::Client::new();
    let skin_name = fetch_skin_name_for_locale_with_base(
        &client,
        &mock_server.uri(),
        "16.22.1",
        "Ahri",
        27,
        103027,
        AppLocale::PtBr,
        Some(&ddragon_dir),
    )
    .await
    .unwrap();
    assert_eq!(skin_name, "Ahri Florescer Espiritual");
    assert!(skins_file_path(&ddragon_dir, AppLocale::PtBr).exists());

    // Clear RAM map and reload from disk to prove 0-network disk hydration
    if let Ok(mut g) = skin_cache().write() {
        g.remove(&AppLocale::PtBr);
    }
    assert_eq!(prewarm_skins_from_dir(&ddragon_dir, AppLocale::PtBr), 1);
    assert_eq!(
        lookup_cached_skin_name(AppLocale::PtBr, 103027).as_deref(),
        Some("Ahri Florescer Espiritual")
    );

    // Save & reload profile with SWR detection + EN-US relocalization
    let profile = PlayerProfile {
        summoner: SummonerInfo {
            summoner_id: 55501,
            game_name: "Tester".to_string(),
            display_name: "Tester".to_string(),
            tag_line: "BR1".to_string(),
            profile_icon_id: 123,
            summoner_level: 100,
        },
        profile_icon_url: "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/profileicon/123.png"
            .to_string(),
        background_skin_id: 103027,
        background_skin_name: skin_name,
        background_splash_url:
            "https://ddragon.leagueoflegends.com/cdn/img/champion/splash/Ahri_27.jpg".to_string(),
        background_splash_fallback_url:
            "https://ddragon.leagueoflegends.com/cdn/img/champion/splash/Ahri_0.jpg".to_string(),
        ranked_stats: vec![RankedQueueStats {
            queue_type: "RANKED_SOLO_5x5".to_string(),
            queue_label: "Solo/Duo".to_string(),
            tier: "EMERALD".to_string(),
            tier_label: "Esmeralda".to_string(),
            division: "II".to_string(),
            league_points: 50,
            wins: 10,
            losses: 8,
        }],
        top_masteries: vec![],
        recent_matches: vec![],
    };

    save_profile_to_dir(&profile_dir, &profile, "16.22.1").unwrap();
    let now = crate::storage::now_unix_secs();
    let loaded_fresh = load_profile_from_dir(&profile_dir, 55501, now).unwrap();
    assert!(!loaded_fresh.is_stale);

    let mut loaded_stale = load_profile_from_dir(&profile_dir, 55501, now + 400).unwrap();
    assert!(loaded_stale.is_stale);

    // Relocalize cached profile to en_US
    let dd = sample_cache();
    relocalize_player_profile(&mut loaded_stale.profile, AppLocale::EnUs, Some(&dd));
    assert_eq!(loaded_stale.profile.ranked_stats[0].tier_label, "Emerald");

    // Still valid after 25h (within 48h TTL)
    assert!(load_profile_from_dir(&profile_dir, 55501, now + 25 * 3600).is_some());

    // Expired after 48h
    assert!(load_profile_from_dir(&profile_dir, 55501, now + 49 * 3600).is_none());

    let _ = std::fs::remove_dir_all(&dir);
}
