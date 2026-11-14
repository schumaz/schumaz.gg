use super::*;
use crate::build_provider::{ItemBlock, RunePageOption};
use std::sync::Mutex;

#[derive(Default)]
struct MockInjectionSink {
    rune_payloads: Mutex<Vec<serde_json::Value>>,
    spell_pairs: Mutex<Vec<(i32, i32)>>,
    item_payloads: Mutex<Vec<serde_json::Value>>,
    fail_spells: bool,
}

impl SafetyInjectionSink for MockInjectionSink {
    async fn inject_runes_for_build(&self, build: &ChampionBuild) -> Result<(), String> {
        let payload = build_rune_page_payload(build)?;
        self.rune_payloads
            .lock()
            .map_err(|_| "lock poisoned".to_string())?
            .push(payload);
        Ok(())
    }

    async fn inject_spells_for_build(
        &self,
        build: &ChampionBuild,
        flash_key: FlashKeyPreference,
    ) -> Result<(), String> {
        if self.fail_spells {
            return Err("transient spell patch failure".to_string());
        }
        let pair = resolve_injection_spell_pair(build, flash_key);
        let _ = build_spells_payload(pair.0, pair.1)?;
        self.spell_pairs
            .lock()
            .map_err(|_| "lock poisoned".to_string())?
            .push(pair);
        Ok(())
    }

    async fn inject_items_for_build(
        &self,
        summoner_id: u64,
        build: &ChampionBuild,
    ) -> Result<(), String> {
        let payload = build_item_sets_payload(summoner_id, build);
        self.item_payloads
            .lock()
            .map_err(|_| "lock poisoned".to_string())?
            .push(payload);
        Ok(())
    }
}

#[test]
fn test_injection_report_serialization() {
    let report = InjectionReport {
        champion_name: "Ahri".to_string(),
        runes_injected: true,
        spells_injected: true,
        items_injected: false,
        message: "Runas injetadas | Feitiços injetados".to_string(),
    };

    let json = serde_json::to_string(&report).unwrap();
    let parsed: InjectionReport = serde_json::from_str(&json).unwrap();
    assert_eq!(report, parsed);
}

#[test]
fn test_resolve_injection_spell_pair_all_modes_and_numeric_opgg_order() {
    let _test_guard = TEST_SELECTIONS_LOCK.blocking_lock();
    clear_champion_selections();

    let build_cleanse = ChampionBuild {
        champion_id: 18,
        spell1_id: 1,
        spell2_id: 4,
        active_position: "".to_string(),
        ..Default::default()
    };
    assert_eq!(
        resolve_injection_spell_pair(&build_cleanse, FlashKeyPreference::D),
        (4, 1)
    );
    assert_eq!(
        resolve_injection_spell_pair(&build_cleanse, FlashKeyPreference::F),
        (1, 4)
    );

    set_selected_secondary_spell(18, 21);
    assert_eq!(
        resolve_injection_spell_pair(&build_cleanse, FlashKeyPreference::D),
        (4, 21)
    );
    assert_eq!(
        resolve_injection_spell_pair(&build_cleanse, FlashKeyPreference::F),
        (21, 4)
    );

    clear_champion_selections();
    let build_jungle = ChampionBuild {
        champion_id: 120,
        spell1_id: 4,
        spell2_id: 11,
        active_position: "jungle".to_string(),
        ..Default::default()
    };
    set_selected_secondary_spell(120, 6);
    assert_eq!(
        resolve_injection_spell_pair(&build_jungle, FlashKeyPreference::D),
        (6, 11)
    );

    clear_champion_selections();
}

