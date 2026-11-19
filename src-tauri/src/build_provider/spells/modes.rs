use super::super::models::SpellMetadata;
use std::collections::HashMap;

pub const SPELL_CLEANSE_ID: i32 = 1;
pub const SPELL_EXHAUST_ID: i32 = 3;
pub const SPELL_FLASH_ID: i32 = 4;
pub const SPELL_GHOST_ID: i32 = 6;
pub const SPELL_HEAL_ID: i32 = 7;
pub const SPELL_SMITE_ID: i32 = 11;
pub const SPELL_TELEPORT_ID: i32 = 12;
pub const SPELL_CLARITY_ID: i32 = 13;
pub const SPELL_IGNITE_ID: i32 = 14;
pub const SPELL_BARRIER_ID: i32 = 21;
pub const SPELL_MARK_ID: i32 = 32;

/// Priority fallback order for ARAM / ARAM Mayhem secondary spells.
pub const ARAM_FALLBACK_SPELLS: [i32; 6] = [
    SPELL_MARK_ID,
    SPELL_HEAL_ID,
    SPELL_BARRIER_ID,
    SPELL_GHOST_ID,
    SPELL_IGNITE_ID,
    SPELL_CLEANSE_ID,
];

/// Priority fallback order for Summoner's Rift (non-jungle) secondary spells.
pub const CLASSIC_FALLBACK_SPELLS: [i32; 7] = [
    SPELL_IGNITE_ID,
    SPELL_TELEPORT_ID,
    SPELL_BARRIER_ID,
    SPELL_HEAL_ID,
    SPELL_EXHAUST_ID,
    SPELL_GHOST_ID,
    SPELL_CLEANSE_ID,
];

/// Priority fallback order for Jungle companion spells paired with Smite (11).
pub const JUNGLE_FALLBACK_SPELLS: [i32; 3] = [SPELL_FLASH_ID, SPELL_GHOST_ID, SPELL_IGNITE_ID];

/// Maps an LCU `queueId` into Riot DataDragon's `summoner.json` mode identifier.
pub fn ddragon_mode_for_queue(queue_id: i32) -> &'static str {
    match queue_id {
        crate::models::QUEUE_ARAM_MAYHEM => "KIWI",
        crate::models::QUEUE_ARAM => "ARAM",
        _ => "CLASSIC",
    }
}

pub(crate) fn is_aram_mode(mode: &str) -> bool {
    mode.eq_ignore_ascii_case("ARAM") || mode.eq_ignore_ascii_case("KIWI")
}

/// Default Riot mode support per known spell ID when DataDragon `summoner.json` modes are absent.
pub fn default_spell_modes(spell_id: i32) -> Option<&'static [&'static str]> {
    match spell_id {
        SPELL_FLASH_ID | SPELL_IGNITE_ID | SPELL_HEAL_ID | SPELL_BARRIER_ID | SPELL_GHOST_ID
        | SPELL_CLEANSE_ID => Some(&["CLASSIC", "ARAM", "KIWI"]),
        SPELL_EXHAUST_ID => Some(&["CLASSIC", "ARAM"]),
        SPELL_SMITE_ID | SPELL_TELEPORT_ID => Some(&["CLASSIC"]),
        SPELL_CLARITY_ID | SPELL_MARK_ID => Some(&["ARAM", "KIWI"]),
        _ => None,
    }
}

/// Returns true when `spell_id` is supported in `mode` according to DataDragon's `modes` list,
/// falling back to `default_spell_modes` if the catalog lacks mode metadata for that spell.
pub fn is_spell_allowed_in_mode(
    spell_id: i32,
    mode: Option<&str>,
    spells_catalog: Option<&HashMap<i32, SpellMetadata>>,
) -> bool {
    if spell_id <= 0 {
        return false;
    }
    let Some(target_mode) = mode else {
        return true;
    };
    if let Some(catalog) = spells_catalog
        && let Some(meta) = catalog.get(&spell_id)
        && !meta.modes.is_empty()
    {
        return meta
            .modes
            .iter()
            .any(|m| m.eq_ignore_ascii_case(target_mode));
    }
    match default_spell_modes(spell_id) {
        Some(modes) => modes.iter().any(|m| m.eq_ignore_ascii_case(target_mode)),
        None => true,
    }
}

/// Returns the ordered secondary spell fallback list appropriate for the active role and mode.
pub fn fallback_secondary_spells_for_mode(is_jungle: bool, mode: Option<&str>) -> &'static [i32] {
    if is_jungle {
        &JUNGLE_FALLBACK_SPELLS
    } else if mode.is_some_and(is_aram_mode) {
        &ARAM_FALLBACK_SPELLS
    } else {
        &CLASSIC_FALLBACK_SPELLS
    }
}

