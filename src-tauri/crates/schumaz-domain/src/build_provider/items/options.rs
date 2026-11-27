use super::super::models::{ItemClassificationCache, ItemStatEntry, UnifiedBuildOption};
use super::affinity::{compute_item_affinity_score_with_cls, extract_core_archetype_tags_with_cls};
use super::classification::{
    is_boot_in, is_component_in, is_starter_in, normalize_tear_item_in,
    snapshot_dynamic_item_classifications,
};
use super::core_final_options::extract_top_core_build_options_with_cls;
use super::tear::resolve_tear_to_completed_item_in;
use std::collections::HashMap;

fn collect_compatible_candidates<'a>(
    cls: &ItemClassificationCache,
    entries: impl Iterator<Item = &'a ItemStatEntry>,
    excluded: &[i32],
    core_tags: &[&str],
    core_is_pure_ad: bool,
    core_is_pure_ap: bool,
) -> Vec<i32> {
    let mut play_by_item: HashMap<i32, i64> = HashMap::new();
    let mut order_seen: HashMap<i32, usize> = HashMap::new();
    let mut idx = 0usize;

    for entry in entries {
        for &id in &entry.ids {
            let norm_id = normalize_tear_item_in(cls, id);
            if !excluded.contains(&norm_id)
                && !is_boot_in(cls, norm_id)
                && !is_starter_in(cls, norm_id)
                && !is_component_in(cls, norm_id)
            {
                let affinity = compute_item_affinity_score_with_cls(
                    cls,
                    norm_id,
                    core_tags,
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
    candidates.into_iter().map(|(id, _, _)| id).collect()
}

/// Builds a cohesive 6-item final build for `core_items` using `cls` by reference (zero locks).
pub fn build_cohesive_final_build_for_core_with_cls(
    cls: &ItemClassificationCache,
    core_items: &[i32],
    fourth_entries: &[ItemStatEntry],
    fifth_entries: &[ItemStatEntry],
    sixth_entries: &[ItemStatEntry],
    candidate_entries: &[ItemStatEntry],
    _damage_type: &str,
) -> Vec<i32> {
    let mut build_6 = core_items.to_vec();
    let (core_tags, _core_has_ap, _core_has_ad, core_is_pure_ad, core_is_pure_ap) =
        extract_core_archetype_tags_with_cls(cls, core_items);

    for depth_entries in [fourth_entries, fifth_entries, sixth_entries] {
        if let Some(best) = collect_compatible_candidates(
            cls,
            depth_entries.iter(),
            &build_6,
            &core_tags,
            core_is_pure_ad,
            core_is_pure_ap,
        )
        .into_iter()
        .next()
        {
            build_6.push(best);
        }
    }

    if build_6.len() < 6 {
        let remaining = fourth_entries
            .iter()
            .chain(fifth_entries.iter())
            .chain(sixth_entries.iter())
            .chain(candidate_entries.iter());
        for id in collect_compatible_candidates(
            cls,
            remaining,
            &build_6,
            &core_tags,
            core_is_pure_ad,
            core_is_pure_ap,
        ) {
            if build_6.len() >= 6 {
                break;
            }
            if !build_6.contains(&id) {
                build_6.push(id);
            }
        }
    }

    build_6
}

pub fn build_cohesive_final_build_for_core(
    core_items: &[i32],
    fourth_entries: &[ItemStatEntry],
    fifth_entries: &[ItemStatEntry],
    sixth_entries: &[ItemStatEntry],
    candidate_entries: &[ItemStatEntry],
    damage_type: &str,
) -> Vec<i32> {
    let cls = snapshot_dynamic_item_classifications();
    build_cohesive_final_build_for_core_with_cls(
        &cls,
        core_items,
        fourth_entries,
        fifth_entries,
        sixth_entries,
        candidate_entries,
        damage_type,
    )
}

/// Builds up to 6 coherent situational / useful items for a specific build option using `cls` by reference.
#[allow(clippy::too_many_arguments)]
pub fn build_situational_items_for_option_with_cls(
    cls: &ItemClassificationCache,
    core_items: &[i32],
    final_items: &[i32],
    fourth_entries: &[ItemStatEntry],
    fifth_entries: &[ItemStatEntry],
    sixth_entries: &[ItemStatEntry],
    candidate_entries: &[ItemStatEntry],
    extra_entries: &[ItemStatEntry],
    general_situational: &[i32],
    _damage_type: &str,
) -> Vec<i32> {
    let (core_tags, _core_has_ap, _core_has_ad, core_is_pure_ad, core_is_pure_ap) =
        extract_core_archetype_tags_with_cls(cls, core_items);

    let all_entries = fourth_entries
        .iter()
        .chain(fifth_entries.iter())
        .chain(sixth_entries.iter())
        .chain(candidate_entries.iter())
        .chain(extra_entries.iter());

    let mut play_by_item: HashMap<i32, i64> = HashMap::new();
    let mut first_seen: HashMap<i32, usize> = HashMap::new();
    let mut idx = 0;

    let mut record = |id: i32, play: i64, accumulate: bool| {
        let norm_id = normalize_tear_item_in(cls, id);
        if !core_items.contains(&norm_id)
            && !final_items.contains(&norm_id)
            && !is_boot_in(cls, norm_id)
            && !is_starter_in(cls, norm_id)
            && !is_component_in(cls, norm_id)
        {
            if accumulate {
                *play_by_item.entry(norm_id).or_insert(0) += play;
            } else {
                play_by_item.entry(norm_id).or_insert(play);
            }
            first_seen.entry(norm_id).or_insert(idx);
            idx += 1;
        }
    };

    for entry in all_entries {
        for &id in &entry.ids {
            record(id, entry.play.max(1), true);
        }
    }
    for &id in general_situational {
        record(id, 1, false);
    }

    let mut candidates: Vec<(i32, i64, usize)> = Vec::new();
    for (id, play) in play_by_item {
        let affinity = compute_item_affinity_score_with_cls(
            cls,
            id,
            &core_tags,
            core_is_pure_ad,
            core_is_pure_ap,
        );
        if affinity > -500_000 {
            let seen = *first_seen.get(&id).unwrap_or(&0);
            candidates.push((id, play, seen));
        }
    }

    candidates.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.2.cmp(&b.2)));

    let mut situational: Vec<i32> = candidates.into_iter().map(|(id, _, _)| id).collect();
    situational.truncate(6);
    situational
}

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
    let cls = snapshot_dynamic_item_classifications();
    build_situational_items_for_option_with_cls(
        &cls,
        core_items,
        final_items,
        fourth_entries,
        fifth_entries,
        sixth_entries,
        candidate_entries,
        extra_entries,
        general_situational,
        damage_type,
    )
}

/// Builds up to 3 unified build options using `cls` by reference (zero global lock overhead).
#[allow(clippy::too_many_arguments)]
pub fn build_unified_build_options_with_cls(
    cls: &ItemClassificationCache,
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
    let all_for_tear: Vec<ItemStatEntry> = core_entries
        .iter()
        .chain(fourth_entries.iter())
        .chain(fifth_entries.iter())
        .chain(sixth_entries.iter())
        .chain(candidate_entries.iter())
        .chain(extra_entries.iter())
        .cloned()
        .collect();
    let tear_upgrade = resolve_tear_to_completed_item_in(cls, &all_for_tear);

    let cand_ids: Vec<i32> = candidate_entries
        .iter()
        .chain(fourth_entries.iter())
        .chain(fifth_entries.iter())
        .chain(sixth_entries.iter())
        .chain(extra_entries.iter())
        .flat_map(|e| e.ids.iter().copied())
        .collect();
    let core_opts = extract_top_core_build_options_with_cls(
        cls,
        core_entries,
        &cand_ids,
        damage_type,
        tear_upgrade,
    );

    let mut unified_options = Vec::new();
    for (idx, core_opt) in core_opts.into_iter().enumerate() {
        let final_6 = build_cohesive_final_build_for_core_with_cls(
            cls,
            &core_opt.item_ids,
            fourth_entries,
            fifth_entries,
            sixth_entries,
            candidate_entries,
            damage_type,
        );

        let situational_items = build_situational_items_for_option_with_cls(
            cls, &core_opt.item_ids, &final_6, fourth_entries, fifth_entries,
            sixth_entries, candidate_entries, extra_entries, general_situational, damage_type,
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
    position: &str,
) -> Vec<UnifiedBuildOption> {
    let cls = snapshot_dynamic_item_classifications();
    build_unified_build_options_with_cls(
        &cls, core_entries, fourth_entries, fifth_entries, sixth_entries,
        candidate_entries, extra_entries, general_situational, damage_type, position,
    )
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
