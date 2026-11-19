use super::*;
use crate::build_provider::*;
use crate::lcu_injector::FlashKeyPreference;
use crate::models::{AppLocale, QUEUE_ARAM, QUEUE_ARAM_MAYHEM};
use std::collections::HashMap;

#[test]
fn test_enrich_build_with_ddragon_populates_urls_spells_and_shards() {
    let mut ddragon_cache = DataDragonCache {
        version: "14.23.1".to_string(),
        champions: HashMap::new(),
        runes: HashMap::new(),
        spells: HashMap::new(),
    };

    ddragon_cache.champions.insert(
        103,
        ChampionMetadata {
            champion_id: 103,
            key_name: "Ahri".to_string(),
            display_name: "Ahri".to_string(),
            icon_url: "https://ddragon.leagueoflegends.com/cdn/14.23.1/img/champion/Ahri.png"
                .to_string(),
        },
    );

    ddragon_cache.runes.insert(
        8100,
        RuneMetadata {
            id: 8100,
            key: "Domination".to_string(),
            name: "Dominação".to_string(),
            icon_url:
                "https://ddragon.leagueoflegends.com/cdn/img/perk-images/Styles/7200_Domination.png"
                    .to_string(),
        },
    );

    let mut build = build_fallback_champion_build(103, "Ahri", 420);
    enrich_build_with_ddragon_with_flash_key(&mut build, &ddragon_cache, FlashKeyPreference::D);

    assert_eq!(
        build.champion_icon_url,
        "https://ddragon.leagueoflegends.com/cdn/14.23.1/img/champion/Ahri.png"
    );
    assert_eq!(build.spells.len(), 2);
    assert_eq!(build.spells[0].name, "Flash");
    assert!(build.spells[0].icon_url.contains("SummonerFlash.png"));
    assert_eq!(build.perks.len(), 9);
    assert_eq!(build.item_blocks.len(), 5);
    assert!(!build.item_blocks[0].item_icon_urls.is_empty());
    assert!(build.item_blocks[0].item_icon_urls[0].contains("/img/item/"));
}

#[test]
fn test_get_spell_metadata_uses_official_pt_br_names() {
    let ver = "16.22.1";
    assert_eq!(get_spell_metadata(ver, 32).name, "Marca");
    assert_eq!(get_spell_metadata(ver, 13).name, "Clareza");
    assert_eq!(get_spell_metadata(ver, 12).name, "Teleporte");
    assert_eq!(get_spell_metadata(ver, 7).name, "Curar");
    assert_eq!(get_spell_metadata(ver, 21).name, "Barreira");
    assert_eq!(get_spell_metadata(ver, 1).name, "Purificar");
    assert_eq!(get_spell_metadata(ver, 3).name, "Exaustão");
    assert_eq!(get_spell_metadata(ver, 6).name, "Fantasma");
    assert_eq!(get_spell_metadata(ver, 14).name, "Incendiar");
    assert_eq!(get_spell_metadata(ver, 11).name, "Golpear");
    assert_eq!(get_spell_metadata(ver, 4).name, "Flash");
    assert!(
        get_spell_metadata(ver, 32)
            .icon_url
            .ends_with("/16.22.1/img/spell/SummonerSnowball.png")
    );
    assert!(
        get_spell_metadata(ver, 13)
            .icon_url
            .ends_with("/16.22.1/img/spell/SummonerMana.png")
    );
}

