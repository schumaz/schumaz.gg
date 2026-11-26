use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ChampionSelections {
    pub champion_id: i32,
    pub rune_page_index: Option<usize>,
    pub selected_spell_pair: Option<(i32, i32)>,
    pub build_index: Option<usize>,
}

static CHAMPION_SELECTIONS: OnceLock<RwLock<HashMap<i32, ChampionSelections>>> = OnceLock::new();

#[cfg(test)]
pub(crate) static TEST_SELECTIONS_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn get_selections_lock() -> &'static RwLock<HashMap<i32, ChampionSelections>> {
    CHAMPION_SELECTIONS.get_or_init(|| RwLock::new(HashMap::new()))
}

fn upsert_champion_selection(champion_id: i32, update: impl FnOnce(&mut ChampionSelections)) {
    if let Ok(mut guard) = get_selections_lock().write() {
        let entry = guard.entry(champion_id).or_insert_with(|| ChampionSelections {
            champion_id,
            ..Default::default()
        });
        update(entry);
    }
}

pub fn set_selected_rune_page(champion_id: i32, page_index: usize) {
    upsert_champion_selection(champion_id, |entry| {
        entry.rune_page_index = Some(page_index);
    });
}

pub fn set_selected_spell_pair(champion_id: i32, spell1_id: i32, spell2_id: i32) {
    if spell1_id <= 0 || spell2_id <= 0 || spell1_id == spell2_id {
        return;
    }
    upsert_champion_selection(champion_id, |entry| {
        entry.selected_spell_pair = Some((spell1_id, spell2_id));
    });
}

pub fn set_selected_build(champion_id: i32, build_index: usize) {
    upsert_champion_selection(champion_id, |entry| {
        entry.build_index = Some(build_index);
    });
}

pub fn get_champion_selections(champion_id: i32) -> Option<ChampionSelections> {
    if let Ok(guard) = get_selections_lock().read() {
        return guard.get(&champion_id).cloned();
    }
    None
}

/// Clears manual selections only for a single champion (e.g. when its build filters change),
/// leaving companion Swiftplay lobby slot selections intact.
pub fn clear_champion_selection_for_champion(champion_id: i32) {
    if let Ok(mut guard) = get_selections_lock().write() {
        guard.remove(&champion_id);
    }
}

/// Returns `true` when both summoner spells are valid, distinct, and allowed in `queue_id`.
pub fn is_spell_pair_valid_for_queue(spell1_id: i32, spell2_id: i32, queue_id: i32) -> bool {
    if spell1_id <= 0
        || spell2_id <= 0
        || spell1_id == spell2_id
        || !crate::models::queue_supports_spells(queue_id)
    {
        return false;
    }

    let mode = crate::build_provider::ddragon_mode_for_queue(queue_id);
    let locale = super::preferences::get_active_locale();
    let ddragon = crate::build_provider::ddragon::get_ddragon_cache_store()
        .read()
        .ok()
        .and_then(|g| g.get(&locale).cloned());
    let catalog = ddragon.as_ref().map(|d| &d.spells);

    crate::build_provider::is_spell_allowed_in_mode(spell1_id, Some(mode), catalog)
        && crate::build_provider::is_spell_allowed_in_mode(spell2_id, Some(mode), catalog)
}

/// Returns the user's manually selected spell pair for `champion_id` only if valid for `queue_id`.
pub fn get_selected_spell_pair_for_queue(
    champion_id: i32,
    queue_id: Option<i32>,
) -> Option<(i32, i32)> {
    let (s1, s2) = get_champion_selections(champion_id)?.selected_spell_pair?;
    if s1 <= 0 || s2 <= 0 || s1 == s2 {
        return None;
    }
    if let Some(q_id) = queue_id.filter(|&q| q != 0)
        && !is_spell_pair_valid_for_queue(s1, s2, q_id)
    {
        return None;
    }
    Some((s1, s2))
}

