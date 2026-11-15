use super::super::models::{ItemBuildOption, ItemStatEntry};
use super::classification::*;

pub fn extract_top_core_build_options(
    core_entries: &[ItemStatEntry],
    all_candidates: &[i32],
    damage_type: &str,
) -> Vec<ItemBuildOption> {
    let tear_upgrade = resolve_tear_to_completed_item(core_entries, "", damage_type);
    let mut options: Vec<ItemBuildOption> = Vec::new();

    // 1. Filter core combinations with >= 2 non-boot, non-starter, non-component items
    let mut valid_combos: Vec<&ItemStatEntry> = core_entries
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

    valid_combos.sort_by_key(|e| std::cmp::Reverse(e.play));

    for combo in valid_combos {
        let mut items = Vec::new();
        for &id in &combo.ids {
            let norm_id = if id == 3070 {
                tear_upgrade
            } else {
                normalize_tear_item_dynamic(id)
            };
            if items.len() < 3
                && !is_boot_item_dynamic(norm_id)
                && !is_starter_item(norm_id)
                && !is_component_item_dynamic(norm_id)
                && !items.contains(&norm_id)
            {
                items.push(norm_id);
            }
        }
        // If combo had only 2 items, fill 3rd from top candidates (excluding boots/starters/components)
        for &cand in all_candidates {
            let norm_cand = if cand == 3070 {
                tear_upgrade
            } else {
                normalize_tear_item_dynamic(cand)
            };
            if items.len() < 3
                && !items.contains(&norm_cand)
                && !is_boot_item_dynamic(norm_cand)
                && !is_starter_item(norm_cand)
                && !is_component_item_dynamic(norm_cand)
            {
                items.push(norm_cand);
            }
        }
        if items.len() == 3 && !options.iter().any(|opt| opt.item_ids == items) {
            options.push(ItemBuildOption {
                item_ids: items,
                item_icon_urls: Vec::new(),
                play: combo.play,
                win: combo.win,
                pick_rate: combo.pick_rate,
            });
        }
    }

    // 2. If fewer than 3 options, build distinct variations from all_candidates
    let default_core = if damage_type == "AP" {
        vec![3118, 4645, 3089, 3157, 3135]
    } else {
        vec![3031, 3094, 3036, 3072, 3026]
    };

    let mut pool: Vec<i32> = Vec::new();
    for &c in all_candidates {
        let norm_c = if c == 3070 {
            tear_upgrade
        } else {
            normalize_tear_item_dynamic(c)
        };
        if !pool.contains(&norm_c)
            && !is_boot_item_dynamic(norm_c)
            && !is_starter_item(norm_c)
            && !is_component_item_dynamic(norm_c)
        {
            pool.push(norm_c);
        }
    }
    for d in default_core {
        if !pool.contains(&d) {
            pool.push(d);
        }
    }

    while options.len() < 3 {
        let shift = options.len();
        let mut var_items = Vec::new();
        // Option 0: items 0, 1, 2
        // Option 1: items 0, 1, 3 or items 1, 0, 2
        // Option 2: items 0, 2, 3 or items 0, 1, 4
        if shift == 0 {
            var_items.extend(pool.iter().take(3).copied());
        } else if shift == 1 {
            if pool.len() >= 4 {
                var_items.push(pool[0]);
                var_items.push(pool[1]);
                var_items.push(pool[3]);
            } else {
                var_items.extend(pool.iter().take(3).copied());
            }
        } else if shift == 2 {
            if pool.len() >= 5 {
                var_items.push(pool[0]);
                var_items.push(pool[2]);
                var_items.push(pool[4]);
            } else if pool.len() >= 4 {
                var_items.push(pool[0]);
                var_items.push(pool[2]);
                var_items.push(pool[3]);
            } else {
                var_items.extend(pool.iter().take(3).copied());
            }
        }

        if var_items.len() < 3 {
            var_items.resize(3, 3089);
        }

        let is_dup = options.iter().any(|opt| opt.item_ids == var_items);
        if !is_dup {
            options.push(ItemBuildOption {
                item_ids: var_items,
                item_icon_urls: Vec::new(),
                play: 1000 / (shift as i64 + 1),
                win: 500 / (shift as i64 + 1),
                pick_rate: 10.0 / (shift as f64 + 1.0),
            });
        } else {
            break;
        }
    }

    options.sort_by_key(|opt| std::cmp::Reverse(opt.play));
    options.truncate(3);
    options
}

