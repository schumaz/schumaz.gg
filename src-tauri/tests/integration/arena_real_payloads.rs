use schumazgg_lib::build_provider::{
    ChampionBuild, ChampionMetadata, DataDragonCache, ItemBlockKind, ItemClassificationCache,
    TEST_CLASSIFICATION_MUTEX, enrich_build_with_ddragon_for_queue_with_flash_key_and_locale,
    is_arena_prismatic_dynamic, is_completed_item_dynamic, normalize_arena_item_id,
    parse_arena_champion_payload, parse_ddragon_item_classifications,
    set_dynamic_item_classifications,
};
use schumazgg_lib::lcu_injector::{FlashKeyPreference, build_item_sets_payload};
use schumazgg_lib::models::{AppLocale, QUEUE_ARENA};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::MutexGuard;

const DDRAGON_MAP30_JSON: &str = include_str!("../fixtures/arena/ddragon_items_map30.json");
const YUUMI_JSON: &str = include_str!("../fixtures/arena/yuumi_350.json");
const MASTER_YI_JSON: &str = include_str!("../fixtures/arena/master_yi_11.json");
const RAKAN_JSON: &str = include_str!("../fixtures/arena/rakan_497.json");
const VOLIBEAR_JSON: &str = include_str!("../fixtures/arena/volibear_106.json");
const SHACO_JSON: &str = include_str!("../fixtures/arena/shaco_35.json");
const EZREAL_JSON: &str = include_str!("../fixtures/arena/ezreal_81.json");

fn lock_classifications() -> MutexGuard<'static, ()> {
    TEST_CLASSIFICATION_MUTEX
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

fn install_real_ddragon_classifications() -> ItemClassificationCache {
    let items_json: Value =
        serde_json::from_str(DDRAGON_MAP30_JSON).expect("valid real DataDragon item fixture");
    let cache = parse_ddragon_item_classifications(&items_json);
    set_dynamic_item_classifications(cache.clone());
    cache
}

fn make_real_ddragon_cache() -> DataDragonCache {
    let mut champions = HashMap::new();
    for (id, key, name) in [
        (350, "Yuumi", "Yuumi"),
        (11, "MasterYi", "Master Yi"),
        (497, "Rakan", "Rakan"),
        (106, "Volibear", "Volibear"),
        (35, "Shaco", "Shaco"),
        (81, "Ezreal", "Ezreal"),
    ] {
        champions.insert(
            id,
            ChampionMetadata {
                champion_id: id,
                key_name: key.to_string(),
                display_name: name.to_string(),
                icon_url: format!(
                    "https://ddragon.leagueoflegends.com/cdn/16.23.1/img/champion/{key}.png"
                ),
            },
        );
    }
    DataDragonCache {
        version: "16.23.1".to_string(),
        champions,
        ..Default::default()
    }
}

fn run_arena_pipeline(
    raw_json: &str,
    champion_id: i32,
    champion_name: &str,
    dd: &DataDragonCache,
) -> ChampionBuild {
    let payload: Value = serde_json::from_str(raw_json).expect("valid real OP.GG Arena JSON");
    let mut build = parse_arena_champion_payload(&payload, champion_id, champion_name, dd)
        .expect("Arena payload should parse cleanly");
    enrich_build_with_ddragon_for_queue_with_flash_key_and_locale(
        &mut build,
        dd,
        FlashKeyPreference::D,
        AppLocale::PtBr,
        Some(QUEUE_ARENA),
    );
    build
}

fn block_ids(build: &ChampionBuild, kind: ItemBlockKind) -> &[i32] {
    &build
        .item_blocks
        .iter()
        .find(|b| b.kind == kind)
        .expect("expected Arena item block")
        .item_ids
}

