use super::super::aram::normalize_opgg_champion_key;
use super::super::mcp_extractors::parse_json_item_entries;
use super::super::rest_extractors::normalize_opgg_tier;
use super::items::build_arena_item_blocks_and_options_with_cls;
use crate::build_provider::items::resolve_classifications_for_ddragon;
use crate::build_provider::models::{ChampionBuild, DataDragonCache};
use crate::models::{ArenaAugmentEntry, ArenaSynergyEntry};
use std::collections::HashMap;

fn round_1(val: f64) -> f64 {
    (val * 10.0).round() / 10.0
}

fn round_2(val: f64) -> f64 {
    (val * 100.0).round() / 100.0
}

fn normalize_pct(raw: f64) -> f64 {
    if !raw.is_finite() || raw <= 0.0 {
        return 0.0;
    }
    let pct = if raw <= 1.0 { raw * 100.0 } else { raw };
    round_1(pct)
}

fn parse_arena_augments(data: &serde_json::Value) -> Vec<ArenaAugmentEntry> {
    let Some(groups) = data.get("augment_group").and_then(|v| v.as_array()) else {
        return Vec::new();
    };

    let mut by_id: HashMap<i32, ArenaAugmentEntry> = HashMap::new();
    for group in groups {
        let group_rarity = group.get("rarity").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        let Some(augs) = group.get("augments").and_then(|v| v.as_array()) else {
            continue;
        };

        for aug in augs {
            let id = aug.get("id").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let play = aug.get("play").and_then(|v| v.as_i64()).unwrap_or(0);
            if id <= 0 || play <= 0 {
                continue;
            }
            let win = aug.get("win").and_then(|v| v.as_i64()).unwrap_or(0).max(0);
            let first_place = aug
                .get("first_place")
                .and_then(|v| v.as_i64())
                .unwrap_or(0)
                .max(0);
            let total_place = aug
                .get("total_place")
                .and_then(|v| v.as_i64())
                .unwrap_or(0)
                .max(0);

            let play_f = play as f64;
            let win_rate = aug
                .get("win_rate")
                .and_then(|v| v.as_f64())
                .filter(|v| v.is_finite() && *v > 0.0)
                .map(normalize_pct)
                .unwrap_or_else(|| round_1((win as f64 / play_f) * 100.0));
            let first_place_rate = round_1((first_place as f64 / play_f) * 100.0);
            let avg_placement = if total_place > 0 {
                round_2(total_place as f64 / play_f)
            } else {
                0.0
            };
            let pick_rate = aug
                .get("pick_rate")
                .and_then(|v| v.as_f64())
                .map(normalize_pct)
                .unwrap_or(0.0);

            let entry = ArenaAugmentEntry {
                id,
                name: String::new(),
                desc: String::new(),
                icon_url: String::new(),
                rarity: group_rarity,
                play,
                win,
                first_place,
                win_rate,
                first_place_rate,
                avg_placement,
                pick_rate,
            };

            let replace = by_id
                .get(&id)
                .map(|existing| entry.play > existing.play)
                .unwrap_or(true);
            if replace {
                by_id.insert(id, entry);
            }
        }
    }

    let mut result: Vec<ArenaAugmentEntry> = by_id.into_values().collect();
    result.sort_by(|a, b| {
        b.play
            .cmp(&a.play)
            .then_with(|| b.win_rate.total_cmp(&a.win_rate))
            .then_with(|| a.id.cmp(&b.id))
    });
    result
}

fn parse_arena_synergies(
    data: &serde_json::Value,
    ddragon: &DataDragonCache,
) -> Vec<ArenaSynergyEntry> {
    let Some(synergies) = data.get("synergies").and_then(|v| v.as_array()) else {
        return Vec::new();
    };

    let mut result = Vec::with_capacity(synergies.len());
    for syn in synergies {
        let champion_id = syn.get("champion_id").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        let play = syn.get("play").and_then(|v| v.as_i64()).unwrap_or(0);
        if champion_id <= 0 || play <= 0 {
            continue;
        }

        let win = syn.get("win").and_then(|v| v.as_i64()).unwrap_or(0).max(0);
        let first_place = syn
            .get("first_place")
            .and_then(|v| v.as_i64())
            .unwrap_or(0)
            .max(0);
        let total_place = syn
            .get("total_place")
            .and_then(|v| v.as_i64())
            .unwrap_or(0)
            .max(0);
        let op_rank = syn.get("op_rank").and_then(|v| v.as_i64()).unwrap_or(0) as i32;

        let play_f = play as f64;
        let win_rate = round_1((win as f64 / play_f) * 100.0);
        let first_place_rate = round_1((first_place as f64 / play_f) * 100.0);
        let avg_placement = if total_place > 0 {
            round_2(total_place as f64 / play_f)
        } else {
            0.0
        };
        let pick_rate = syn
            .get("pick_rate")
            .and_then(|v| v.as_f64())
            .map(normalize_pct)
            .unwrap_or(0.0);

        let dd_meta = ddragon.champions.get(&champion_id);
        let name = dd_meta
            .map(|m| m.display_name.clone())
            .unwrap_or_else(|| format!("Champion {}", champion_id));
        let key = dd_meta
            .map(|m| normalize_opgg_champion_key(&m.key_name))
            .unwrap_or_else(|| champion_id.to_string());
        let icon_url = dd_meta.map(|m| m.icon_url.clone()).unwrap_or_default();

        result.push(ArenaSynergyEntry {
            champion_id,
            key,
            name,
            icon_url,
            op_rank,
            play,
            win,
            first_place,
            win_rate,
            first_place_rate,
            avg_placement,
            pick_rate,
        });
    }

    result.sort_by(|a, b| {
        let a_valid = if a.op_rank > 0 { 0 } else { 1 };
        let b_valid = if b.op_rank > 0 { 0 } else { 1 };
        a_valid
            .cmp(&b_valid)
            .then_with(|| a.op_rank.cmp(&b.op_rank))
            .then_with(|| b.win_rate.total_cmp(&a.win_rate))
            .then_with(|| b.play.cmp(&a.play))
            .then_with(|| a.champion_id.cmp(&b.champion_id))
    });
    result
}

