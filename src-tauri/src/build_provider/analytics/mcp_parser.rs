use super::super::items::*;
use super::super::models::{ChampionBuild, ItemStatEntry, RunePageOption, SpellMetadata};
use super::super::runes::extract_all_mcp_runes;

pub fn extract_all_core_items_from_text(text: &str) -> Vec<ItemStatEntry> {
    let mut entries = Vec::new();
    let mut search_slice = text;
    let prefixes = [
        "CoreItems(",
        "FourthItem(",
        "FifthItem(",
        "SixthItem(",
        "Boots(",
        "Boot(",
    ];
    loop {
        let next_match = prefixes
            .iter()
            .filter_map(|p| search_slice.find(p).map(|idx| (idx, p.len())))
            .min_by_key(|(idx, _)| *idx);
        let Some((ci_idx, prefix_len)) = next_match else {
            break;
        };
        let after_ci = &search_slice[ci_idx + prefix_len..];
        let maybe_brackets = after_ci
            .find('[')
            .zip(after_ci.find(']'))
            .filter(|(open, close)| open < close);
        if let Some((open_bracket, close_bracket)) = maybe_brackets {
            let inside = &after_ci[open_bracket + 1..close_bracket];
            let ids: Vec<i32> = inside
                .split(',')
                .filter_map(|s| s.trim().parse::<i32>().ok())
                .collect();
            if !ids.is_empty() {
                let rest_after_bracket = &after_ci[close_bracket + 1..];
                let mut play = 1;
                let mut win = 0;
                let mut pick_rate = 0.0;
                if let Some(close_paren) = rest_after_bracket.find(')') {
                    let stats_str = &rest_after_bracket[..close_paren];
                    let parts: Vec<&str> = stats_str
                        .trim_start_matches(',')
                        .split(',')
                        .map(|s| s.trim())
                        .filter(|s| !s.is_empty())
                        .collect();
                    if let Some(Ok(p)) = parts.first().map(|s| s.parse::<i64>()) {
                        play = p;
                    }
                    if let Some(Ok(w)) = parts.get(1).map(|s| s.parse::<i64>()) {
                        win = w;
                    }
                    if let Some(Ok(pr)) = parts.get(2).map(|s| s.parse::<f64>()) {
                        pick_rate = pr;
                    }
                }
                entries.push(ItemStatEntry {
                    ids,
                    play,
                    win,
                    pick_rate,
                });
            }
            search_slice = &after_ci[close_bracket + 1..];
            continue;
        }
        break;
    }
    entries
}

/// Extracts separate item lists (`last_items`, `fourth_items`, `fifth_items`, `sixth_items`) from OP.GG MCP text.
pub fn extract_mcp_grouped_item_lists(text: &str) -> Vec<(String, Vec<ItemStatEntry>)> {
    let data_fields: Vec<String> = if let Some(def_idx) = text.find("class Data:") {
        let line_rest = &text[def_idx + 11..];
        let end_line = line_rest.find('\n').unwrap_or(line_rest.len());
        line_rest[..end_line]
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| {
                matches!(
                    s.as_str(),
                    "boots" | "last_items" | "fourth_items" | "fifth_items" | "sixth_items"
                )
            })
            .collect()
    } else {
        Vec::new()
    };

    let mut groups: Vec<Vec<ItemStatEntry>> = Vec::new();
    let Some(data_idx) = text.find("Data(") else {
        return Vec::new();
    };
    let mut slice = &text[data_idx + 5..];
    if let Some(runes_idx) = slice.find("Runes(") {
        slice = &slice[..runes_idx];
    }

    let bytes = slice.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'[' {
            let mut j = i + 1;
            while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            let rem = &slice[j..];
            if rem.starts_with("CoreItems(")
                || rem.starts_with("FourthItem(")
                || rem.starts_with("FifthItem(")
                || rem.starts_with("SixthItem(")
                || rem.starts_with("Boots(")
                || rem.starts_with("Boot(")
            {
                let mut depth = 1;
                let mut k = i + 1;
                while k < bytes.len() && depth > 0 {
                    if bytes[k] == b'[' {
                        depth += 1;
                    } else if bytes[k] == b']' {
                        depth -= 1;
                    }
                    k += 1;
                }
                let list_str = &slice[i..k];
                let items = extract_all_core_items_from_text(list_str);
                groups.push(items);
                i = k;
                continue;
            }
        }
        i += 1;
    }

    let mut result = Vec::new();
    if !data_fields.is_empty() && data_fields.len() == groups.len() {
        for (name, items) in data_fields.into_iter().zip(groups) {
            result.push((name, items));
        }
    } else {
        let default_names = ["fourth_items", "fifth_items", "sixth_items", "last_items"];
        for (idx, items) in groups.into_iter().enumerate() {
            let name = default_names
                .get(idx)
                .copied()
                .unwrap_or("other")
                .to_string();
            result.push((name, items));
        }
    }
    result
}