fn assert_arena_invariants(
    champ_label: &str,
    build: &ChampionBuild,
    cls: &ItemClassificationCache,
) {
    assert!(
        !build.supports_runes,
        "{champ_label}: runes must be disabled"
    );
    assert!(
        !build.supports_spells,
        "{champ_label}: spells must be disabled"
    );
    assert_eq!(build.spell1_id, 0);
    assert_eq!(build.spell2_id, 0);
    assert!(build.spells.is_empty());
    assert_eq!(build.item_blocks.len(), 5);
    assert_eq!(build.item_blocks[0].kind, ItemBlockKind::Prismatic);
    assert_eq!(build.item_blocks[1].kind, ItemBlockKind::Core);
    assert_eq!(build.item_blocks[2].kind, ItemBlockKind::Boots);
    assert_eq!(build.item_blocks[3].kind, ItemBlockKind::Final);
    assert_eq!(build.item_blocks[4].kind, ItemBlockKind::Situational);
    assert!(!build.build_options.is_empty());
    assert!(!build.arena_augments.is_empty());
    assert!(!build.arena_synergies.is_empty());

    let sr_starters_or_bad: HashSet<i32> = [
        1054, 1055, 1056, 2003, 2031, 3006, 3020, 3047, 3111, 3158, 3119, 220000, 220001, 220003,
        220007, 220012, 223042, 223040, 223121, 222530,
    ]
    .into_iter()
    .collect();

    for block in &build.item_blocks {
        assert_eq!(
            block.item_ids.len(),
            block.item_icon_urls.len(),
            "{champ_label}: block {:?} icon URLs must match item IDs",
            block.kind
        );
        for &id in &block.item_ids {
            assert!(
                cls.map30_items.contains(&id),
                "{champ_label}: item {id} in {:?} is NOT enabled on Map 30",
                block.kind
            );
            assert!(
                !sr_starters_or_bad.contains(&id),
                "{champ_label}: forbidden/evolved/SR item {id} leaked into {:?}",
                block.kind
            );
        }
    }

    let mut seen_core_sets: HashSet<Vec<i32>> = HashSet::new();
    let pick_rate_sum: f64 = build.build_options.iter().map(|o| o.pick_rate).sum();
    assert!(
        (99.5..=100.5).contains(&pick_rate_sum),
        "{champ_label}: build option pick_rate sum must be ~100%, got {pick_rate_sum}"
    );

    for opt in &build.build_options {
        let mut sorted_core = opt.core_item_ids.clone();
        sorted_core.sort_unstable();
        assert!(
            seen_core_sets.insert(sorted_core),
            "{champ_label}: duplicate/permuted core trio detected: {:?}",
            opt.core_item_ids
        );

        for &id in opt
            .core_item_ids
            .iter()
            .chain(opt.final_item_ids.iter())
            .chain(opt.situational_item_ids.iter())
        {
            assert!(
                cls.map30_items.contains(&id),
                "{champ_label}: option {} item {id} is NOT on Map 30",
                opt.name
            );
            assert!(
                !cls.arena_prismatics.contains(&id),
                "{champ_label}: Prismatic {id} leaked into {}",
                opt.name
            );
            assert!(
                !cls.boots.contains(&id),
                "{champ_label}: Boot {id} leaked into {}",
                opt.name
            );
            assert!(
                !cls.arena_consumables.contains(&id),
                "{champ_label}: Anvil/Consumable {id} leaked into {}",
                opt.name
            );
            assert!(
                !sr_starters_or_bad.contains(&id),
                "{champ_label}: forbidden item {id} leaked into {}",
                opt.name
            );
        }
    }

    let item_set = build_item_sets_payload(999_001, build);
    let maps = item_set
        .pointer("/itemSets/0/associatedMaps")
        .and_then(|v| v.as_array())
        .expect("associatedMaps in LCU item set payload");
    assert!(
        maps.iter().any(|m| m.as_i64() == Some(30)),
        "{champ_label}: LCU item set payload must include Map 30"
    );
}

#[test]
fn test_real_arena_ddragon_classifications_handle_447111_223121_4017_and_anvils() {
    let _lock = lock_classifications();
    let cls = install_real_ddragon_classifications();

    // 447111 (Armadura Sangrenta do Suserano, 2500g) MUST be a completed Map 30 legendary, NEVER Prismatic
    assert!(cls.map30_items.contains(&447111));
    assert!(is_completed_item_dynamic(447111));
    assert!(!is_arena_prismatic_dynamic(447111));

    // Real 2750g Prismatics MUST be classified as arena_prismatics and NOT completed legendaries
    for prism in [443090, 447109, 447105, 446667, 447120] {
        assert!(
            is_arena_prismatic_dynamic(prism),
            "expected prismatic {prism}"
        );
        assert!(!is_completed_item_dynamic(prism));
    }

    // Riot's real DataDragon bug: 223121 (Fimbulwinter) has `specialRecipe: 3119` (SR ID outside Map 30).
    // Normalization MUST resolve to Map 30 base `223119`, NEVER `3119`.
    assert_eq!(normalize_arena_item_id(223121), 223119);
    assert_eq!(normalize_arena_item_id(223042), 223004);
    assert_eq!(normalize_arena_item_id(223040), 223003);
    assert_eq!(normalize_arena_item_id(222530), 222526);

    // 4-digit Arena exclusives and anvils/Shardholder
    assert!(cls.map30_items.contains(&4017));
    assert!(is_completed_item_dynamic(4017));
    assert!(cls.arena_consumables.contains(&220007));
    assert!(cls.arena_consumables.contains(&220012));
}

