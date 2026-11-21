use super::augments::fetch_aram_mayhem_augments_internal;
use super::cache::{
    ARAM_CACHE_TTL, ARAM_MAYHEM_CACHE_TTL, CachedAramEntry, aram_augments_file_path,
    aram_disk_ttl_secs, aram_ram_ttl, aram_tier_file_path, get_augments_cache, get_tier_cache,
    is_aram_patch_compatible, prewarm_aram_tier_lists_from_dir_at,
};
use super::{OPGG_ARAM_MAYHEM_WEB_URL, OPGG_ARAM_REST_URL, fetch_aram_tier_list_internal};
use crate::build_provider::models::DataDragonCache;
use crate::models::{
    AppLocale, AramAugmentEntry, AramChampionRank, AramMode, ArenaAugmentMetadata, ChampionMetadata,
};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn make_cache_test_dd() -> DataDragonCache {
    let mut champions = HashMap::new();
    champions.insert(
        4,
        ChampionMetadata {
            champion_id: 4,
            key_name: "TwistedFate".to_string(),
            display_name: "Twisted Fate".to_string(),
            icon_url:
                "https://ddragon.leagueoflegends.com/cdn/16.25.1/img/champion/TwistedFate.png"
                    .to_string(),
        },
    );
    let mut arena_augments = HashMap::new();
    arena_augments.insert(
        3075,
        ArenaAugmentMetadata {
            id: 3075,
            name: "Coice".to_string(),
            desc: "Dano extra.".to_string(),
            icon_url: "https://raw.communitydragon.org/latest/recoil.png".to_string(),
            rarity: 4,
            ability_slot: None,
        },
    );
    DataDragonCache {
        version: "16.25.1".to_string(),
        champions,
        arena_augments,
        ..Default::default()
    }
}

fn sample_rank() -> Vec<AramChampionRank> {
    vec![AramChampionRank {
        id: 4,
        key: "twistedfate".to_string(),
        name: "Twisted Fate".to_string(),
        icon_url: "https://ddragon.leagueoflegends.com/cdn/16.25.1/img/champion/TwistedFate.png"
            .to_string(),
        tier: 0,
        rank: 1,
        win_rate: Some(54.2),
        pick_rate: Some(6.1),
    }]
}

#[test]
fn test_aram_mode_ttl_constants_and_patch_compatibility() {
    assert_eq!(aram_ram_ttl(AramMode::Aram), Duration::from_secs(15 * 60));
    assert_eq!(aram_ram_ttl(AramMode::Aram), ARAM_CACHE_TTL);
    assert_eq!(
        aram_ram_ttl(AramMode::AramMayhem),
        Duration::from_secs(60 * 60)
    );
    assert_eq!(aram_ram_ttl(AramMode::AramMayhem), ARAM_MAYHEM_CACHE_TTL);

    assert_eq!(aram_disk_ttl_secs(AramMode::Aram), 48 * 3600);
    assert_eq!(aram_disk_ttl_secs(AramMode::AramMayhem), 72 * 3600);

    assert!(is_aram_patch_compatible("16.25", Some("16.25")));
    assert!(!is_aram_patch_compatible("16.24", Some("16.25")));
    assert!(is_aram_patch_compatible("", Some("16.25")));
    assert!(is_aram_patch_compatible("16.24", None));
}