/// Helper to parse JSON item entries (either single object, array of ids, or array of item stat objects).
pub fn parse_json_item_entries(val: Option<&serde_json::Value>) -> Vec<ItemStatEntry> {
    let mut entries = Vec::new();
    let Some(v) = val else {
        return entries;
    };
    if let Some(arr) = v.as_array() {
        for elem in arr {
            if let Some(num) = elem.as_i64() {
                entries.push(ItemStatEntry {
                    ids: vec![num as i32],
                    play: 1,
                    win: 0,
                    pick_rate: 0.0,
                });
            } else if let Some(obj) = elem.as_object() {
                let ids: Vec<i32> = obj
                    .get("ids")
                    .and_then(|ids_val| ids_val.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|x| x.as_i64().map(|n| n as i32))
                            .collect()
                    })
                    .unwrap_or_default();
                let play = obj.get("play").and_then(|p| p.as_i64()).unwrap_or(1);
                let win = obj.get("win").and_then(|w| w.as_i64()).unwrap_or(0);
                let pick_rate = obj
                    .get("pick_rate")
                    .and_then(|pr| pr.as_f64())
                    .unwrap_or(0.0);
                if !ids.is_empty() {
                    entries.push(ItemStatEntry {
                        ids,
                        play,
                        win,
                        pick_rate,
                    });
                }
            }
        }
    } else if let Some(obj) = v.as_object() {
        let ids: Vec<i32> = obj
            .get("ids")
            .and_then(|ids_val| ids_val.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_i64().map(|n| n as i32))
                    .collect()
            })
            .unwrap_or_default();
        let play = obj.get("play").and_then(|p| p.as_i64()).unwrap_or(1);
        let win = obj.get("win").and_then(|w| w.as_i64()).unwrap_or(0);
        let pick_rate = obj
            .get("pick_rate")
            .and_then(|pr| pr.as_f64())
            .unwrap_or(0.0);
        if !ids.is_empty() {
            entries.push(ItemStatEntry {
                ids,
                play,
                win,
                pick_rate,
            });
        }
    }
    entries
}

// Builds the 5 item blocks conforming to the user's requirements:
// 1. Itens Iniciais (Starter + Potion)
// 2. Primeiros 3 Itens (Top most played Core Build combination of 3 non-boot items)
// 3. 3 Botas Mais Feitas (Always strictly ordered by most absolute games played: play descending)
// 4. Build Final (6 items: 3 Core items + #1 most played 4th item + #1 most played 5th item + #1 most played 6th item)
// 5. Itens Situacionais / Úteis (Non-boot items not in core or final build, strictly ordered by play descending)