#[test]
fn test_real_arena_yuumi_350_contract() {
    let _lock = lock_classifications();
    let cls = install_real_ddragon_classifications();
    let dd = make_real_ddragon_cache();

    let build = run_arena_pipeline(YUUMI_JSON, 350, "Yuumi", &dd);
    assert_arena_invariants("Yuumi", &build, &cls);

    // Boots: Ionianas (223158) + Feiticeiro (223020), NO Grevas do Berserker (223006)
    let boots = block_ids(&build, ItemBlockKind::Boots);
    assert_eq!(boots, &[223158, 223020]);
    assert!(!boots.contains(&223006));

    // Top 5 Prismatics: Promessa Empírea, Virtude Radiante, Glacieterno, Presente Sanguinário, Capa do Piromante
    let prismatics = block_ids(&build, ItemBlockKind::Prismatic);
    assert_eq!(&prismatics[..5], &[447105, 446667, 446656, 443062, 447118]);

    // Core 1: Pedra Lunar (226617) -> Turíbulo Ardente (223504) -> Auronúcleo (226621)
    assert_eq!(
        block_ids(&build, ItemBlockKind::Core),
        &[226617, 223504, 226621]
    );
    assert_eq!(
        build.build_options[0].core_item_ids,
        vec![226617, 223504, 226621]
    );
}

#[test]
fn test_real_arena_master_yi_11_contract() {
    let _lock = lock_classifications();
    let cls = install_real_ddragon_classifications();
    let dd = make_real_ddragon_cache();

    let build = run_arena_pipeline(MASTER_YI_JSON, 11, "Master Yi", &dd);
    assert_arena_invariants("Master Yi", &build, &cls);

    // Boots include Grevas do Berserker (223006) and Grevas Vorazes (223008)
    let boots = block_ids(&build, ItemBlockKind::Boots);
    assert_eq!(&boots[..2], &[223006, 223008]);

    // Top 5 Prismatics: Colheita do Ceifador, Garras de Aço Sombrio, Debilitador, Espada da Miragem, Companheiro Hexraio
    let prismatics = block_ids(&build, ItemBlockKind::Prismatic);
    assert_eq!(&prismatics[..5], &[443090, 443054, 443069, 447100, 443081]);

    // Core 1: Rei Destruído (223153) -> Guinsoo (223124) -> Mata-Cráquens (226672)
    // Core 2: Rei Destruído (223153) -> Guinsoo (223124) -> Terminus (223302)
    assert!(build.build_options.len() >= 2);
    assert_eq!(
        build.build_options[0].core_item_ids,
        vec![223153, 223124, 226672]
    );
    assert_eq!(
        build.build_options[1].core_item_ids,
        vec![223153, 223124, 223302]
    );
}

#[test]
fn test_real_arena_rakan_497_contract() {
    let _lock = lock_classifications();
    let cls = install_real_ddragon_classifications();
    let dd = make_real_ddragon_cache();

    let build = run_arena_pipeline(RAKAN_JSON, 497, "Rakan", &dd);
    assert_arena_invariants("Rakan", &build, &cls);

    // Boots: Passos de Mercúrio (223111), Botas Galvanizadas de Aço (223047), Ionianas (223158)
    assert_eq!(
        block_ids(&build, ItemBlockKind::Boots),
        &[223111, 223047, 223158]
    );

    // Top 5 Prismatics contain Crueldade (447109), Manopla do Buraco Negro (447122),
    // Bastão Eletrizante (447119), Virtude Radiante (446667), Promessa Empírea (447105)
    let top5_prism: HashSet<i32> = block_ids(&build, ItemBlockKind::Prismatic)[..5]
        .iter()
        .copied()
        .collect();
    for expected in [447109, 447122, 447119, 446667, 447105] {
        assert!(
            top5_prism.contains(&expected),
            "Rakan top 5 prismatics missing {expected}: {:?}",
            block_ids(&build, ItemBlockKind::Prismatic)
        );
    }

    // Core 1: Coração de Aço (223084) -> Armadura de Espinhos (223075) -> Rookern Lamúrico (222504)
    // Core 2: Égide de Fogo Solar (223068) -> Jak'Sho (226665) -> Armadura de Espinhos (223075)
    assert!(build.build_options.len() >= 2);
    assert_eq!(
        build.build_options[0].core_item_ids,
        vec![223084, 223075, 222504]
    );
    assert_eq!(
        build.build_options[1].core_item_ids,
        vec![223068, 226665, 223075]
    );
}

