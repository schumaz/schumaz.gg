use super::super::models::{DataDragonCache, ItemClassificationCache};
#[cfg(test)]
use super::tear::*;
use std::ops::{Deref, RangeInclusive};
use std::sync::{Arc, OnceLock, RwLock};

pub static TEST_CLASSIFICATION_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

static DYNAMIC_ITEM_CLASSIFICATIONS: OnceLock<RwLock<Option<Arc<ItemClassificationCache>>>> =
    OnceLock::new();
static EMPTY_ITEM_CLASSIFICATIONS: OnceLock<Arc<ItemClassificationCache>> = OnceLock::new();

/// Item ID range reserved for Arena (`CHERRY`) anvils & Shardholder (`220000` Stat Anvil, `220007` Prismatic Item Anvil, `220012` Shardholder).
pub const ARENA_ANVIL_ID_RANGE: RangeInclusive<i32> = 220_000..=220_012;
/// Item ID prefix range for Arena `44xxxx` items (note: `447111` costs `2500g` and is NOT a Prismatic).
pub const ARENA_PRISMATIC_ID_RANGE: RangeInclusive<i32> = 440_000..=449_999;
/// Canonical DataDragon total gold cost shared by all 46 real Arena Prismatic items (`sell == 1100`).
pub const ARENA_PRISMATIC_GOLD: i64 = 2750;

/// Zero-copy view over an `ItemClassificationCache`, borrowing directly from `DataDragonCache`
/// when available or falling back to an `Arc` snapshot of the global classification store.
pub enum ClassificationsRef<'a> {
    Borrowed(&'a ItemClassificationCache),
    Shared(Arc<ItemClassificationCache>),
}

impl Deref for ClassificationsRef<'_> {
    type Target = ItemClassificationCache;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Borrowed(cls) => cls,
            Self::Shared(arc) => arc,
        }
    }
}

fn classification_store() -> &'static RwLock<Option<Arc<ItemClassificationCache>>> {
    DYNAMIC_ITEM_CLASSIFICATIONS.get_or_init(|| RwLock::new(None))
}

pub fn set_dynamic_item_classifications(cache: ItemClassificationCache) {
    if let Ok(mut guard) = classification_store().write() {
        *guard = Some(Arc::new(cache));
    }
}

/// Clears dynamic item classifications from L1 RAM.
pub fn clear_dynamic_item_classifications() {
    if let Ok(mut guard) = classification_store().write() {
        *guard = None;
    }
}

pub fn get_dynamic_item_classifications() -> Option<ItemClassificationCache> {
    classification_store()
        .read()
        .ok()
        .and_then(|g| g.as_ref().map(|arc| (**arc).clone()))
}

/// Returns an O(1) `Arc` snapshot of the active dynamic item classifications without deep-cloning.
pub fn snapshot_dynamic_item_classifications() -> Arc<ItemClassificationCache> {
    if let Ok(guard) = classification_store().read()
        && let Some(arc) = guard.as_ref()
    {
        return Arc::clone(arc);
    }
    Arc::clone(EMPTY_ITEM_CLASSIFICATIONS.get_or_init(|| Arc::new(ItemClassificationCache::default())))
}

/// Resolves item classifications preferring `ddragon.item_classifications` (zero locks) and falling
/// back to a single O(1) `Arc` snapshot of the global classification cache.
pub fn resolve_classifications_for_ddragon(
    ddragon: Option<&DataDragonCache>,
) -> ClassificationsRef<'_> {
    if let Some(dd) = ddragon {
        let cls = &dd.item_classifications;
        if !cls.boots.is_empty()
            || !cls.completed.is_empty()
            || !cls.item_tags.is_empty()
            || !cls.map11_items.is_empty()
            || !cls.map12_items.is_empty()
            || !cls.map30_items.is_empty()
        {
            return ClassificationsRef::Borrowed(cls);
        }
    }
    ClassificationsRef::Shared(snapshot_dynamic_item_classifications())
}

/// Returns DataDragon tags for `id` directly from `cls` without locking.
pub fn item_tags_in(cls: &ItemClassificationCache, id: i32) -> &[String] {
    cls.item_tags.get(&id).map(Vec::as_slice).unwrap_or(&[])
}

pub fn is_boot_in(cls: &ItemClassificationCache, id: i32) -> bool {
    cls.boots.contains(&id) || is_boot_item(id)
}

pub fn is_component_in(cls: &ItemClassificationCache, id: i32) -> bool {
    cls.components.contains(&id) || is_component_item(id)
}

pub fn is_completed_in(cls: &ItemClassificationCache, id: i32) -> bool {
    cls.completed.contains(&id)
}

pub fn is_starter_in(cls: &ItemClassificationCache, id: i32) -> bool {
    cls.starters.contains(&id) || is_starter_item(id)
}

