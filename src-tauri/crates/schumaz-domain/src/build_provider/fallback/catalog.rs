use super::super::items::{
    is_boot_in, is_component_in, is_consumable_in, is_offensive_ad_tag, is_offensive_ap_tag,
    is_starter_in, is_tear_completed_item, item_tags_in,
};
use super::super::models::ItemClassificationCache;
use super::archetype::ChampionArchetype;

fn score_tags_for_archetype(
    tags: &[String],
    archetype: ChampionArchetype,
    weights: &[(&str, i32)],
) -> i32 {
    let has_ap = tags.iter().any(|t| is_offensive_ap_tag(t));
    let has_ad = tags.iter().any(|t| is_offensive_ad_tag(t));
    match archetype {
        ChampionArchetype::Mage | ChampionArchetype::AssassinAp | ChampionArchetype::Support => {
            if has_ad && !has_ap {
                return -10_000;
            }
        }
        ChampionArchetype::Marksman
        | ChampionArchetype::Fighter
        | ChampionArchetype::AssassinAd
        | ChampionArchetype::Tank => {
            if has_ap && !has_ad {
                return -10_000;
            }
        }
    }
    weights
        .iter()
        .filter(|&&(tag, _)| tags.iter().any(|t| t == tag))
        .map(|&(_, w)| w)
        .sum()
}

#[derive(Debug, Default)]
pub(super) struct FallbackItemBlocks {
    pub starter: Vec<i32>,
    pub core: Vec<i32>,
    pub boots: Vec<i32>,
    pub final_build: Vec<i32>,
    pub situational: Vec<i32>,
}

