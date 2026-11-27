use super::super::models::{ItemClassificationCache, ItemStatEntry};
use std::collections::HashMap;
use std::ops::RangeInclusive;

pub static TEST_CLASSIFICATION_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

static DYNAMIC_ITEM_CLASSIFICATIONS: std::sync::RwLock<Option<ItemClassificationCache>> =
    std::sync::RwLock::new(None);

/// Item ID range reserved for Arena (`CHERRY`) anvils & Shardholder (`220000` Stat Anvil, `220007` Prismatic Item Anvil, `220012` Shardholder).
pub const ARENA_ANVIL_ID_RANGE: RangeInclusive<i32> = 220_000..=220_012;
/// Item ID prefix range for Arena `44xxxx` items (note: `447111` costs `2500g` and is NOT a Prismatic).
pub const ARENA_PRISMATIC_ID_RANGE: RangeInclusive<i32> = 440_000..=449_999;
/// Canonical DataDragon total gold cost shared by all 46 real Arena Prismatic items (`sell == 1100`).
pub const ARENA_PRISMATIC_GOLD: i64 = 2750;

pub fn set_dynamic_item_classifications(cache: ItemClassificationCache) {
    if let Ok(mut guard) = DYNAMIC_ITEM_CLASSIFICATIONS.write() {
        *guard = Some(cache);
    }
}

pub fn get_dynamic_item_classifications() -> Option<ItemClassificationCache> {
    DYNAMIC_ITEM_CLASSIFICATIONS
        .read()
        .ok()
        .and_then(|g| g.clone())
}

/// Dynamically retrieves DataDragon tags for an item.
pub fn get_item_tags_dynamic(id: i32) -> Vec<String> {
    if let Ok(guard) = DYNAMIC_ITEM_CLASSIFICATIONS.read()
        && let Some(cache) = guard.as_ref()
        && let Some(tags) = cache.item_tags.get(&id)
    {
        return tags.clone();
    }
    Vec::new()
}

/// Dynamically checks whether an item is a boot using Riot DataDragon tags or fallback catalog.
pub fn is_boot_item_dynamic(id: i32) -> bool {
    if let Ok(guard) = DYNAMIC_ITEM_CLASSIFICATIONS.read()
        && let Some(cache) = guard.as_ref()
        && cache.boots.contains(&id)
    {
        return true;
    }
    is_boot_item(id)
}

/// Dynamically checks whether an item is a component using Riot DataDragon recipe tree or fallback catalog.
pub fn is_component_item_dynamic(id: i32) -> bool {
    if let Ok(guard) = DYNAMIC_ITEM_CLASSIFICATIONS.read()
        && let Some(cache) = guard.as_ref()
        && cache.components.contains(&id)
    {
        return true;
    }
    is_component_item(id)
}

/// Dynamically checks whether an item is a completed legendary item using Riot DataDragon cache.
pub fn is_completed_item_dynamic(id: i32) -> bool {
    if let Ok(guard) = DYNAMIC_ITEM_CLASSIFICATIONS.read()
        && let Some(cache) = guard.as_ref()
        && cache.completed.contains(&id)
    {
        return true;
    }
    false
}

/// Dynamically checks whether `id` is a real Arena Prismatic (`44xxxx` with `gold == 2750` from DataDragon).
/// Never classifies `447111` (Overlord's Bloodmail, `2500g`) as a Prismatic.
pub fn is_arena_prismatic_dynamic(id: i32) -> bool {
    if let Ok(guard) = DYNAMIC_ITEM_CLASSIFICATIONS.read()
        && let Some(cache) = guard.as_ref()
        && cache.arena_prismatics.contains(&id)
    {
        return true;
    }
    false
}

/// Dynamically checks whether `id` is enabled on Riot Map `30` (Arena).
/// Returns `true` when the DataDragon Map 30 catalog has not been populated yet.
pub fn is_map30_item_dynamic(id: i32) -> bool {
    if let Ok(guard) = DYNAMIC_ITEM_CLASSIFICATIONS.read()
        && let Some(cache) = guard.as_ref()
        && !cache.map30_items.is_empty()
    {
        return cache.map30_items.contains(&id);
    }
    true
}

/// Returns `true` for Arena anvils (`220000..=220012`), juices/consumables, unpurchasable non-evolved
/// Map 30 items, and any item tagged `"Consumable"` in DataDragon `item.json`.
pub fn is_arena_anvil_or_consumable(id: i32) -> bool {
    if ARENA_ANVIL_ID_RANGE.contains(&id) {
        return true;
    }
    if let Ok(guard) = DYNAMIC_ITEM_CLASSIFICATIONS.read()
        && let Some(cache) = guard.as_ref()
        && cache.arena_consumables.contains(&id)
    {
        return true;
    }
    matches!(id, 2142..=2146 | 222141 | 3400 | 3599 | 3600)
}

/// Normalizes an evolved Arena item (Map `30`) to its purchasable Map `30` base item while
/// preventing Riot's DataDragon `specialRecipe` bug (`223121 -> 3119`) from leaking SR IDs.
pub fn normalize_arena_item_id(id: i32) -> i32 {
    match id {
        223042 => return 223004, // Arena Muramana -> Arena Manamune
        223040 => return 223003, // Arena Seraph's Embrace -> Arena Archangel's Staff
        223121 => return 223119, // Arena Fimbulwinter -> Arena Winter's Approach
        222530 => return 222526, // Arena Diadem of Songs -> Arena Whispering Circlet
        _ => {}
    }
    if let Ok(guard) = DYNAMIC_ITEM_CLASSIFICATIONS.read()
        && let Some(cache) = guard.as_ref()
        && let Some(&base_id) = cache.evolved_fallbacks.get(&id)
    {
        if cache.map30_items.is_empty() || cache.map30_items.contains(&base_id) {
            return base_id;
        }
        let map30_candidate = 220_000 + base_id;
        if cache.map30_items.contains(&map30_candidate) {
            return map30_candidate;
        }
    }
    id
}

