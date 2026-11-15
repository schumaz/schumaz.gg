use super::super::models::{ItemStatEntry, UnifiedBuildOption};
use super::affinity::*;
use super::classification::*;
use super::core_final_options::extract_top_core_build_options;
use std::collections::HashMap;

pub fn build_cohesive_final_build_for_core(
    core_items: &[i32],
    fourth_entries: &[ItemStatEntry],
    fifth_entries: &[ItemStatEntry],
    sixth_entries: &[ItemStatEntry],
    candidate_entries: &[ItemStatEntry],
    damage_type: &str,
) -> Vec<i32> {
    let mut build_6 = core_items.to_vec();
    let (core_tags, _core_has_ap, _core_has_ad, core_is_pure_ad, core_is_pure_ap) =
        extract_core_archetype_tags(core_items);

    let all_for_tear: Vec<ItemStatEntry> = fourth_entries
        .iter()
        .chain(fifth_entries.iter())
        .chain(sixth_entries.iter())
        .chain(candidate_entries.iter())
        .cloned()
        .collect();
    let tear_upgrade = resolve_tear_to_completed_item(&all_for_tear, "", damage_type);

    let pick_best_item_for_slot =
        |entries: &[ItemStatEntry], current_build: &[i32]| -> Option<i32> {
            let mut play_by_item: HashMap<i32, i64> = HashMap::new();
            let mut order_seen: HashMap<i32, usize> = HashMap::new();
            let mut idx = 0;
            for entry in entries {
                for &id in &entry.ids {
                    let norm_id = if id == 3070 {
                        tear_upgrade
                    } else {
                        normalize_tear_item_dynamic(id)
                    };
                    if !current_build.contains(&norm_id)
                        && !is_boot_item_dynamic(norm_id)
                        && !is_starter_item(norm_id)
                        && !is_component_item_dynamic(norm_id)
                    {
                        let affinity = compute_item_affinity_score(
                            norm_id,
                            &core_tags,
                            core_is_pure_ad,
                            core_is_pure_ap,
                        );
                        if affinity > -500_000 {
                            *play_by_item.entry(norm_id).or_insert(0) += entry.play.max(1);
                            order_seen.entry(norm_id).or_insert(idx);
                            idx += 1;
                        }
                    }
                }
            }
            let mut candidates: Vec<(i32, i64, usize)> = play_by_item
                .into_iter()
                .map(|(id, play)| (id, play, *order_seen.get(&id).unwrap_or(&0)))
                .collect();
            candidates.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.2.cmp(&b.2)));
            candidates.first().map(|&(id, _, _)| id)
        };

    // Sequential slot selection:
    // Slot 4: #1 most played compatible item from fourth_entries
    if let Some(item_4) = pick_best_item_for_slot(fourth_entries, &build_6) {
        build_6.push(item_4);
    }
    // Slot 5: #1 most played compatible item from fifth_entries
    if let Some(item_5) = pick_best_item_for_slot(fifth_entries, &build_6) {
        build_6.push(item_5);
    }
    // Slot 6: #1 most played compatible item from sixth_entries
    if let Some(item_6) = pick_best_item_for_slot(sixth_entries, &build_6) {
        build_6.push(item_6);
    }

    // Dynamic late-game completion fallback if any depth lists were exhausted or sparse
    if build_6.len() < 6 {
        let all_remaining_entries = fourth_entries
            .iter()
            .chain(fifth_entries.iter())
            .chain(sixth_entries.iter())
            .chain(candidate_entries.iter());

        let mut fallback_play: HashMap<i32, i64> = HashMap::new();
        let mut fallback_seen: HashMap<i32, usize> = HashMap::new();
        let mut f_idx = 0;
        for entry in all_remaining_entries {
            for &id in &entry.ids {
                let norm_id = if id == 3070 {
                    tear_upgrade
                } else {
                    normalize_tear_item_dynamic(id)
                };
                if !build_6.contains(&norm_id)
                    && !is_boot_item_dynamic(norm_id)
                    && !is_starter_item(norm_id)
                    && !is_component_item_dynamic(norm_id)
                {
                    let affinity = compute_item_affinity_score(
                        norm_id,
                        &core_tags,
                        core_is_pure_ad,
                        core_is_pure_ap,
                    );
                    if affinity > -500_000 {
                        *fallback_play.entry(norm_id).or_insert(0) += entry.play.max(1);
                        fallback_seen.entry(norm_id).or_insert(f_idx);
                        f_idx += 1;
                    }
                }
            }
        }
        let mut fallback_cands: Vec<(i32, i64, usize)> = fallback_play
            .into_iter()
            .map(|(id, play)| (id, play, *fallback_seen.get(&id).unwrap_or(&0)))
            .collect();
        fallback_cands.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.2.cmp(&b.2)));
        for (id, _, _) in fallback_cands {
            if build_6.len() < 6 && !build_6.contains(&id) {
                build_6.push(id);
            }
        }
    }

    build_6
}

