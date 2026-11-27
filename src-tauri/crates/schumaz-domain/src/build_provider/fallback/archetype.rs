use super::super::models::{ChampionProfile, DataDragonCache, RuneTreeCatalog};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChampionArchetype {
    Mage,
    Marksman,
    Fighter,
    AssassinAd,
    AssassinAp,
    Tank,
    Support,
}

/// Infers a champion's combat archetype purely from DataDragon `tags` and `info` ratings.
pub fn infer_champion_archetype(profile: &ChampionProfile) -> ChampionArchetype {
    let has_tag = |target: &str| profile.tags.iter().any(|t| t.eq_ignore_ascii_case(target));
    if has_tag("Marksman") {
        return ChampionArchetype::Marksman;
    }
    if has_tag("Tank")
        && (profile.defense > profile.magic || (!has_tag("Mage") && !has_tag("Support")))
        && !has_tag("Fighter")
    {
        return ChampionArchetype::Tank;
    }
    if has_tag("Support") {
        return if profile.magic > profile.attack && has_tag("Mage") {
            ChampionArchetype::Mage
        } else if has_tag("Tank") && profile.defense >= profile.magic {
            ChampionArchetype::Tank
        } else {
            ChampionArchetype::Support
        };
    }
    if has_tag("Assassin") {
        return if profile.magic > profile.attack {
            ChampionArchetype::AssassinAp
        } else {
            ChampionArchetype::AssassinAd
        };
    }
    if has_tag("Mage") {
        return if profile.attack > profile.magic + 2 {
            ChampionArchetype::Fighter
        } else {
            ChampionArchetype::Mage
        };
    }
    if has_tag("Fighter") {
        return if profile.magic > profile.attack + 2 {
            ChampionArchetype::Mage
        } else {
            ChampionArchetype::Fighter
        };
    }
    if has_tag("Tank") {
        return ChampionArchetype::Tank;
    }
    if profile.magic > profile.attack && profile.magic >= profile.defense {
        ChampionArchetype::Mage
    } else if profile.defense > profile.attack && profile.defense > profile.magic {
        ChampionArchetype::Tank
    } else {
        ChampionArchetype::Fighter
    }
}

/// Infers `"AP"` or `"AD"` dynamically from a champion's DataDragon `ChampionProfile`.
pub fn infer_damage_type_from_profile(profile: &ChampionProfile) -> &'static str {
    match infer_champion_archetype(profile) {
        ChampionArchetype::Mage | ChampionArchetype::AssassinAp | ChampionArchetype::Support => {
            if profile.attack > profile.magic + 1 {
                "AD"
            } else {
                "AP"
            }
        }
        ChampionArchetype::Tank => {
            if profile.magic > profile.attack {
                "AP"
            } else {
                "AD"
            }
        }
        ChampionArchetype::Marksman
        | ChampionArchetype::Fighter
        | ChampionArchetype::AssassinAd => "AD",
    }
}

fn select_tree_by_preference<'a>(
    trees: &'a [RuneTreeCatalog],
    prefs: &[&str],
    min_slots: usize,
    exclude_id: Option<i32>,
) -> Option<&'a RuneTreeCatalog> {
    for &pref in prefs {
        if let Some(tree) = trees.iter().find(|t| {
            Some(t.id) != exclude_id
                && t.slots.len() >= min_slots
                && t.key.eq_ignore_ascii_case(pref)
        }) {
            return Some(tree);
        }
    }
    trees
        .iter()
        .find(|t| Some(t.id) != exclude_id && t.slots.len() >= min_slots)
}

pub(super) fn derive_fallback_runes(
    ddragon: Option<&DataDragonCache>,
    archetype: ChampionArchetype,
) -> (i32, i32, Vec<i32>) {
    let Some(dd) = ddragon.filter(|d| !d.rune_trees.is_empty()) else {
        return (0, 0, Vec::new());
    };
    let (primary_prefs, secondary_prefs): (&[&str], &[&str]) = match archetype {
        ChampionArchetype::Mage => (
            &["Sorcery", "Domination", "Precision", "Inspiration"],
            &["Inspiration", "Domination", "Sorcery", "Resolve"],
        ),
        ChampionArchetype::Marksman => (
            &["Precision", "Domination", "Sorcery"],
            &["Inspiration", "Domination", "Resolve", "Sorcery"],
        ),
        ChampionArchetype::Fighter => (
            &["Precision", "Resolve", "Domination"],
            &["Resolve", "Inspiration", "Domination", "Precision"],
        ),
        ChampionArchetype::AssassinAd => (
            &["Domination", "Precision", "Sorcery"],
            &["Precision", "Inspiration", "Sorcery", "Resolve"],
        ),
        ChampionArchetype::AssassinAp => (
            &["Domination", "Sorcery", "Precision"],
            &["Sorcery", "Inspiration", "Precision", "Resolve"],
        ),
        ChampionArchetype::Tank => (
            &["Resolve", "Precision", "Inspiration"],
            &["Precision", "Inspiration", "Sorcery", "Domination"],
        ),
        ChampionArchetype::Support => (
            &["Sorcery", "Resolve", "Inspiration"],
            &["Inspiration", "Resolve", "Sorcery"],
        ),
    };

    let Some(primary) = select_tree_by_preference(&dd.rune_trees, primary_prefs, 4, None) else {
        return (0, 0, Vec::new());
    };
    let Some(secondary) =
        select_tree_by_preference(&dd.rune_trees, secondary_prefs, 3, Some(primary.id))
    else {
        return (0, 0, Vec::new());
    };

    let mut perks = Vec::with_capacity(9);
    for slot in primary.slots.iter().take(4) {
        let Some(&perk_id) = slot.first() else {
            return (0, 0, Vec::new());
        };
        perks.push(perk_id);
    }
    let Some(&sec1) = secondary.slots.get(1).and_then(|s| s.first()) else {
        return (0, 0, Vec::new());
    };
    let Some(&sec2) = secondary.slots.get(2).and_then(|s| s.first()) else {
        return (0, 0, Vec::new());
    };
    perks.push(sec1);
    perks.push(sec2);

    let shards = match archetype {
        ChampionArchetype::Marksman | ChampionArchetype::Fighter => [5005, 5008, 5001],
        ChampionArchetype::Tank => [5007, 5001, 5011],
        _ => [5008, 5008, 5001],
    };
    perks.extend(shards);
    (primary.id, secondary.id, perks)
}
