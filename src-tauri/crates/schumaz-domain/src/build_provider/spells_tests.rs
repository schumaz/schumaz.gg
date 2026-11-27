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

fn pair_ids(options: &[SpellPairOption]) -> Vec<(i32, i32)> {
    options
        .iter()
        .map(|o| (o.spell1_id, o.spell2_id))
        .collect()
}

#[test]
fn test_extract_ahri_mid_real_opgg_json_pairs_and_stats() {
    let ahri_json = serde_json::json!({
        "data": {
            "position": "mid",
            "win_rate": 51.4,
            "pick_rate": 11.2,
            "runes": {
                "primary_style_id": 8100,
                "sub_style_id": 8200,
                "primary_perk_ids": [8112, 8126, 8138, 8106],
                "sub_perk_ids": [8226, 8210],
                "stat_shards": [5008, 5008, 5001]
            },
            "summoner_spells": [
                { "ids": [4, 14], "win": 43679, "play": 84666, "pick_rate": 0.6954 },
                { "ids": [4, 12], "win": 17612, "play": 34826, "pick_rate": 0.2861 },
                { "ids": [4, 21], "win": 310, "play": 620, "pick_rate": 0.0051 }
            ],
            "items": {
                "starters": [1056, 2003],
                "core": [3118, 4645, 3157]
            }
        }
    });

    let build = parse_opgg_analysis_payload(&ahri_json, 103, "Ahri")
        .expect("Parsing Ahri with real OP.GG summoner_spells pairs should succeed");

    assert_eq!((build.spell1_id, build.spell2_id), (4, 14));
    assert_eq!(pair_ids(&build.spell_options), vec![(4, 14), (4, 12)]);
    assert_eq!(build.spell_options[0].play, 84666);
    assert_eq!(build.spell_options[0].win, 43679);
    assert_eq!(build.spell_options[0].pick_rate, 69.5);
    assert_eq!(build.spell_options[1].play, 34826);
    assert_eq!(build.spell_options[1].win, 17612);
    assert_eq!(build.spell_options[1].pick_rate, 28.6);
}

#[test]
fn test_extract_spell_pairs_preserves_distinct_primary_spells_across_options() {
    // Gwen / Camille: Teleport+Ignite (12, 14) vs Flash+Teleport (4, 12)
    let gwen_json = serde_json::json!({
        "summoner_spells": [
            { "ids": [12, 14], "play": 42000, "win": 21800, "pick_rate": 0.582 },
            { "ids": [12, 4], "play": 28000, "win": 14100, "pick_rate": 0.388 }
        ]
    });
    let (gwen_top, gwen_opts) = extract_summoner_spell_pairs(&gwen_json, false);
    assert_eq!(gwen_top, (12, 14));
    assert_eq!(pair_ids(&gwen_opts), vec![(12, 14), (4, 12)]);

    // Darius: Flash+Ghost (4, 6) vs Ghost+Teleport (6, 12) - non-Flash 2nd pair preserved intact!
    let darius_json = serde_json::json!({
        "summoner_spells": [
            { "ids": [4, 6], "play": 51000, "win": 26500, "pick_rate": 0.64 },
            { "ids": [6, 12], "play": 18500, "win": 9400, "pick_rate": 0.23 }
        ]
    });
    let (darius_top, darius_opts) = extract_summoner_spell_pairs(&darius_json, false);
    assert_eq!(darius_top, (4, 6));
    assert_eq!(pair_ids(&darius_opts), vec![(4, 6), (6, 12)]);
}

#[test]
fn test_extract_summoner_spell_pairs_jungle_and_non_smite_filtering() {
    let hecarim_json = serde_json::json!({
        "summoner_spells": [
            { "ids": [4, 14], "play": 99999, "win": 50000 },
            { "ids": [6, 11], "play": 45000, "win": 23000, "pick_rate": 0.72 },
            { "ids": [4, 11], "play": 12000, "win": 6000, "pick_rate": 0.19 }
        ]
    });
    let (jng_top, jng_opts) = extract_summoner_spell_pairs(&hecarim_json, true);
    assert_eq!(jng_top, (6, 11));
    assert_eq!(pair_ids(&jng_opts), vec![(6, 11), (4, 11)]);
    assert_eq!(jng_opts[0].pick_rate, 72.0);

    // Shaco numerically-ordered [11, 14] normalizes to (14, 11)
    let shaco_json = serde_json::json!({
        "summoner_spells": [
            { "ids": [11, 14], "play": 38000, "win": 20000 },
            { "ids": [11, 4], "play": 5000, "win": 2500 }
        ]
    });
    let (shaco_top, shaco_opts) = extract_summoner_spell_pairs(&shaco_json, true);
    assert_eq!(shaco_top, (14, 11));
    assert_eq!(pair_ids(&shaco_opts), vec![(14, 11), (4, 11)]);
}