#[test]
fn test_enrich_build_with_ddragon_with_flash_key_applies_preference_in_all_modes() {
    let cache = DataDragonCache {
        version: "16.22.1".to_string(),
        champions: HashMap::new(),
        runes: HashMap::new(),
        spells: HashMap::new(),
    };

    // 1. ARAM Tristana: Flash (4) + Curar (7) primary, Barreira (21) secondary
    let mut tristana_aram = ChampionBuild {
        champion_id: 18,
        champion_name: "Tristana".to_string(),
        spell1_id: 7,
        spell2_id: 4,
        spell_choices: vec![
            SpellMetadata {
                id: 7,
                ..Default::default()
            },
            SpellMetadata {
                id: 21,
                ..Default::default()
            },
        ],
        active_position: String::new(),
        ..Default::default()
    };

    enrich_build_with_ddragon_with_flash_key(&mut tristana_aram, &cache, FlashKeyPreference::D);
    assert_eq!(tristana_aram.spell1_id, 4);
    assert_eq!(tristana_aram.spell2_id, 7);
    assert_eq!(tristana_aram.spells[0].name, "Flash");
    assert_eq!(tristana_aram.spells[1].name, "Curar");
    assert_eq!(tristana_aram.spell_choices[0].name, "Curar");
    assert_eq!(tristana_aram.spell_choices[1].name, "Barreira");

    enrich_build_with_ddragon_with_flash_key(&mut tristana_aram, &cache, FlashKeyPreference::F);
    assert_eq!(tristana_aram.spell1_id, 7);
    assert_eq!(tristana_aram.spell2_id, 4);
    assert_eq!(tristana_aram.spells[0].name, "Curar");
    assert_eq!(tristana_aram.spells[1].name, "Flash");

    // 2. ARAM with Marca (32) + Flash (4) in numeric/reversed order [32, 4]
    let mut ahri_aram = ChampionBuild {
        champion_id: 103,
        champion_name: "Ahri".to_string(),
        spell1_id: 32,
        spell2_id: 4,
        active_position: String::new(),
        ..Default::default()
    };
    enrich_build_with_ddragon_with_flash_key(&mut ahri_aram, &cache, FlashKeyPreference::D);
    assert_eq!(ahri_aram.spell1_id, 4);
    assert_eq!(ahri_aram.spell2_id, 32);
    assert_eq!(ahri_aram.spells[0].name, "Flash");
    assert_eq!(ahri_aram.spells[1].name, "Marca");

    // 3. ARAM Mayhem with numerically ordered [1, 4] (Purificar + Flash)
    let mut cleanse_mayhem = ChampionBuild {
        champion_id: 222,
        champion_name: "Jinx".to_string(),
        spell1_id: 1,
        spell2_id: 4,
        supports_runes: false,
        active_position: String::new(),
        ..Default::default()
    };
    enrich_build_with_ddragon_with_flash_key(&mut cleanse_mayhem, &cache, FlashKeyPreference::D);
    assert_eq!(cleanse_mayhem.spell1_id, 4);
    assert_eq!(cleanse_mayhem.spell2_id, 1);
    assert_eq!(cleanse_mayhem.spells[0].name, "Flash");
    assert_eq!(cleanse_mayhem.spells[1].name, "Purificar");
    assert_eq!(cleanse_mayhem.spell_choices[1].id, 32);

    // 4. Summoner's Rift non-jungle ("mid") also respects FlashKeyPreference
    let mut mid_sr = ChampionBuild {
        champion_id: 103,
        champion_name: "Ahri".to_string(),
        spell1_id: 4,
        spell2_id: 12,
        active_position: "mid".to_string(),
        ..Default::default()
    };
    enrich_build_with_ddragon_with_flash_key(&mut mid_sr, &cache, FlashKeyPreference::F);
    assert_eq!(mid_sr.spell1_id, 12);
    assert_eq!(mid_sr.spell2_id, 4);
    assert_eq!(mid_sr.spells[0].name, "Teleporte");
    assert_eq!(mid_sr.spells[1].name, "Flash");
}