pub(super) fn derive_fallback_items(
    cls: &ItemClassificationCache,
    queue_id: i32,
    archetype: ChampionArchetype,
    uses_mana: bool,
) -> FallbackItemBlocks {
    let is_aram = queue_id == 450 || queue_id == crate::models::QUEUE_ARAM_MAYHEM;
    let map_items = if is_aram { &cls.map12_items } else { &cls.map11_items };
    if map_items.is_empty() {
        return FallbackItemBlocks::default();
    }

    let gold_of = |id: i32| cls.item_gold.get(&id).copied().unwrap_or(0);
    let starter_weights: &[(&str, i32)] = match archetype {
        ChampionArchetype::Mage | ChampionArchetype::AssassinAp => {
            &[("SpellDamage", 100), ("ManaRegen", 40), ("Mana", 40), ("Health", 20)]
        }
        ChampionArchetype::Marksman | ChampionArchetype::Fighter | ChampionArchetype::AssassinAd => {
            &[("Damage", 100), ("LifeSteal", 30), ("AttackSpeed", 30), ("Health", 20)]
        }
        ChampionArchetype::Tank => {
            &[("Health", 100), ("HealthRegen", 50), ("Armor", 50), ("SpellBlock", 50)]
        }
        ChampionArchetype::Support => &[("ManaRegen", 80), ("Health", 40), ("SpellDamage", 30)],
    };
    let mut starter_cands: Vec<(i32, i32, i32)> = cls
        .starters
        .iter()
        .copied()
        .filter(|&id| map_items.contains(&id) && !is_consumable_in(cls, id))
        .filter(|&id| !item_tags_in(cls, id).iter().any(|t| t == "Jungle" || t == "GoldPer"))
        .filter_map(|id| {
            let mut score = score_tags_for_archetype(item_tags_in(cls, id), archetype, starter_weights);
            if score <= 0 {
                return None;
            }
            let gold = gold_of(id);
            if is_aram && gold >= 700 {
                score += 40;
            }
            Some((id, score, gold))
        })
        .collect();
    starter_cands.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| b.2.cmp(&a.2)).then_with(|| a.0.cmp(&b.0)));

    let mut starter_ids = Vec::new();
    if let Some(&(best_starter, _, _)) = starter_cands.first() {
        starter_ids.push(best_starter);
    }
    let mut potion_cands: Vec<(i32, i32)> = cls
        .consumables
        .iter()
        .copied()
        .filter(|&id| map_items.contains(&id) && item_tags_in(cls, id).iter().any(|t| t == "HealthRegen"))
        .map(|id| (id, gold_of(id)))
        .collect();
    potion_cands.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
    if let Some(&(pot_id, _)) = potion_cands.first() {
        starter_ids.push(pot_id);
    }

    let boot_weights: &[(&str, i32)] = match archetype {
        ChampionArchetype::Mage | ChampionArchetype::AssassinAp => &[
            ("MagicPenetration", 100), ("CooldownReduction", 70), ("AbilityHaste", 70),
            ("SpellBlock", 30), ("Armor", 20),
        ],
        ChampionArchetype::Marksman => &[("AttackSpeed", 100), ("Armor", 40), ("SpellBlock", 40), ("NonbootsMovement", 30)],
        ChampionArchetype::Fighter => &[
            ("Armor", 80), ("SpellBlock", 80), ("Tenacity", 40),
            ("CooldownReduction", 60), ("AbilityHaste", 60), ("AttackSpeed", 30),
        ],
        ChampionArchetype::AssassinAd => &[
            ("CooldownReduction", 90), ("AbilityHaste", 90), ("SpellBlock", 60),
            ("Armor", 50), ("NonbootsMovement", 40),
        ],
        ChampionArchetype::Tank => &[("Armor", 100), ("SpellBlock", 100), ("Tenacity", 60), ("NonbootsMovement", 50)],
        ChampionArchetype::Support => &[("CooldownReduction", 100), ("AbilityHaste", 100), ("NonbootsMovement", 60), ("Armor", 40), ("SpellBlock", 40)],
    };
    let mut boot_cands: Vec<(i32, i32, i32)> = cls
        .boots
        .iter()
        .copied()
        .filter(|&id| map_items.contains(&id) && (800..=1500).contains(&gold_of(id)))
        .map(|id| (id, score_tags_for_archetype(item_tags_in(cls, id), archetype, boot_weights), gold_of(id)))
        .collect();
    boot_cands.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| b.2.cmp(&a.2)).then_with(|| a.0.cmp(&b.0)));
    let boots_items: Vec<i32> = boot_cands.into_iter().take(3).map(|(id, _, _)| id).collect();

    let comp_weights: &[(&str, i32)] = match archetype {
        ChampionArchetype::Mage | ChampionArchetype::AssassinAp => &[
            ("SpellDamage", 100), ("MagicPenetration", 80), ("AbilityHaste", 50),
            ("CooldownReduction", 50), ("Mana", 30), ("Health", 20), ("Armor", 20), ("SpellBlock", 20),
        ],
        ChampionArchetype::Marksman => &[
            ("Damage", 90), ("CriticalStrike", 80), ("AttackSpeed", 70), ("ArmorPenetration", 60), ("LifeSteal", 40),
        ],
        ChampionArchetype::Fighter => &[
            ("Damage", 90), ("Health", 70), ("AbilityHaste", 50), ("CooldownReduction", 50),
            ("AttackSpeed", 40), ("LifeSteal", 40), ("ArmorPenetration", 40), ("Armor", 30), ("SpellBlock", 30),
        ],
        ChampionArchetype::AssassinAd => &[
            ("Damage", 100), ("ArmorPenetration", 90), ("AbilityHaste", 60), ("CooldownReduction", 60),
            ("NonbootsMovement", 30), ("Armor", 15), ("SpellBlock", 15),
        ],
        ChampionArchetype::Tank => &[
            ("Health", 100), ("Armor", 80), ("SpellBlock", 80), ("HealthRegen", 40), ("Tenacity", 40), ("AbilityHaste", 40),
        ],
        ChampionArchetype::Support => &[
            ("ManaRegen", 100), ("Health", 70), ("AbilityHaste", 70), ("CooldownReduction", 70),
            ("SpellDamage", 60), ("Armor", 30), ("SpellBlock", 30),
        ],
    };
    let mut comp_cands: Vec<(i32, i32, i32)> = cls
        .completed
        .iter()
        .copied()
        .filter(|&id| {
            map_items.contains(&id)
                && !is_boot_in(cls, id)
                && !is_starter_in(cls, id)
                && !is_component_in(cls, id)
                && !is_consumable_in(cls, id)
        })
        .filter_map(|id| {
            let tags = item_tags_in(cls, id);
            if !uses_mana && tags.iter().any(|t| t == "Mana") {
                return None;
            }
            let score = score_tags_for_archetype(tags, archetype, comp_weights);
            (score > 0).then_some((id, score, gold_of(id)))
        })
        .collect();
    comp_cands.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| b.2.cmp(&a.2)).then_with(|| a.0.cmp(&b.0)));

    let mut ranked = Vec::new();
    let mut has_tear = false;
    for (id, _, _) in comp_cands {
        if is_tear_completed_item(id) {
            if has_tear {
                continue;
            }
            has_tear = true;
        }
        ranked.push(id);
        if ranked.len() == 12 {
            break;
        }
    }

    FallbackItemBlocks {
        starter: starter_ids,
        core: ranked.iter().take(3).copied().collect(),
        boots: boots_items,
        final_build: ranked.iter().take(6).copied().collect(),
        situational: ranked.iter().skip(6).take(6).copied().collect(),
    }
}
