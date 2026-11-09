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
mod tests {
    use super::*;
    use crate::build_provider::*;
        
    #[test]
    fn test_build_ordering_strictly_by_play_count() {
        // Item 3118 (Malignance): 10,000 games, 5,100 wins (51.0% win rate)
        // Item 3089 (Rabadon): 1,000 games, 650 wins (65.0% win rate)
        // Item 4645 (Shadowflame): 8,000 games, 4,200 wins (52.5% win rate)
        // Item 3157 (Zhonya): 6,000 games, 3,300 wins (55.0% win rate)
        let starter = Some(ItemStatEntry {
            ids: vec![1056, 2003],
            play: 10000,
            win: 5000,
            pick_rate: 0.9,
        });
        let boots = vec![ItemStatEntry {
            ids: vec![3020],
            play: 8000,
            win: 4000,
            pick_rate: 0.8,
        }];
        let candidates = vec![
            ItemStatEntry {
                ids: vec![3118],
                play: 10000,
                win: 5100,
                pick_rate: 0.7,
            },
            ItemStatEntry {
                ids: vec![3089],
                play: 1000,
                win: 650,
                pick_rate: 0.2,
            },
            ItemStatEntry {
                ids: vec![4645],
                play: 8000,
                win: 4200,
                pick_rate: 0.5,
            },
            ItemStatEntry {
                ids: vec![3157],
                play: 6000,
                win: 3300,
                pick_rate: 0.4,
            },
        ];

        // Ordering MUST strictly follow play count descending (absolute matches played):
        let popular_blocks = build_5_item_blocks(starter, boots, candidates, "AP");
        // Ordered by play count: 3118 (10k), 4645 (8k), 3157 (6k)
        assert_eq!(popular_blocks[1].item_ids, vec![3118, 4645, 3157]);
    }

    #[test]
    fn test_build_final_and_situational_exclude_components_and_cap_at_6() {
        let starter = Some(ItemStatEntry {
            ids: vec![1055, 2003],
            play: 10000,
            win: 5000,
            pick_rate: 0.9,
        });
        let boots = vec![ItemStatEntry {
            ids: vec![3006],
            play: 10000,
            win: 5000,
            pick_rate: 0.8,
        }];
        let candidates = vec![
            ItemStatEntry {
                ids: vec![6672, 3085, 3031],
                play: 16000,
                win: 8000,
                pick_rate: 0.5,
            }, // Core
            ItemStatEntry {
                ids: vec![3032],
                play: 12000,
                win: 6000,
                pick_rate: 0.4,
            }, // Yun Tal (4th)
            ItemStatEntry {
                ids: vec![1038],
                play: 11000,
                win: 5500,
                pick_rate: 0.35,
            }, // B.F. Sword (component - MUST BE SKIPPED!)
            ItemStatEntry {
                ids: vec![3036],
                play: 10000,
                win: 5000,
                pick_rate: 0.3,
            }, // LDR (5th)
            ItemStatEntry {
                ids: vec![3026],
                play: 9000,
                win: 4500,
                pick_rate: 0.28,
            }, // GA (6th)
            ItemStatEntry {
                ids: vec![2523],
                play: 8000,
                win: 4000,
                pick_rate: 0.25,
            }, // Situational 1
            ItemStatEntry {
                ids: vec![2512],
                play: 7000,
                win: 3500,
                pick_rate: 0.2,
            }, // Situational 2
            ItemStatEntry {
                ids: vec![3033],
                play: 6000,
                win: 3000,
                pick_rate: 0.18,
            }, // Situational 3
            ItemStatEntry {
                ids: vec![6673],
                play: 4000,
                win: 2000,
                pick_rate: 0.12,
            }, // Situational 4
            ItemStatEntry {
                ids: vec![3072],
                play: 3000,
                win: 1500,
                pick_rate: 0.1,
            }, // Situational 5
            ItemStatEntry {
                ids: vec![3139],
                play: 2000,
                win: 1000,
                pick_rate: 0.08,
            }, // Situational 6
            ItemStatEntry {
                ids: vec![3046],
                play: 1000,
                win: 500,
                pick_rate: 0.04,
            }, // Situational 7 (should be dropped by cap)
        ];

        let blocks = build_5_item_blocks(starter, boots, candidates, "AD");

        // Core items
        assert_eq!(blocks[1].item_ids, vec![6672, 3085, 3031]);

        // Build Final: 6 items, must NEVER contain component 1038 (B.F. Sword)!
        // Should contain core (6672, 3085, 3031) followed by 3032 (4th), 3036 (5th), and 3026 (6th)
        assert_eq!(blocks[3].item_ids, vec![6672, 3085, 3031, 3032, 3036, 3026]);

        // Situational items: max 6 items, must not contain 1038 or any core/final items
        assert_eq!(blocks[4].item_ids.len(), 6);
        assert!(!blocks[4].item_ids.contains(&1038));
        for &id in &blocks[4].item_ids {
            assert!(!blocks[1].item_ids.contains(&id));
            assert!(!blocks[3].item_ids.contains(&id));
        }
    }

