use super::*;
use crate::build_provider::*;
use crate::models::{QUEUE_ARAM, QUEUE_ARAM_MAYHEM, QUEUE_SOLO_DUO};
use std::collections::HashMap;

fn sample_ddragon_spells_catalog() -> HashMap<i32, SpellMetadata> {
    let entries: &[(i32, &str, &[&str])] = &[
        (4, "Flash", &["CLASSIC", "ARAM", "KIWI"]),
        (3, "Exaustão", &["CLASSIC", "ARAM"]),
        (12, "Teleporte", &["CLASSIC"]),
        (11, "Golpear", &["CLASSIC"]),
        (32, "Marca", &["ARAM", "KIWI"]),
        (7, "Curar", &["CLASSIC", "ARAM", "KIWI"]),
        (21, "Barreira", &["CLASSIC", "ARAM", "KIWI"]),
        (6, "Fantasma", &["CLASSIC", "ARAM", "KIWI"]),
        (14, "Incendiar", &["CLASSIC", "ARAM", "KIWI"]),
        (1, "Purificar", &["CLASSIC", "ARAM", "KIWI"]),
        (13, "Clareza", &["ARAM", "KIWI"]),
    ];
    entries
        .iter()
        .map(|&(id, name, modes)| {
            (
                id,
                SpellMetadata {
                    id,
                    name: name.to_string(),
                    icon_url: String::new(),
                    modes: modes.iter().map(|m| (*m).to_string()).collect(),
                    ..Default::default()
                },
            )
        })
        .collect()
}

#[test]
fn test_extract_summoner_spell_choices_yunara_and_jungle() {
    let yunara_json = serde_json::json!({
        "summoner_spells": [
            { "ids": [4, 21], "play": 61208, "win": 30365 },
            { "ids": [3, 4], "play": 17302, "win": 8780 }
        ]
    });
    let (top_spells, choices) = extract_summoner_spell_choices(&yunara_json, false);
    assert_eq!(top_spells, (4, 21));
    assert_eq!(choices, vec![21, 3]);

    let hecarim_json = serde_json::json!({
        "summoner_spells": [
            { "ids": [6, 11], "play": 45000, "win": 23000 },
            { "ids": [4, 11], "play": 12000, "win": 6000 }
        ]
    });
    let (jng_spells, jng_choices) = extract_summoner_spell_choices(&hecarim_json, true);
    assert_eq!(jng_spells, (6, 11));
    assert_eq!(jng_choices, vec![6, 4]);

    let shaco_json = serde_json::json!({
        "summoner_spells": [
            { "ids": [14, 11], "play": 38000, "win": 20000 },
            { "ids": [4, 11], "play": 5000, "win": 2500 }
        ]
    });
    let (_, shaco_choices) = extract_summoner_spell_choices(&shaco_json, true);
    assert_eq!(shaco_choices, vec![14, 4]);
}

#[test]
fn test_extract_summoner_spell_choices_aram_tristana_and_numeric_opgg_order() {
    let tristana_aram = serde_json::json!({
        "summoner_spells": [
            { "ids": [4, 32], "play": 8500, "win": 4300 },
            { "ids": [4, 7], "play": 42000, "win": 22500 },
            { "ids": [4, 21], "play": 19500, "win": 10300 }
        ]
    });
    let (top_spells, choices) = extract_summoner_spell_choices(&tristana_aram, false);
    assert_eq!(top_spells, (4, 7));
    assert_eq!(choices, vec![7, 21]);

    let cleanse_flash = serde_json::json!({
        "summoner_spells": [
            { "ids": [1, 4], "play": 28000, "win": 14500 },
            { "ids": [3, 4], "play": 12000, "win": 6100 }
        ]
    });
    let (cleanse_top, cleanse_choices) = extract_summoner_spell_choices(&cleanse_flash, false);
    assert_eq!(cleanse_top, (4, 1));
    assert_eq!(cleanse_choices, vec![1, 3]);
}

#[test]
fn test_extract_most_used_summoner_spells_from_opgg_rest_structure() {
    let json = serde_json::json!({
        "data": {
            "win_rate": 49.5,
            "pick_rate": 12.1,
            "runes": {
                "primary_style_id": 8000,
                "sub_style_id": 8300,
                "primary_perk_ids": [8008, 9111, 9104, 8017],
                "sub_perk_ids": [8304, 8345],
                "stat_shards": [5005, 5008, 5011]
            },
            "summoner_spells": [
                { "ids": [4, 21], "win": 30822, "play": 62087, "pick_rate": 0.7006 },
                { "ids": [3, 4], "win": 8894, "play": 17529, "pick_rate": 0.1978 },
                { "ids": [4, 14], "win": 250, "play": 500, "pick_rate": 0.005 }
            ],
            "items": {
                "starters": [1055, 2003],
                "core": [3031, 3094, 3036]
            }
        }
    });

    let build = parse_opgg_analysis_payload(&json, 804, "Yunara")
        .expect("Parsing Yunara with OP.GG summoner_spells array should succeed");

    assert_eq!(build.spell1_id, 4);
    assert_eq!(build.spell2_id, 21);
}

