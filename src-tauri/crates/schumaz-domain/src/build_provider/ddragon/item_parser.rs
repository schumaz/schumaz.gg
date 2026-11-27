use super::super::items::{
    ARENA_ANVIL_ID_RANGE, ARENA_PRISMATIC_GOLD, ARENA_PRISMATIC_ID_RANGE, is_boot_item,
    is_tear_completed_item,
};
use super::super::models::ItemClassificationCache;
use std::collections::{HashMap, HashSet};

/// Parses DataDragon `item.json` into dynamic classifications (boots, components, completed items,
/// Map 30 items, Arena Prismatics `44xxxx` at `2750g`, and Arena anvils/consumables).
pub fn parse_ddragon_item_classifications(
    items_json: &serde_json::Value,
) -> ItemClassificationCache {
    let mut boots = HashSet::new();
    let mut components = HashSet::new();
    let mut completed = HashSet::new();
    let mut evolved_fallbacks = HashMap::new();
    let mut item_tags = HashMap::new();
    let mut arena_prismatics = HashSet::new();
    let mut map30_items = HashSet::new();
    let mut arena_consumables = HashSet::new();
    let mut map11_items = HashSet::new();
    let mut map12_items = HashSet::new();
    let mut starters = HashSet::new();
    let mut consumables = HashSet::new();
    let mut item_gold = HashMap::new();

    // Default static fallbacks for Tear of the Goddess upgrades (SR/ARAM + Arena Map 30):
    evolved_fallbacks.insert(3042, 3004); // Muramana -> Manamune
    evolved_fallbacks.insert(3040, 3003); // Seraph's Embrace -> Archangel's Staff
    evolved_fallbacks.insert(3121, 3119); // Fimbulwinter -> Winter's Approach
    evolved_fallbacks.insert(2530, 2526); // Diadem of Songs -> Whispering Circlet
    evolved_fallbacks.insert(223042, 223004); // Arena Muramana -> Arena Manamune
    evolved_fallbacks.insert(223040, 223003); // Arena Seraph's Embrace -> Arena Archangel's Staff
    evolved_fallbacks.insert(223121, 223119); // Arena Fimbulwinter -> Arena Winter's Approach
    evolved_fallbacks.insert(222530, 222526); // Arena Diadem of Songs -> Arena Whispering Circlet

    let Some(data) = items_json.get("data").and_then(|d| d.as_object()) else {
        return ItemClassificationCache {
            boots,
            components,
            completed,
            evolved_fallbacks,
            item_tags,
            arena_prismatics,
            map30_items,
            arena_consumables,
            map11_items,
            map12_items,
            starters,
            consumables,
            item_gold,
        };
    };

    // Pass 1: collect Map 30 enabled items first so specialRecipe validation can guard against
    // Riot's DataDragon bug where Map 30 evolutions (e.g. 223121) reference an SR base ID (3119).
    for (id_str, item_val) in data {
        if let Ok(id) = id_str.parse::<i32>()
            && item_val
                .pointer("/maps/30")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
        {
            map30_items.insert(id);
        }
    }

    // Pass 2: classify every item safely for both SR/ARAM and Arena (Map 30).
    for (id_str, item_val) in data {
        let Ok(id) = id_str.parse::<i32>() else {
            continue;
        };

        let has_into = item_val
            .get("into")
            .and_then(|into_arr| into_arr.as_array())
            .map(|a| !a.is_empty())
            .unwrap_or(false);
        let gold = item_val
            .pointer("/gold/total")
            .and_then(|g| g.as_i64())
            .unwrap_or(0);
        let purchasable = item_val
            .pointer("/gold/purchasable")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let tags: Vec<&str> = item_val
            .get("tags")
            .and_then(|t| t.as_array())
            .map(|arr| arr.iter().filter_map(|x| x.as_str()).collect())
            .unwrap_or_default();
        let mut string_tags: Vec<String> = tags.iter().map(|s| s.to_string()).collect();

        // Supplement missing offensive tags from numeric `stats` so newly added Riot items
        // are still classified accurately for AD/AP archetype and damage inference.
        if let Some(stats) = item_val.get("stats").and_then(|s| s.as_object()) {
            let stat_tag_pairs = [
                ("FlatPhysicalDamageMod", "Damage"),
                ("FlatMagicDamageMod", "SpellDamage"),
                ("FlatCritChanceMod", "CriticalStrike"),
                ("PercentAttackSpeedMod", "AttackSpeed"),
            ];
            for (stat_key, tag_name) in stat_tag_pairs {
                if stats
                    .get(stat_key)
                    .and_then(|v| v.as_f64())
                    .is_some_and(|val| val > 0.0)
                    && !string_tags.iter().any(|t| t == tag_name)
                {
                    string_tags.push(tag_name.to_string());
                }
            }
        }
        item_tags.insert(id, string_tags);

        let in_store = item_val
            .get("inStore")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let special_recipe = item_val
            .get("specialRecipe")
            .and_then(|v| v.as_i64())
            .map(|v| v as i32);

        // Unpurchasable Map 30 items without a specialRecipe (e.g. `220012` Shardholder, quest items)
        // cannot be purchased in the Arena shop and must be excluded from recommended legendaries.
        let is_unpurchasable_map30_non_evolved =
            map30_items.contains(&id) && (!in_store || !purchasable) && special_recipe.is_none();
        let is_consumable = tags.contains(&"Consumable")
            || ARENA_ANVIL_ID_RANGE.contains(&id)
            || is_unpurchasable_map30_non_evolved;
        if is_consumable {
            arena_consumables.insert(id);
        }

        // All 46 real Arena Prismatics are `44xxxx` costing exactly `2750g`.
        // Crucially, `447111` (Overlord's Bloodmail) costs `2500g` and MUST remain a purchasable legendary.
        let is_prismatic = ARENA_PRISMATIC_ID_RANGE.contains(&id) && gold == ARENA_PRISMATIC_GOLD;
        if is_prismatic {
            arena_prismatics.insert(id);
        }

        if !in_store && let Some(base_id) = special_recipe {
            if map30_items.contains(&id) && !map30_items.contains(&base_id) {
                let map30_candidate = 220_000 + base_id;
                if map30_items.contains(&map30_candidate) {
                    evolved_fallbacks.insert(id, map30_candidate);
                }
                // Never overwrite a Map 30 item's fallback with an invalid non-Map-30 ID (e.g. 3119).
            } else {
                evolved_fallbacks.insert(id, base_id);
            }
        }

        let is_tear_completed = is_tear_completed_item(id);
        let hide_from_all = item_val
            .get("hideFromAll")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let has_req_champ = item_val
            .get("requiredChampion")
            .and_then(|v| v.as_str())
            .is_some_and(|s| !s.trim().is_empty());
        let has_req_ally = item_val
            .get("requiredAlly")
            .and_then(|v| v.as_str())
            .is_some_and(|s| !s.trim().is_empty());
        let is_trinket = tags.contains(&"Trinket");
        let is_shop_standard = in_store
            && purchasable
            && !hide_from_all
            && !has_req_champ
            && !has_req_ally
            && special_recipe.is_none()
            && !is_trinket
            && id < 200_000;
        let map11_enabled = item_val
            .pointer("/maps/11")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let map12_enabled = item_val
            .pointer("/maps/12")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if is_shop_standard {
            if map11_enabled {
                map11_items.insert(id);
            }
            if map12_enabled {
                map12_items.insert(id);
            }
        }

        if tags.contains(&"Consumable") && !ARENA_ANVIL_ID_RANGE.contains(&id) && is_shop_standard {
            consumables.insert(id);
            starters.insert(id);
        }

        if is_shop_standard
            && !tags.contains(&"Boots")
            && !is_consumable
            && (((tags.contains(&"Lane") || tags.contains(&"Jungle") || tags.contains(&"GoldPer"))
                && (1..=600).contains(&gold))
                || (map12_enabled && !map11_enabled && !has_into && (700..=1100).contains(&gold)))
        {
            starters.insert(id);
        }

        item_gold.insert(id, gold.max(0) as i32);

        if tags.contains(&"Boots") {
            boots.insert(id);
        }
        if has_into && !tags.contains(&"Boots") && !is_tear_completed && !is_consumable {
            components.insert(id);
        }
        if (!has_into || is_tear_completed)
            && gold >= 1600
            && !tags.contains(&"Boots")
            && !is_consumable
            && !is_prismatic
        {
            completed.insert(id);
        }
    }

    ItemClassificationCache {
        boots,
        components,
        completed,
        evolved_fallbacks,
        item_tags,
        arena_prismatics,
        map30_items,
        arena_consumables,
        map11_items,
        map12_items,
        starters,
        consumables,
        item_gold,
    }
}

