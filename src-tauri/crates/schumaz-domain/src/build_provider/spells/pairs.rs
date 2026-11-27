use super::super::models::{SpellMetadata, SpellPairOption};
use super::modes::{
    SPELL_FLASH_ID, SPELL_SMITE_ID, fallback_secondary_spells_for_mode, is_spell_allowed_in_mode,
    sanitize_spell_pair_for_mode,
};
use std::collections::HashMap;

/// Normalizes a summoner spell pair order before user D/F preference is applied:
/// - Non-jungle: Flash (4) defaults to slot 0 when present.
/// - Jungle: Flash (4) defaults to slot 0 when present; otherwise Smite (11) defaults to slot 1.
pub fn normalize_spell_pair_order(s1: i32, s2: i32, is_jungle: bool) -> (i32, i32) {
    if is_jungle {
        if s1 == SPELL_FLASH_ID {
            (SPELL_FLASH_ID, s2)
        } else if s2 == SPELL_FLASH_ID {
            (SPELL_FLASH_ID, s1)
        } else if s1 == SPELL_SMITE_ID && s2 != SPELL_SMITE_ID {
            (s2, SPELL_SMITE_ID)
        } else {
            (s1, s2)
        }
    } else if s2 == SPELL_FLASH_ID && s1 != SPELL_FLASH_ID {
        (SPELL_FLASH_ID, s1)
    } else {
        (s1, s2)
    }
}

pub(crate) fn unordered_spell_pair_key(s1: i32, s2: i32) -> (i32, i32) {
    (s1.min(s2), s1.max(s2))
}

pub(crate) fn is_valid_pair_for_mode(
    s1: i32,
    s2: i32,
    is_jungle: bool,
    mode: Option<&str>,
    spells_catalog: Option<&HashMap<i32, SpellMetadata>>,
) -> bool {
    s1 > 0
        && s2 > 0
        && s1 != s2
        && is_spell_allowed_in_mode(s1, mode, spells_catalog)
        && is_spell_allowed_in_mode(s2, mode, spells_catalog)
        && (!is_jungle
            || !is_spell_allowed_in_mode(SPELL_SMITE_ID, mode, spells_catalog)
            || s1 == SPELL_SMITE_ID
            || s2 == SPELL_SMITE_ID)
}

/// Validates the primary `(spell1_id, spell2_id)` pair against `mode` and Jungle rules,
/// falling back to the first valid pair in `existing_options` or mode-aware defaults.
pub fn sanitize_top_pair_from_options(
    spell1_id: i32,
    spell2_id: i32,
    existing_options: &[SpellPairOption],
    is_jungle: bool,
    mode: Option<&str>,
    spells_catalog: Option<&HashMap<i32, SpellMetadata>>,
) -> (i32, i32) {
    if is_valid_pair_for_mode(spell1_id, spell2_id, is_jungle, mode, spells_catalog) {
        return normalize_spell_pair_order(spell1_id, spell2_id, is_jungle);
    }
    if let Some(opt) = existing_options.iter().find(|opt| {
        is_valid_pair_for_mode(opt.spell1_id, opt.spell2_id, is_jungle, mode, spells_catalog)
    }) {
        return normalize_spell_pair_order(opt.spell1_id, opt.spell2_id, is_jungle);
    }
    let preferred_ids: Vec<i32> = existing_options
        .iter()
        .flat_map(|opt| [opt.spell1_id, opt.spell2_id])
        .filter(|&id| is_spell_allowed_in_mode(id, mode, spells_catalog))
        .collect();
    let (s1, s2) = sanitize_spell_pair_for_mode(
        spell1_id,
        spell2_id,
        &preferred_ids,
        is_jungle,
        mode,
        spells_catalog,
    );
    normalize_spell_pair_order(s1, s2, is_jungle)
}

