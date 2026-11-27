use super::super::models::{ItemClassificationCache, ItemStatEntry};
use super::classification::{
    is_boot_in, is_component_in, is_starter_in, normalize_tear_item_in,
    snapshot_dynamic_item_classifications,
};
use std::collections::HashMap;

pub const TEAR_COMPONENT_IDS: [i32; 3] = [3070, 323070, 773070];
pub const TEAR_COMPLETED_ITEMS: [i32; 4] = [2526, 3003, 3004, 3119];
pub const ARENA_TEAR_COMPLETED_ITEMS: [i32; 4] = [222526, 223003, 223004, 223119];

pub fn is_tear_component(id: i32) -> bool {
    TEAR_COMPONENT_IDS.contains(&id)
}

pub fn is_tear_completed_item(id: i32) -> bool {
    TEAR_COMPLETED_ITEMS.contains(&id) || ARENA_TEAR_COMPLETED_ITEMS.contains(&id)
}

/// Detects the champion's most-played completed purchasable Tear legendary item across payload entries.
/// Returns `None` when no completed/evolved Tear item appears anywhere in `entries`, so raw Tear
/// components bought solely for early mana never invent a Tear legendary.
pub fn resolve_tear_to_completed_item_in(
    cls: &ItemClassificationCache,
    entries: &[ItemStatEntry],
) -> Option<i32> {
    let mut play_by_item: HashMap<i32, (i64, usize)> = HashMap::new();
    let mut seen_counter = 0usize;

    for entry in entries {
        for &id in &entry.ids {
            let norm = normalize_tear_item_in(cls, id);
            if TEAR_COMPLETED_ITEMS.contains(&norm) {
                let slot = play_by_item.entry(norm).or_insert_with(|| {
                    let idx = seen_counter;
                    seen_counter += 1;
                    (0, idx)
                });
                slot.0 += entry.play.max(1);
            }
        }
    }

    play_by_item
        .into_iter()
        .max_by(|(id_a, (play_a, idx_a)), (id_b, (play_b, idx_b))| {
            play_a
                .cmp(play_b)
                .then_with(|| idx_b.cmp(idx_a))
                .then_with(|| id_b.cmp(id_a))
        })
        .map(|(id, _)| id)
}

/// Convenience wrapper acquiring a single classification snapshot.
pub fn resolve_tear_to_completed_item(entries: &[ItemStatEntry]) -> Option<i32> {
    let cls = snapshot_dynamic_item_classifications();
    resolve_tear_to_completed_item_in(&cls, entries)
}

/// Normalizes a slice of item IDs by replacing transformed items with purchasable bases
/// and safely handling raw Tear of the Goddess components (`3070`, `323070`, `773070`).
pub fn resolve_tear_item_ids_in(
    cls: &ItemClassificationCache,
    ids: &[i32],
    tear_upgrade: Option<i32>,
) -> Vec<i32> {
    let mut normalized = Vec::new();
    let mut has_tear_component = false;

    for &id in ids {
        if is_tear_component(id) {
            has_tear_component = true;
            continue;
        }
        let norm = normalize_tear_item_in(cls, id);
        if !normalized.contains(&norm) {
            normalized.push(norm);
        }
    }

    if !has_tear_component {
        return normalized;
    }

    let has_completed_tear = normalized
        .iter()
        .any(|id| TEAR_COMPLETED_ITEMS.contains(id));
    if has_completed_tear {
        return normalized;
    }

    let completed_count = normalized
        .iter()
        .filter(|&&id| {
            !is_boot_in(cls, id) && !is_starter_in(cls, id) && !is_component_in(cls, id)
        })
        .count();

    if completed_count > 0
        && completed_count < 3
        && let Some(upgrade_id) = tear_upgrade
        && !normalized.contains(&upgrade_id)
    {
        normalized.push(upgrade_id);
    }

    normalized
}

/// Convenience wrapper acquiring a single classification snapshot.
pub fn resolve_tear_item_ids(ids: &[i32], tear_upgrade: Option<i32>) -> Vec<i32> {
    let cls = snapshot_dynamic_item_classifications();
    resolve_tear_item_ids_in(&cls, ids, tear_upgrade)
}

/// Normalizes a slice of item IDs by replacing transformed items with purchasable bases
/// and deduplicating while preserving original order.
pub fn normalize_item_ids_dedup_in(cls: &ItemClassificationCache, ids: &[i32]) -> Vec<i32> {
    let mut result = Vec::new();
    for &id in ids {
        let norm = normalize_tear_item_in(cls, id);
        if !result.contains(&norm) {
            result.push(norm);
        }
    }
    result
}

/// Convenience wrapper acquiring a single classification snapshot.
pub fn normalize_item_ids_dedup(ids: &[i32]) -> Vec<i32> {
    let cls = snapshot_dynamic_item_classifications();
    normalize_item_ids_dedup_in(&cls, ids)
}
