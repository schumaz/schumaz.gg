use super::classification::*;

pub fn is_offensive_ad_tag(tag: &str) -> bool {
    matches!(
        tag,
        "Damage" | "CriticalStrike" | "LifeSteal" | "ArmorPenetration" | "AttackSpeed" | "OnHit"
    )
}

pub fn is_offensive_ap_tag(tag: &str) -> bool {
    matches!(tag, "SpellDamage" | "MagicPenetration")
}

pub fn is_universal_defense_tag(tag: &str) -> bool {
    matches!(tag, "Armor" | "SpellBlock" | "Health" | "HealthRegen" | "Tenacity")
}

pub fn is_utility_mana_tag(tag: &str) -> bool {
    matches!(tag, "Mana" | "AbilityHaste" | "CooldownReduction" | "NonbootsMovement")
}

/// Computes dynamic synergy affinity score between a candidate item and core tags (zero hardcoded IDs).
pub fn compute_item_affinity_score(
    item_id: i32,
    core_tags: &[String],
    core_is_pure_ad: bool,
    core_is_pure_ap: bool,
) -> i64 {
    let tags = get_item_tags_dynamic(item_id);
    if tags.is_empty() {
        return 0;
    }
    let has_offensive_ap = tags.iter().any(|t| is_offensive_ap_tag(t));
    let has_offensive_ad = tags.iter().any(|t| is_offensive_ad_tag(t));

    // Strict direct offensive archetype conflict blocking:
    // Pure AD archetype rejects purely offensive AP items (e.g. Rabadon, Void Staff)
    if core_is_pure_ad && has_offensive_ap && !has_offensive_ad {
        return -1_000_000;
    }
    // Pure AP archetype rejects purely offensive AD items (e.g. Infinity Edge, Collector, LDR)
    if core_is_pure_ap && has_offensive_ad && !has_offensive_ap {
        return -1_000_000;
    }

    let mut score = 0;

    // Special utility/mana synergy:
    // If core builds mana (e.g. Manamune, Archangel, Tear), an item with Mana (e.g. Frozen Heart) gets mana synergy bonus
    let core_has_mana = core_tags.iter().any(|t| t == "Mana");
    let item_has_mana = tags.iter().any(|t| t == "Mana");
    if core_has_mana && item_has_mana {
        score += 10_000;
    }

    // Direct shared tags synergy
    for t in &tags {
        if core_tags.contains(t) {
            score += 1_000;
        }
    }

    score
}

/// Extracts combined tags and archetype flags (AP/AD/pure) for a core item trio.
pub fn extract_core_archetype_tags(core_items: &[i32]) -> (Vec<String>, bool, bool, bool, bool) {
    let mut core_tags: Vec<String> = Vec::new();
    for &id in core_items {
        for tag in get_item_tags_dynamic(id) {
            if !core_tags.contains(&tag) {
                core_tags.push(tag);
            }
        }
    }
    let core_has_ap = core_tags.iter().any(|t| t == "SpellDamage");
    let core_has_ad = core_tags.iter().any(|t| is_offensive_ad_tag(t));
    let core_is_pure_ad = core_has_ad && !core_has_ap;
    let core_is_pure_ap = core_has_ap && !core_has_ad;
    (core_tags, core_has_ap, core_has_ad, core_is_pure_ad, core_is_pure_ap)
}

/// Dynamically builds a cohesive 6-item final build for a specific core trio
/// using strict sequential slot selection (4th item from fourth_entries, 5th from fifth_entries,
#[cfg(test)]
mod tests {
    use super::*;
        use super::super::classification::TEST_CLASSIFICATION_MUTEX;
    use crate::build_provider::models::ItemClassificationCache;

    #[test]
    fn test_strict_offensive_archetype_blocking_remains_intact() {
        let _lock = TEST_CLASSIFICATION_MUTEX.lock().unwrap();
        let mut cache = ItemClassificationCache::default();
        cache.item_tags.insert(6697, vec!["Damage".to_string(), "ArmorPenetration".to_string()]);
        cache.item_tags.insert(3089, vec!["SpellDamage".to_string()]);
        cache.item_tags.insert(3135, vec!["SpellDamage".to_string(), "MagicPenetration".to_string()]);
        cache.item_tags.insert(3031, vec!["Damage".to_string(), "CriticalStrike".to_string()]);
        cache.item_tags.insert(6676, vec!["Damage".to_string(), "CriticalStrike".to_string()]);
        set_dynamic_item_classifications(cache);

        let ad_core_tags = vec!["Damage".to_string(), "ArmorPenetration".to_string()];
        assert_eq!(compute_item_affinity_score(3089, &ad_core_tags, true, false), -1_000_000);
        assert_eq!(compute_item_affinity_score(3135, &ad_core_tags, true, false), -1_000_000);

        let ap_core_tags = vec!["SpellDamage".to_string()];
        assert_eq!(compute_item_affinity_score(3031, &ap_core_tags, false, true), -1_000_000);
        assert_eq!(compute_item_affinity_score(6676, &ap_core_tags, false, true), -1_000_000);
    }
}