pub fn is_consumable_in(cls: &ItemClassificationCache, id: i32) -> bool {
    cls.consumables.contains(&id)
}

pub fn is_arena_prismatic_in(cls: &ItemClassificationCache, id: i32) -> bool {
    cls.arena_prismatics.contains(&id)
}

pub fn is_map30_item_in(cls: &ItemClassificationCache, id: i32) -> bool {
    cls.map30_items.is_empty() || cls.map30_items.contains(&id)
}

pub fn is_arena_anvil_or_consumable_in(cls: &ItemClassificationCache, id: i32) -> bool {
    ARENA_ANVIL_ID_RANGE.contains(&id)
        || cls.arena_consumables.contains(&id)
        || matches!(id, 2142..=2146 | 222141 | 3400 | 3599 | 3600)
}

pub fn normalize_arena_item_id_in(cls: &ItemClassificationCache, id: i32) -> i32 {
    match id {
        223042 => return 223004,
        223040 => return 223003,
        223121 => return 223119,
        222530 => return 222526,
        _ => {}
    }
    if let Some(&base_id) = cls.evolved_fallbacks.get(&id) {
        if cls.map30_items.is_empty() || cls.map30_items.contains(&base_id) {
            return base_id;
        }
        let map30_candidate = 220_000 + base_id;
        if cls.map30_items.contains(&map30_candidate) {
            return map30_candidate;
        }
    }
    id
}

pub fn normalize_tear_item_in(cls: &ItemClassificationCache, id: i32) -> i32 {
    if let Some(&base_id) = cls.evolved_fallbacks.get(&id) {
        return base_id;
    }
    match id {
        3042 => 3004,
        3040 => 3003,
        3121 => 3119,
        2530 => 2526,
        223042 => 223004,
        223040 => 223003,
        223121 => 223119,
        222530 => 222526,
        other => other,
    }
}

/// Dynamically retrieves DataDragon tags for an item.
pub fn get_item_tags_dynamic(id: i32) -> Vec<String> {
    let cls = snapshot_dynamic_item_classifications();
    item_tags_in(&cls, id).to_vec()
}

/// Dynamically checks whether an item is a boot using Riot DataDragon tags or fallback catalog.
pub fn is_boot_item_dynamic(id: i32) -> bool {
    let cls = snapshot_dynamic_item_classifications();
    is_boot_in(&cls, id)
}

/// Dynamically checks whether an item is a component using Riot DataDragon recipe tree or fallback catalog.
pub fn is_component_item_dynamic(id: i32) -> bool {
    let cls = snapshot_dynamic_item_classifications();
    is_component_in(&cls, id)
}

/// Dynamically checks whether an item is a completed legendary item using Riot DataDragon cache.
pub fn is_completed_item_dynamic(id: i32) -> bool {
    let cls = snapshot_dynamic_item_classifications();
    is_completed_in(&cls, id)
}

/// Dynamically checks whether `id` is a real Arena Prismatic (`44xxxx` with `gold == 2750` from DataDragon).
/// Never classifies `447111` (Overlord's Bloodmail, `2500g`) as a Prismatic.
pub fn is_arena_prismatic_dynamic(id: i32) -> bool {
    let cls = snapshot_dynamic_item_classifications();
    is_arena_prismatic_in(&cls, id)
}

/// Dynamically checks whether `id` is enabled on Riot Map `30` (Arena).
/// Returns `true` when the DataDragon Map 30 catalog has not been populated yet.
pub fn is_map30_item_dynamic(id: i32) -> bool {
    let cls = snapshot_dynamic_item_classifications();
    is_map30_item_in(&cls, id)
}

/// Returns `true` for Arena anvils (`220000..=220012`), juices/consumables, unpurchasable non-evolved
/// Map 30 items, and any item tagged `"Consumable"` in DataDragon `item.json`.
pub fn is_arena_anvil_or_consumable(id: i32) -> bool {
    let cls = snapshot_dynamic_item_classifications();
    is_arena_anvil_or_consumable_in(&cls, id)
}

/// Normalizes an evolved Arena item (Map `30`) to its purchasable Map `30` base item while
/// preventing Riot's DataDragon `specialRecipe` bug (`223121 -> 3119`) from leaking SR IDs.
pub fn normalize_arena_item_id(id: i32) -> i32 {
    let cls = snapshot_dynamic_item_classifications();
    normalize_arena_item_id_in(&cls, id)
}

/// Normalizes an evolved item (like Muramana, Seraph's Embrace, Fimbulwinter, Diadem of Songs) to its purchasable base.
pub fn normalize_tear_item_dynamic(id: i32) -> i32 {
    let cls = snapshot_dynamic_item_classifications();
    normalize_tear_item_in(&cls, id)
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