#[tokio::test]
async fn test_safety_reinject_with_runes_mode_preserves_manual_selections() {
    let _test_guard = TEST_SELECTIONS_LOCK.lock().await;
    clear_champion_selections();

    let build = ChampionBuild {
        champion_id: 103,
        champion_name: "Ahri".to_string(),
        primary_style_id: 8100,
        sub_style_id: 8200,
        selected_perk_ids: vec![8112, 8126, 8138, 8135, 8226, 8210, 5008, 5008, 5001],
        rune_pages: vec![
            RunePageOption {
                primary_style_id: 8100,
                sub_style_id: 8200,
                selected_perk_ids: vec![8112, 8126, 8138, 8135, 8226, 8210, 5008, 5008, 5001],
                ..Default::default()
            },
            RunePageOption {
                primary_style_id: 8300,
                sub_style_id: 8200,
                selected_perk_ids: vec![8369, 8304, 8345, 8347, 8226, 8210, 5008, 5008, 5001],
                ..Default::default()
            },
        ],
        spell1_id: 4,
        spell2_id: 14,
        item_blocks: vec![ItemBlock {
            title: "Primeiros 3 Itens".to_string(),
            item_ids: vec![6655, 3020, 4645],
            ..Default::default()
        }],
        supports_runes: true,
        ..Default::default()
    };

    // Simulate user manual selections made before FINALIZATION timer expires
    set_selected_rune_page(103, 1);
    set_selected_secondary_spell(103, 12); // Teleport instead of Ignite

    let prefs = UserPreferences::default();
    let sink = MockInjectionSink::default();

    let (report, applied) = safety_reinject_build_with_sink(&sink, 999, 420, &build, &prefs).await;

    assert!(report.runes_injected);
    assert!(report.spells_injected);
    assert!(report.items_injected);
    assert!(applied.matches_expected(103, 420, &build, &prefs));

    // Verify all three categories were injected and manual selections were respected
    let rune_payloads = sink.rune_payloads.lock().unwrap();
    assert_eq!(rune_payloads.len(), 1);
    assert_eq!(rune_payloads[0]["primaryStyleId"], 8300);

    let spell_pairs = sink.spell_pairs.lock().unwrap();
    assert_eq!(spell_pairs.as_slice(), &[(4, 12)]);

    let item_payloads = sink.item_payloads.lock().unwrap();
    assert_eq!(item_payloads.len(), 1);

    clear_champion_selections();
}

#[tokio::test]
async fn test_safety_reinject_aram_mayhem_skips_runes_and_injects_spells_and_items() {
    let _test_guard = TEST_SELECTIONS_LOCK.lock().await;
    clear_champion_selections();

    let build_mayhem = ChampionBuild {
        champion_id: 222,
        champion_name: "Jinx".to_string(),
        spell1_id: 4,
        spell2_id: 32,
        item_blocks: vec![ItemBlock {
            title: "Primeiros 3 Itens".to_string(),
            item_ids: vec![3032, 6675, 3031],
            ..Default::default()
        }],
        supports_runes: false,
        ..Default::default()
    };

    let prefs = UserPreferences::default();
    let sink = MockInjectionSink::default();

    let (report, applied) =
        safety_reinject_build_with_sink(&sink, 999, 2400, &build_mayhem, &prefs).await;

    assert!(!report.runes_injected);
    assert!(report.spells_injected);
    assert!(report.items_injected);
    assert!(applied.matches_expected(222, 2400, &build_mayhem, &prefs));

    // Runes must NEVER be injected in ARAM Mayhem even when auto_inject_runes is true
    assert_eq!(sink.rune_payloads.lock().unwrap().len(), 0);
    assert_eq!(sink.spell_pairs.lock().unwrap().as_slice(), &[(4, 32)]);
    assert_eq!(sink.item_payloads.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn test_applied_injection_state_detects_partial_failure() {
    let _test_guard = TEST_SELECTIONS_LOCK.lock().await;
    clear_champion_selections();

    let build = ChampionBuild {
        champion_id: 103,
        champion_name: "Ahri".to_string(),
        primary_style_id: 8100,
        sub_style_id: 8200,
        selected_perk_ids: vec![8112, 8126, 8138, 8135, 8226, 8210, 5008, 5008, 5001],
        spell1_id: 4,
        spell2_id: 14,
        supports_runes: true,
        ..Default::default()
    };
    let prefs = UserPreferences::default();
    let sink = MockInjectionSink {
        fail_spells: true,
        ..Default::default()
    };

    let (report, applied) = safety_reinject_build_with_sink(&sink, 999, 420, &build, &prefs).await;
    assert!(report.runes_injected);
    assert!(!report.spells_injected);
    assert!(!applied.matches_expected(103, 420, &build, &prefs));
}