    #[test]
    fn test_situational_items_filtered_and_sorted_by_play() {
        let starter = Some(ItemStatEntry {
            ids: vec![1056, 2003],
            play: 50000,
            win: 25000,
            pick_rate: 0.9,
        });
        let boots = vec![ItemStatEntry {
            ids: vec![3020],
            play: 40000,
            win: 20000,
            pick_rate: 0.8,
        }];
        let candidates = vec![
            ItemStatEntry {
                ids: vec![3118, 4645, 3157],
                play: 20000,
                win: 10000,
                pick_rate: 0.4,
            }, // Core
            ItemStatEntry {
                ids: vec![3089],
                play: 8000,
                win: 4500,
                pick_rate: 0.3,
            }, // Final build 4th
            ItemStatEntry {
                ids: vec![3135],
                play: 5000,
                win: 2500,
                pick_rate: 0.2,
            }, // Final build 5th
            ItemStatEntry {
                ids: vec![3102],
                play: 4000,
                win: 2100,
                pick_rate: 0.18,
            }, // Final build 6th
            ItemStatEntry {
                ids: vec![4646],
                play: 1500,
                win: 700,
                pick_rate: 0.1,
            }, // Situational (1500 plays)
            ItemStatEntry {
                ids: vec![4629],
                play: 3200,
                win: 1600,
                pick_rate: 0.15,
            }, // Situational (3200 plays)
            ItemStatEntry {
                ids: vec![3165],
                play: 700,
                win: 350,
                pick_rate: 0.05,
            }, // Situational (700 plays)
        ];

        let blocks = build_5_item_blocks(starter, boots, candidates, "AP");

        // Clean core (first 3): 3118, 4645, 3157
        assert_eq!(blocks[1].item_ids, vec![3118, 4645, 3157]);
        // Final build (6 items): 3118, 4645, 3157, 3089, 3135, 3102
        assert_eq!(blocks[3].item_ids, vec![3118, 4645, 3157, 3089, 3135, 3102]);

        // Situational items:
        // Must NOT contain any item from core or final build:
        for &id in &blocks[4].item_ids {
            assert!(!blocks[1].item_ids.contains(&id));
            assert!(!blocks[3].item_ids.contains(&id));
            assert!(!is_boot_item(id));
        }

        // Must be ordered strictly by play count descending:
        // 4629 (3200 plays) should appear BEFORE 4646 (1500 plays) which is BEFORE 3165 (700 plays)!
        let pos_4629 = blocks[4].item_ids.iter().position(|&x| x == 4629).unwrap();
        let pos_4646 = blocks[4].item_ids.iter().position(|&x| x == 4646).unwrap();
        let pos_3165 = blocks[4].item_ids.iter().position(|&x| x == 3165).unwrap();
        assert!(pos_4629 < pos_4646);
        assert!(pos_4646 < pos_3165);
    }

