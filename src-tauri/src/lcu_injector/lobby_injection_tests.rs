use super::*;
use crate::build_provider::{ItemBlock, ItemBlockKind};
use crate::lcu_injector::{
    FlashKeyPreference, TEST_SELECTIONS_LOCK, clear_champion_selections, set_selected_spell_pair,
};
use crate::models::AppLocale;
use std::sync::Mutex;

#[derive(Default)]
struct MockLobbySink {
    patches: Mutex<Vec<Vec<LobbySlotPatch>>>,
    rune_champs: Mutex<Vec<i32>>,
    item_champ_sets: Mutex<Vec<Vec<i32>>>,
    fail_slots: bool,
}

impl LobbyInjectionSink for MockLobbySink {
    async fn patch_player_slots(&self, patches: &[LobbySlotPatch]) -> Result<bool, String> {
        if self.fail_slots {
            return Err("lobby slot locked".to_string());
        }
        self.patches
            .lock()
            .map_err(|_| "lock poisoned".to_string())?
            .push(patches.to_vec());
        Ok(true)
    }

    async fn inject_runes_for_build(&self, build: &ChampionBuild) -> Result<(), String> {
        self.rune_champs
            .lock()
            .map_err(|_| "lock poisoned".to_string())?
            .push(build.champion_id);
        Ok(())
    }

    async fn inject_multi_item_sets(
        &self,
        _summoner_id: u64,
        builds: &[&ChampionBuild],
    ) -> Result<(), String> {
        self.item_champ_sets
            .lock()
            .map_err(|_| "lock poisoned".to_string())?
            .push(builds.iter().map(|b| b.champion_id).collect());
        Ok(())
    }
}

fn sample_build(champ_id: i32, name: &str, pos: &str) -> ChampionBuild {
    ChampionBuild {
        champion_id: champ_id,
        champion_name: name.to_string(),
        active_position: pos.to_string(),
        primary_style_id: 8100,
        sub_style_id: 8200,
        selected_perk_ids: vec![8112, 8143, 8138, 8106, 8210, 8236, 5008, 5008, 5001],
        spell1_id: 4,
        spell2_id: 14,
        item_blocks: vec![ItemBlock {
            kind: ItemBlockKind::Starter,
            title: "1. Itens Iniciais".to_string(),
            item_ids: vec![1056, 2003],
            ..Default::default()
        }],
        supports_runes: true,
        supports_spells: true,
        ..Default::default()
    }
}

#[tokio::test]
async fn test_inject_lobby_preselect_respects_preferences_and_multi_slot_builds() {
    let _guard = TEST_SELECTIONS_LOCK.lock().await;
    clear_champion_selections();

    let ahri = sample_build(103, "Ahri", "mid");
    let mut lee = sample_build(64, "LeeSin", "jungle");
    lee.spell2_id = 11;
    set_selected_spell_pair(64, 4, 11);

    let prefs = UserPreferences {
        flash_key: FlashKeyPreference::F,
        language: AppLocale::PtBr,
        ..Default::default()
    };
    let sink = MockLobbySink::default();

    let report = inject_lobby_preselect_with_sink(
        &sink,
        42,
        0,
        &[(0, ahri.clone()), (1, lee.clone())],
        true,
        &prefs,
    )
    .await;

    assert!(report.runes_injected);
    assert!(report.spells_injected);
    assert!(report.items_injected);
    assert_eq!(report.champion_name, "Ahri");

    {
        let patches = sink.patches.lock().unwrap();
        assert_eq!(patches.len(), 1);
        assert_eq!(patches[0].len(), 2);
        // Flash on F => Ignite (14) on D + Flash (4) on F; Jungle Smite (11) on D + Flash (4) on F
        assert_eq!(patches[0][0].spells, Some((14, 4)));
        assert_eq!(patches[0][1].spells, Some((11, 4)));
    }
    assert_eq!(sink.rune_champs.lock().unwrap().as_slice(), &[103]);
    assert_eq!(
        sink.item_champ_sets.lock().unwrap().as_slice(),
        &[vec![103, 64]]
    );

    // When playerSlots is locked (Matchmaking), skip slot patching and spells while still updating items/page
    let locked_sink = MockLobbySink::default();
    let locked_report = inject_lobby_preselect_with_sink(
        &locked_sink,
        42,
        1,
        &[(0, ahri), (1, lee)],
        false,
        &prefs,
    )
    .await;
    assert!(locked_report.runes_injected);
    assert!(!locked_report.spells_injected);
    assert!(locked_report.items_injected);
    assert_eq!(locked_sink.patches.lock().unwrap().len(), 0);
    assert_eq!(locked_sink.rune_champs.lock().unwrap().as_slice(), &[64]);

    clear_champion_selections();
}