fn pick_first_allowed_spell(
    preferred: &[i32],
    fallbacks: &[i32],
    exclude: i32,
    mode: Option<&str>,
    spells_catalog: Option<&HashMap<i32, SpellMetadata>>,
) -> i32 {
    for &id in preferred.iter().chain(fallbacks.iter()) {
        if id != exclude && is_spell_allowed_in_mode(id, mode, spells_catalog) {
            return id;
        }
    }
    if exclude != SPELL_FLASH_ID && is_spell_allowed_in_mode(SPELL_FLASH_ID, mode, spells_catalog) {
        return SPELL_FLASH_ID;
    }
    if mode.is_some_and(is_aram_mode) {
        SPELL_MARK_ID
    } else {
        SPELL_IGNITE_ID
    }
}

/// Ensures `(spell1_id, spell2_id)` are both allowed in `mode` and distinct, preserving valid
/// anchor spells (Flash 4 or Smite 11 in jungle) and replacing disallowed spells from
/// `preferred_choices` before falling back to mode-aware defaults.
pub fn sanitize_spell_pair_for_mode(
    spell1_id: i32,
    spell2_id: i32,
    preferred_choices: &[i32],
    is_jungle: bool,
    mode: Option<&str>,
    spells_catalog: Option<&HashMap<i32, SpellMetadata>>,
) -> (i32, i32) {
    let fallbacks = fallback_secondary_spells_for_mode(is_jungle, mode);
    let s1_ok = is_spell_allowed_in_mode(spell1_id, mode, spells_catalog);
    let s2_ok = is_spell_allowed_in_mode(spell2_id, mode, spells_catalog);

    if s1_ok && s2_ok && spell1_id != spell2_id {
        return (spell1_id, spell2_id);
    }

    if is_jungle && is_spell_allowed_in_mode(SPELL_SMITE_ID, mode, spells_catalog) {
        let companion = if s1_ok && spell1_id != SPELL_SMITE_ID {
            spell1_id
        } else if s2_ok && spell2_id != SPELL_SMITE_ID {
            spell2_id
        } else {
            pick_first_allowed_spell(
                preferred_choices,
                fallbacks,
                SPELL_SMITE_ID,
                mode,
                spells_catalog,
            )
        };
        return (companion, SPELL_SMITE_ID);
    }

    if s1_ok {
        let replacement = pick_first_allowed_spell(
            preferred_choices,
            fallbacks,
            spell1_id,
            mode,
            spells_catalog,
        );
        return (spell1_id, replacement);
    }
    if s2_ok {
        let replacement = pick_first_allowed_spell(
            preferred_choices,
            fallbacks,
            spell2_id,
            mode,
            spells_catalog,
        );
        return if spell2_id == SPELL_FLASH_ID {
            (SPELL_FLASH_ID, replacement)
        } else {
            (replacement, spell2_id)
        };
    }

    let primary = if is_spell_allowed_in_mode(SPELL_FLASH_ID, mode, spells_catalog) {
        SPELL_FLASH_ID
    } else {
        pick_first_allowed_spell(preferred_choices, fallbacks, 0, mode, spells_catalog)
    };
    let secondary =
        pick_first_allowed_spell(preferred_choices, fallbacks, primary, mode, spells_catalog);
    (primary, secondary)
}

/// Builds up to 2 valid, distinct secondary spell choices for `(spell1_id, spell2_id)` in `mode`,
/// filtering out unsupported spells and filling missing slots with mode-aware fallbacks.
pub fn sanitize_spell_choices_for_mode(
    spell1_id: i32,
    spell2_id: i32,
    preferred_choices: &[i32],
    is_jungle: bool,
    mode: Option<&str>,
    spells_catalog: Option<&HashMap<i32, SpellMetadata>>,
) -> Vec<i32> {
    let primary_spell = if is_jungle {
        SPELL_SMITE_ID
    } else if spell1_id == SPELL_FLASH_ID || spell2_id == SPELL_FLASH_ID {
        SPELL_FLASH_ID
    } else {
        spell1_id
    };
    let active_secondary = if spell1_id == primary_spell {
        spell2_id
    } else {
        spell1_id
    };

    let mut choices: Vec<i32> = Vec::with_capacity(2);
    if active_secondary != primary_spell
        && is_spell_allowed_in_mode(active_secondary, mode, spells_catalog)
    {
        choices.push(active_secondary);
    }

    let fallbacks = fallback_secondary_spells_for_mode(is_jungle, mode);
    for &candidate in preferred_choices.iter().chain(fallbacks.iter()) {
        if choices.len() >= 2 {
            break;
        }
        if candidate != primary_spell
            && !choices.contains(&candidate)
            && is_spell_allowed_in_mode(candidate, mode, spells_catalog)
        {
            choices.push(candidate);
        }
    }

    choices.truncate(2);
    choices
}