/// Builds up to 2 valid, distinct summoner spell pair options for `mode`, preserving real
/// pair stats when available and synthesizing mode-aware fallback pairs when fewer than 2 exist.
pub fn sanitize_spell_pair_options_for_mode(
    top_spell1_id: i32,
    top_spell2_id: i32,
    preferred_pairs: &[SpellPairOption],
    is_jungle: bool,
    mode: Option<&str>,
    spells_catalog: Option<&HashMap<i32, SpellMetadata>>,
) -> Vec<SpellPairOption> {
    let mut options: Vec<SpellPairOption> = Vec::with_capacity(2);
    let mut seen_keys: Vec<(i32, i32)> = Vec::with_capacity(2);

    if is_valid_pair_for_mode(top_spell1_id, top_spell2_id, is_jungle, mode, spells_catalog) {
        let top_key = unordered_spell_pair_key(top_spell1_id, top_spell2_id);
        let (norm_s1, norm_s2) = normalize_spell_pair_order(top_spell1_id, top_spell2_id, is_jungle);
        let stats = preferred_pairs
            .iter()
            .find(|p| unordered_spell_pair_key(p.spell1_id, p.spell2_id) == top_key);
        options.push(SpellPairOption {
            spell1_id: norm_s1,
            spell2_id: norm_s2,
            play: stats.map_or(0, |s| s.play),
            win: stats.map_or(0, |s| s.win),
            pick_rate: stats.map_or(0.0, |s| s.pick_rate),
            spells: Vec::new(),
        });
        seen_keys.push(top_key);
    }

    for pair in preferred_pairs {
        if options.len() >= 2 {
            break;
        }
        if !is_valid_pair_for_mode(pair.spell1_id, pair.spell2_id, is_jungle, mode, spells_catalog) {
            continue;
        }
        let key = unordered_spell_pair_key(pair.spell1_id, pair.spell2_id);
        if seen_keys.contains(&key) {
            continue;
        }
        let (norm_s1, norm_s2) =
            normalize_spell_pair_order(pair.spell1_id, pair.spell2_id, is_jungle);
        options.push(SpellPairOption {
            spell1_id: norm_s1,
            spell2_id: norm_s2,
            play: pair.play,
            win: pair.win,
            pick_rate: pair.pick_rate,
            spells: Vec::new(),
        });
        seen_keys.push(key);
    }

    if options.len() < 2 {
        let base_pair = options
            .first()
            .map(|o| (o.spell1_id, o.spell2_id))
            .unwrap_or((top_spell1_id, top_spell2_id));
        if !is_jungle
            && base_pair.0 != SPELL_FLASH_ID
            && base_pair.1 != SPELL_FLASH_ID
            && is_valid_pair_for_mode(SPELL_FLASH_ID, base_pair.0, is_jungle, mode, spells_catalog)
        {
            let key = unordered_spell_pair_key(SPELL_FLASH_ID, base_pair.0);
            if !seen_keys.contains(&key) {
                options.push(SpellPairOption {
                    spell1_id: SPELL_FLASH_ID,
                    spell2_id: base_pair.0,
                    ..Default::default()
                });
                seen_keys.push(key);
            }
        }
        let anchor = if is_jungle {
            SPELL_SMITE_ID
        } else if base_pair.0 == SPELL_FLASH_ID || base_pair.1 == SPELL_FLASH_ID {
            SPELL_FLASH_ID
        } else {
            base_pair.0
        };
        for &cand in fallback_secondary_spells_for_mode(is_jungle, mode) {
            if options.len() >= 2 {
                break;
            }
            if cand == anchor || !is_valid_pair_for_mode(anchor, cand, is_jungle, mode, spells_catalog)
            {
                continue;
            }
            let key = unordered_spell_pair_key(anchor, cand);
            if seen_keys.contains(&key) {
                continue;
            }
            let (norm_s1, norm_s2) = normalize_spell_pair_order(anchor, cand, is_jungle);
            options.push(SpellPairOption {
                spell1_id: norm_s1,
                spell2_id: norm_s2,
                ..Default::default()
            });
            seen_keys.push(key);
        }
    }

    options.truncate(2);
    options
}