#[test]
fn test_extract_spell_pairs_deduplicates_unordered_pairs_and_synthesizes_fallback() {
    let dup_json = serde_json::json!({
        "summoner_spells": [
            { "ids": [4, 14], "play": 10000, "win": 5200, "pick_rate": 0.50 },
            { "ids": [14, 4], "play": 2000, "win": 1050, "pick_rate": 0.10 }
        ]
    });
    let (top, opts) = extract_summoner_spell_pairs(&dup_json, false);
    assert_eq!(top, (4, 14));
    assert_eq!(opts.len(), 2);
    assert_eq!(opts[0].spell1_id, 4);
    assert_eq!(opts[0].spell2_id, 14);
    assert_eq!(opts[0].play, 12000);
    assert_eq!(opts[0].win, 6250);
    assert_eq!(opts[0].pick_rate, 60.0);
    assert_eq!((opts[1].spell1_id, opts[1].spell2_id), (4, 12));
    assert_eq!(opts[1].play, 0);
    assert_eq!(opts[1].pick_rate, 0.0);

    // Numeric OP.GG ordering [1, 4] and [3, 4] normalizes Flash first
    let cleanse_flash = serde_json::json!({
        "summoner_spells": [
            { "ids": [1, 4], "play": 28000, "win": 14500 },
            { "ids": [3, 4], "play": 12000, "win": 6100 }
        ]
    });
    let (cleanse_top, cleanse_opts) = extract_summoner_spell_pairs(&cleanse_flash, false);
    assert_eq!(cleanse_top, (4, 1));
    assert_eq!(pair_ids(&cleanse_opts), vec![(4, 1), (4, 3)]);
}

#[test]
fn test_mode_filtering_for_spell_pairs_across_mayhem_aram_and_sr() {
    let catalog = sample_ddragon_spells_catalog();
    let lulu_aram = serde_json::json!({
        "summoner_spells": [
            { "ids": [3, 4], "play": 22000, "win": 11500, "pick_rate": 0.52 },
            { "ids": [4, 7], "play": 12000, "win": 6200, "pick_rate": 0.29 },
            { "ids": [4, 21], "play": 7000, "win": 3600, "pick_rate": 0.17 }
        ]
    });

    let mayhem_mode = ddragon_mode_for_queue(QUEUE_ARAM_MAYHEM);
    let (mayhem_top, mayhem_opts) = extract_summoner_spell_pairs_for_mode(
        &lulu_aram,
        false,
        Some(mayhem_mode),
        Some(&catalog),
    );
    assert_eq!(mayhem_top, (4, 7));
    assert_eq!(pair_ids(&mayhem_opts), vec![(4, 7), (4, 21)]);

    let aram_mode = ddragon_mode_for_queue(QUEUE_ARAM);
    let (aram_top, aram_opts) =
        extract_summoner_spell_pairs_for_mode(&lulu_aram, false, Some(aram_mode), Some(&catalog));
    assert_eq!(aram_top, (4, 3));
    assert_eq!(pair_ids(&aram_opts), vec![(4, 3), (4, 7)]);

    // Only Exhaust + Flash in ARAM Mayhem -> falls back to (4, 32) and (4, 7)
    let exhaust_only = serde_json::json!({
        "summoner_spells": [{ "ids": [3, 4], "play": 15000, "win": 7800 }]
    });
    let (fb_top, fb_opts) = extract_summoner_spell_pairs_for_mode(
        &exhaust_only,
        false,
        Some(mayhem_mode),
        Some(&catalog),
    );
    assert_eq!(fb_top, (4, 32));
    assert_eq!(pair_ids(&fb_opts), vec![(4, 32), (4, 7)]);

    // Summoner's Rift filters Mark (32) and keeps Teleport (12)
    let sr_with_mark = serde_json::json!({
        "summoner_spells": [
            { "ids": [4, 32], "play": 30000, "win": 15000 },
            { "ids": [4, 12], "play": 10000, "win": 5200 }
        ]
    });
    let (sr_top, sr_opts) = extract_summoner_spell_pairs_for_mode(
        &sr_with_mark,
        false,
        Some(ddragon_mode_for_queue(QUEUE_SOLO_DUO)),
        Some(&catalog),
    );
    assert_eq!(sr_top, (4, 12));
    assert_eq!(pair_ids(&sr_opts), vec![(4, 12), (4, 14)]);
}