/// Builds up to 6 coherent situational / useful items for a specific build option,
/// strictly excluding core items, final build items, boots, starters, and components,
/// while filtering out conflicting archetypes.
#[allow(clippy::too_many_arguments)]
pub fn build_situational_items_for_option(
    core_items: &[i32],
    final_items: &[i32],
    fourth_entries: &[ItemStatEntry],
    fifth_entries: &[ItemStatEntry],
    sixth_entries: &[ItemStatEntry],
    candidate_entries: &[ItemStatEntry],
    extra_entries: &[ItemStatEntry],
    general_situational: &[i32],
    damage_type: &str,
) -> Vec<i32> {
    let (core_tags, _core_has_ap, _core_has_ad, core_is_pure_ad, core_is_pure_ap) =
        extract_core_archetype_tags(core_items);

    let all_for_tear: Vec<ItemStatEntry> = fourth_entries
        .iter()
        .chain(fifth_entries.iter())
        .chain(sixth_entries.iter())
        .chain(candidate_entries.iter())
        .chain(extra_entries.iter())
        .cloned()
        .collect();
    let tear_upgrade = resolve_tear_to_completed_item(&all_for_tear, "", damage_type);

    let all_entries = fourth_entries
        .iter()
        .chain(fifth_entries.iter())
        .chain(sixth_entries.iter())
        .chain(candidate_entries.iter())
        .chain(extra_entries.iter());

    let mut play_by_item: HashMap<i32, i64> = HashMap::new();
    let mut first_seen: HashMap<i32, usize> = HashMap::new();
    let mut idx = 0;

    for entry in all_entries {
        for &id in &entry.ids {
            let norm_id = if id == 3070 {
                tear_upgrade
            } else {
                normalize_tear_item_dynamic(id)
            };
            if !core_items.contains(&norm_id)
                && !final_items.contains(&norm_id)
                && !is_boot_item_dynamic(norm_id)
                && !is_starter_item(norm_id)
                && !is_component_item_dynamic(norm_id)
            {
                *play_by_item.entry(norm_id).or_insert(0) += entry.play.max(1);
                first_seen.entry(norm_id).or_insert(idx);
                idx += 1;
            }
        }
    }

    // Include items from general_situational as candidate pool if not already present
    for &id in general_situational {
        let norm_id = if id == 3070 {
            tear_upgrade
        } else {
            normalize_tear_item_dynamic(id)
        };
        if !core_items.contains(&norm_id)
            && !final_items.contains(&norm_id)
            && !is_boot_item_dynamic(norm_id)
            && !is_starter_item(norm_id)
            && !is_component_item_dynamic(norm_id)
        {
            play_by_item.entry(norm_id).or_insert(1);
            first_seen.entry(norm_id).or_insert(idx);
            idx += 1;
        }
    }

    let mut candidates: Vec<(i32, i64, usize)> = Vec::new();
    for (id, play) in play_by_item {
        let affinity =
            compute_item_affinity_score(id, &core_tags, core_is_pure_ad, core_is_pure_ap);
        if affinity > -500_000 {
            let seen = *first_seen.get(&id).unwrap_or(&0);
            candidates.push((id, play, seen));
        }
    }

    // Sort strictly by play descending, tie-break by order seen
    candidates.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.2.cmp(&b.2)));

    let mut situational: Vec<i32> = candidates.into_iter().map(|(id, _, _)| id).collect();
    situational.truncate(6);
    situational
}

/// Builds up to 3 unified build options (each with 3 core items, 6 matching final items, and coherent situational items).
#[allow(clippy::too_many_arguments)]
pub fn build_unified_build_options(
    core_entries: &[ItemStatEntry],
    fourth_entries: &[ItemStatEntry],
    fifth_entries: &[ItemStatEntry],
    sixth_entries: &[ItemStatEntry],
    candidate_entries: &[ItemStatEntry],
    extra_entries: &[ItemStatEntry],
    general_situational: &[i32],
    damage_type: &str,
    _position: &str,
) -> Vec<UnifiedBuildOption> {
    let cand_ids: Vec<i32> = candidate_entries
        .iter()
        .flat_map(|e| e.ids.clone())
        .collect();
    let core_opts = extract_top_core_build_options(core_entries, &cand_ids, damage_type);

    let mut unified_options = Vec::new();
    for (idx, core_opt) in core_opts.into_iter().enumerate() {
        let final_6 = build_cohesive_final_build_for_core(
            &core_opt.item_ids,
            fourth_entries,
            fifth_entries,
            sixth_entries,
            candidate_entries,
            damage_type,
        );

        let situational_items = build_situational_items_for_option(
            &core_opt.item_ids,
            &final_6,
            fourth_entries,
            fifth_entries,
            sixth_entries,
            candidate_entries,
            extra_entries,
            general_situational,
            damage_type,
        );

        let name = if idx == 0 {
            "Build 1 (Mais Feita)".to_string()
        } else {
            format!("Build {}", idx + 1)
        };

        unified_options.push(UnifiedBuildOption {
            name,
            core_item_ids: core_opt.item_ids,
            core_item_icon_urls: Vec::new(),
            final_item_ids: final_6,
            final_item_icon_urls: Vec::new(),
            situational_item_ids: situational_items,
            situational_item_icon_urls: Vec::new(),
            play: core_opt.play,
            win: core_opt.win,
            pick_rate: core_opt.pick_rate,
        });
    }

    unified_options
}

#[cfg(test)]
#[path = "options_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "options_synergy_tests.rs"]
mod synergy_tests;

#[cfg(test)]
#[path = "options_situational_tests.rs"]
mod situational_tests;
