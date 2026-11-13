use super::super::models::{ItemBlock, ItemStatEntry};
use super::classification::*;
use std::collections::HashMap;

pub fn build_5_item_blocks(
    starter_entry: Option<ItemStatEntry>,
    boot_entries: Vec<ItemStatEntry>,
    candidate_entries: Vec<ItemStatEntry>,
    damage_type: &str,
) -> Vec<ItemBlock> {
    build_5_item_blocks_with_depth_lists(
        starter_entry.into_iter().collect(),
        boot_entries,
        candidate_entries,
        Vec::new(),
        Vec::new(),
        Vec::new(),
        damage_type,
        "",
    )
}

/// Builds the 5 item blocks using explicit OP.GG Core builds and separate Fourth, Fifth, and Sixth item lists.
#[allow(clippy::too_many_arguments)]
pub fn build_5_item_blocks_with_depth_lists(
    starter_entries: Vec<ItemStatEntry>,
    boot_entries: Vec<ItemStatEntry>,
    candidate_entries: Vec<ItemStatEntry>,
    fourth_entries: Vec<ItemStatEntry>,
    fifth_entries: Vec<ItemStatEntry>,
    sixth_entries: Vec<ItemStatEntry>,
    damage_type: &str,
    position: &str,
) -> Vec<ItemBlock> {
    // 1. Itens Iniciais:
    // - Default (ADC, TOP, MID): #1 most played starter item from OP.GG list + Potion (2003 / 2031)
    // - Jungle: #1 most played Jungle Pet (1101, 1102, 1103) + Potion (2003)
    // - Support: Only Potion (2003 / 2031) since support quest item is pre-granted/bought
    let norm_pos = position.to_lowercase();
    let is_support = norm_pos == "support" || norm_pos == "supp" || norm_pos == "utility";
    let is_jungle = norm_pos == "jungle" || norm_pos == "jng";

    let starter_ids: Vec<i32> = if is_support {
        let mut potion_id = 2003;
        for entry in &starter_entries {
            for &id in &entry.ids {
                if id == 2003 || id == 2031 || id == 2033 {
                    potion_id = id;
                    break;
                }
            }
            if potion_id != 2003 {
                break;
            }
        }
        vec![potion_id]
    } else if is_jungle {
        let mut pet_counts: HashMap<i32, i64> = HashMap::new();
        let mut potion_id = 2003;
        for entry in &starter_entries {
            for &id in &entry.ids {
                if matches!(id, 1101..=1103) {
                    *pet_counts.entry(id).or_insert(0) += entry.play.max(1);
                } else if id == 2003 || id == 2031 || id == 2033 {
                    potion_id = id;
                }
            }
        }
        let best_pet = pet_counts
            .into_iter()
            .max_by_key(|&(_, play)| play)
            .map(|(id, _)| id)
            .unwrap_or(1103);
        vec![best_pet, potion_id]
    } else {
        // Standard lanes (ADC, TOP, MID, or fallback):
        let mut starter_counts: HashMap<i32, (i64, usize)> = HashMap::new();
        let mut seen_idx = 0;
        let mut potion_id = 2003;
        for entry in &starter_entries {
            for &id in &entry.ids {
                if id == 2003 || id == 2031 || id == 2033 {
                    potion_id = id;
                } else if !matches!(id, 1101..=1103) && !matches!(id, 3865..=3877) {
                    let slot = starter_counts.entry(id).or_insert_with(|| {
                        let i = seen_idx;
                        seen_idx += 1;
                        (0, i)
                    });
                    slot.0 += entry.play.max(1);
                }
            }
        }
        let mut sorted_starters: Vec<(i32, i64, usize)> = starter_counts
            .into_iter()
            .map(|(id, (p, idx))| (id, p, idx))
            .collect();
        sorted_starters.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.2.cmp(&b.2)));
        let best_starter = sorted_starters
            .first()
            .map(|(id, _, _)| *id)
            .unwrap_or_else(|| if damage_type == "AP" { 1056 } else { 1055 });
        if best_starter == 2033 {
            vec![2033]
        } else {
            vec![best_starter, potion_id]
        }
    };

    // Future-proof contextual sets directly from OP.GG structures:
    let known_boots: std::collections::HashSet<i32> = boot_entries
        .iter()
        .flat_map(|e| e.ids.iter().copied())
        .collect();

    let known_starters: std::collections::HashSet<i32> = starter_entries
        .iter()
        .flat_map(|e| e.ids.iter().copied())
        .collect();

    let is_boot = |id: i32| -> bool { known_boots.contains(&id) || is_boot_item_dynamic(id) };
    let is_starter = |id: i32| -> bool {
        known_starters.contains(&id) || starter_ids.contains(&id) || is_starter_item(id)
    };
    let is_comp = |id: i32| -> bool { is_component_item_dynamic(id) };

    // 2. 3 Botas Mais Feitas: Always ranked by absolute games played (play count descending)
    let mut boot_map: HashMap<i32, (i64, usize)> = HashMap::new();
    let mut boot_seen_counter = 0;
    for entry in &boot_entries {
        for &id in &entry.ids {
            if is_boot(id) || (!is_starter(id) && !is_comp(id)) {
                let slot = boot_map.entry(id).or_insert_with(|| {
                    let idx = boot_seen_counter;
                    boot_seen_counter += 1;
                    (0, idx)
                });
                slot.0 += entry.play.max(1);
            }
        }
    }
    // Only supplement with candidate_entries if boot_entries provided fewer than 3 boots
    if boot_map.len() < 3 {
        for entry in &candidate_entries {
            for &id in &entry.ids {
                if is_boot(id) {
                    let slot = boot_map.entry(id).or_insert_with(|| {
                        let idx = boot_seen_counter;
                        boot_seen_counter += 1;
                        (0, idx)
                    });
                    slot.0 += entry.play.max(1);
                }
            }
        }
    }
    let mut boot_pairs: Vec<(i32, i64, usize)> = boot_map
        .into_iter()
        .map(|(id, (play, idx))| (id, play, idx))
        .collect();
    boot_pairs.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.2.cmp(&b.2)));
    let mut boots_list: Vec<i32> = boot_pairs.into_iter().map(|(id, _, _)| id).collect();

    let archetype_boots = if damage_type == "AP" {
        vec![3020, 3158, 3111, 3047]
    } else {
        vec![3006, 3047, 3111, 3009]
    };
    for b in archetype_boots {
        if boots_list.len() < 3 && !boots_list.contains(&b) {
            boots_list.push(b);
        }
    }
    boots_list.truncate(3);

    // 3. Aggregate all non-boot, non-starter candidate items with play counts
    #[derive(Debug, Clone)]
    struct ItemAggregate {
        id: i32,
        play: i64,
        first_seen_index: usize,
    }

    let all_entries_for_tear: Vec<ItemStatEntry> = candidate_entries
        .iter()
        .chain(fourth_entries.iter())
        .chain(fifth_entries.iter())
        .chain(sixth_entries.iter())
        .cloned()
        .collect();
    let tear_upgrade = resolve_tear_to_completed_item(&all_entries_for_tear, position, damage_type);

    let mut item_map: HashMap<i32, (i64, usize)> = HashMap::new();
    let mut item_seen_counter = 0;
    for entry in candidate_entries
        .iter()
        .chain(fourth_entries.iter())
        .chain(fifth_entries.iter())
        .chain(sixth_entries.iter())
    {
        for &id in &entry.ids {
            let norm_id = if id == 3070 {
                tear_upgrade
            } else {
                normalize_tear_item_dynamic(id)
            };
            if !is_boot(norm_id) && !is_starter(norm_id) && !is_comp(norm_id) {
                let entry_play = entry.play.max(1);
                let slot = item_map.entry(norm_id).or_insert_with(|| {
                    let idx = item_seen_counter;
                    item_seen_counter += 1;
                    (0, idx)
                });
                slot.0 += entry_play;
            }
        }
    }

    let mut all_candidates: Vec<ItemAggregate> = item_map
        .into_iter()
        .map(|(id, (play, first_seen_index))| ItemAggregate {
            id,
            play,
            first_seen_index,
        })
        .collect();

    // Sort strictly by absolute games played (play count descending):
    all_candidates.sort_by(|a, b| {
        b.play
            .cmp(&a.play)
            .then_with(|| a.first_seen_index.cmp(&b.first_seen_index))
    });

    // Block 2: Primeiros 3 Itens
    // The 3 core items must strictly appear in sequential order of progression:
    // (1st most built item -> 2nd item after the 1st -> 3rd item after the 2nd).
    let mut clean_core: Vec<i32> = Vec::new();

    // Check if candidate_entries contains multi-item core combinations (length >= 2 non-boot items)
    let mut core_combos: Vec<&ItemStatEntry> = candidate_entries
        .iter()
        .filter(|e| {
            let non_boot_count = e
                .ids
                .iter()
                .filter(|&&id| {
                    !is_boot_item_dynamic(id)
                        && !is_starter_item(id)
                        && !is_component_item_dynamic(id)
                })
                .count();
            non_boot_count >= 2
        })
        .collect();

    if !core_combos.is_empty() {
        core_combos.sort_by_key(|a| std::cmp::Reverse(a.play));
        let best_combo = core_combos[0];
        for &id in &best_combo.ids {
            let norm_id = if id == 3070 {
                tear_upgrade
            } else {
                normalize_tear_item_dynamic(id)
            };
            if clean_core.len() < 3
                && !is_boot_item_dynamic(norm_id)
                && !is_starter_item(norm_id)
                && !is_component_item_dynamic(norm_id)
                && !clean_core.contains(&norm_id)
            {
                clean_core.push(norm_id);
            }
        }
    }

    // Fill up to 3 core items from the top sorted candidates if needed
    for cand in &all_candidates {
        let norm_id = if cand.id == 3070 {
            tear_upgrade
        } else {
            normalize_tear_item_dynamic(cand.id)
        };
        if clean_core.len() < 3 && !clean_core.contains(&norm_id) {
            clean_core.push(norm_id);
        }
    }

    let default_core = if damage_type == "AP" {
        vec![3118, 4645, 3089]
    } else {
        vec![3031, 3094, 3036]
    };
    for d in default_core {
        if clean_core.len() < 3 && !clean_core.contains(&d) {
            clean_core.push(d);
        }
    }

    // Block 4: Build Final (6 items: 3 Core items + most built 4th + most built 5th + most built 6th)
    // The first 3 items MUST be identical to clean_core in the EXACT SAME sequential order,
    // followed by the top valid non-duplicate item from fourth_items, fifth_items, and sixth_items.
    let mut final_build: Vec<i32> = clean_core.clone();

    let pick_top_from_depth_list =
        |entries: &[ItemStatEntry], current_build: &[i32]| -> Option<i32> {
            let mut sorted: Vec<&ItemStatEntry> = entries.iter().collect();
            sorted.sort_by_key(|e| std::cmp::Reverse(e.play));
            for entry in sorted {
                for &id in &entry.ids {
                    let norm_id = if id == 3070 {
                        tear_upgrade
                    } else {
                        normalize_tear_item_dynamic(id)
                    };
                    if !current_build.contains(&norm_id)
                        && !is_component_item_dynamic(norm_id)
                        && !is_starter_item(norm_id)
                        && !is_boot_item_dynamic(norm_id)
                    {
                        return Some(norm_id);
                    }
                }
            }
            None
        };

    for depth_list in [&fourth_entries, &fifth_entries, &sixth_entries] {
        if final_build.len() < 6
            && let Some(item_id) = pick_top_from_depth_list(depth_list, &final_build)
        {
            final_build.push(item_id);
        }
    }

    // Fallback: fill remaining slots up to 6 items from top candidates sorted by play count
    for cand in &all_candidates {
        if final_build.len() < 6
            && !final_build.contains(&cand.id)
            && !is_component_item_dynamic(cand.id)
            && !is_starter_item(cand.id)
            && !is_boot_item_dynamic(cand.id)
        {
            final_build.push(cand.id);
        }
    }
    let default_final = if damage_type == "AP" {
        vec![3157, 3135, 4629, 3102]
    } else {
        vec![3072, 3026, 3139, 3033]
    };
    for d in default_final {
        if final_build.len() < 6 && !final_build.contains(&d) {
            final_build.push(d);
        }
    }

    // Block 5: Itens Situacionais / Úteis
    // Strictly non-boot items that are NOT in clean_core and NOT in final_build,
    // sorted strictly in descending order of absolute games played (play count)!
    let mut situational_candidates: Vec<&ItemAggregate> = all_candidates
        .iter()
        .filter(|cand| {
            let norm_id = if cand.id == 3070 {
                tear_upgrade
            } else {
                normalize_tear_item_dynamic(cand.id)
            };
            !clean_core.contains(&norm_id)
                && !final_build.contains(&norm_id)
                && !is_component_item_dynamic(norm_id)
                && !is_starter_item(norm_id)
                && !is_boot_item_dynamic(norm_id)
        })
        .collect();
    situational_candidates.sort_by(|a, b| {
        b.play
            .cmp(&a.play)
            .then_with(|| a.first_seen_index.cmp(&b.first_seen_index))
    });

    let mut situational_items: Vec<i32> = Vec::new();
    for c in situational_candidates {
        let norm_id = if c.id == 3070 {
            tear_upgrade
        } else {
            normalize_tear_item_dynamic(c.id)
        };
        if !situational_items.contains(&norm_id)
            && !clean_core.contains(&norm_id)
            && !final_build.contains(&norm_id)
        {
            situational_items.push(norm_id);
        }
        if situational_items.len() == 6 {
            break;
        }
    }

    situational_items.truncate(6);

    vec![
        ItemBlock {
            title: "Itens Iniciais".to_string(),
            item_ids: starter_ids,
            item_icon_urls: vec![],
        },
        ItemBlock {
            title: "Primeiros 3 Itens".to_string(),
            item_ids: clean_core,
            item_icon_urls: vec![],
        },
        ItemBlock {
            title: "3 Botas Mais Feitas".to_string(),
            item_ids: boots_list,
            item_icon_urls: vec![],
        },
        ItemBlock {
            title: "Build Final".to_string(),
            item_ids: final_build,
            item_icon_urls: vec![],
        },
        ItemBlock {
            title: "Itens Situacionais / Úteis".to_string(),
            item_ids: situational_items,
            item_icon_urls: vec![],
        },
    ]
}

#[cfg(test)]
#[path = "blocks_tests.rs"]
mod tests;