/// Normalizes an evolved item (like Muramana, Seraph's Embrace, Fimbulwinter, Diadem of Songs) to its purchasable base.
pub fn normalize_tear_item_dynamic(id: i32) -> i32 {
    if let Ok(guard) = DYNAMIC_ITEM_CLASSIFICATIONS.read()
        && let Some(cache) = guard.as_ref()
        && let Some(&base_id) = cache.evolved_fallbacks.get(&id)
    {
        return base_id;
    }
    match id {
        3042 => 3004,     // Muramana -> Manamune
        3040 => 3003,     // Seraph's Embrace -> Archangel's Staff
        3121 => 3119,     // Fimbulwinter -> Winter's Approach
        2530 => 2526,     // Diadem of Songs -> Whispering Circlet
        223042 => 223004, // Arena Muramana -> Arena Manamune
        223040 => 223003, // Arena Seraph's Embrace -> Arena Archangel's Staff
        223121 => 223119, // Arena Fimbulwinter -> Arena Winter's Approach
        222530 => 222526, // Arena Diadem of Songs -> Arena Whispering Circlet
        other => other,
    }
}

pub const TEAR_COMPONENT_IDS: [i32; 3] = [3070, 323070, 773070];

pub fn is_tear_component(id: i32) -> bool {
    TEAR_COMPONENT_IDS.contains(&id)
}

pub const TEAR_COMPLETED_ITEMS: [i32; 4] = [2526, 3003, 3004, 3119];

pub const ARENA_TEAR_COMPLETED_ITEMS: [i32; 4] = [222526, 223003, 223004, 223119];

pub fn is_tear_completed_item(id: i32) -> bool {
    TEAR_COMPLETED_ITEMS.contains(&id) || ARENA_TEAR_COMPLETED_ITEMS.contains(&id)
}

/// Detects the champion's most-played completed purchasable Tear legendary item across payload entries.
/// Returns `None` when no completed/evolved Tear item appears anywhere in `entries`, so raw Tear
/// components bought solely for early mana never invent a Tear legendary.
pub fn resolve_tear_to_completed_item(entries: &[ItemStatEntry]) -> Option<i32> {
    let mut play_by_item: HashMap<i32, (i64, usize)> = HashMap::new();
    let mut seen_counter = 0usize;

    for entry in entries {
        for &id in &entry.ids {
            let norm = normalize_tear_item_dynamic(id);
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

/// Normalizes a slice of item IDs by replacing transformed items with purchasable bases
/// and safely handling raw Tear of the Goddess components (`3070`, `323070`, `773070`).
pub fn resolve_tear_item_ids(ids: &[i32], tear_upgrade: Option<i32>) -> Vec<i32> {
    let mut normalized = Vec::new();
    let mut has_tear_component = false;

    for &id in ids {
        if is_tear_component(id) {
            has_tear_component = true;
            continue;
        }
        let norm = normalize_tear_item_dynamic(id);
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
            !is_boot_item_dynamic(id) && !is_starter_item(id) && !is_component_item_dynamic(id)
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

/// Normalizes a slice of item IDs by replacing transformed items with purchasable bases
/// and deduplicating while preserving original order.
pub fn normalize_item_ids_dedup(ids: &[i32]) -> Vec<i32> {
    let mut result = Vec::new();
    for &id in ids {
        let norm = normalize_tear_item_dynamic(id);
        if !result.contains(&norm) {
            result.push(norm);
        }
    }
    result
}

pub fn is_component_item(id: i32) -> bool {
    matches!(
        id,
        1001 | 1004
            | 1006
            | 1011
            | 1018
            | 1026
            | 1027
            | 1028
            | 1029
            | 1031
            | 1033
            | 1036
            | 1037
            | 1038
            | 1042
            | 1043
            | 1052
            | 1057
            | 1058
            | 2015
            | 2420
            | 3024
            | 3035
            | 3044
            | 3051
            | 3057
            | 3067
            | 3070
            | 323070
            | 773070
            | 3076
            | 3077
            | 3082
            | 3086
            | 3105
            | 3108
            | 3113
            | 3114
            | 3123
            | 3133
            | 3134
            | 3140
            | 3144
            | 3145
            | 3147
            | 3155
            | 3211
            | 3801
            | 3802
            | 3803
            | 3916
    )
}

pub fn is_starter_item(id: i32) -> bool {
    matches!(
        id,
        1054 | 1055 | 1056 | 1082 | 1083 | 1086 | 1101..=1103 | 2003 | 2031 | 2033 | 2051 | 2055
            | 2138 | 2139 | 2140 | 3112 | 3177 | 3184 | 3865..=3877
    )
}

pub fn is_boot_item(item_id: i32) -> bool {
    matches!(
        item_id,
        1001 | 2422 | 3006 | 3008 | 3009 | 3010 | 3013 | 3020 | 3047 | 3111 | 3117 | 3158 | 3170
            ..=3176 | 223005 | 223006 | 223008 | 223009 | 223020 | 223047 | 223111 | 223158
    )
}

#[cfg(test)]
#[path = "classification_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "tear_resolution_tests.rs"]
mod tear_tests;
