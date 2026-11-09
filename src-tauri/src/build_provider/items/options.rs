use super::super::models::{ItemBuildOption, ItemStatEntry, UnifiedBuildOption};
use super::affinity::*;
use super::classification::*;
use std::collections::HashMap;

pub fn build_cohesive_final_build_for_core(
    core_items: &[i32],
    fourth_entries: &[ItemStatEntry],
    fifth_entries: &[ItemStatEntry],
    sixth_entries: &[ItemStatEntry],
    candidate_entries: &[ItemStatEntry],
    _damage_type: &str,
) -> Vec<i32> {
    let mut build_6 = core_items.to_vec();
    let (core_tags, _core_has_ap, _core_has_ad, core_is_pure_ad, core_is_pure_ap) =
        extract_core_archetype_tags(core_items);

    let pick_best_item_for_slot = |entries: &[ItemStatEntry], current_build: &[i32]| -> Option<i32> {
        let mut play_by_item: HashMap<i32, i64> = HashMap::new();
        let mut order_seen: HashMap<i32, usize> = HashMap::new();
        let mut idx = 0;
        for entry in entries {
            for &id in &entry.ids {
                let norm_id = normalize_tear_item_dynamic(id);
                if !current_build.contains(&norm_id)
                    && !is_boot_item_dynamic(norm_id)
                    && !is_starter_item(norm_id)
                    && !is_component_item_dynamic(norm_id)
                {
                    let affinity = compute_item_affinity_score(norm_id, &core_tags, core_is_pure_ad, core_is_pure_ap);
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
                let norm_id = normalize_tear_item_dynamic(id);
                if !build_6.contains(&norm_id)
                    && !is_boot_item_dynamic(norm_id)
                    && !is_starter_item(norm_id)
                    && !is_component_item_dynamic(norm_id)
                {
                    let affinity = compute_item_affinity_score(norm_id, &core_tags, core_is_pure_ad, core_is_pure_ap);
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
    _damage_type: &str,
) -> Vec<i32> {
    let (core_tags, _core_has_ap, _core_has_ad, core_is_pure_ad, core_is_pure_ap) =
        extract_core_archetype_tags(core_items);

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
            let norm_id = normalize_tear_item_dynamic(id);
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
        let norm_id = normalize_tear_item_dynamic(id);
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
        let affinity = compute_item_affinity_score(id, &core_tags, core_is_pure_ad, core_is_pure_ap);
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
    let cand_ids: Vec<i32> = candidate_entries.iter().flat_map(|e| e.ids.clone()).collect();
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
        // If combo had only 2 items, fill 3rd from top candidates
        for &cand in all_candidates {
            let norm_cand = if cand == 3070 {
                tear_upgrade
            } else {
                normalize_tear_item_dynamic(cand)
            };
            if items.len() < 3 && !items.contains(&norm_cand) {
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
        if !pool.contains(&norm_c) {
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
mod tests {
    use super::*;
    use crate::build_provider::*;
    use super::super::classification::TEST_CLASSIFICATION_MUTEX;
    use crate::build_provider::models::ItemClassificationCache;

    #[test]
    fn test_shaco_ad_picks_ldr_ga_youmuu_sequentially_per_slot() {
        let _lock = TEST_CLASSIFICATION_MUTEX.lock().unwrap();
        let mut cache = ItemClassificationCache::default();
        cache.item_tags.insert(6697, vec!["Damage".to_string(), "ArmorPenetration".to_string()]); // Cyclosword
        cache.item_tags.insert(6676, vec!["Damage".to_string(), "CriticalStrike".to_string(), "ArmorPenetration".to_string()]); // Collector
        cache.item_tags.insert(3031, vec!["Damage".to_string(), "CriticalStrike".to_string()]); // Infinity Edge
        cache.item_tags.insert(3036, vec!["Damage".to_string(), "ArmorPenetration".to_string()]); // Lord Dominik's Regards
        cache.item_tags.insert(3026, vec!["Damage".to_string(), "Armor".to_string()]); // Guardian Angel
        cache.item_tags.insert(3142, vec!["Damage".to_string(), "ArmorPenetration".to_string(), "NonbootsMovement".to_string()]); // Youmuu's Ghostblade
        cache.item_tags.insert(3179, vec!["Damage".to_string(), "ArmorPenetration".to_string(), "AbilityHaste".to_string()]); // Umbral Glaive
        cache.item_tags.insert(3089, vec!["SpellDamage".to_string()]); // Rabadon
        cache.item_tags.insert(3157, vec!["SpellDamage".to_string(), "Armor".to_string()]); // Zhonya
        cache.completed.extend(vec![6697, 6676, 3031, 3036, 3026, 3142, 3179, 3089, 3157]);
        set_dynamic_item_classifications(cache);

        let core = vec![6697, 6676, 3031];
        let fourth = vec![
            ItemStatEntry { ids: vec![3036], play: 2790, win: 1578, pick_rate: 56.56 }, // LDR -> 1st for 4th slot
            ItemStatEntry { ids: vec![3031], play: 2266, win: 1304, pick_rate: 57.55 }, // IE (already in core)
        ];
        let fifth = vec![
            ItemStatEntry { ids: vec![3026], play: 567, win: 359, pick_rate: 63.32 }, // GA -> 1st for 5th slot
            ItemStatEntry { ids: vec![3031], play: 476, win: 288, pick_rate: 60.50 }, // IE (already in core)
            ItemStatEntry { ids: vec![3089], play: 370, win: 244, pick_rate: 65.95 }, // Rabadon (blocked: AP offensive!)
            ItemStatEntry { ids: vec![3036], play: 314, win: 205, pick_rate: 65.29 }, // LDR (already in 4th slot)
            ItemStatEntry { ids: vec![3157], play: 260, win: 142, pick_rate: 54.62 }, // Zhonya (blocked: AP offensive!)
        ];
        let sixth = vec![
            ItemStatEntry { ids: vec![3142], play: 110, win: 71, pick_rate: 64.55 }, // Youmuu -> 1st for 6th slot
            ItemStatEntry { ids: vec![3026], play: 44, win: 25, pick_rate: 56.82 }, // GA (already in 5th slot)
        ];
        let candidates = vec![
            ItemStatEntry { ids: vec![3179], play: 5000, win: 2500, pick_rate: 0.5 }, // Umbral Glaive in candidate pool
        ];

        let final_6 = build_cohesive_final_build_for_core(
            &core,
            &fourth,
            &fifth,
            &sixth,
            &candidates,
            "AD",
        );

        // Strict sequential slot selection:
        // Slot 4: 3036 (LDR)
        // Slot 5: 3026 (GA)
        // Slot 6: 3142 (Youmuu's Ghostblade)
        // Umbral Glaive (3179) must NOT displace slot 6!
        // Rabadon (3089) and Zhonya (3157) must NOT be picked!
        assert_eq!(final_6, vec![6697, 6676, 3031, 3036, 3026, 3142]);
    }

    #[test]
    fn test_vayne_adc_accepts_jaksho_and_randuins_universal_defense() {
        let _lock = TEST_CLASSIFICATION_MUTEX.lock().unwrap();
        let mut cache = ItemClassificationCache::default();
        cache.item_tags.insert(6672, vec!["Damage".to_string(), "AttackSpeed".to_string()]); // Kraken
        cache.item_tags.insert(3124, vec!["Damage".to_string(), "AttackSpeed".to_string(), "OnHit".to_string()]); // Guinsoo
        cache.item_tags.insert(3302, vec!["Damage".to_string(), "AttackSpeed".to_string(), "ArmorPenetration".to_string(), "MagicPenetration".to_string()]); // Terminus
        cache.item_tags.insert(3153, vec!["Damage".to_string(), "AttackSpeed".to_string(), "LifeSteal".to_string()]); // BotRK
        cache.item_tags.insert(6665, vec!["Armor".to_string(), "SpellBlock".to_string(), "Health".to_string(), "AbilityHaste".to_string()]); // Jak'Sho
        cache.item_tags.insert(3143, vec!["Armor".to_string(), "Health".to_string()]); // Randuin's Omen
        cache.item_tags.insert(3089, vec!["SpellDamage".to_string()]); // Rabadon (offensive AP)
        cache.completed.extend(vec![6672, 3124, 3302, 3153, 6665, 3143, 3089]);
        set_dynamic_item_classifications(cache);

        let core = vec![6672, 3124, 3302];
        let fourth = vec![
            ItemStatEntry { ids: vec![3153], play: 3000, win: 1600, pick_rate: 0.6 },
        ];
        let fifth = vec![
            ItemStatEntry { ids: vec![6665], play: 2500, win: 1350, pick_rate: 0.5 }, // Jak'Sho
        ];
        let sixth = vec![
            ItemStatEntry { ids: vec![3089], play: 5000, win: 2500, pick_rate: 0.7 }, // Rabadon (blocked!)
            ItemStatEntry { ids: vec![3143], play: 1200, win: 650, pick_rate: 0.25 }, // Randuin
        ];

        let final_6 = build_cohesive_final_build_for_core(
            &core,
            &fourth,
            &fifth,
            &sixth,
            &[],
            "AD",
        );

        assert_eq!(final_6, vec![6672, 3124, 3302, 3153, 6665, 3143]);
        assert!(!final_6.contains(&3089), "Rabadon must be blocked from AD build");
    }

    #[test]
    fn test_ezreal_mana_synergy_recognizes_frozen_heart() {
        let _lock = TEST_CLASSIFICATION_MUTEX.lock().unwrap();
        let mut cache = ItemClassificationCache::default();
        cache.item_tags.insert(3078, vec!["Damage".to_string(), "AttackSpeed".to_string(), "Health".to_string(), "AbilityHaste".to_string()]); // Triforce
        cache.item_tags.insert(3004, vec!["Damage".to_string(), "Mana".to_string(), "AbilityHaste".to_string()]); // Manamune
        cache.item_tags.insert(3036, vec!["Damage".to_string(), "ArmorPenetration".to_string()]); // LDR
        cache.item_tags.insert(3072, vec!["Damage".to_string(), "LifeSteal".to_string(), "CriticalStrike".to_string()]); // BT
        cache.item_tags.insert(3110, vec!["Armor".to_string(), "Mana".to_string(), "AbilityHaste".to_string()]); // Frozen Heart
        cache.item_tags.insert(3153, vec!["Damage".to_string(), "AttackSpeed".to_string(), "LifeSteal".to_string()]); // BotRK
        cache.completed.extend(vec![3078, 3004, 3036, 3072, 3110, 3153]);
        set_dynamic_item_classifications(cache);

        let core = vec![3078, 3004, 3036];
        let fourth = vec![
            ItemStatEntry { ids: vec![3072], play: 2000, win: 1100, pick_rate: 0.4 },
        ];
        let fifth = vec![
            ItemStatEntry { ids: vec![3110], play: 1500, win: 850, pick_rate: 0.3 }, // Frozen Heart
        ];
        let sixth = vec![
            ItemStatEntry { ids: vec![3153], play: 1000, win: 550, pick_rate: 0.2 },
        ];

        let final_6 = build_cohesive_final_build_for_core(
            &core,
            &fourth,
            &fifth,
            &sixth,
            &[],
            "AD",
        );

        assert_eq!(final_6, vec![3078, 3004, 3036, 3072, 3110, 3153]);
        assert!(final_6.contains(&3110), "Frozen Heart must be recognized via Mana synergy");
    }

    #[test]
    fn test_option_situational_excludes_core_final_boots_components() {
        let core = vec![3031, 3094, 3036];
        let final_items = vec![3031, 3094, 3036, 3026, 3072, 3139];
        let fourth = vec![ItemStatEntry { ids: vec![3026], play: 1000, win: 500, pick_rate: 0.1 }];
        let fifth = vec![ItemStatEntry { ids: vec![3072], play: 800, win: 400, pick_rate: 0.1 }];
        let sixth = vec![ItemStatEntry { ids: vec![3139], play: 600, win: 300, pick_rate: 0.1 }];
        let candidates = vec![
            ItemStatEntry { ids: vec![3006], play: 5000, win: 2500, pick_rate: 0.5 }, // Greaves (boot) -> exclude
            ItemStatEntry { ids: vec![1055], play: 4000, win: 2000, pick_rate: 0.4 }, // Doran's Blade (starter) -> exclude
            ItemStatEntry { ids: vec![1038], play: 3000, win: 1500, pick_rate: 0.3 }, // BF Sword (component) -> exclude
            ItemStatEntry { ids: vec![3046], play: 2500, win: 1200, pick_rate: 0.25 }, // Phantom Dancer -> VALID
            ItemStatEntry { ids: vec![3085], play: 2000, win: 1000, pick_rate: 0.2 }, // Runaan's Hurricane -> VALID
            ItemStatEntry { ids: vec![3078], play: 1500, win: 750, pick_rate: 0.15 }, // Trinity Force -> VALID
            ItemStatEntry { ids: vec![6676], play: 1200, win: 600, pick_rate: 0.12 }, // Collector -> VALID
            ItemStatEntry { ids: vec![3153], play: 1000, win: 500, pick_rate: 0.1 }, // Blade of Ruined King -> VALID
        ];

        let sit = build_situational_items_for_option(
            &core,
            &final_items,
            &fourth,
            &fifth,
            &sixth,
            &candidates,
            &[],
            &[],
            "AD",
        );

        // Core and final build must be completely excluded
        for id in &core {
            assert!(!sit.contains(id), "Core item {id} must not be in situational");
        }
        for id in &final_items {
            assert!(!sit.contains(id), "Final item {id} must not be in situational");
        }
        // Boots, starters, and components must be excluded
        assert!(!sit.contains(&3006), "Boot 3006 must not be in situational");
        assert!(!sit.contains(&1055), "Starter 1055 must not be in situational");
        assert!(!sit.contains(&1038), "Component 1038 must not be in situational");

        // Valid situational items must be present
        assert_eq!(sit, vec![3046, 3085, 3078, 6676, 3153]);
    }

    #[test]
    fn test_option_situational_filters_by_archetype() {
        let _lock = TEST_CLASSIFICATION_MUTEX.lock().unwrap();
        let mut cache = ItemClassificationCache::default();
        cache.item_tags.insert(6697, vec!["Damage".to_string(), "ArmorPenetration".to_string()]);
        cache.item_tags.insert(6676, vec!["Damage".to_string(), "CriticalStrike".to_string()]);
        cache.item_tags.insert(3031, vec!["Damage".to_string(), "CriticalStrike".to_string()]);
        cache.item_tags.insert(3036, vec!["Damage".to_string(), "ArmorPenetration".to_string()]);
        cache.item_tags.insert(3026, vec!["Damage".to_string(), "Armor".to_string()]);
        cache.item_tags.insert(3142, vec!["Damage".to_string(), "ArmorPenetration".to_string()]);
        cache.item_tags.insert(3046, vec!["Damage".to_string(), "CriticalStrike".to_string()]);
        cache.item_tags.insert(3078, vec!["Damage".to_string(), "AttackSpeed".to_string()]);
        cache.item_tags.insert(3153, vec!["Damage".to_string(), "LifeSteal".to_string()]);
        cache.item_tags.insert(3089, vec!["SpellDamage".to_string()]);
        cache.item_tags.insert(3157, vec!["SpellDamage".to_string()]);
        cache.completed.extend(vec![6697, 6676, 3031, 3036, 3026, 3142, 3046, 3078, 3153, 3089, 3157]);
        set_dynamic_item_classifications(cache);

        let ad_core = vec![6697, 6676, 3031];
        let ad_final = vec![6697, 6676, 3031, 3036, 3026, 3142];
        let candidates = vec![
            ItemStatEntry { ids: vec![3089], play: 10000, win: 5000, pick_rate: 0.5 },
            ItemStatEntry { ids: vec![3157], play: 8000, win: 4000, pick_rate: 0.4 },
            ItemStatEntry { ids: vec![3046], play: 3000, win: 1500, pick_rate: 0.3 },
            ItemStatEntry { ids: vec![3078], play: 2000, win: 1000, pick_rate: 0.2 },
            ItemStatEntry { ids: vec![3153], play: 1000, win: 500, pick_rate: 0.1 },
        ];

        let sit = build_situational_items_for_option(
            &ad_core,
            &ad_final,
            &[],
            &[],
            &[],
            &candidates,
            &[],
            &[],
            "AD",
        );

        assert!(!sit.contains(&3089), "AP item Rabadon must be excluded from AD build");
        assert!(!sit.contains(&3157), "AP item Zhonya must be excluded from AD build");
        assert!(sit.contains(&3046));
        assert!(sit.contains(&3078));
        assert!(sit.contains(&3153));
    }

    #[test]
    fn test_option_situational_hybrid_keeps_both() {
        let _lock = TEST_CLASSIFICATION_MUTEX.lock().unwrap();
        let mut cache = ItemClassificationCache::default();
        cache.item_tags.insert(3115, vec!["SpellDamage".to_string(), "AttackSpeed".to_string()]);
        cache.item_tags.insert(3124, vec!["Damage".to_string(), "SpellDamage".to_string()]);
        cache.item_tags.insert(3089, vec!["SpellDamage".to_string()]);
        cache.item_tags.insert(3046, vec!["Damage".to_string(), "CriticalStrike".to_string()]);
        cache.item_tags.insert(3157, vec!["SpellDamage".to_string()]);
        cache.item_tags.insert(3072, vec!["Damage".to_string(), "LifeSteal".to_string()]);
        cache.completed.extend(vec![3115, 3124, 3089, 3046, 3157, 3072]);
        set_dynamic_item_classifications(cache);

        let hybrid_core = vec![3115, 3124, 3089];
        let hybrid_final = vec![3115, 3124, 3089, 3157];
        let candidates = vec![
            ItemStatEntry { ids: vec![3046], play: 2000, win: 1000, pick_rate: 0.3 },
            ItemStatEntry { ids: vec![3072], play: 1500, win: 750, pick_rate: 0.2 },
        ];

        let sit = build_situational_items_for_option(
            &hybrid_core,
            &hybrid_final,
            &[],
            &[],
            &[],
            &candidates,
            &[],
            &[],
            "HYBRID",
        );

        assert!(sit.contains(&3046), "Hybrid build should keep AD item PD");
        assert!(sit.contains(&3072), "Hybrid build should keep AD item BT");
    }

    #[test]
    fn test_option_situational_min_5_sorted_by_play() {
        let core = vec![1, 2, 3];
        let final_items = vec![1, 2, 3, 4, 5, 6];
        let fourth = vec![
            ItemStatEntry { ids: vec![101], play: 100, win: 50, pick_rate: 0.1 },
            ItemStatEntry { ids: vec![102], play: 500, win: 250, pick_rate: 0.2 },
        ];
        let fifth = vec![
            ItemStatEntry { ids: vec![103], play: 300, win: 150, pick_rate: 0.15 },
        ];
        let sixth = vec![
            ItemStatEntry { ids: vec![104], play: 800, win: 400, pick_rate: 0.3 },
        ];
        let candidates = vec![
            ItemStatEntry { ids: vec![105], play: 200, win: 100, pick_rate: 0.1 },
            ItemStatEntry { ids: vec![106], play: 400, win: 200, pick_rate: 0.18 },
        ];

        let sit = build_situational_items_for_option(
            &core,
            &final_items,
            &fourth,
            &fifth,
            &sixth,
            &candidates,
            &[],
            &[],
            "AD",
        );

        // Should have 6 items sorted by play count descending:
        // 104 (800) > 102 (500) > 106 (400) > 103 (300) > 105 (200) > 101 (100)
        assert_eq!(sit, vec![104, 102, 106, 103, 105, 101]);
    }

    #[test]
    fn test_extract_top_core_and_final_build_options() {
        let core_entries = vec![
            ItemStatEntry {
                ids: vec![3118, 4645, 3089],
                play: 15000,
                win: 8000,
                pick_rate: 60.0,
            },
            ItemStatEntry {
                ids: vec![3118, 3157, 3089],
                play: 5000,
                win: 2600,
                pick_rate: 20.0,
            },
            ItemStatEntry {
                ids: vec![3118, 4645, 3135],
                play: 2000,
                win: 1050,
                pick_rate: 8.0,
            },
            ItemStatEntry {
                ids: vec![3118, 3165, 3089],
                play: 500,
                win: 250,
                pick_rate: 2.0,
            },
        ];
        let candidates = vec![3118, 4645, 3089, 3157, 3135, 3165, 4629, 3102];
        let core_opts = extract_top_core_build_options(&core_entries, &candidates, "AP");
        assert_eq!(core_opts.len(), 3);
        assert_eq!(core_opts[0].item_ids, vec![3118, 4645, 3089]);
        assert_eq!(core_opts[0].play, 15000);
        assert_eq!(core_opts[1].item_ids, vec![3118, 3157, 3089]);
        assert_eq!(core_opts[1].play, 5000);
        assert_eq!(core_opts[2].item_ids, vec![3118, 4645, 3135]);
        assert_eq!(core_opts[2].play, 2000);

        let fourth = vec![
            ItemStatEntry {
                ids: vec![3157],
                play: 10000,
                win: 5500,
                pick_rate: 40.0,
            },
            ItemStatEntry {
                ids: vec![3135],
                play: 3000,
                win: 1600,
                pick_rate: 15.0,
            },
        ];
        let fifth = vec![ItemStatEntry {
            ids: vec![3135],
            play: 8000,
            win: 4200,
            pick_rate: 35.0,
        }];
        let sixth = vec![
            ItemStatEntry {
                ids: vec![4629],
                play: 6000,
                win: 3100,
                pick_rate: 30.0,
            },
            ItemStatEntry {
                ids: vec![3102],
                play: 2000,
                win: 1000,
                pick_rate: 10.0,
            },
        ];

        let final_opts = build_3_final_build_options(
            &core_opts[0].item_ids,
            &fourth,
            &fifth,
            &sixth,
            &candidates,
            "AP",
        );
        assert_eq!(final_opts.len(), 3);
        for opt in &final_opts {
            assert_eq!(opt.item_ids.len(), 6);
            assert_eq!(&opt.item_ids[0..3], &[3118, 4645, 3089]);
        }
    }
}