/// Builds up to 3 distinct 6-item FinalBuildOptions using primary core and depth/candidate items.
pub fn build_3_final_build_options(
    primary_core: &[i32],
    fourth_entries: &[ItemStatEntry],
    fifth_entries: &[ItemStatEntry],
    sixth_entries: &[ItemStatEntry],
    all_candidates: &[i32],
    damage_type: &str,
) -> Vec<ItemBuildOption> {
    let all_for_tear: Vec<ItemStatEntry> = fourth_entries
        .iter()
        .chain(fifth_entries.iter())
        .chain(sixth_entries.iter())
        .cloned()
        .collect();
    let tear_upgrade = resolve_tear_to_completed_item(&all_for_tear, "", damage_type);
    let mut options: Vec<ItemBuildOption> = Vec::new();

    let pick_all_from_depth_list = |entries: &[ItemStatEntry]| -> Vec<i32> {
        let mut sorted: Vec<&ItemStatEntry> = entries.iter().collect();
        sorted.sort_by_key(|e| std::cmp::Reverse(e.play));
        let mut ids = Vec::new();
        for entry in sorted {
            for &id in &entry.ids {
                let norm_id = if id == 3070 {
                    tear_upgrade
                } else {
                    normalize_tear_item_dynamic(id)
                };
                if !ids.contains(&norm_id)
                    && !is_component_item_dynamic(norm_id)
                    && !is_starter_item(norm_id)
                    && !is_boot_item_dynamic(norm_id)
                {
                    ids.push(norm_id);
                }
            }
        }
        ids
    };

    let f_ids = pick_all_from_depth_list(fourth_entries);
    let fi_ids = pick_all_from_depth_list(fifth_entries);
    let s_ids = pick_all_from_depth_list(sixth_entries);

    let default_late = if damage_type == "AP" {
        vec![3157, 3135, 4629, 3102, 4646]
    } else {
        vec![3072, 3026, 3139, 3033, 3091]
    };

    // Variation 0: Top 4th + Top 5th + Top 6th
    // Variation 1: 2nd 4th / alternative late items
    // Variation 2: 3rd 4th / alternative defensive/situational items
    for var_idx in 0..3 {
        let mut build_6 = normalize_item_ids_dedup(primary_core);

        let mut available_late = Vec::new();
        if var_idx == 0 {
            available_late.extend(&f_ids);
            available_late.extend(&fi_ids);
            available_late.extend(&s_ids);
        } else if var_idx == 1 {
            if f_ids.len() > 1 {
                available_late.extend(&f_ids[1..]);
            }
            if fi_ids.len() > 1 {
                available_late.extend(&fi_ids[1..]);
            }
            available_late.extend(&s_ids);
            available_late.extend(&f_ids);
        } else {
            if s_ids.len() > 1 {
                available_late.extend(&s_ids[1..]);
            }
            if fi_ids.len() > 1 {
                available_late.extend(&fi_ids[1..]);
            }
            available_late.extend(&f_ids);
        }
        available_late.extend(all_candidates);
        available_late.extend(&default_late);

        for &item in &available_late {
            let norm_item = if item == 3070 {
                tear_upgrade
            } else {
                normalize_tear_item_dynamic(item)
            };
            if build_6.len() < 6
                && !build_6.contains(&norm_item)
                && !is_boot_item_dynamic(norm_item)
                && !is_starter_item(norm_item)
                && !is_component_item_dynamic(norm_item)
            {
                build_6.push(norm_item);
            }
        }

        if build_6.len() == 6 && !options.iter().any(|opt| opt.item_ids == build_6) {
            options.push(ItemBuildOption {
                item_ids: build_6,
                item_icon_urls: Vec::new(),
                play: 5000 / (var_idx as i64 + 1),
                win: 2600 / (var_idx as i64 + 1),
                pick_rate: 30.0 / (var_idx as f64 + 1.0),
            });
        }
    }

    if options.is_empty() {
        let mut fallback_6 = primary_core.to_vec();
        for &d in &default_late {
            if fallback_6.len() < 6 && !fallback_6.contains(&d) {
                fallback_6.push(d);
            }
        }
        options.push(ItemBuildOption {
            item_ids: fallback_6,
            item_icon_urls: Vec::new(),
            play: 1,
            win: 1,
            pick_rate: 100.0,
        });
    }

    options.sort_by_key(|opt| std::cmp::Reverse(opt.play));
    options.truncate(3);
    options
}

#[cfg(test)]
#[path = "core_final_options_tests.rs"]
mod tests;