#[test]
fn test_prewarm_aram_respects_72h_mayhem_ttl_48h_classic_ttl_and_patch_invalidation() {
    let dir = std::env::temp_dir().join(format!("schumazgg_aram_prewarm_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let dd = make_cache_test_dd();
    let now = 2_000_000_000u64;
    let age_60h = now - 60 * 3600; // > 48h (expired for Classic), < 72h (fresh for Mayhem)

    let classic_path = aram_tier_file_path(&dir, AramMode::Aram, AppLocale::PtBr);
    let mayhem_path = aram_tier_file_path(&dir, AramMode::AramMayhem, AppLocale::PtBr);
    crate::storage::write_lz4_envelope_at(&classic_path, "16.25", &sample_rank(), age_60h).unwrap();
    crate::storage::write_lz4_envelope_at(&mayhem_path, "16.25", &sample_rank(), age_60h).unwrap();

    let classic_key = format!("aram:{}:{}", AppLocale::PtBr.as_str(), OPGG_ARAM_REST_URL);
    let mayhem_key = format!(
        "mayhem:{}:{}",
        AppLocale::PtBr.as_str(),
        OPGG_ARAM_MAYHEM_WEB_URL
    );
    if let Ok(mut g) = get_tier_cache().write() {
        g.remove(&classic_key);
        g.remove(&mayhem_key);
    }

    // Same patch (16.25) at 60h age -> only Mayhem is prewarmed (1 of 2)
    let warmed =
        prewarm_aram_tier_lists_from_dir_at(&dir, AppLocale::PtBr, &dd, now, Some("16.25"));
    assert_eq!(warmed, 1);

    // Patch changed to 16.26 -> even fresh Mayhem is invalidated (0 of 2)
    let warmed_new_patch =
        prewarm_aram_tier_lists_from_dir_at(&dir, AppLocale::PtBr, &dd, now, Some("16.26"));
    assert_eq!(warmed_new_patch, 0);

    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn test_mayhem_tier_and_augments_72h_disk_hit_and_patch_invalidation() {
    let dir =
        std::env::temp_dir().join(format!("schumazgg_mayhem_ttl_patch_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let dd = make_cache_test_dd();
    let client = reqwest::Client::new();
    let now = crate::storage::now_unix_secs();
    let age_60h = now.saturating_sub(60 * 3600);

    // 1. Mayhem tier list aged 60h on disk (same patch 16.25) -> 0 network calls
    let mayhem_path = aram_tier_file_path(&dir, AramMode::AramMayhem, AppLocale::PtBr);
    crate::storage::write_lz4_envelope_at(&mayhem_path, "16.25", &sample_rank(), age_60h).unwrap();

    let mock_server = MockServer::start().await;
    let inner_tier = r#"1a:["$","$L32",null,{"champions":[{"key":"twisted_fate","name":"Twisted Fate","image_url":"","champion_id":4,"id":4,"tier":1,"rank":1}]}]"#;
    let html_tier = format!(
        r#"<html><script>self.__next_f.push([1,{}])</script></html>"#,
        serde_json::to_string(inner_tier).unwrap()
    );
    Mock::given(method("GET"))
        .and(path("/mayhem_ttl_tier"))
        .respond_with(ResponseTemplate::new(200).set_body_string(html_tier))
        .expect(1)
        .mount(&mock_server)
        .await;

    let tier_url = format!("{}/mayhem_ttl_tier", mock_server.uri());
    let disk_hit = fetch_aram_tier_list_internal(
        &client,
        AramMode::AramMayhem,
        &dd,
        "http://127.0.0.1:9/unused",
        &tier_url,
        AppLocale::PtBr,
        Some(&dir),
        Some("16.25"),
    )
    .await
    .unwrap();
    assert_eq!(disk_hit[0].id, 4);

    // Same URL with patch bumped to 16.26 -> invalidates BOTH RAM (60 min) and disk (72h), triggers network (.expect(1))
    let refreshed = fetch_aram_tier_list_internal(
        &client,
        AramMode::AramMayhem,
        &dd,
        "http://127.0.0.1:9/unused",
        &tier_url,
        AppLocale::PtBr,
        Some(&dir),
        Some("16.26"),
    )
    .await
    .unwrap();
    assert_eq!(refreshed[0].id, 4);

    // 2. Mayhem augments aged 60h on disk (same patch 16.25) -> disk hit (0 network calls);
    //    patch bump to 16.26 invalidates RAM + disk and fetches from network (.expect(1)).
    let aug_path = aram_augments_file_path(&dir, AppLocale::PtBr, "twistedfate");
    let cached_augs = vec![AramAugmentEntry {
        id: 3075,
        name: "Coice".to_string(),
        tier: 0,
        rarity: Some(4),
        performance: 99.6,
        popular: 3.8,
        icon_url: "https://raw.communitydragon.org/latest/recoil.png".to_string(),
        desc: "Dano extra.".to_string(),
    }];
    crate::storage::write_lz4_envelope_at(&aug_path, "16.25", &cached_augs, age_60h).unwrap();

    let inner_aug = r#"6c:["$","$L6d",null,{"data":[{"id":3075,"tier":0,"performance":99.6,"popular":3.8,"desc":"Dano extra."}]}]"#;
    let html_aug = format!(
        r#"<html><script>self.__next_f.push([1,{}])</script></html>"#,
        serde_json::to_string(inner_aug).unwrap()
    );
    Mock::given(method("GET"))
        .and(path("/mayhem_ttl_aug/twistedfate/augments"))
        .respond_with(ResponseTemplate::new(200).set_body_string(html_aug))
        .expect(1)
        .mount(&mock_server)
        .await;

    let aug_base = format!("{}/mayhem_ttl_aug", mock_server.uri());
    let aug_disk_hit = fetch_aram_mayhem_augments_internal(
        &client,
        "Twisted Fate",
        &aug_base,
        AppLocale::PtBr,
        &dd,
        Some(&dir),
        Some("16.25"),
    )
    .await
    .unwrap();
    assert_eq!(aug_disk_hit[0].id, 3075);

    // Verify 40-minute-old RAM entry is still valid within 60-min Mayhem RAM TTL
    let ram_key = format!("{}:{}:twistedfate", AppLocale::PtBr.as_str(), aug_base);
    if let Ok(mut g) = get_augments_cache().write() {
        g.insert(
            ram_key,
            CachedAramEntry {
                cached_at: Instant::now() - Duration::from_secs(40 * 60),
                patch: "16.25".to_string(),
                payload: cached_augs,
            },
        );
    }
    let aug_ram_hit = fetch_aram_mayhem_augments_internal(
        &client,
        "Twisted Fate",
        &aug_base,
        AppLocale::PtBr,
        &dd,
        Some(&dir),
        Some("16.25"),
    )
    .await
    .unwrap();
    assert_eq!(aug_ram_hit[0].id, 3075);

    // Patch bump to 16.26 invalidates both RAM and disk -> hits mock server once
    let aug_refreshed = fetch_aram_mayhem_augments_internal(
        &client,
        "Twisted Fate",
        &aug_base,
        AppLocale::PtBr,
        &dd,
        Some(&dir),
        Some("16.26"),
    )
    .await
    .unwrap();
    assert_eq!(aug_refreshed[0].id, 3075);

    let _ = std::fs::remove_dir_all(&dir);
}
