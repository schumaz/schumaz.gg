use super::super::models::{ItemBlock, ItemBlockKind, ItemStatEntry};
use super::classification::*;
use super::starters_boots::{select_starter_items, select_top_boots};
use std::collections::{HashMap, HashSet};

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

#[derive(Debug, Clone)]
struct ItemAggregate {
    id: i32,
    play: i64,
    first_seen_index: usize,
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
    let starter_ids = select_starter_items(&starter_entries, damage_type, position);

    // Detect confirmed Tear legendary upgrade from payload entries (never inventing one from raw 3070)
    let all_entries_for_tear: Vec<ItemStatEntry> = candidate_entries
        .iter()
        .chain(fourth_entries.iter())
        .chain(fifth_entries.iter())
        .chain(sixth_entries.iter())
        .cloned()
        .collect();
    let tear_upgrade = resolve_tear_to_completed_item(&all_entries_for_tear);

    // Future-proof contextual sets directly from OP.GG structures:
    let known_boots: HashSet<i32> = boot_entries
        .iter()
        .flat_map(|e| e.ids.iter().copied())
        .collect();

    let is_boot = |id: i32| -> bool { known_boots.contains(&id) || is_boot_item_dynamic(id) };
    let is_comp = |id: i32| -> bool { is_component_item_dynamic(id) };

    let known_core_or_depth_items: HashSet<i32> = candidate_entries
        .iter()
        .filter(|e| {
            e.ids
                .iter()
                .filter(|&&id| !is_boot(id) && !is_starter_item(id) && !is_comp(id))
                .count()
                >= 2
        })
        .flat_map(|e| resolve_tear_item_ids(&e.ids, tear_upgrade))
        .chain(
            fourth_entries
                .iter()
                .chain(fifth_entries.iter())
                .chain(sixth_entries.iter())
                .flat_map(|e| e.ids.iter().copied())
                .filter(|&id| !is_tear_component(id))
                .map(normalize_tear_item_dynamic),
        )
        .collect();

    // Only treat unknown items in `starter_entries` as starters if they are NOT boots,
    // NOT components, NOT completed DataDragon items, and NOT core/depth items.
    let known_starters: HashSet<i32> = starter_entries
        .iter()
        .flat_map(|e| e.ids.iter().copied())
        .filter(|&id| {
            let norm = normalize_tear_item_dynamic(id);
            !is_boot(id)
                && !is_comp(id)
                && !is_completed_item_dynamic(id)
                && !known_core_or_depth_items.contains(&norm)
        })
        .collect();

    let is_starter = |id: i32| -> bool { known_starters.contains(&id) || is_starter_item(id) };

    // 2. 3 Botas Mais Feitas: Always ranked by absolute games played (play count descending)
    let boots_list = select_top_boots(
        &boot_entries,
        &candidate_entries,
        damage_type,
        &is_boot,
        &is_starter,
        &is_comp,
    );

    // 3. Aggregate all non-boot, non-starter candidate items with play counts
    let mut item_map: HashMap<i32, (i64, usize)> = HashMap::new();
    let mut item_seen_counter = 0;
    for entry in candidate_entries
        .iter()
        .chain(fourth_entries.iter())
        .chain(fifth_entries.iter())
        .chain(sixth_entries.iter())
    {
        for norm_id in resolve_tear_item_ids(&entry.ids, tear_upgrade) {
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
    let mut clean_core: Vec<i32> = Vec::new();
    let mut core_combos: Vec<&ItemStatEntry> = candidate_entries
        .iter()
        .filter(|e| {
            let non_boot_count = e
                .ids
                .iter()
                .filter(|&&id| !is_boot(id) && !is_starter(id) && !is_comp(id))
                .count();
            non_boot_count >= 2
        })
        .collect();

    if !core_combos.is_empty() {
        core_combos.sort_by_key(|a| std::cmp::Reverse(a.play));
        let best_combo = core_combos[0];
        for norm_id in resolve_tear_item_ids(&best_combo.ids, tear_upgrade) {
            if clean_core.len() < 3
                && !is_boot(norm_id)
                && !is_starter(norm_id)
                && !is_comp(norm_id)
                && !clean_core.contains(&norm_id)
            {
                clean_core.push(norm_id);
            }
        }
    }

    for cand in &all_candidates {
        let norm_id = normalize_tear_item_dynamic(cand.id);
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
    let mut final_build: Vec<i32> = clean_core.clone();

    let pick_top_from_depth_list =
        |entries: &[ItemStatEntry], current_build: &[i32]| -> Option<i32> {
            let mut sorted: Vec<&ItemStatEntry> = entries.iter().collect();
            sorted.sort_by_key(|e| std::cmp::Reverse(e.play));
            for entry in sorted {
                for &id in &entry.ids {
                    let norm_id = normalize_tear_item_dynamic(id);
                    if !current_build.contains(&norm_id)
                        && !is_comp(norm_id)
                        && !is_starter(norm_id)
                        && !is_boot(norm_id)
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

    for cand in &all_candidates {
        if final_build.len() < 6
            && !final_build.contains(&cand.id)
            && !is_comp(cand.id)
            && !is_starter(cand.id)
            && !is_boot(cand.id)
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
    let mut situational_candidates: Vec<&ItemAggregate> = all_candidates
        .iter()
        .filter(|cand| {
            let norm_id = normalize_tear_item_dynamic(cand.id);
            !clean_core.contains(&norm_id)
                && !final_build.contains(&norm_id)
                && !is_comp(norm_id)
                && !is_starter(norm_id)
                && !is_boot(norm_id)
        })
        .collect();
    situational_candidates.sort_by(|a, b| {
        b.play
            .cmp(&a.play)
            .then_with(|| a.first_seen_index.cmp(&b.first_seen_index))
    });

    let mut situational_items: Vec<i32> = Vec::new();
    for c in situational_candidates {
        let norm_id = normalize_tear_item_dynamic(c.id);
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
            kind: ItemBlockKind::Starter,
            title: "Itens Iniciais".to_string(),
            item_ids: starter_ids,
            item_icon_urls: vec![],
        },
        ItemBlock {
            kind: ItemBlockKind::Core,
            title: "Primeiros 3 Itens".to_string(),
            item_ids: clean_core,
            item_icon_urls: vec![],
        },
        ItemBlock {
            kind: ItemBlockKind::Boots,
            title: "3 Botas Mais Feitas".to_string(),
            item_ids: boots_list,
            item_icon_urls: vec![],
        },
        ItemBlock {
            kind: ItemBlockKind::Final,
            title: "Build Final".to_string(),
            item_ids: final_build,
            item_icon_urls: vec![],
        },
        ItemBlock {
            kind: ItemBlockKind::Situational,
            title: "Itens Situacionais / Úteis".to_string(),
            item_ids: situational_items,
            item_icon_urls: vec![],
        },
    ]
}

#[cfg(test)]
#[path = "blocks_boots_tests.rs"]
mod boots_tests;
#[cfg(test)]
#[path = "blocks_starter_tests.rs"]
mod starter_tests;
#[cfg(test)]
#[path = "blocks_tests.rs"]
mod tests;
