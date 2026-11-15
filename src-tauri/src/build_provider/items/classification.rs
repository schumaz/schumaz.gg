use super::super::models::{ItemClassificationCache, ItemStatEntry};

pub static TEST_CLASSIFICATION_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

static DYNAMIC_ITEM_CLASSIFICATIONS: std::sync::RwLock<Option<ItemClassificationCache>> =
    std::sync::RwLock::new(None);

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

/// Normalizes an evolved item (like Muramana, Seraph's Embrace, Fimbulwinter, Diadem of Songs) to its purchasable base.
pub fn normalize_tear_item_dynamic(id: i32) -> i32 {
    if let Ok(guard) = DYNAMIC_ITEM_CLASSIFICATIONS.read()
        && let Some(cache) = guard.as_ref()
        && let Some(&base_id) = cache.evolved_fallbacks.get(&id)
    {
        return base_id;
    }
    match id {
        3042 => 3004, // Muramana -> Manamune
        3040 => 3003, // Seraph's Embrace -> Archangel's Staff
        3121 => 3119, // Fimbulwinter -> Winter's Approach
        2530 => 2526, // Diadem of Songs -> Whispering Circlet
        other => other,
    }
}

pub const TEAR_COMPLETED_ITEMS: [i32; 4] = [
    2526, // Whispering Circlet (Circuleto Sussurrante)
    3003, // Archangel's Staff (Cajado do Arcanjo)
    3004, // Manamune
    3119, // Winter's Approach (Aproximação do Inverno)
];

/// Resolves a raw Tear of the Goddess (3070) into its completed purchasable legendary item
/// by inspecting the champion's candidate/depth entries or defaulting by position and damage type.
pub fn resolve_tear_to_completed_item(
    entries: &[ItemStatEntry],
    position: &str,
    damage_type: &str,
) -> i32 {
    // 1. Look for any completed tear item in the champion's item entries
    for entry in entries {
        for &id in &entry.ids {
            let norm = normalize_tear_item_dynamic(id);
            if TEAR_COMPLETED_ITEMS.contains(&norm) {
                return norm;
            }
        }
    }

    // 2. Fallback based on champion position / archetype
    let norm_pos = position.to_lowercase();
    if norm_pos == "support" || norm_pos == "supp" || norm_pos == "utility" {
        2526 // Whispering Circlet (Circuleto Sussurrante)
    } else if damage_type == "AP" {
        3003 // Archangel's Staff (Cajado do Arcanjo)
    } else if damage_type == "TANK" || norm_pos == "top" {
        3119 // Winter's Approach (Aproximação do Inverno)
    } else {
        3004 // Manamune
    }
}

/// Normalizes a slice of item IDs by replacing transformed items with purchasable bases
/// and optionally promoting Tear of the Goddess (3070) to its completed item,
/// deduplicating while preserving original order.
pub fn resolve_tear_item_ids(ids: &[i32], tear_upgrade: i32) -> Vec<i32> {
    let mut result = Vec::new();
    for &id in ids {
        let norm = if id == 3070 {
            tear_upgrade
        } else {
            normalize_tear_item_dynamic(id)
        };
        if !result.contains(&norm) {
            result.push(norm);
        }
    }
    result
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
        1001 // Boots of Speed
        | 1004 // Faerie Charm
        | 1006 // Rejuvenation Bead
        | 1011 // Giant's Belt
        | 1018 // Cloak of Agility
        | 1026 // Blasting Wand
        | 1027 // Sapphire Crystal
        | 1028 // Ruby Crystal
        | 1029 // Cloth Armor
        | 1031 // Chain Vest
        | 1033 // Null-Magic Mantle
        | 1036 // Long Sword
        | 1037 // Pickaxe
        | 1038 // B. F. Sword
        | 1042 // Dagger
        | 1043 // Recurve Bow
        | 1052 // Amplifying Tome
        | 1057 // Negatron Cloak
        | 1058 // Needlessly Large Rod
        | 2015 // Kircheis Shard
        | 2420 // Seeker's Armguard
        | 3024 // Glacial Buckler
        | 3035 // Last Whisper
        | 3044 // Phage
        | 3051 // Hearthbound Axe
        | 3057 // Sheen
        | 3067 // Kindlegem
        | 3070 // Tear of the Goddess
        | 3076 // Bramble Vest
        | 3077 // Tiamat
        | 3082 // Warden's Mail
        | 3086 // Zeal
        | 3105 // Aegis of the Legion
        | 3108 // Fiendish Codex
        | 3113 // Aether Wisp
        | 3114 // Forbidden Idol
        | 3123 // Executioner's Calling
        | 3133 // Caulfield's Warhammer
        | 3134 // Serrated Dirk
        | 3140 // Quicksilver Sash
        | 3144 // Scout's Slingshot
        | 3145 // Hextech Alternator
        | 3147 // Haunting Guise
        | 3155 // Hexdrinker
        | 3211 // Spectre's Cowl
        | 3801 // Crystalline Bracer
        | 3802 // Lost Chapter
        | 3803 // Catalyst of Aeons
        | 3916 // Oblivion Orb
    )
}

pub fn is_starter_item(id: i32) -> bool {
    matches!(
        id,
        1054 | 1055 | 1056 | 1082 | 1083 | 1086 | 1101..=1103 | 2003 | 2031 | 2033 | 2051 | 2055 | 2138 | 2139 | 2140 | 3112 | 3177 | 3184 | 3865..=3877
    )
}

pub fn is_boot_item(item_id: i32) -> bool {
    matches!(
        item_id,
        1001 // Boots
        | 2422 // Slightly Magical Footwear
        | 3006 // Berserker's Greaves
        | 3008 // Gluttonous Greaves / Symbiotic Soles
        | 3009 // Boots of Swiftness
        | 3010 // Symbiotic Soles
        | 3013 // Synchronized Souls (empowered)
        | 3020 // Sorcerer's Shoes
        | 3047 // Plated Steelcaps
        | 3111 // Mercury's Treads
        | 3117 // Boots of Mobility
        | 3158 // Ionian Boots of Lucidity
        | 3170..=3176 // Tier 3 boots (Swiftmarch, Crimson Lucidity, Gunmetal Greaves, etc.)
    )
}

#[cfg(test)]
#[path = "classification_tests.rs"]
mod tests;
