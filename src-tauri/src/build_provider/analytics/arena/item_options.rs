use super::items::{AggregatedItemStat, is_valid_arena_legendary};
use crate::build_provider::items::{
    compute_item_affinity_score, extract_core_archetype_tags, is_tear_completed_item,
    normalize_arena_item_id,
};
use crate::build_provider::models::{ItemStatEntry, UnifiedBuildOption};
use std::collections::{HashMap, HashSet};

const MAX_ARENA_BUILD_OPTIONS: usize = 3;
const MAX_ARENA_FINAL_ITEMS: usize = 6;
const MAX_ARENA_SITUATIONAL_ITEMS: usize = 6;

#[derive(Debug, Clone)]
struct CoreCandidate {
    display_ids: Vec<i32>,
    best_entry_play: i64,
    best_entry_win: i64,
    total_play: i64,
    total_win: i64,
}

fn clean_arena_core_ids(ids: &[i32], known_prismatics: &HashSet<i32>) -> Vec<i32> {
    let mut cleaned = Vec::with_capacity(3);
    for &raw_id in ids {
        let norm = normalize_arena_item_id(raw_id);
        if !is_valid_arena_legendary(norm, known_prismatics) {
            continue;
        }
        if !cleaned.contains(&norm) {
            cleaned.push(norm);
        }
        if cleaned.len() == 3 {
            break;
        }
    }
    cleaned
}

fn canonical_trio_key(ids: &[i32]) -> Vec<i32> {
    let mut key = ids.to_vec();
    key.sort_unstable();
    key
}

fn rank_compatible_legendaries(
    core_ids: &[i32],
    legendary_pool: &[AggregatedItemStat],
) -> Vec<i32> {
    let (core_tags, _, _, core_is_pure_ad, core_is_pure_ap) = extract_core_archetype_tags(core_ids);
    let core_has_tear = core_ids.iter().copied().any(is_tear_completed_item);

    let mut scored: Vec<(i32, i64, i64, i64)> = Vec::new();
    for item in legendary_pool {
        if core_ids.contains(&item.id) {
            continue;
        }
        if core_has_tear && is_tear_completed_item(item.id) {
            continue;
        }
        let affinity =
            compute_item_affinity_score(item.id, &core_tags, core_is_pure_ad, core_is_pure_ap);
        if affinity <= -500_000 {
            continue;
        }
        scored.push((item.id, item.play, affinity, item.win));
    }

    scored.sort_by(|a, b| {
        b.1.cmp(&a.1)
            .then_with(|| b.2.cmp(&a.2))
            .then_with(|| b.3.cmp(&a.3))
            .then_with(|| a.0.cmp(&b.0))
    });
    scored.into_iter().map(|(id, _, _, _)| id).collect()
}

fn upsert_candidate(
    map: &mut HashMap<Vec<i32>, CoreCandidate>,
    cleaned: Vec<i32>,
    play: i64,
    win: i64,
) {
    let key = canonical_trio_key(&cleaned);
    let slot = map.entry(key).or_insert_with(|| CoreCandidate {
        display_ids: cleaned.clone(),
        best_entry_play: -1,
        best_entry_win: -1,
        total_play: 0,
        total_win: 0,
    });
    slot.total_play = slot.total_play.saturating_add(play);
    slot.total_win = slot.total_win.saturating_add(win);
    if play > slot.best_entry_play || (play == slot.best_entry_play && win > slot.best_entry_win) {
        slot.best_entry_play = play;
        slot.best_entry_win = win;
        slot.display_ids = cleaned;
    }
}

