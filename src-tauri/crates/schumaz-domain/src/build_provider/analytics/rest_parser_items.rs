use super::super::items::{
    build_5_item_blocks_with_cls, build_unified_build_options_with_cls,
    resolve_classifications_for_ddragon, resolve_tear_item_ids_in,
    resolve_tear_to_completed_item_in,
};
use super::super::models::{
    DataDragonCache, ItemBlock, ItemClassificationCache, ItemStatEntry, SpellMetadata,
    UnifiedBuildOption,
};
use super::mcp_extractors::parse_json_item_entries;
use super::rest_extractors::{extract_primary_champion_role, infer_damage_type_with_cls};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Default)]
pub struct McpDepthItems {
    pub fourth: Vec<ItemStatEntry>,
    pub fifth: Vec<ItemStatEntry>,
    pub sixth: Vec<ItemStatEntry>,
}

pub struct RestAnalysisParseOptions<'a> {
    pub champion_id: i32,
    pub champion_name: &'a str,
    pub supports_runes: bool,
    pub spell_mode: Option<&'a str>,
    pub ddragon: Option<&'a DataDragonCache>,
    pub spells_catalog: Option<&'a HashMap<i32, SpellMetadata>>,
    pub allowed_lane_champions: Option<&'a HashSet<i32>>,
    pub position_override: Option<&'a str>,
    pub mcp_depth_override: Option<McpDepthItems>,
}

fn json_has_explicit_play_stats(val: Option<&serde_json::Value>) -> bool {
    match val {
        Some(serde_json::Value::Array(arr)) => arr.iter().any(|elem| {
            elem.as_object()
                .and_then(|obj| obj.get("play"))
                .and_then(|p| p.as_i64())
                .is_some()
        }),
        Some(serde_json::Value::Object(obj)) => {
            obj.get("play").and_then(|p| p.as_i64()).is_some()
        }
        _ => false,
    }
}

fn extract_starter_entries(starter_val: Option<&serde_json::Value>) -> Vec<ItemStatEntry> {
    if let Some(arr) = starter_val.and_then(|v| v.as_array())
        && !arr.is_empty()
        && arr.iter().all(|x| x.is_i64())
    {
        vec![ItemStatEntry {
            ids: arr.iter().filter_map(|x| x.as_i64().map(|n| n as i32)).collect(),
            play: 1,
            win: 0,
            pick_rate: 0.0,
        }]
    } else {
        parse_json_item_entries(starter_val)
    }
}

fn normalize_tear_entries(
    cls: &ItemClassificationCache,
    entries: &mut Vec<ItemStatEntry>,
    tear_upgrade: Option<i32>,
) {
    for entry in entries.iter_mut() {
        entry.ids = resolve_tear_item_ids_in(cls, &entry.ids, tear_upgrade);
    }
    entries.retain(|e| !e.ids.is_empty());
}

pub(super) fn assemble_analysis_items(
    analysis: &serde_json::Value,
    champion_id: i32,
    position: &str,
    ddragon: Option<&DataDragonCache>,
    mcp_depth_override: Option<McpDepthItems>,
) -> Result<(Vec<ItemBlock>, Vec<UnifiedBuildOption>), String> {
    let starter_entries = extract_starter_entries(
        analysis.pointer("/items/starters").or_else(|| analysis.pointer("/starter_items")),
    );
    let boot_entries = parse_json_item_entries(
        analysis.pointer("/items/boots").or_else(|| analysis.pointer("/boots")),
    );
    let core_val = analysis.pointer("/items/core").or_else(|| analysis.pointer("/core_items"));
    let mut core_entries = parse_json_item_entries(core_val);
    let other_entries = parse_json_item_entries(
        analysis.pointer("/items/situational").or_else(|| analysis.pointer("/last_items")),
    );

    if core_entries.is_empty() && other_entries.is_empty() {
        return Err("Missing core_items and last_items in OP.GG payload".to_string());
    }

    if !json_has_explicit_play_stats(core_val) && !core_entries.iter().any(|e| e.play > 1) {
        for (idx, entry) in core_entries.iter_mut().enumerate() {
            entry.play = 10000 - (idx as i64);
        }
    } else {
        core_entries.sort_by(|a, b| b.play.cmp(&a.play).then_with(|| b.win.cmp(&a.win)));
        if core_entries.iter().any(|e| e.play >= 2) {
            core_entries.retain(|e| e.play >= 2);
        }
    }

    let core_ids = core_entries.first().map(|e| e.ids.clone()).unwrap_or_default();
    let mut candidate_entries = core_entries.clone();
    let cls = resolve_classifications_for_ddragon(ddragon);
    let damage_type_owned = infer_damage_type_with_cls(
        analysis.get("damage_type").and_then(|v| v.as_str()),
        &core_entries,
        &other_entries,
        extract_primary_champion_role(analysis),
        champion_id,
        ddragon,
        &cls,
    );
    let damage_type = damage_type_owned.as_str();

    let depth = mcp_depth_override.unwrap_or_default();
    let mut fourth_entries = if !depth.fourth.is_empty() {
        depth.fourth
    } else {
        parse_json_item_entries(
            analysis
                .pointer("/fourth_items")
                .or_else(|| analysis.pointer("/items/fourth_items"))
                .or_else(|| analysis.pointer("/items/final_build")),
        )
    };
    let mut fifth_entries = if !depth.fifth.is_empty() {
        depth.fifth
    } else {
        parse_json_item_entries(analysis.pointer("/fifth_items").or_else(|| analysis.pointer("/items/fifth_items")))
    };
    let mut sixth_entries = if !depth.sixth.is_empty() {
        depth.sixth
    } else {
        parse_json_item_entries(analysis.pointer("/sixth_items").or_else(|| analysis.pointer("/items/sixth_items")))
    };

    for entry in other_entries {
        let filtered_ids: Vec<i32> = entry.ids.iter().filter(|id| !core_ids.contains(id)).copied().collect();
        if !filtered_ids.is_empty() {
            candidate_entries.push(ItemStatEntry {
                ids: filtered_ids,
                play: entry.play,
                win: entry.win,
                pick_rate: entry.pick_rate,
            });
        }
    }

    let all_for_tear: Vec<ItemStatEntry> = candidate_entries
        .iter()
        .chain(fourth_entries.iter())
        .chain(fifth_entries.iter())
        .chain(sixth_entries.iter())
        .cloned()
        .collect();
    let tear_upgrade = resolve_tear_to_completed_item_in(&cls, &all_for_tear);
    normalize_tear_entries(&cls, &mut core_entries, tear_upgrade);
    normalize_tear_entries(&cls, &mut candidate_entries, tear_upgrade);
    normalize_tear_entries(&cls, &mut fourth_entries, tear_upgrade);
    normalize_tear_entries(&cls, &mut fifth_entries, tear_upgrade);
    normalize_tear_entries(&cls, &mut sixth_entries, tear_upgrade);

    let item_blocks = build_5_item_blocks_with_cls(
        &cls,
        starter_entries,
        boot_entries,
        candidate_entries.clone(),
        fourth_entries.clone(),
        fifth_entries.clone(),
        sixth_entries.clone(),
        damage_type,
        position,
    );
    let general_situational = item_blocks.get(4).map(|b| b.item_ids.clone()).unwrap_or_default();
    let build_options = build_unified_build_options_with_cls(
        &cls,
        &core_entries,
        &fourth_entries,
        &fifth_entries,
        &sixth_entries,
        &candidate_entries,
        &[],
        &general_situational,
        damage_type,
        position,
    );
    Ok((item_blocks, build_options))
}
