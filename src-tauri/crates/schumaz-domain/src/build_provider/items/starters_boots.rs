use super::super::models::{ItemClassificationCache, ItemStatEntry};
use super::classification::{is_consumable_in, item_tags_in};
use std::collections::HashMap;

fn is_consumable_starter_id(cls: &ItemClassificationCache, id: i32) -> bool {
    is_consumable_in(cls, id) || matches!(id, 2003 | 2031 | 2033 | 2055)
}

fn is_potion_consumable_id(cls: &ItemClassificationCache, id: i32) -> bool {
    matches!(id, 2003 | 2031 | 2033)
        || (is_consumable_in(cls, id) && item_tags_in(cls, id).iter().any(|t| t == "HealthRegen"))
}

fn is_jungle_pet_id(cls: &ItemClassificationCache, id: i32) -> bool {
    matches!(id, 1101..=1103)
        || (cls.starters.contains(&id)
            && !is_consumable_in(cls, id)
            && item_tags_in(cls, id).iter().any(|t| t == "Jungle"))
}

fn is_support_quest_id(cls: &ItemClassificationCache, id: i32) -> bool {
    matches!(id, 3865..=3877)
        || (cls.starters.contains(&id)
            && !is_consumable_in(cls, id)
            && item_tags_in(cls, id).iter().any(|t| t == "GoldPer"))
}

/// Selects the starter item IDs for Block 1 ("Itens Iniciais") respecting support/jungle/lane rules
/// without fabricating hardcoded fallback items when no starter data is present.
pub(crate) fn select_starter_items(
    cls: &ItemClassificationCache,
    starter_entries: &[ItemStatEntry],
    _damage_type: &str,
    position: &str,
) -> Vec<i32> {
    let norm_pos = position.to_lowercase();
    let is_support = norm_pos == "support" || norm_pos == "supp" || norm_pos == "utility";
    let is_jungle = norm_pos == "jungle" || norm_pos == "jng";

    let mut sorted_starter_entries: Vec<(usize, &ItemStatEntry)> =
        starter_entries.iter().enumerate().collect();
    sorted_starter_entries.sort_by(|a, b| b.1.play.cmp(&a.1.play).then_with(|| a.0.cmp(&b.0)));

    if is_support {
        let Some((_, first)) = sorted_starter_entries.first() else {
            return Vec::new();
        };
        let potion_id = first
            .ids
            .iter()
            .copied()
            .find(|&id| is_potion_consumable_id(cls, id))
            .unwrap_or(2003);
        vec![potion_id]
    } else if is_jungle {
        let mut pet_counts: HashMap<i32, (i64, usize)> = HashMap::new();
        let mut pet_seen = 0;
        for (_, entry) in &sorted_starter_entries {
            for &id in &entry.ids {
                if is_jungle_pet_id(cls, id) {
                    let slot = pet_counts.entry(id).or_insert_with(|| {
                        let i = pet_seen;
                        pet_seen += 1;
                        (0, i)
                    });
                    slot.0 += entry.play.max(1);
                }
            }
        }
        let mut sorted_pets: Vec<(i32, i64, usize)> = pet_counts
            .into_iter()
            .map(|(id, (p, idx))| (id, p, idx))
            .collect();
        sorted_pets.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.2.cmp(&b.2)));
        let Some(&(best_pet, _, _)) = sorted_pets.first() else {
            return Vec::new();
        };

        // Preserve the potion from the most-played starter entry containing the chosen pet
        let top_pet_entry = sorted_starter_entries
            .iter()
            .find(|(_, e)| e.ids.contains(&best_pet))
            .or_else(|| sorted_starter_entries.first());

        if let Some((_, entry)) = top_pet_entry {
            if let Some(&potion_id) = entry
                .ids
                .iter()
                .find(|&&id| is_potion_consumable_id(cls, id))
            {
                vec![best_pet, potion_id]
            } else {
                vec![best_pet]
            }
        } else {
            vec![best_pet]
        }
    } else if let Some((_, top_entry)) = sorted_starter_entries.first() {
        let mut non_consumables: Vec<i32> = Vec::new();
        let mut consumables: Vec<i32> = Vec::new();
        for &id in &top_entry.ids {
            if is_consumable_starter_id(cls, id) {
                if !consumables.contains(&id) {
                    consumables.push(id);
                }
            } else if !is_jungle_pet_id(cls, id)
                && !is_support_quest_id(cls, id)
                && !non_consumables.contains(&id)
            {
                non_consumables.push(id);
            }
        }
        let mut combined = non_consumables;
        combined.extend(consumables);
        combined
    } else {
        Vec::new()
    }
}

/// Selects up to 3 boots sorted by absolute games played descending without hardcoded padding.
pub(crate) fn select_top_boots(
    boot_entries: &[ItemStatEntry],
    candidate_entries: &[ItemStatEntry],
    is_boot: &dyn Fn(i32) -> bool,
    is_starter: &dyn Fn(i32) -> bool,
    is_comp: &dyn Fn(i32) -> bool,
) -> Vec<i32> {
    let mut boot_map: HashMap<i32, (i64, usize)> = HashMap::new();
    let mut boot_seen_counter = 0;
    for entry in boot_entries {
        for &id in &entry.ids {
            if is_boot(id) || (!is_starter(id) && !is_comp(id)) {
                let slot = boot_map.entry(id).or_insert_with(|| {
                    let idx = boot_seen_counter;
                    boot_seen_counter += 1;
                    (0, idx)
                });
                slot.0 += entry.play.max(1);
            }
        }
    }
    // Only supplement with candidate_entries if boot_entries provided fewer than 3 boots
    if boot_map.len() < 3 {
        for entry in candidate_entries {
            for &id in &entry.ids {
                if is_boot(id) {
                    let slot = boot_map.entry(id).or_insert_with(|| {
                        let idx = boot_seen_counter;
                        boot_seen_counter += 1;
                        (0, idx)
                    });
                    slot.0 += entry.play.max(1);
                }
            }
        }
    }
    let mut boot_pairs: Vec<(i32, i64, usize)> = boot_map
        .into_iter()
        .map(|(id, (play, idx))| (id, play, idx))
        .collect();
    boot_pairs.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.2.cmp(&b.2)));
    let mut boots_list: Vec<i32> = boot_pairs.into_iter().map(|(id, _, _)| id).collect();
    boots_list.truncate(3);
    boots_list
}