pub(crate) fn extract_arena_champion_play_count(json: &serde_json::Value) -> i64 {
    let data = json.get("data").unwrap_or(json);
    data.pointer("/summary/average_stats/play")
        .and_then(|v| v.as_i64())
        .unwrap_or(0)
}

/// Parses OP.GG's `GET /api/global/champions/arena/{champion_id}` payload into an Arena (`3x6`)
/// `ChampionBuild` with `supports_runes: false`, `supports_spells: false`, dedicated Map `30`
/// item blocks, augments, and ally synergies.
pub fn parse_arena_champion_payload(
    opgg_json: &serde_json::Value,
    champion_id: i32,
    champion_name: &str,
    ddragon: &DataDragonCache,
) -> Result<ChampionBuild, String> {
    let data = opgg_json.get("data").unwrap_or(opgg_json);

    let boot_entries = parse_json_item_entries(data.get("boots"));
    let prism_entries = parse_json_item_entries(data.get("prism_items"));
    let core_entries = parse_json_item_entries(data.get("core_items"));
    let last_entries = parse_json_item_entries(data.get("last_items"));

    if boot_entries.is_empty()
        && prism_entries.is_empty()
        && core_entries.is_empty()
        && last_entries.is_empty()
    {
        return Err("Missing Arena item sections in OP.GG payload".to_string());
    }

    let stats = data.pointer("/summary/average_stats");
    let play = stats
        .and_then(|s| s.get("play"))
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let win = stats
        .and_then(|s| s.get("win"))
        .and_then(|v| v.as_i64())
        .unwrap_or(0)
        .max(0);
    let first_place = stats
        .and_then(|s| s.get("first_place"))
        .and_then(|v| v.as_i64())
        .unwrap_or(0)
        .max(0);
    let total_place = stats
        .and_then(|s| s.get("total_place"))
        .and_then(|v| v.as_i64())
        .unwrap_or(0)
        .max(0);

    let (win_rate, first_place_rate, avg_placement) = if play > 0 {
        let play_f = play as f64;
        (
            round_1((win as f64 / play_f) * 100.0),
            Some(round_1((first_place as f64 / play_f) * 100.0)),
            (total_place > 0).then(|| round_2(total_place as f64 / play_f)),
        )
    } else {
        (0.0, None, None)
    };

    let pick_rate = stats
        .and_then(|s| s.get("pick_rate"))
        .and_then(|v| v.as_f64())
        .map(normalize_pct)
        .unwrap_or(0.0);

    let is_rip = data
        .pointer("/summary/is_rip")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let tier = if is_rip {
        None
    } else {
        stats
            .and_then(|s| s.pointer("/tier_data/tier").or_else(|| s.get("tier")))
            .and_then(|v| v.as_i64())
            .and_then(|raw| normalize_opgg_tier(raw as i32))
    };

    let cls = resolve_classifications_for_ddragon(Some(ddragon));
    let (item_blocks, build_options) = build_arena_item_blocks_and_options_with_cls(
        &cls,
        &boot_entries,
        &prism_entries,
        &core_entries,
        &last_entries,
    );
    let arena_augments = parse_arena_augments(data);
    let arena_synergies = parse_arena_synergies(data, ddragon);
    let skill_order = crate::build_provider::skills::extract_rest_skill_order(data);

    Ok(ChampionBuild {
        champion_id,
        champion_name: champion_name.to_string(),
        win_rate,
        pick_rate,
        tier,
        primary_style_id: 0,
        sub_style_id: 0,
        selected_perk_ids: Vec::new(),
        spell1_id: 0,
        spell2_id: 0,
        item_blocks,
        build_options,
        active_position: String::new(),
        active_tier: "arena".to_string(),
        active_region: "global".to_string(),
        supports_runes: false,
        supports_spells: false,
        arena_augments,
        arena_synergies,
        first_place_rate,
        avg_placement,
        skill_order,
        ..Default::default()
    })
}
