use super::preferences::FlashKeyPreference;

pub const SPELL_FLASH_ID: i32 = 4;
pub const SPELL_SMITE_ID: i32 = 11;

/// Applies Flash key preference (D vs F), swapping spells if needed so that Flash (ID 4)
/// matches the user's preferred slot (spell1Id = D, spell2Id = F).
pub fn apply_flash_key_preference(
    spell1_id: i32,
    spell2_id: i32,
    flash_key: FlashKeyPreference,
) -> (i32, i32) {
    let should_swap = (spell1_id == SPELL_FLASH_ID && flash_key == FlashKeyPreference::F)
        || (spell2_id == SPELL_FLASH_ID && flash_key == FlashKeyPreference::D);

    if should_swap {
        (spell2_id, spell1_id)
    } else {
        (spell1_id, spell2_id)
    }
}

/// Applies mandatory Jungle Smite rule (ID 11) and positions Flash (ID 4)
/// according to the user's preferred slot (D vs F).
pub fn apply_jungle_smite_and_flash_preference(
    mut spell1_id: i32,
    mut spell2_id: i32,
    flash_key: FlashKeyPreference,
    is_jungle: bool,
) -> (i32, i32) {
    if !is_jungle {
        return apply_flash_key_preference(spell1_id, spell2_id, flash_key);
    }

    // 1. Ensure Smite (11) is present
    if spell1_id != SPELL_SMITE_ID && spell2_id != SPELL_SMITE_ID {
        if spell1_id == SPELL_FLASH_ID {
            spell2_id = SPELL_SMITE_ID;
        } else if spell2_id == SPELL_FLASH_ID {
            spell1_id = SPELL_SMITE_ID;
        } else {
            spell2_id = SPELL_SMITE_ID;
        }
    }

    // 2. Position Flash and Smite respecting flash_key
    if spell1_id == SPELL_FLASH_ID || spell2_id == SPELL_FLASH_ID {
        let other_spell = if spell1_id == SPELL_FLASH_ID {
            spell2_id
        } else {
            spell1_id
        };
        match flash_key {
            FlashKeyPreference::D => (SPELL_FLASH_ID, other_spell),
            FlashKeyPreference::F => (other_spell, SPELL_FLASH_ID),
        }
    } else {
        // Champion doesn't use Flash (e.g. Ghost 6 + Smite 11 or Ignite 14 + Smite 11)
        (spell1_id, spell2_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_flash_key_preference_flash_on_d() {
        // Flash is already on D (spell1) -> stays (4, 14)
        assert_eq!(
            apply_flash_key_preference(4, 14, FlashKeyPreference::D),
            (4, 14)
        );

        // Flash is on F (spell2) -> inverted to D (4, 14)
        assert_eq!(
            apply_flash_key_preference(14, 4, FlashKeyPreference::D),
            (4, 14)
        );
    }

    #[test]
    fn test_apply_flash_key_preference_flash_on_f() {
        // Flash is on D (spell1) -> inverted to F (14, 4)
        assert_eq!(
            apply_flash_key_preference(4, 14, FlashKeyPreference::F),
            (14, 4)
        );

        // Flash is already on F (spell2) -> stays (14, 4)
        assert_eq!(
            apply_flash_key_preference(14, 4, FlashKeyPreference::F),
            (14, 4)
        );
    }

    #[test]
    fn test_apply_flash_key_preference_without_flash_preserves_order() {
        // Neither spell is Flash (e.g. Teleport 12 + Ignite 14) -> order preserved
        assert_eq!(
            apply_flash_key_preference(12, 14, FlashKeyPreference::D),
            (12, 14)
        );
        assert_eq!(
            apply_flash_key_preference(12, 14, FlashKeyPreference::F),
            (12, 14)
        );
    }

    #[test]
    fn test_jungle_smite_rule_and_flash_preference() {
        // Scenario 1: Jungle with Flash on D -> Flash (4) on D, Smite (11) on F
        assert_eq!(
            apply_jungle_smite_and_flash_preference(4, 14, FlashKeyPreference::D, true),
            (4, 11)
        );

        // Scenario 2: Jungle with Flash on F -> Smite (11) on D, Flash (4) on F
        assert_eq!(
            apply_jungle_smite_and_flash_preference(4, 14, FlashKeyPreference::F, true),
            (11, 4)
        );

        // Scenario 3: Jungle with already Smite + Flash
        assert_eq!(
            apply_jungle_smite_and_flash_preference(11, 4, FlashKeyPreference::D, true),
            (4, 11)
        );
        assert_eq!(
            apply_jungle_smite_and_flash_preference(11, 4, FlashKeyPreference::F, true),
            (11, 4)
        );

        // Scenario 4: Jungle without Flash (Ghost 6 + Ignite 14) -> Ghost 6 + Smite 11
        assert_eq!(
            apply_jungle_smite_and_flash_preference(6, 14, FlashKeyPreference::D, true),
            (6, 11)
        );

        // Scenario 5: Non-jungle ignores jungle rule and only applies standard flash preference
        assert_eq!(
            apply_jungle_smite_and_flash_preference(4, 14, FlashKeyPreference::F, false),
            (14, 4)
        );
    }
}