#[test]
fn test_aram_mayhem_filters_exhaust_while_classic_aram_preserves_it_when_supported() {
    let catalog = sample_ddragon_spells_catalog();
    let lulu_aram = serde_json::json!({
        "summoner_spells": [
            { "ids": [3, 4], "play": 22000, "win": 11500 },
            { "ids": [4, 7], "play": 12000, "win": 6200 },
            { "ids": [4, 21], "play": 7000, "win": 3600 }
        ]
    });

    // ARAM Mayhem (2400 -> "KIWI"): Exhaust (3) must be filtered out; Flash + Heal (7) promoted.
    let mayhem_mode = ddragon_mode_for_queue(QUEUE_ARAM_MAYHEM);
    assert_eq!(mayhem_mode, "KIWI");
    let (mayhem_top, mayhem_choices) = extract_summoner_spell_choices_for_mode(
        &lulu_aram,
        false,
        Some(mayhem_mode),
        Some(&catalog),
    );
    assert_eq!(mayhem_top, (4, 7));
    assert_eq!(mayhem_choices, vec![7, 21]);
    assert!(!mayhem_choices.contains(&SPELL_EXHAUST_ID));
    assert!(!mayhem_choices.contains(&SPELL_TELEPORT_ID));

    // Classic ARAM (450 -> "ARAM"): Exhaust (3) remains allowed while DataDragon lists "ARAM".
    let aram_mode = ddragon_mode_for_queue(QUEUE_ARAM);
    assert_eq!(aram_mode, "ARAM");
    let (aram_top, aram_choices) =
        extract_summoner_spell_choices_for_mode(&lulu_aram, false, Some(aram_mode), Some(&catalog));
    assert_eq!(aram_top, (4, 3));
    assert_eq!(aram_choices, vec![3, 7]);

    // If DataDragon removes "ARAM" from Exhaust, Classic ARAM also drops it dynamically.
    let mut updated_catalog = catalog.clone();
    updated_catalog.get_mut(&3).unwrap().modes = vec!["CLASSIC".to_string()];
    let (aram_no_exh_top, aram_no_exh_choices) = extract_summoner_spell_choices_for_mode(
        &lulu_aram,
        false,
        Some(aram_mode),
        Some(&updated_catalog),
    );
    assert_eq!(aram_no_exh_top, (4, 7));
    assert_eq!(aram_no_exh_choices, vec![7, 21]);
}

#[test]
fn test_aram_and_mayhem_fallbacks_never_emit_exhaust_in_mayhem_or_teleport_in_aram() {
    let catalog = sample_ddragon_spells_catalog();

    // Jayce in ARAM Mayhem: #1 is Mark (32), #2 is Exhaust (3) -> must drop Exhaust and fall back to Heal (7).
    let jayce_only_mark_and_exhaust = serde_json::json!({
        "summoner_spells": [
            { "ids": [4, 32], "play": 18000, "win": 9200 },
            { "ids": [3, 4], "play": 9500, "win": 4800 }
        ]
    });
    let (jayce_top, jayce_choices) = extract_summoner_spell_choices_for_mode(
        &jayce_only_mark_and_exhaust,
        false,
        Some(ddragon_mode_for_queue(QUEUE_ARAM_MAYHEM)),
        Some(&catalog),
    );
    assert_eq!(jayce_top, (4, 32));
    assert_eq!(jayce_choices, vec![32, 7]);
    assert!(!jayce_choices.contains(&SPELL_EXHAUST_ID));
    assert!(!jayce_choices.contains(&SPELL_TELEPORT_ID));

    // Only Exhaust + Flash in ARAM Mayhem -> replaces Exhaust with Mark (32), secondary Heal (7).
    let exhaust_only = serde_json::json!({
        "summoner_spells": [
            { "ids": [3, 4], "play": 15000, "win": 7800 }
        ]
    });
    let (fallback_top, fallback_choices) = extract_summoner_spell_choices_for_mode(
        &exhaust_only,
        false,
        Some(ddragon_mode_for_queue(QUEUE_ARAM_MAYHEM)),
        Some(&catalog),
    );
    assert_eq!(fallback_top, (4, 32));
    assert_eq!(fallback_choices, vec![32, 7]);

    // Ignite + Teleport in Classic ARAM (450) and ARAM Mayhem (2400) -> Teleport (12) is never emitted.
    let ignite_and_tp = serde_json::json!({
        "summoner_spells": [
            { "ids": [4, 12], "play": 30000, "win": 15000 },
            { "ids": [4, 14], "play": 10000, "win": 5000 }
        ]
    });
    for queue_id in [QUEUE_ARAM, QUEUE_ARAM_MAYHEM] {
        let mode = ddragon_mode_for_queue(queue_id);
        let (top, choices) = extract_summoner_spell_choices_for_mode(
            &ignite_and_tp,
            false,
            Some(mode),
            Some(&catalog),
        );
        assert_eq!(top, (4, 14));
        assert_eq!(choices, vec![14, 32]);
        assert!(!choices.contains(&SPELL_TELEPORT_ID));
    }

    // Summoner's Rift (420 -> "CLASSIC"): Teleport (12) is allowed, Mark (32) is filtered out.
    let sr_with_mark = serde_json::json!({
        "summoner_spells": [
            { "ids": [4, 32], "play": 30000, "win": 15000 },
            { "ids": [4, 12], "play": 10000, "win": 5200 }
        ]
    });
    let (sr_top, sr_choices) = extract_summoner_spell_choices_for_mode(
        &sr_with_mark,
        false,
        Some(ddragon_mode_for_queue(QUEUE_SOLO_DUO)),
        Some(&catalog),
    );
    assert_eq!(sr_top, (4, 12));
    assert!(!sr_choices.contains(&SPELL_MARK_ID));
    assert_eq!(sr_choices, vec![12, 14]);
}