/// Extracts up to 3 distinct Arena core trios, strictly deduplicating permuted trios
/// (e.g. `[A, B, C]` and `[A, C, B]`) while preserving the highest-play display order.
fn extract_arena_core_candidates(
    core_entries: &[ItemStatEntry],
    legendary_pool: &[AggregatedItemStat],
    known_prismatics: &HashSet<i32>,
) -> Vec<CoreCandidate> {
    let mut trio_map: HashMap<Vec<i32>, CoreCandidate> = HashMap::new();
    let mut pair_map: HashMap<Vec<i32>, CoreCandidate> = HashMap::new();

    for entry in core_entries {
        let cleaned = clean_arena_core_ids(&entry.ids, known_prismatics);
        let play = entry.play.max(0);
        let win = entry.win.max(0);
        if cleaned.len() == 3 {
            upsert_candidate(&mut trio_map, cleaned, play, win);
        } else if cleaned.len() == 2 {
            upsert_candidate(&mut pair_map, cleaned, play, win);
        }
    }

    let mut trios: Vec<CoreCandidate> = trio_map.into_values().collect();
    trios.sort_by(|a, b| {
        b.total_play
            .cmp(&a.total_play)
            .then_with(|| b.total_win.cmp(&a.total_win))
            .then_with(|| a.display_ids.cmp(&b.display_ids))
    });

    let mut seen_keys: HashSet<Vec<i32>> = trios
        .iter()
        .map(|c| canonical_trio_key(&c.display_ids))
        .collect();

    if trios.len() < MAX_ARENA_BUILD_OPTIONS {
        let mut pairs: Vec<CoreCandidate> = pair_map.into_values().collect();
        pairs.sort_by(|a, b| {
            b.total_play
                .cmp(&a.total_play)
                .then_with(|| b.total_win.cmp(&a.total_win))
        });
        for mut pair in pairs {
            if trios.len() >= MAX_ARENA_BUILD_OPTIONS {
                break;
            }
            if let Some(&third) =
                rank_compatible_legendaries(&pair.display_ids, legendary_pool).first()
            {
                pair.display_ids.push(third);
                let key = canonical_trio_key(&pair.display_ids);
                if seen_keys.insert(key) {
                    trios.push(pair);
                }
            }
        }
    }

    if trios.is_empty() && !legendary_pool.is_empty() {
        let fallback_core: Vec<i32> = legendary_pool.iter().take(3).map(|i| i.id).collect();
        if !fallback_core.is_empty() {
            trios.push(CoreCandidate {
                display_ids: fallback_core,
                best_entry_play: 1,
                best_entry_win: 0,
                total_play: 1,
                total_win: 0,
            });
        }
    }

    trios.truncate(MAX_ARENA_BUILD_OPTIONS);
    trios
}

fn build_final_and_situational(
    core_ids: &[i32],
    legendary_pool: &[AggregatedItemStat],
) -> (Vec<i32>, Vec<i32>) {
    let compatible = rank_compatible_legendaries(core_ids, legendary_pool);
    let mut final_ids = core_ids.to_vec();
    let mut has_tear = final_ids.iter().copied().any(is_tear_completed_item);

    for &id in &compatible {
        if final_ids.len() >= MAX_ARENA_FINAL_ITEMS {
            break;
        }
        if final_ids.contains(&id) || (has_tear && is_tear_completed_item(id)) {
            continue;
        }
        if is_tear_completed_item(id) {
            has_tear = true;
        }
        final_ids.push(id);
    }

    let mut situational_ids = Vec::with_capacity(MAX_ARENA_SITUATIONAL_ITEMS);
    for &id in &compatible {
        if situational_ids.len() >= MAX_ARENA_SITUATIONAL_ITEMS {
            break;
        }
        if !final_ids.contains(&id) && !situational_ids.contains(&id) {
            situational_ids.push(id);
        }
    }

    (final_ids, situational_ids)
}

/// Builds up to 3 distinct, archetype-cohesive `UnifiedBuildOption`s for Arena.
pub(super) fn build_arena_unified_options(
    core_entries: &[ItemStatEntry],
    legendary_pool: &[AggregatedItemStat],
    known_prismatics: &HashSet<i32>,
) -> Vec<UnifiedBuildOption> {
    let core_candidates =
        extract_arena_core_candidates(core_entries, legendary_pool, known_prismatics);
    let total_core_play: i64 = core_candidates.iter().map(|c| c.total_play.max(0)).sum();

    let mut build_options = Vec::with_capacity(core_candidates.len());
    for (idx, candidate) in core_candidates.into_iter().enumerate() {
        let (final_item_ids, situational_item_ids) =
            build_final_and_situational(&candidate.display_ids, legendary_pool);
        let pick_rate = if total_core_play > 0 {
            candidate.total_play as f64 / total_core_play as f64
        } else {
            0.0
        };
        let name = if idx == 0 {
            "Build 1 (Mais Feita)".to_string()
        } else {
            format!("Build {}", idx + 1)
        };
        build_options.push(UnifiedBuildOption {
            name,
            core_item_ids: candidate.display_ids,
            final_item_ids,
            situational_item_ids,
            play: candidate.total_play,
            win: candidate.total_win,
            pick_rate,
            ..Default::default()
        });
    }

    build_options
}