#[test]
fn test_enrichment_sanitizes_exhaust_in_mayhem_and_teleport_in_aram_fallbacks() {
    let cache = DataDragonCache {
        version: "16.22.1".to_string(),
        champions: HashMap::new(),
        runes: HashMap::new(),
        spells: HashMap::new(),
    };

    // Stale/legacy ARAM Mayhem build with Flash + Exhaust and [Exhaust, Heal] choices
    let mut lulu_mayhem = ChampionBuild {
        champion_id: 117,
        champion_name: "Lulu".to_string(),
        spell1_id: 4,
        spell2_id: 3,
        spell_choices: vec![
            SpellMetadata {
                id: 3,
                ..Default::default()
            },
            SpellMetadata {
                id: 7,
                ..Default::default()
            },
        ],
        supports_runes: false,
        ..Default::default()
    };
    enrich_build_with_ddragon_for_queue_with_flash_key_and_locale(
        &mut lulu_mayhem,
        &cache,
        FlashKeyPreference::F,
        AppLocale::PtBr,
        Some(QUEUE_ARAM_MAYHEM),
    );
    assert_eq!(lulu_mayhem.spell1_id, 7);
    assert_eq!(lulu_mayhem.spell2_id, 4);
    assert_eq!(
        lulu_mayhem
            .spell_choices
            .iter()
            .map(|s| s.id)
            .collect::<Vec<_>>(),
        vec![7, 32]
    );

    // Classic ARAM (450) with Ignite (14) and empty choices must fall back to Mark (32), never Teleport (12)
    let mut ignite_aram = ChampionBuild {
        champion_id: 103,
        champion_name: "Ahri".to_string(),
        spell1_id: 4,
        spell2_id: 14,
        supports_runes: true,
        ..Default::default()
    };
    enrich_build_with_ddragon_for_queue_with_flash_key_and_locale(
        &mut ignite_aram,
        &cache,
        FlashKeyPreference::D,
        AppLocale::PtBr,
        Some(QUEUE_ARAM),
    );
    assert_eq!(
        ignite_aram
            .spell_choices
            .iter()
            .map(|s| s.id)
            .collect::<Vec<_>>(),
        vec![14, 32]
    );

    // Barrier (21) with empty choices in ARAM Mayhem must fall back to Mark (32), never Exhaust (3)
    let mut barrier_mayhem = ChampionBuild {
        champion_id: 18,
        champion_name: "Tristana".to_string(),
        spell1_id: 4,
        spell2_id: 21,
        supports_runes: false,
        ..Default::default()
    };
    enrich_build_with_ddragon_for_queue_with_flash_key_and_locale(
        &mut barrier_mayhem,
        &cache,
        FlashKeyPreference::D,
        AppLocale::PtBr,
        Some(QUEUE_ARAM_MAYHEM),
    );
    assert_eq!(
        barrier_mayhem
            .spell_choices
            .iter()
            .map(|s| s.id)
            .collect::<Vec<_>>(),
        vec![21, 32]
    );
}

#[test]
fn test_en_us_localization_for_shards_spells_and_build_labels() {
    assert_eq!(
        get_stat_shard_metadata_for_locale(5008, AppLocale::EnUs)
            .unwrap()
            .name,
        "Adaptive Force"
    );
    assert_eq!(
        get_stat_shard_metadata_for_locale(5005, AppLocale::EnUs)
            .unwrap()
            .name,
        "Attack Speed"
    );
    assert_eq!(
        get_stat_shard_metadata_for_locale(5001, AppLocale::EnUs)
            .unwrap()
            .name,
        "Health Scaling"
    );

    assert_eq!(
        get_spell_metadata_for_locale("16.22.1", 14, AppLocale::EnUs).name,
        "Ignite"
    );
    assert_eq!(
        get_spell_metadata_for_locale("16.22.1", 32, AppLocale::EnUs).name,
        "Mark"
    );
    assert_eq!(
        get_spell_metadata_for_locale("16.22.1", 7, AppLocale::EnUs).name,
        "Heal"
    );

    let mut build = build_fallback_champion_build(103, "Ahri", 420);
    build.build_options.push(UnifiedBuildOption {
        name: "Build 1 (Mais Feita)".to_string(),
        ..Default::default()
    });
    let cache = DataDragonCache {
        version: "16.22.1".to_string(),
        champions: HashMap::new(),
        runes: HashMap::new(),
        spells: HashMap::new(),
    };

    enrich_build_with_ddragon_with_flash_key_and_locale(
        &mut build,
        &cache,
        FlashKeyPreference::D,
        AppLocale::EnUs,
    );

    let titles: Vec<&str> = build.item_blocks.iter().map(|b| b.title.as_str()).collect();
    assert!(titles.contains(&"Starter Items"));
    assert!(titles.contains(&"First 3 Items"));
    assert!(titles.contains(&"Top 3 Boots"));
    assert!(titles.contains(&"Final Build"));
    assert!(titles.contains(&"Situational / Utility Items"));
    assert_eq!(build.build_options[0].name, "Build 1 (Most Popular)");
    assert_eq!(build.spells[1].name, "Ignite");
}
