use super::super::models::{ItemBuildOption, ItemStatEntry};
use super::classification::*;

pub fn extract_top_core_build_options(
    core_entries: &[ItemStatEntry],
    all_candidates: &[i32],
    damage_type: &str,
) -> Vec<ItemBuildOption> {
    let tear_upgrade = resolve_tear_to_completed_item(core_entries).or_else(|| {
        all_candidates
            .iter()
            .map(|&id| normalize_tear_item_dynamic(id))
            .find(|id| TEAR_COMPLETED_ITEMS.contains(id))
    });
    extract_top_core_build_options_with_tear(
        core_entries,
        all_candidates,
        damage_type,
        tear_upgrade,
    )
}

pub fn extract_top_core_build_options_with_tear(
    core_entries: &[ItemStatEntry],
    all_candidates: &[i32],
    damage_type: &str,
    tear_upgrade: Option<i32>,
) -> Vec<ItemBuildOption> {
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
        for norm_id in resolve_tear_item_ids(&combo.ids, tear_upgrade) {
            if items.len() < 3
                && !is_boot_item_dynamic(norm_id)
                && !is_starter_item(norm_id)
                && !is_component_item_dynamic(norm_id)
                && !items.contains(&norm_id)
            {
                items.push(norm_id);
            }
        }

        // Fill up to 3 core items from all_candidates if needed
        if items.len() < 3 {
            for &cand in all_candidates {
                let norm_cand = normalize_tear_item_dynamic(cand);
                if items.len() < 3
                    && !items.contains(&norm_cand)
                    && !is_boot_item_dynamic(norm_cand)
                    && !is_starter_item(norm_cand)
                    && !is_component_item_dynamic(norm_cand)
                {
                    items.push(norm_cand);
                }
            }
        }

        let default_cores = if damage_type == "AP" {
            vec![3089, 3157, 4645]
        } else {
            vec![3031, 3072, 3026]
        };
        for def in default_cores {
            if items.len() < 3 && !items.contains(&def) {
                items.push(def);
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

    if options.is_empty() {
        let mut fallback = Vec::new();
        for &c in all_candidates {
            let norm_c = normalize_tear_item_dynamic(c);
            if fallback.len() < 3
                && !fallback.contains(&norm_c)
                && !is_boot_item_dynamic(norm_c)
                && !is_starter_item(norm_c)
                && !is_component_item_dynamic(norm_c)
            {
                fallback.push(norm_c);
            }
        }
        let default_cores = if damage_type == "AP" {
            vec![3089, 3157, 4645]
        } else {
            vec![3031, 3072, 3026]
        };
        for def in default_cores {
            if fallback.len() < 3 && !fallback.contains(&def) {
                fallback.push(def);
            }
        }
        options.push(ItemBuildOption {
            item_ids: fallback,
            item_icon_urls: Vec::new(),
            play: 1,
            win: 1,
            pick_rate: 100.0,
        });
    }

    let total_play: i64 = options.iter().map(|o| o.play).sum();
    if total_play > 0 {
        for opt in &mut options {
            opt.pick_rate = ((opt.play as f64) / (total_play as f64)) * 100.0;
        }
    }

    options.sort_by_key(|opt| std::cmp::Reverse(opt.play));
    options.truncate(3);
    options
}

#[cfg(test)]
#[path = "core_final_options_tests.rs"]
mod tests;
