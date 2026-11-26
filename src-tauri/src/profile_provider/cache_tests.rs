use std::collections::HashMap;

use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::super::skins::{
    fetch_skin_name_for_locale_with_base, lookup_cached_skin_name, prewarm_skins_from_dir,
    skin_cache, skins_file_path,
};
use super::*;
use crate::models::{
    ArenaAugmentMetadata, ChampionMetadata, ItemMetadata, RankedQueueStats, RecentMatchInfo,
    RuneMetadata, SpellMetadata, SummonerInfo,
};

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

    let mut items = HashMap::new();
    items.insert(
        3089,
        ItemMetadata {
            id: 3089,
            name: "Rabadon's Deathcap".to_string(),
            gold_total: 3500,
            stats: vec!["130 Ability Power".to_string()],
            plaintext: "Massively increases Ability Power".to_string(),
        },
    );
    items.insert(
        3340,
        ItemMetadata {
            id: 3340,
            name: "Stealth Ward".to_string(),
            gold_total: 0,
            stats: vec![],
            plaintext: "Places an invisible ward.".to_string(),
        },
    );

    let mut runes = HashMap::new();
    runes.insert(
        8112,
        RuneMetadata {
            id: 8112,
            key: "Electrocute".to_string(),
            name: "Electrocute".to_string(),
            icon_url: "https://ddragon.leagueoflegends.com/cdn/img/perk-images/Styles/Domination/Electrocute/Electrocute.png".to_string(),
            short_desc: "Burst damage on 3 hits.".to_string(),
        },
    );
    runes.insert(
        8100,
        RuneMetadata {
            id: 8100,
            key: "Domination".to_string(),
            name: "Domination".to_string(),
            icon_url: "https://ddragon.leagueoflegends.com/cdn/img/perk-images/Styles/7200_Domination.png".to_string(),
            short_desc: String::new(),
        },
    );
    runes.insert(
        8200,
        RuneMetadata {
            id: 8200,
            key: "Sorcery".to_string(),
            name: "Sorcery".to_string(),
            icon_url: "https://ddragon.leagueoflegends.com/cdn/img/perk-images/Styles/7202_Sorcery.png".to_string(),
            short_desc: String::new(),
        },
    );

    let mut spells = HashMap::new();
    spells.insert(
        4,
        SpellMetadata {
            id: 4,
            name: "Flash".to_string(),
            icon_url: "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/spell/SummonerFlash.png"
                .to_string(),
            description: "Teleports your champion.".to_string(),
            cooldown_secs: Some(300),
            ..Default::default()
        },
    );
    spells.insert(
        14,
        SpellMetadata {
            id: 14,
            name: "Ignite".to_string(),
            icon_url: "https://ddragon.leagueoflegends.com/cdn/16.22.1/img/spell/SummonerDot.png"
                .to_string(),
            description: "Ignites target enemy.".to_string(),
            cooldown_secs: Some(180),
            ..Default::default()
        },
    );

    let mut arena_augments = HashMap::new();
    arena_augments.insert(
        501,
        ArenaAugmentMetadata {
            id: 501,
            name: "Eureka".to_string(),
            desc: "Gain Ability Haste equal to AP.".to_string(),
            icon_url: "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/assets/ux/cherry/augments/icons/eureka_small.png".to_string(),
            rarity: 8,
            ability_slot: None,
        },
    );

    DataDragonCache {
        version: "16.22.1".to_string(),
        champions,
        runes,
        spells,
        items,
        arena_augments,
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

    // Save & reload profile with SWR detection + EN-US relocalization (including match item_metadata)
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
        recent_matches: vec![
            RecentMatchInfo {
                game_id: 42,
                queue_id: 420,
                champion_id: 103,
                spell1_id: 4,
                spell2_id: 14,
                primary_rune_id: 8112,
                primary_style_id: 8100,
                sub_style_id: 8200,
                item_ids: vec![3089, 0, 0, 0, 0, 0],
                trinket_id: 3340,
                ..Default::default()
            },
            RecentMatchInfo {
                game_id: 43,
                queue_id: 1700,
                game_mode_key: "arena".to_string(),
                champion_id: 103,
                arena_placement: Some(1),
                augment_ids: vec![501],
                ..Default::default()
            },
        ],
        history_next_beg_index: 10,
        history_has_more: true,
        item_metadata: HashMap::new(),
        rune_metadata: HashMap::new(),
        spell_metadata: HashMap::new(),
        augment_metadata: HashMap::new(),
    };

    save_profile_to_dir(&profile_dir, &profile, "16.22.1").unwrap();
    let now = crate::storage::now_unix_secs();
    let loaded_fresh = load_profile_from_dir(&profile_dir, 55501, now).unwrap();
    assert!(!loaded_fresh.is_stale);
    assert_eq!(loaded_fresh.profile.history_next_beg_index, 10);
    assert!(loaded_fresh.profile.history_has_more);

    let mut loaded_stale = load_profile_from_dir(&profile_dir, 55501, now + 400).unwrap();
    assert!(loaded_stale.is_stale);

    // Relocalize cached profile to en_US -> rehydrates item (incl. trinket), rune, spell & augment metadata
    let dd = sample_cache();
    relocalize_player_profile(&mut loaded_stale.profile, AppLocale::EnUs, Some(&dd));
    assert_eq!(loaded_stale.profile.ranked_stats[0].tier_label, "Emerald");
    assert_eq!(
        loaded_stale
            .profile
            .item_metadata
            .get(&3089)
            .map(|m| m.name.as_str()),
        Some("Rabadon's Deathcap")
    );
    assert_eq!(
        loaded_stale
            .profile
            .item_metadata
            .get(&3340)
            .map(|m| m.name.as_str()),
        Some("Stealth Ward")
    );
    assert_eq!(
        loaded_stale
            .profile
            .rune_metadata
            .get(&8112)
            .map(|m| m.name.as_str()),
        Some("Electrocute")
    );
    assert!(!loaded_stale.profile.recent_matches[0]
        .primary_rune_icon_url
        .is_empty());
    assert_eq!(
        loaded_stale
            .profile
            .spell_metadata
            .get(&14)
            .map(|m| m.name.as_str()),
        Some("Ignite")
    );
    assert_eq!(
        loaded_stale
            .profile
            .augment_metadata
            .get(&501)
            .map(|m| m.name.as_str()),
        Some("Eureka")
    );

    // Still valid after 25h (within 48h TTL)
    assert!(load_profile_from_dir(&profile_dir, 55501, now + 25 * 3600).is_some());

    // Expired after 48h
    assert!(load_profile_from_dir(&profile_dir, 55501, now + 49 * 3600).is_none());

    let _ = std::fs::remove_dir_all(&dir);
}