pub fn clear_champion_selections() {
    if let Ok(mut guard) = get_selections_lock().write() {
        guard.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_champion_selections_lock_in_persistence() {
        let _test_guard = TEST_SELECTIONS_LOCK.blocking_lock();
        clear_champion_selections();
        assert_eq!(get_champion_selections(103), None);

        // User selected rune page index 1, spell pair (4, 3), and build index 0 for Ahri (103)
        set_selected_rune_page(103, 1);
        set_selected_spell_pair(103, 4, 3);
        set_selected_build(103, 0);

        let sel = get_champion_selections(103).expect("Selections should exist for champ 103");
        assert_eq!(sel.rune_page_index, Some(1));
        assert_eq!(sel.selected_spell_pair, Some((4, 3)));
        assert_eq!(sel.build_index, Some(0));

        // Invalid pair updates are ignored
        set_selected_spell_pair(103, 4, 4);
        assert_eq!(
            get_champion_selections(103).unwrap().selected_spell_pair,
            Some((4, 3))
        );

        // Different champion query should return None when unset, and store independently when set
        assert_eq!(get_champion_selections(804), None);
        set_selected_rune_page(804, 2);
        set_selected_spell_pair(804, 4, 14);
        assert_eq!(get_champion_selections(103).unwrap().rune_page_index, Some(1));
        assert_eq!(get_champion_selections(804).unwrap().rune_page_index, Some(2));
        assert_eq!(get_champion_selections(804).unwrap().selected_spell_pair, Some((4, 14)));

        // Clearing selections resets back to None
        clear_champion_selections();
        assert_eq!(get_champion_selections(103), None);
    }

    #[test]
    fn test_spell_pair_queue_validation_and_single_champion_clear() {
        use crate::models::{
            QUEUE_ARAM, QUEUE_ARAM_MAYHEM, QUEUE_ARENA, QUEUE_PRACTICE_TOOL, QUEUE_SOLO_DUO,
        };

        let _test_guard = TEST_SELECTIONS_LOCK.blocking_lock();
        clear_champion_selections();

        assert!(is_spell_pair_valid_for_queue(4, 12, QUEUE_SOLO_DUO));
        assert!(is_spell_pair_valid_for_queue(4, 12, QUEUE_PRACTICE_TOOL));
        assert!(!is_spell_pair_valid_for_queue(4, 12, QUEUE_ARAM));
        assert!(!is_spell_pair_valid_for_queue(4, 12, QUEUE_ARAM_MAYHEM));
        assert!(!is_spell_pair_valid_for_queue(4, 12, QUEUE_ARENA));

        assert!(is_spell_pair_valid_for_queue(4, 32, QUEUE_ARAM));
        assert!(is_spell_pair_valid_for_queue(4, 32, QUEUE_ARAM_MAYHEM));
        assert!(!is_spell_pair_valid_for_queue(4, 32, QUEUE_SOLO_DUO));

        assert!(is_spell_pair_valid_for_queue(4, 3, QUEUE_ARAM));
        assert!(!is_spell_pair_valid_for_queue(4, 3, QUEUE_ARAM_MAYHEM));

        set_selected_spell_pair(103, 4, 12);
        set_selected_spell_pair(222, 4, 7);
        assert_eq!(
            get_selected_spell_pair_for_queue(103, Some(QUEUE_SOLO_DUO)),
            Some((4, 12))
        );
        assert_eq!(get_selected_spell_pair_for_queue(103, Some(QUEUE_ARAM)), None);
        assert_eq!(
            get_selected_spell_pair_for_queue(103, Some(QUEUE_ARAM_MAYHEM)),
            None
        );

        clear_champion_selection_for_champion(103);
        assert_eq!(get_champion_selections(103), None);
        assert_eq!(
            get_selected_spell_pair_for_queue(222, Some(QUEUE_ARAM)),
            Some((4, 7))
        );

        clear_champion_selections();
    }
}