    #[test]
    fn test_starter_items_by_position_adc_top_mid_jungle_support() {
        // Standard ADC (Yunara): Doran's Blade (1055, 76k) > Doran's Shield (1054, 12k) + Potion (2003)
        let adc_starters = vec![
            ItemStatEntry {
                ids: vec![1055, 2003],
                play: 76942,
                win: 38123,
                pick_rate: 0.8,
            },
            ItemStatEntry {
                ids: vec![1054, 2003],
                play: 12000,
                win: 6000,
                pick_rate: 0.15,
            },
        ];
        let adc_blocks = build_5_item_blocks_with_depth_lists(
            adc_starters,
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
            "AD",
            "adc",
        );
        assert_eq!(adc_blocks[0].title, "Itens Iniciais");
        assert_eq!(adc_blocks[0].item_ids, vec![1055, 2003]);

        // Standard Top (Tank): Doran's Shield (1054, 40k) > Doran's Blade (1055, 5k) + Potion (2003)
        let top_starters = vec![
            ItemStatEntry {
                ids: vec![1054, 2003],
                play: 40000,
                win: 20000,
                pick_rate: 0.7,
            },
            ItemStatEntry {
                ids: vec![1055, 2003],
                play: 5000,
                win: 2500,
                pick_rate: 0.1,
            },
        ];
        let top_blocks = build_5_item_blocks_with_depth_lists(
            top_starters,
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
            "AD",
            "top",
        );
        assert_eq!(top_blocks[0].item_ids, vec![1054, 2003]);

        // Standard Mid (Mage): Doran's Ring (1056, 30k) > Dark Seal (1082, 5k) + Potion (2003)
        let mid_starters = vec![
            ItemStatEntry {
                ids: vec![1056, 2003],
                play: 30000,
                win: 15000,
                pick_rate: 0.8,
            },
            ItemStatEntry {
                ids: vec![1082, 2003],
                play: 5000,
                win: 2500,
                pick_rate: 0.1,
            },
        ];
        let mid_blocks = build_5_item_blocks_with_depth_lists(
            mid_starters,
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
            "AP",
            "mid",
        );
        assert_eq!(mid_blocks[0].item_ids, vec![1056, 2003]);

        // Jungle: Mosstomper (1102, 50k) > Gustwalker (1103, 20k) > Scorchclaw (1101, 10k) + Potion (2003)
        let jng_starters = vec![
            ItemStatEntry {
                ids: vec![1102, 2003],
                play: 50000,
                win: 25000,
                pick_rate: 0.6,
            },
            ItemStatEntry {
                ids: vec![1103, 2003],
                play: 20000,
                win: 10000,
                pick_rate: 0.25,
            },
            ItemStatEntry {
                ids: vec![1101, 2003],
                play: 10000,
                win: 5000,
                pick_rate: 0.15,
            },
        ];
        let jng_blocks = build_5_item_blocks_with_depth_lists(
            jng_starters,
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
            "AD",
            "jungle",
        );
        assert_eq!(jng_blocks[0].item_ids, vec![1102, 2003]);

        // Support: Quest item is pre-granted/bought, must ONLY contain Potion!
        let supp_starters = vec![ItemStatEntry {
            ids: vec![3865, 2003, 2003],
            play: 60000,
            win: 30000,
            pick_rate: 0.95,
        }];
        let supp_blocks = build_5_item_blocks_with_depth_lists(
            supp_starters,
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
            "AP",
            "support",
        );
        assert_eq!(supp_blocks[0].item_ids, vec![2003]);
    }

