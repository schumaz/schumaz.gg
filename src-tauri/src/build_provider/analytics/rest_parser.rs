use super::mcp_parser::parse_opgg_mcp_text;
use super::super::spells::extract_summoner_spell_choices;
use super::mcp_extractors::parse_json_item_entries;
use super::super::items::*;
use super::super::models::{ChampionBuild, ItemStatEntry, RunePageOption, SpellMetadata};
use super::super::runes::extract_top_rune_pages;

pub fn parse_opgg_analysis_payload(
    opgg_json: &serde_json::Value,
    champion_id: i32,
    champion_name: &str,
) -> Result<ChampionBuild, String> {
    // If the response is wrapped in MCP content text (e.g. result.content[0].text), use text parser!
    if let Some(text) = opgg_json
        .pointer("/result/content/0/text")
        .and_then(|v| v.as_str())
    {
        return parse_opgg_mcp_text(text, champion_id, champion_name);
    }

    let analysis = opgg_json
        .get("data")
        .or_else(|| opgg_json.pointer("/result/data"))
        .unwrap_or(opgg_json);

    let raw_win_rate = analysis
        .get("win_rate")
        .or_else(|| analysis.pointer("/stats/win_rate"))
        .or_else(|| analysis.pointer("/summary/average_stats/win_rate"))
        .or_else(|| analysis.pointer("/average_stats/win_rate"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    let win_rate = if raw_win_rate > 0.0 && raw_win_rate <= 1.0 {
        (raw_win_rate * 1000.0).round() / 10.0
    } else {
        raw_win_rate
    };

    let raw_pick_rate = analysis
        .get("pick_rate")
        .or_else(|| analysis.pointer("/stats/pick_rate"))
        .or_else(|| analysis.pointer("/summary/average_stats/pick_rate"))
        .or_else(|| analysis.pointer("/average_stats/pick_rate"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    let pick_rate = if raw_pick_rate > 0.0 && raw_pick_rate <= 1.0 {
        (raw_pick_rate * 1000.0).round() / 10.0
    } else {
        raw_pick_rate
    };

    // Extract rune styles and perks
    let mut primary_style_id = analysis
        .pointer("/runes/primary_style_id")
        .or_else(|| analysis.pointer("/runes/primary_page_id"))
        .or_else(|| analysis.pointer("/runes/0/primary_page_id"))
        .and_then(|v| v.as_i64())
        .ok_or_else(|| "Missing primary_style_id in OP.GG runes".to_string())?
        as i32;

    let mut sub_style_id = analysis
        .pointer("/runes/sub_style_id")
        .or_else(|| analysis.pointer("/runes/secondary_page_id"))
        .or_else(|| analysis.pointer("/runes/0/secondary_page_id"))
        .and_then(|v| v.as_i64())
        .ok_or_else(|| "Missing sub_style_id in OP.GG runes".to_string())?
        as i32;

    let mut selected_perk_ids = Vec::with_capacity(9);

    if let Some(primary_perks) = analysis
        .pointer("/runes/primary_perk_ids")
        .or_else(|| analysis.pointer("/runes/0/primary_rune_ids"))
        .and_then(|v| v.as_array())
    {
        for perk in primary_perks {
            if let Some(id) = perk.as_i64() {
                selected_perk_ids.push(id as i32);
            }
        }
    }

    if let Some(sub_perks) = analysis
        .pointer("/runes/sub_perk_ids")
        .or_else(|| analysis.pointer("/runes/0/secondary_rune_ids"))
        .and_then(|v| v.as_array())
    {
        for perk in sub_perks {
            if let Some(id) = perk.as_i64() {
                selected_perk_ids.push(id as i32);
            }
        }
    }

    if let Some(stat_shards) = analysis
        .pointer("/runes/stat_shards")
        .or_else(|| analysis.pointer("/runes/0/stat_mod_ids"))
        .and_then(|v| v.as_array())
    {
        for shard in stat_shards {
            if let Some(id) = shard.as_i64() {
                selected_perk_ids.push(id as i32);
            }
        }
    }

    if selected_perk_ids.len() != 9 {
        return Err(format!(
            "Invalid perk count in OP.GG payload: expected 9 perks, found {}",
            selected_perk_ids.len()
        ));
    }

    let is_jungle = analysis
        .get("position")
        .or_else(|| analysis.pointer("/meta/position"))
        .or_else(|| analysis.pointer("/summary/position"))
        .and_then(|v| v.as_str())
        .map(|p| p.eq_ignore_ascii_case("jungle"))
        .unwrap_or(false);

    // Extract summoner spells: selects the most played combination and top 2 choices for secondary spell slot
    let ((spell1_id, spell2_id), spell_choice_ids) =
        extract_summoner_spell_choices(analysis, is_jungle);

    let damage_type = analysis
        .get("damage_type")
        .and_then(|v| v.as_str())
        .unwrap_or("AP");

    // Extract item entries from JSON
    let starter_entries = parse_json_item_entries(
        analysis
            .pointer("/items/starters")
            .or_else(|| analysis.pointer("/starter_items")),
    );

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

    let mut other_entries = Vec::new();
    other_entries.extend(parse_json_item_entries(
        analysis
            .pointer("/items/situational")
            .or_else(|| analysis.pointer("/last_items")),
    ));
    other_entries.extend(fourth_entries.clone());
    other_entries.extend(fifth_entries.clone());
    other_entries.extend(sixth_entries.clone());

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

    let position = analysis
        .get("position")
        .or_else(|| analysis.pointer("/meta/position"))
        .or_else(|| analysis.pointer("/summary/position"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let all_for_tear: Vec<ItemStatEntry> = core_entries
        .iter()
        .chain(candidate_entries.iter())
        .chain(fourth_entries.iter())
        .chain(fifth_entries.iter())
        .chain(sixth_entries.iter())
        .cloned()
        .collect();
    let tear_upgrade = resolve_tear_to_completed_item(&all_for_tear, position, damage_type);

    let normalize_entries = |entries: &mut [ItemStatEntry]| {
        for entry in entries.iter_mut() {
            entry.ids = resolve_tear_item_ids(&entry.ids, tear_upgrade);
        }
    };
    normalize_entries(&mut core_entries);
    normalize_entries(&mut candidate_entries);
    normalize_entries(&mut fourth_entries);
    normalize_entries(&mut fifth_entries);
    normalize_entries(&mut sixth_entries);

    let cand_ids: Vec<i32> = candidate_entries
        .iter()
        .flat_map(|e| e.ids.clone())
        .collect();
    let core_build_options = extract_top_core_build_options(&core_entries, &cand_ids, damage_type);
    let primary_core = if let Some(first_core) = core_build_options.first() {
        first_core.item_ids.clone()
    } else {
        vec![]
    };
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
    let general_situational = item_blocks.get(4).map(|b| b.item_ids.clone()).unwrap_or_default();

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
    let final_build_options = build_3_final_build_options(
        &primary_core,
        &fourth_entries,
        &fifth_entries,
        &sixth_entries,
        &cand_ids,
        damage_type,
    );

    let spell_choices = spell_choice_ids
        .into_iter()
        .map(|id| SpellMetadata {
            id,
            name: String::new(),
            icon_url: String::new(),
        })
        .collect();

    let mut top_rune_pages = extract_top_rune_pages(analysis);
    if top_rune_pages.is_empty() {
        top_rune_pages.push(RunePageOption {
            primary_style_id,
            sub_style_id,
            selected_perk_ids: selected_perk_ids.clone(),
            play: 1,
            win: 1,
            pick_rate: 1.0,
            ..Default::default()
        });
    } else {
        let first = &top_rune_pages[0];
        primary_style_id = first.primary_style_id;
        sub_style_id = first.sub_style_id;
        selected_perk_ids = first.selected_perk_ids.clone();
    }

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
        core_build_options,
        final_build_options,
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_core_builds_and_separate_fourth_fifth_sixth_lists_yunara() {
        // Mirrors the exact OP.GG Yunara tables from the user's screenshot:
        // Core builds:
        //   1. [6672, 3085, 3031] -> 19,057 Games
        //   2. [3032, 3085, 3031] -> 15,006 Games
        //   3. [6672, 3085, 2523] -> 5,163 Games
        // Fourth Item:
        //   1. [3036] (LDR) -> 18,151 Games
        //   2. [3031] (IE) -> 8,958 Games
        //   3. [3033] (Mortal Reminder) -> 5,651 Games
        // Fifth Item:
        //   1. [3026] (Guardian Angel) -> 4,832 Games
        //   2. [3072] (Bloodthirster) -> 2,585 Games
        //   3. [6673] (Shieldbow) -> 2,549 Games
        // Sixth Item:
        //   1. [3026] (Guardian Angel) -> 2,035 Games (already picked as 5th item!)
        //   2. [3072] (Bloodthirster) -> 1,036 Games (must be picked as 6th item!)
        //   3. [3139] (Mercurial Scimitar) -> 638 Games
        let json = serde_json::json!({
            "data": {
                "damage_type": "AD",
                "win_rate": 50.0,
                "pick_rate": 11.9,
                "runes": {
                    "primary_style_id": 8000,
                    "sub_style_id": 8300,
                    "primary_perk_ids": [8008, 9111, 9104, 8017],
                    "sub_perk_ids": [8304, 8345],
                    "stat_shards": [5005, 5008, 5011]
                },
                "summoner_spells": [
                    { "ids": [4, 21], "play": 62087, "win": 30822 }
                ],
                "starter_items": [
                    { "ids": [1055, 2003], "play": 76942, "win": 38123 }
                ],
                "boots": [
                    { "ids": [3006], "play": 42413, "win": 22681 }
                ],
                "core_items": [
                    { "ids": [3032, 3085, 3031], "play": 15006, "win": 8883 },
                    { "ids": [6672, 3085, 3031], "play": 19057, "win": 10673 },
                    { "ids": [6672, 3085, 2523], "play": 5163, "win": 2880 }
                ],
                "fourth_items": [
                    { "ids": [3036], "play": 18151, "win": 11137 },
                    { "ids": [3031], "play": 8958, "win": 5236 },
                    { "ids": [3033], "play": 5651, "win": 3192 }
                ],
                "fifth_items": [
                    { "ids": [3026], "play": 4832, "win": 3108 },
                    { "ids": [3072], "play": 2585, "win": 1612 },
                    { "ids": [6673], "play": 2549, "win": 1570 }
                ],
                "sixth_items": [
                    { "ids": [3026], "play": 2035, "win": 1333 },
                    { "ids": [3072], "play": 1036, "win": 630 },
                    { "ids": [3139], "play": 638, "win": 401 }
                ],
                "last_items": [
                    { "ids": [3085], "play": 75928, "win": 39359 },
                    { "ids": [3031], "play": 59024, "win": 33741 },
                    { "ids": [6672], "play": 50399, "win": 24649 },
                    { "ids": [3032], "play": 34739, "win": 17553 },
                    { "ids": [3036], "play": 30559, "win": 17570 }
                ]
            }
        });

        let build = parse_opgg_analysis_payload(&json, 804, "Yunara").expect(
            "Parsing Yunara build with core_items and 4th/5th/6th item lists should succeed",
        );

        // Block 2 (Primeiros 3 Itens): must pick [6672, 3085, 3031] because 19,057 > 15,006
        assert_eq!(build.item_blocks[1].item_ids, vec![6672, 3085, 3031]);

        // Block 4 (Build Final): 3 Core ([6672, 3085, 3031]) + 4th (3036) + 5th (3026) + 6th (3072 since 3026 is already in 5th)
        assert_eq!(
            build.item_blocks[3].item_ids,
            vec![6672, 3085, 3031, 3036, 3026, 3072]
        );
    }

    #[test]
    fn test_opgg_payload_normalizes_tear_items_in_blocks_and_options() {
        let analysis_json = serde_json::json!({
            "summary": { "position": "adc" },
            "core_items": [
                { "ids": [3078, 3042, 3036], "play": 15000, "win": 8000, "pick_rate": 45.0 }
            ],
            "boots": [
                { "ids": [3158], "play": 20000, "win": 11000, "pick_rate": 60.0 }
            ],
            "starters": [
                { "ids": [1055, 2003], "play": 25000, "win": 13000, "pick_rate": 80.0 }
            ],
            "fourth_items": [
                { "ids": [3072], "play": 8000, "win": 4500, "pick_rate": 30.0 }
            ],
            "fifth_items": [
                { "ids": [3026], "play": 6000, "win": 3200, "pick_rate": 25.0 }
            ],
            "sixth_items": [
                { "ids": [3139], "play": 4000, "win": 2100, "pick_rate": 20.0 }
            ],
            "last_items": [
                { "ids": [3040], "play": 3000, "win": 1600, "pick_rate": 15.0 }
            ],
            "runes": [
                {
                    "primary_page_id": 8000,
                    "primary_rune_ids": [8005, 9111, 9104, 8014],
                    "secondary_page_id": 8200,
                    "secondary_rune_ids": [8210, 8226],
                    "stat_mod_ids": [5005, 5008, 5001],
                    "play": 20000,
                    "win": 10500
                }
            ],
            "spells": [
                { "ids": [4, 7], "play": 20000, "win": 10500, "pick_rate": 80.0 }
            ]
        });

        let build = parse_opgg_analysis_payload(&analysis_json, 81, "Ezreal")
            .expect("Build parsing should succeed");

        // Core block must contain 3004 (Manamune), NOT 3042 (Muramana)
        assert_eq!(build.item_blocks[1].item_ids, vec![3078, 3004, 3036]);
        assert!(!build.item_blocks[1].item_ids.contains(&3042));

        // Final build must contain 3004, NOT 3042
        assert!(build.item_blocks[3].item_ids.contains(&3004));
        assert!(!build.item_blocks[3].item_ids.contains(&3042));

        // Core options must have 3004, NOT 3042
        assert_eq!(build.core_build_options[0].item_ids, vec![3078, 3004, 3036]);

        // Situational items with 3040 must normalize to 3003
        assert!(!build.item_blocks[4].item_ids.contains(&3040));
    }

    #[test]
    fn test_parse_opgg_analysis_extracts_9_runes_spells_and_items() {
        let opgg_fixture = serde_json::json!({
            "data": {
                "win_rate": 51.85,
                "pick_rate": 8.42,
                "runes": {
                    "primary_style_id": 8100,
                    "sub_style_id": 8200,
                    "primary_perk_ids": [8112, 8126, 8138, 8135],
                    "sub_perk_ids": [8226, 8210],
                    "stat_shards": [5008, 5008, 5001]
                },
                "spells": [4, 14],
                "items": {
                    "starters": [1056, 2003],
                    "core": [3285, 3020, 3089],
                    "situational": [3157, 3135],
                    "boots": [3020, 3111, 3047]
                }
            }
        });

        let result = parse_opgg_analysis_payload(&opgg_fixture, 103, "Ahri");
        assert!(result.is_ok());

        let build = result.unwrap();
        assert_eq!(build.champion_id, 103);
        assert_eq!(build.champion_name, "Ahri");
        assert_eq!(build.primary_style_id, 8100);
        assert_eq!(build.sub_style_id, 8200);
        assert_eq!(build.selected_perk_ids.len(), 9);
        assert_eq!(
            build.selected_perk_ids,
            vec![8112, 8126, 8138, 8135, 8226, 8210, 5008, 5008, 5001]
        );
        assert_eq!(build.spell1_id, 4);
        assert_eq!(build.spell2_id, 14);
        assert_eq!(build.item_blocks.len(), 5);
        assert_eq!(build.item_blocks[0].title, "Itens Iniciais");
        assert_eq!(build.item_blocks[1].title, "Primeiros 3 Itens");
        // 3020 is a boot, so it was filtered out and filled with situational item 3157, ensuring 3 core non-boot items!
        assert_eq!(build.item_blocks[1].item_ids, vec![3285, 3089, 3157]);
        assert_eq!(build.item_blocks[2].title, "3 Botas Mais Feitas");
        assert_eq!(build.item_blocks[2].item_ids, vec![3020, 3111, 3047]);
        assert_eq!(build.item_blocks[3].title, "Build Final");
        assert_eq!(build.item_blocks[4].title, "Itens Situacionais / Úteis");
        // Verify situational items are disjoint from core and final build!
        for &item in &build.item_blocks[4].item_ids {
            assert!(!build.item_blocks[1].item_ids.contains(&item));
            assert!(!build.item_blocks[3].item_ids.contains(&item));
            assert!(!is_boot_item(item));
        }
    }
}