/// Parses DataDragon `item.json` to identify completed legendary/mythic items (gold >= 1600, no 'into' recipes).
pub fn parse_ddragon_completed_items(items_json: &serde_json::Value) -> HashSet<i32> {
    let mut completed = HashSet::new();
    if let Some(data) = items_json.get("data").and_then(|d| d.as_object()) {
        for (id_str, item_val) in data {
            if let Ok(id) = id_str.parse::<i32>() {
                let has_into = item_val
                    .get("into")
                    .and_then(|into_arr| into_arr.as_array())
                    .map(|a| !a.is_empty())
                    .unwrap_or(false);
                let gold = item_val
                    .pointer("/gold/total")
                    .and_then(|g| g.as_i64())
                    .unwrap_or(0);
                let is_consumable = item_val
                    .get("tags")
                    .and_then(|t| t.as_array())
                    .is_some_and(|arr| arr.iter().any(|x| x.as_str() == Some("Consumable")))
                    || ARENA_ANVIL_ID_RANGE.contains(&id);
                let is_prismatic =
                    ARENA_PRISMATIC_ID_RANGE.contains(&id) && gold == ARENA_PRISMATIC_GOLD;
                if !has_into && gold >= 1600 && !is_boot_item(id) && !is_consumable && !is_prismatic
                {
                    completed.insert(id);
                }
            }
        }
    }
    completed
}
