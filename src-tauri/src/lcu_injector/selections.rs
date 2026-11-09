use std::sync::{OnceLock, RwLock};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ChampionSelections {
    pub champion_id: i32,
    pub rune_page_index: Option<usize>,
    pub secondary_spell_id: Option<i32>,
    pub build_index: Option<usize>,
    pub core_build_index: Option<usize>,
    pub final_build_index: Option<usize>,
}

static CHAMPION_SELECTIONS: OnceLock<RwLock<Option<ChampionSelections>>> = OnceLock::new();

fn get_selections_lock() -> &'static RwLock<Option<ChampionSelections>> {
    CHAMPION_SELECTIONS.get_or_init(|| RwLock::new(None))
}

pub fn set_selected_rune_page(champion_id: i32, page_index: usize) {
    if let Ok(mut guard) = get_selections_lock().write() {
        let current = guard.get_or_insert_with(|| ChampionSelections {
            champion_id,
            rune_page_index: None,
            secondary_spell_id: None,
            build_index: None,
            core_build_index: None,
            final_build_index: None,
        });
        if current.champion_id != champion_id {
            *current = ChampionSelections {
                champion_id,
                rune_page_index: Some(page_index),
                secondary_spell_id: None,
                build_index: None,
                core_build_index: None,
                final_build_index: None,
            };
        } else {
            current.rune_page_index = Some(page_index);
        }
    }
}

pub fn set_selected_secondary_spell(champion_id: i32, spell_id: i32) {
    if let Ok(mut guard) = get_selections_lock().write() {
        let current = guard.get_or_insert_with(|| ChampionSelections {
            champion_id,
            rune_page_index: None,
            secondary_spell_id: None,
            build_index: None,
            core_build_index: None,
            final_build_index: None,
        });
        if current.champion_id != champion_id {
            *current = ChampionSelections {
                champion_id,
                rune_page_index: None,
                secondary_spell_id: Some(spell_id),
                build_index: None,
                core_build_index: None,
                final_build_index: None,
            };
        } else {
            current.secondary_spell_id = Some(spell_id);
        }
    }
}



pub fn set_selected_build(champion_id: i32, build_index: usize) {
    if let Ok(mut guard) = get_selections_lock().write() {
        let current = guard.get_or_insert_with(|| ChampionSelections {
            champion_id,
            rune_page_index: None,
            secondary_spell_id: None,
            build_index: None,
            core_build_index: None,
            final_build_index: None,
        });
        if current.champion_id != champion_id {
            *current = ChampionSelections {
                champion_id,
                rune_page_index: None,
                secondary_spell_id: None,
                build_index: Some(build_index),
                core_build_index: None,
                final_build_index: None,
            };
        } else {
            current.build_index = Some(build_index);
        }
    }
}
pub fn set_selected_core_build(champion_id: i32, core_index: usize) {
    if let Ok(mut guard) = get_selections_lock().write() {
        let current = guard.get_or_insert_with(|| ChampionSelections {
            champion_id,
            rune_page_index: None,
            secondary_spell_id: None,
            build_index: None,
            core_build_index: None,
            final_build_index: None,
        });
        if current.champion_id != champion_id {
            *current = ChampionSelections {
                champion_id,
                rune_page_index: None,
                secondary_spell_id: None,
                build_index: None,
                core_build_index: Some(core_index),
                final_build_index: None,
            };
        } else {
            current.core_build_index = Some(core_index);
        }
    }
}

pub fn set_selected_final_build(champion_id: i32, final_index: usize) {
    if let Ok(mut guard) = get_selections_lock().write() {
        let current = guard.get_or_insert_with(|| ChampionSelections {
            champion_id,
            rune_page_index: None,
            secondary_spell_id: None,
            build_index: None,
            core_build_index: None,
            final_build_index: None,
        });
        if current.champion_id != champion_id {
            *current = ChampionSelections {
                champion_id,
                rune_page_index: None,
                secondary_spell_id: None,
                build_index: None,
                core_build_index: None,
                final_build_index: Some(final_index),
            };
        } else {
            current.final_build_index = Some(final_index);
        }
    }
}

pub fn get_champion_selections(champion_id: i32) -> Option<ChampionSelections> {
    if let Ok(guard) = get_selections_lock().read()
        && let Some(sel) = guard.as_ref()
        && sel.champion_id == champion_id
    {
        return Some(sel.clone());
    }
    None
}

pub fn clear_champion_selections() {
    if let Ok(mut guard) = get_selections_lock().write() {
        *guard = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_champion_selections_lock_in_persistence() {
        clear_champion_selections();
        assert_eq!(get_champion_selections(103), None);

        // User selected rune page index 1 and secondary spell 3 (Exhaust) for Ahri (103)
        set_selected_rune_page(103, 1);
        set_selected_secondary_spell(103, 3);

        let sel = get_champion_selections(103).expect("Selections should exist for champ 103");
        assert_eq!(sel.rune_page_index, Some(1));
        assert_eq!(sel.secondary_spell_id, Some(3));

        // Different champion query should return None
        assert_eq!(get_champion_selections(804), None);

        // Clearing selections resets back to None
        clear_champion_selections();
        assert_eq!(get_champion_selections(103), None);
    }
}