#[test]
fn test_real_arena_volibear_106_preserves_447111_bloodmail() {
    let _lock = lock_classifications();
    let cls = install_real_ddragon_classifications();
    let dd = make_real_ddragon_cache();

    let build = run_arena_pipeline(VOLIBEAR_JSON, 106, "Volibear", &dd);
    assert_arena_invariants("Volibear", &build, &cls);

    // Core 1: Coração de Aço (223084) -> Armadura Sangrenta do Suserano (447111) -> Semblante Espiritual (223065)
    assert_eq!(
        block_ids(&build, ItemBlockKind::Core),
        &[223084, 447111, 223065]
    );
    assert_eq!(
        build.build_options[0].core_item_ids,
        vec![223084, 447111, 223065]
    );
    assert!(
        !block_ids(&build, ItemBlockKind::Prismatic).contains(&447111),
        "447111 (2500g) must NOT appear in Prismatic block"
    );
}

#[test]
fn test_real_arena_shaco_35_separates_ap_and_ad_archetypes() {
    let _lock = lock_classifications();
    let cls = install_real_ddragon_classifications();
    let dd = make_real_ddragon_cache();

    let build = run_arena_pipeline(SHACO_JSON, 35, "Shaco", &dd);
    assert_arena_invariants("Shaco", &build, &cls);

    assert_eq!(build.build_options.len(), 3);
    // Build 1 (AP): Liandry (226653) -> Tocha de Chamas Negras (222503) -> Rabadon (223089)
    let ap_opt = &build.build_options[0];
    assert_eq!(ap_opt.core_item_ids, vec![226653, 222503, 223089]);
    for ad_id in [
        226699, 226701, 223814, 226676, 223031, 223036, 226697, 226692, 4017,
    ] {
        assert!(
            !ap_opt.final_item_ids.contains(&ad_id)
                && !ap_opt.situational_item_ids.contains(&ad_id),
            "Pure AD item {ad_id} leaked into AP Shaco Build 1"
        );
    }

    // Build 2 (AD Crit) & Build 3 (AD Lethality): zero pure AP items in Final/Situational
    let ad_crit = &build.build_options[1];
    let ad_leth = &build.build_options[2];
    assert_eq!(ad_crit.core_item_ids, vec![226676, 223031, 223036]);
    assert_eq!(ad_leth.core_item_ids, vec![226699, 226701, 223814]);
    for ad_opt in [ad_crit, ad_leth] {
        for ap_id in [226653, 222503, 223089, 223135, 223157, 224645] {
            assert!(
                !ad_opt.final_item_ids.contains(&ap_id)
                    && !ad_opt.situational_item_ids.contains(&ap_id),
                "Pure AP item {ap_id} leaked into AD Shaco {}",
                ad_opt.name
            );
        }
    }
}

#[test]
fn test_real_arena_ezreal_81_tear_normalization_and_4017_exclusive() {
    let _lock = lock_classifications();
    let cls = install_real_ddragon_classifications();
    let dd = make_real_ddragon_cache();

    let build = run_arena_pipeline(EZREAL_JSON, 81, "Ezreal", &dd);
    assert_arena_invariants("Ezreal", &build, &cls);

    // Core 1 normalizes 223042 (Muramana) -> 223004 (Manamune)
    assert_eq!(
        build.build_options[0].core_item_ids,
        vec![223004, 223031, 223036]
    );

    // 4-digit Arena exclusive 4017 (Machadinha Infernal) is preserved in Final or Situational
    let has_4017 = build
        .build_options
        .iter()
        .any(|opt| opt.final_item_ids.contains(&4017) || opt.situational_item_ids.contains(&4017));
    assert!(
        has_4017,
        "4-digit Arena exclusive 4017 (Machadinha Infernal) must be preserved in Ezreal's build"
    );
}