/// Parses OP.GG MCP text output into a ChampionBuild.
pub fn parse_opgg_mcp_text(
    text: &str,
    champion_id: i32,
    champion_name: &str,
) -> Result<ChampionBuild, String> {
    // 1. Extract win_rate and pick_rate: AverageStats(...)
    let mut win_rate = 50.0;
    let mut pick_rate = 5.0;

    let avg_fields: Vec<&str> = if let Some(def_idx) = text.find("class AverageStats:") {
        let line_rest = &text[def_idx + 19..];
        let end_line = line_rest.find('\n').unwrap_or(line_rest.len());
        line_rest[..end_line]
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect()
    } else {
        Vec::new()
    };

    if let Some(avg_idx) = text.find("AverageStats(") {
        let after_avg = &text[avg_idx + 13..];
        if let Some(close_idx) = after_avg.find(')') {
            let parts: Vec<&str> = after_avg[..close_idx].split(',').collect();
            if !avg_fields.is_empty() && avg_fields.len() == parts.len() {
                for (field_name, val_str) in avg_fields.iter().zip(parts.iter()) {
                    if let Ok(num) = val_str.trim().parse::<f64>() {
                        if *field_name == "win_rate" {
                            win_rate = if num <= 1.0 {
                                (num * 1000.0).round() / 10.0
                            } else {
                                num
                            };
                        } else if *field_name == "pick_rate" {
                            pick_rate = if num <= 1.0 {
                                (num * 1000.0).round() / 10.0
                            } else {
                                num
                            };
                        }
                    }
                }
            } else {
                // Heuristic fallback: play count is an integer > 100, win_rate and pick_rate are <= 100
                let float_candidates: Vec<f64> = parts
                    .iter()
                    .filter_map(|s| s.trim().parse::<f64>().ok())
                    .filter(|&v| v <= 100.0)
                    .collect();
                if float_candidates.len() >= 2 {
                    let w = float_candidates[0];
                    let p = float_candidates[1];
                    win_rate = if w <= 1.0 {
                        (w * 1000.0).round() / 10.0
                    } else {
                        w
                    };
                    pick_rate = if p <= 1.0 {
                        (p * 1000.0).round() / 10.0
                    } else {
                        p
                    };
                } else if float_candidates.len() == 1 {
                    let w = float_candidates[0];
                    win_rate = if w <= 1.0 {
                        (w * 1000.0).round() / 10.0
                    } else {
                        w
                    };
                }
            }
        }
    }

    // 2. Extract Runes: Runes(8100,[8112,8139,8140,8106],8200,[8210,8226],[5005,5008,5001])
    let runes_idx = text
        .find("Runes(")
        .ok_or_else(|| "Missing Runes block in OP.GG text".to_string())?;
    let after_runes = &text[runes_idx + 6..];

    // Find primary style id
    let comma1 = after_runes
        .find(',')
        .ok_or_else(|| "Invalid Runes format".to_string())?;
    let mut primary_style_id: i32 = after_runes[..comma1]
        .trim()
        .parse()
        .map_err(|e| format!("Invalid primary style id: {}", e))?;

    // Primary perks array: [ ... ]
    let p_open = after_runes
        .find('[')
        .ok_or_else(|| "Missing primary perks [".to_string())?;
    let p_close = after_runes
        .find(']')
        .ok_or_else(|| "Missing primary perks ]".to_string())?;
    let primary_perks: Vec<i32> = after_runes[p_open + 1..p_close]
        .split(',')
        .filter_map(|s| s.trim().parse::<i32>().ok())
        .collect();

    let after_primary = &after_runes[p_close + 1..];
    let after_primary_trim =
        after_primary.trim_start_matches(|c: char| c == ',' || c.is_whitespace());

    // Sub style id
    let comma2 = after_primary_trim
        .find(',')
        .ok_or_else(|| "Invalid sub style format".to_string())?;
    let mut sub_style_id: i32 = after_primary_trim[..comma2]
        .trim()
        .parse()
        .map_err(|e| format!("Invalid sub style id: {}", e))?;

    // Secondary perks array: [ ... ]
    let s_open = after_primary_trim
        .find('[')
        .ok_or_else(|| "Missing secondary perks [".to_string())?;
    let s_close = after_primary_trim
        .find(']')
        .ok_or_else(|| "Missing secondary perks ]".to_string())?;
    let secondary_perks: Vec<i32> = after_primary_trim[s_open + 1..s_close]
        .split(',')
        .filter_map(|s| s.trim().parse::<i32>().ok())
        .collect();

    // Stat mods array: [ ... ]
    let after_sec = &after_primary_trim[s_close + 1..];
    let m_open = after_sec
        .find('[')
        .ok_or_else(|| "Missing stat mods [".to_string())?;
    let m_close = after_sec
        .find(']')
        .ok_or_else(|| "Missing stat mods ]".to_string())?;
    let stat_mods: Vec<i32> = after_sec[m_open + 1..m_close]
        .split(',')
        .filter_map(|s| s.trim().parse::<i32>().ok())
        .collect();

    let mut selected_perk_ids = Vec::with_capacity(9);
    selected_perk_ids.extend(primary_perks);
    selected_perk_ids.extend(secondary_perks);
    selected_perk_ids.extend(stat_mods);

    if selected_perk_ids.len() != 9 {
        return Err(format!(
            "Expected 9 perks, extracted {}",
            selected_perk_ids.len()
        ));
    }

    // 3. Extract all CoreItems entries with stats
    let all_entries = extract_all_core_items_from_text(text);

    // 4. Extract summoner spells
    let mut spell1_id = 4;
    let mut spell2_id = 14;
    if let Some(ss_idx) = text.find("SummonerSpells(") {
        let after_ss = &text[ss_idx + 15..];
        let maybe_brackets = after_ss
            .find('[')
            .zip(after_ss.find(']'))
            .filter(|(open, close)| open < close);
        if let Some((o, c)) = maybe_brackets {
            let ids: Vec<i32> = after_ss[o + 1..c]
                .split(',')
                .filter_map(|s| s.trim().parse::<i32>().ok())
                .collect();
            if ids.len() >= 2 {
                spell1_id = ids[0];
                spell2_id = ids[1];
            }
        }
    } else {
        for entry in &all_entries {
            if entry.ids.len() == 2
                && matches!(entry.ids[0], 4 | 14 | 12 | 7 | 21 | 11 | 6 | 3 | 1 | 32)
            {
                spell1_id = entry.ids[0];
                spell2_id = entry.ids[1];
                break;
            }
        }
    }

    // Filter out spell arrays from items entries
    let item_candidate_entries: Vec<ItemStatEntry> = all_entries
        .into_iter()
        .filter(|entry| {
            !(entry.ids.len() == 2
                && matches!(entry.ids[0], 4 | 14 | 12 | 7 | 21 | 11 | 6 | 3 | 1 | 32))
        })
        .collect();

    let is_ap = text.contains("\"AP\"")
        || (!text.contains("\"AD\"")
            && (champion_name.to_lowercase().contains("ahri")
                || champion_name.to_lowercase().contains("lux")
                || champion_name.to_lowercase().contains("syndra")
                || champion_name.to_lowercase().contains("veigar")));
    let damage_type = if is_ap { "AP" } else { "AD" };

    let starter_entries = if let Some(e) = item_candidate_entries.get(2) {
        vec![e.clone()]
    } else {
        Vec::new()
    };

    let grouped_lists = extract_mcp_grouped_item_lists(text);
    let mut fourth_entries = Vec::new();
    let mut fifth_entries = Vec::new();
    let mut sixth_entries = Vec::new();
    let mut last_entries = Vec::new();
    let mut boots_from_mcp = Vec::new();
    for (name, items) in grouped_lists {
        match name.as_str() {
            "boots" => boots_from_mcp = items,
            "fourth_items" => fourth_entries = items,
            "fifth_items" => fifth_entries = items,
            "sixth_items" => sixth_entries = items,
            "last_items" => last_entries = items,
            _ => last_entries.extend(items),
        }
    }

    let boot_entries = if !boots_from_mcp.is_empty() {
        boots_from_mcp
    } else {
        item_candidate_entries.get(1).cloned().into_iter().collect()
    };

    let mut candidate_entries = Vec::new();
    let core_ids = if let Some(first) = item_candidate_entries.first() {
        let mut core_entry = first.clone();
        if core_entry.play == 1 {
            core_entry.play = 10000;
        }
        let ids = core_entry.ids.clone();
        candidate_entries.push(core_entry);
        ids
    } else {
        Vec::new()
    };

    if !last_entries.is_empty() {
        for entry in &last_entries {
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
    } else if item_candidate_entries.len() > 3 {
        for entry in &item_candidate_entries[3..] {
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
    }

    let position = if text.contains("\"position\": \"jungle\"") || text.contains("\"jungle\"") {
        "jungle"
    } else if text.contains("\"position\": \"support\"") || text.contains("\"support\"") {
        "support"
    } else {
        ""
    };

    let all_for_tear: Vec<ItemStatEntry> = candidate_entries
        .iter()
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
    normalize_entries(&mut candidate_entries);
    normalize_entries(&mut fourth_entries);
    normalize_entries(&mut fifth_entries);
    normalize_entries(&mut sixth_entries);

    let cand_ids: Vec<i32> = candidate_entries
        .iter()
        .flat_map(|e| e.ids.clone())
        .collect();
    let core_build_options =
        extract_top_core_build_options(&candidate_entries, &cand_ids, damage_type);
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
        &candidate_entries,
        &fourth_entries,
        &fifth_entries,
        &sixth_entries,
        &candidate_entries,
        &last_entries,
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

    let spell_choices = vec![SpellMetadata {
        id: spell2_id,
        name: String::new(),
        icon_url: String::new(),
    }];

    let mut top_rune_pages = extract_all_mcp_runes(text);
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

/// Extracts the top summoner spells and the 2 most popular choices for the secondary spell slot
#[cfg(test)]
mod tests {
    use super::*;
    use crate::build_provider::*;

    #[test]
    fn test_parse_opgg_mcp_text_real_ahri_payload() {
        let ahri_mcp_json = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "content": [
                    {
                        "type": "text",
                        "text": "class LolGetChampionAnalysis: data\nclass Data: summary,core_items,boots,starter_items,fourth_items,fifth_items,sixth_items,summoner_spells,runes\nclass Summary: average_stats\nclass AverageStats: win_rate,pick_rate\nclass CoreItems: ids\nclass Runes: primary_page_id,primary_rune_ids,secondary_page_id,secondary_rune_ids,stat_mod_ids\n\nLolGetChampionAnalysis(Data(Summary(AverageStats(0.51,0.12)),CoreItems([3118,4645,3157]),CoreItems([3020]),CoreItems([1056,2003,2003]),[CoreItems([3089]),CoreItems([3157]),CoreItems([3135])],[CoreItems([3089]),CoreItems([3135]),CoreItems([3157])],[CoreItems([4629]),CoreItems([3135]),CoreItems([4646])],CoreItems([4,14]),Runes(8100,[8112,8139,8140,8106],8200,[8210,8226],[5005,5008,5001])))\""
                    }
                ]
            }
        });

        let build = parse_opgg_analysis_payload(&ahri_mcp_json, 103, "Ahri")
            .expect("Parsing real Ahri MCP payload should succeed");

        assert_eq!(build.champion_id, 103);
        assert_eq!(build.champion_name, "Ahri");
        // win_rate: 0.51 converted to 51.0%, pick_rate: 0.12 converted to 12.0%
        assert_eq!(build.win_rate, 51.0);
        assert_eq!(build.pick_rate, 12.0);

        // Runes: Domination 8100 + Sorcery 8200
        assert_eq!(build.primary_style_id, 8100);
        assert_eq!(build.sub_style_id, 8200);
        assert_eq!(
            build.selected_perk_ids,
            vec![8112, 8139, 8140, 8106, 8210, 8226, 5005, 5008, 5001]
        );

        // Spells: Flash (4) + Ignite (14)
        assert_eq!(build.spell1_id, 4);
        assert_eq!(build.spell2_id, 14);

        // Core 3 items: Malignance (3118), Shadowflame (4645), Zhonya's (3157) - NO BOOTS!
        assert_eq!(build.item_blocks[1].title, "Primeiros 3 Itens");
        assert_eq!(build.item_blocks[1].item_ids, vec![3118, 4645, 3157]);
        for item in &build.item_blocks[1].item_ids {
            assert!(
                !is_boot_item(*item),
                "Boot item {} must not be in Core 3 items",
                item
            );
        }

        assert_eq!(build.item_blocks.len(), 5);
        // Position 3: Boots block: Sorcerer's Shoes (3020)
        assert_eq!(build.item_blocks[2].title, "3 Botas Mais Feitas");
        assert!(build.item_blocks[2].item_ids.contains(&3020));

        // Position 4: Build Final
        assert_eq!(build.item_blocks[3].title, "Build Final");
        for &core_item in &build.item_blocks[1].item_ids {
            assert!(build.item_blocks[3].item_ids.contains(&core_item));
        }

        // Position 5: Itens Situacionais / Úteis
        assert_eq!(build.item_blocks[4].title, "Itens Situacionais / Úteis");
        assert!(!build.item_blocks[4].item_ids.is_empty());
        for &item in &build.item_blocks[4].item_ids {
            assert!(
                !build.item_blocks[1].item_ids.contains(&item),
                "Situational item {} cannot be in Core 3",
                item
            );
            assert!(
                !build.item_blocks[3].item_ids.contains(&item),
                "Situational item {} cannot be in Final Build",
                item
            );
            assert!(
                !is_boot_item(item),
                "Situational item {} cannot be a boot",
                item
            );
        }
    }

    #[test]
    fn test_parse_opgg_mcp_text_with_play_in_average_stats() {
        let text_with_play = "class LolGetChampionAnalysis: data\nclass Data: summary\nclass Summary: average_stats\nclass AverageStats: play,win_rate,pick_rate\n\nLolGetChampionAnalysis(Data(Summary(AverageStats(80342,0.512,0.118)),CoreItems([3118,4645,3157]),CoreItems([3020]),CoreItems([1056,2003]),CoreItems([4,14]),Runes(8100,[8112,8139,8140,8106],8200,[8210,8226],[5005,5008,5001])))";
        let build = parse_opgg_mcp_text(text_with_play, 103, "Ahri")
            .expect("Parsing payload with play field in AverageStats should succeed");
        assert_eq!(build.win_rate, 51.2);
        assert_eq!(build.pick_rate, 11.8);
        assert_eq!(build.item_blocks[1].item_ids, vec![3118, 4645, 3157]);
        // Build final first 3 items must be identical to clean_core in exact sequential order
        assert_eq!(&build.item_blocks[3].item_ids[..3], &[3118, 4645, 3157]);
    }
}
