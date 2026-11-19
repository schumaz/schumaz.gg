use super::super::items::*;
use super::super::models::{ChampionBuild, ItemStatEntry, SpellMetadata};
use super::super::spells::extract_summoner_spell_choices_for_mode;
use super::mcp_extractors::parse_json_item_entries;
use super::mcp_parser::parse_opgg_mcp_text_for_mode;
use super::rest_extractors::{
    extract_rest_runes_and_pages, extract_rest_win_and_pick_rates, infer_rest_damage_type,
};
use std::collections::HashMap;

pub fn parse_opgg_analysis_payload(
    opgg_json: &serde_json::Value,
    champion_id: i32,
    champion_name: &str,
) -> Result<ChampionBuild, String> {
    parse_opgg_analysis_payload_with_options(opgg_json, champion_id, champion_name, true)
}

pub fn parse_opgg_analysis_payload_with_options(
    opgg_json: &serde_json::Value,
    champion_id: i32,
    champion_name: &str,
    supports_runes: bool,
) -> Result<ChampionBuild, String> {
    parse_opgg_analysis_payload_for_mode(
        opgg_json,
        champion_id,
        champion_name,
        supports_runes,
        None,
        None,
    )
}

/// Parses an OP.GG REST or wrapped MCP payload into a `ChampionBuild`, filtering summoner spells
/// against DataDragon's `modes` for `spell_mode` when provided.
pub fn parse_opgg_analysis_payload_for_mode(
    opgg_json: &serde_json::Value,
    champion_id: i32,
    champion_name: &str,
    supports_runes: bool,
    spell_mode: Option<&str>,
    spells_catalog: Option<&HashMap<i32, SpellMetadata>>,
) -> Result<ChampionBuild, String> {
    let json_supports_runes = opgg_json
        .get("supports_runes")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    // If the response is wrapped in MCP content text (e.g. result.content[0].text), use text parser!
    if let Some(text) = opgg_json
        .pointer("/result/content/0/text")
        .and_then(|v| v.as_str())
    {
        return parse_opgg_mcp_text_for_mode(
            text,
            champion_id,
            champion_name,
            supports_runes && json_supports_runes,
            spell_mode,
            spells_catalog,
        );
    }

    let analysis = opgg_json
        .get("data")
        .or_else(|| opgg_json.pointer("/result/data"))
        .unwrap_or(opgg_json);

    let effective_supports_runes = supports_runes
        && json_supports_runes
        && analysis
            .get("supports_runes")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

    let (win_rate, pick_rate) = extract_rest_win_and_pick_rates(analysis);
    let (primary_style_id, sub_style_id, selected_perk_ids, top_rune_pages) =
        extract_rest_runes_and_pages(analysis, effective_supports_runes)?;

    let position = analysis
        .get("position")
        .or_else(|| analysis.pointer("/meta/position"))
        .or_else(|| analysis.pointer("/summary/position"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let is_jungle = position.eq_ignore_ascii_case("jungle");

    // Extract summoner spells filtered by active mode and DataDragon spell catalog
    let ((spell1_id, spell2_id), spell_choice_ids) =
        extract_summoner_spell_choices_for_mode(analysis, is_jungle, spell_mode, spells_catalog);

    // Extract item entries from JSON
    let starter_val = analysis
        .pointer("/items/starters")
        .or_else(|| analysis.pointer("/starter_items"));
    let starter_entries = if let Some(arr) = starter_val.and_then(|v| v.as_array())
        && !arr.is_empty()
        && arr.iter().all(|x| x.is_i64())
    {
        vec![ItemStatEntry {
            ids: arr
                .iter()
                .filter_map(|x| x.as_i64().map(|n| n as i32))
                .collect(),
            play: 1,
            win: 0,
            pick_rate: 0.0,
        }]
    } else {
        parse_json_item_entries(starter_val)
    };

    let boot_entries = parse_json_item_entries(
        analysis
            .pointer("/items/boots")
            .or_else(|| analysis.pointer("/boots")),
    );

    let mut candidate_entries = Vec::new();
    let mut core_entries = parse_json_item_entries(
        analysis
            .pointer("/items/core")
            .or_else(|| analysis.pointer("/core_items")),
    );
    // Select the #1 most played Core Build combination to identify core_ids
    core_entries.sort_by_key(|e| std::cmp::Reverse(e.play));
    let mut core_ids = Vec::new();
    for (idx, entry) in core_entries.iter_mut().enumerate() {
        if entry.play == 1 {
            entry.play = 10000 - (idx as i64);
        }
        if idx == 0 {
            core_ids.extend_from_slice(&entry.ids);
        }
    }
    candidate_entries.extend(core_entries.clone());

    let damage_type_owned = infer_rest_damage_type(analysis, &core_entries);
    let damage_type = damage_type_owned.as_str();

    let mut fourth_entries = parse_json_item_entries(
        analysis
            .pointer("/fourth_items")
            .or_else(|| analysis.pointer("/items/fourth_items"))
            .or_else(|| analysis.pointer("/items/final_build")),
    );
    let mut fifth_entries = parse_json_item_entries(
        analysis
            .pointer("/fifth_items")
            .or_else(|| analysis.pointer("/items/fifth_items")),
    );
    let mut sixth_entries = parse_json_item_entries(
        analysis
            .pointer("/sixth_items")
            .or_else(|| analysis.pointer("/items/sixth_items")),
    );

    let other_entries = parse_json_item_entries(
        analysis
            .pointer("/items/situational")
            .or_else(|| analysis.pointer("/last_items")),
    );

    for entry in other_entries {
        let filtered_ids: Vec<i32> = entry
            .ids
            .iter()
            .filter(|id| !core_ids.contains(id))
            .copied()
            .collect();
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
    let tear_upgrade = resolve_tear_to_completed_item(&all_for_tear);

    let normalize_entries = |entries: &mut Vec<ItemStatEntry>| {
        for entry in entries.iter_mut() {
            entry.ids = resolve_tear_item_ids(&entry.ids, tear_upgrade);
        }
        entries.retain(|e| !e.ids.is_empty());
    };
    normalize_entries(&mut core_entries);
    normalize_entries(&mut candidate_entries);
    normalize_entries(&mut fourth_entries);
    normalize_entries(&mut fifth_entries);
    normalize_entries(&mut sixth_entries);

    let item_blocks = build_5_item_blocks_with_depth_lists(
        starter_entries,
        boot_entries,
        candidate_entries.clone(),
        fourth_entries.clone(),
        fifth_entries.clone(),
        sixth_entries.clone(),
        damage_type,
        position,
    );
    let general_situational = item_blocks
        .get(4)
        .map(|b| b.item_ids.clone())
        .unwrap_or_default();

    let build_options = build_unified_build_options(
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

    let spell_choices = spell_choice_ids
        .into_iter()
        .map(|id| SpellMetadata {
            id,
            ..Default::default()
        })
        .collect();

    Ok(ChampionBuild {
        champion_id,
        champion_name: champion_name.to_string(),
        win_rate,
        pick_rate,
        primary_style_id,
        sub_style_id,
        selected_perk_ids,
        spell1_id,
        spell2_id,
        item_blocks,
        spell_choices,
        rune_pages: top_rune_pages,
        build_options,
        supports_runes: effective_supports_runes,
        ..Default::default()
    })
}

#[cfg(test)]
#[path = "rest_parser_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "rest_parser_aram_tests.rs"]
mod aram_tests;