    #[test]
    fn test_boots_ranking_prioritizes_most_played_boots() {
        // Boot 3020 (Sorcerer's): 50,000 games, 50% win rate
        // Boot 3158 (Lucidity): 5,000 games, 60% win rate
        // Boot 3111 (Mercs): 10,000 games, 52% win rate
        let starter = Some(ItemStatEntry {
            ids: vec![1056, 2003],
            play: 50000,
            win: 25000,
            pick_rate: 0.9,
        });
        let boots = vec![
            ItemStatEntry {
                ids: vec![3158],
                play: 5000,
                win: 3000,
                pick_rate: 0.1,
            },
            ItemStatEntry {
                ids: vec![3020],
                play: 50000,
                win: 25000,
                pick_rate: 0.7,
            },
            ItemStatEntry {
                ids: vec![3111],
                play: 10000,
                win: 5200,
                pick_rate: 0.2,
            },
        ];
        let candidates = vec![ItemStatEntry {
            ids: vec![3118, 4645, 3089],
            play: 20000,
            win: 11000,
            pick_rate: 0.5,
        }];

        let blocks = build_5_item_blocks(starter, boots, candidates, "AP");

        // Boots MUST be sorted by play count descending: 3020 (50k) -> 3111 (10k) -> 3158 (5k)
        assert_eq!(blocks[2].title, "3 Botas Mais Feitas");
        assert_eq!(blocks[2].item_ids, vec![3020, 3111, 3158]);
    }

    #[test]
    fn test_boots_ranking_yunara_with_synchronized_souls() {
        // Mirrors OP.GG boots table for Yunara:
        // 1. 3008 (Synchronized Souls) -> 41,774 Games (53.06%)
        // 2. 3006 (Berserker's Greaves) -> 33,119 Games (42.07%)
        // 3. 3047 (Plated Steelcaps) -> 572 Games (0.73%)
        // 4. 3009 (Boots of Swiftness) -> 257 Games (0.33%)
        // 5. 3111 (Mercury's Treads) -> 252 Games (0.32%)
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
                    { "ids": [3008], "play": 41774, "win": 22345, "pick_rate": 0.5306 },
                    { "ids": [3006], "play": 33119, "win": 15735, "pick_rate": 0.4207 },
                    { "ids": [3047], "play": 572, "win": 286, "pick_rate": 0.0073 },
                    { "ids": [3009], "play": 257, "win": 128, "pick_rate": 0.0033 },
                    { "ids": [3111], "play": 252, "win": 126, "pick_rate": 0.0032 }
                ],
                "core_items": [
                    { "ids": [6672, 3085, 3031], "play": 19057, "win": 10673 }
                ],
                "fourth_items": [
                    { "ids": [3036], "play": 18151, "win": 11137 }
                ],
                "fifth_items": [
                    { "ids": [3026], "play": 4832, "win": 3108 }
                ],
                "sixth_items": [
                    { "ids": [3072], "play": 1036, "win": 630 }
                ]
            }
        });

        let build = parse_opgg_analysis_payload(&json, 804, "Yunara")
            .expect("Parsing Yunara build with boots list should succeed");

        // Verify Block 3 (3 Botas Mais Feitas):
        assert_eq!(build.item_blocks[2].title, "3 Botas Mais Feitas");
        // Must be in strict order of play count descending: 3008 (41,774) -> 3006 (33,119) -> 3047 (572)
        assert_eq!(build.item_blocks[2].item_ids, vec![3008, 3006, 3047]);

        // Verify boots never leak into Core (Block 1), Final Build (Block 3), or Situational (Block 4)
        for &boot_id in &[3008, 3006, 3047, 3009, 3111] {
            assert!(
                !build.item_blocks[1].item_ids.contains(&boot_id),
                "Boot {boot_id} leaked into core"
            );
            assert!(
                !build.item_blocks[3].item_ids.contains(&boot_id),
                "Boot {boot_id} leaked into final build"
            );
            assert!(
                !build.item_blocks[4].item_ids.contains(&boot_id),
                "Boot {boot_id} leaked into situational"
            );
        }

        // Verify is_boot_item recognizes 3008, 3010, and 3013
        assert!(is_boot_item(3008));
        assert!(is_boot_item(3010));
        assert!(is_boot_item(3013));
    }
}
