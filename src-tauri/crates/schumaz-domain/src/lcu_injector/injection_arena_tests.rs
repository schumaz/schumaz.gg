use super::tests::MockInjectionSink;
use super::*;
use crate::build_provider::{ItemBlock, ItemBlockKind};
use crate::lcu_injector::{InjectionTargets, TEST_SELECTIONS_LOCK, clear_champion_selections};
use crate::models::{AppLocale, QUEUE_ARENA};

#[test]
fn test_plan_safety_injection_skips_runes_and_spells_for_arena() {
    let build_arena = ChampionBuild {
        champion_id: 350,
        champion_name: "Yuumi".to_string(),
        supports_runes: false,
        supports_spells: false,
        ..Default::default()
    };
    let prefs = UserPreferences::default();

    assert_eq!(
        plan_safety_injection(&build_arena, &prefs),
        InjectionTargets {
            runes: false,
            spells: false,
            items: true,
        }
    );
}

#[tokio::test]
async fn test_safety_reinject_arena_skips_runes_and_spells_and_injects_only_items() {
    let _test_guard = TEST_SELECTIONS_LOCK.lock().await;
    clear_champion_selections();

    let build_arena = ChampionBuild {
        champion_id: 350,
        champion_name: "Yuumi".to_string(),
        spell1_id: 0,
        spell2_id: 0,
        item_blocks: vec![ItemBlock {
            kind: ItemBlockKind::Core,
            title: "Primeiros 3 Itens".to_string(),
            item_ids: vec![226617, 223504, 226620],
            ..Default::default()
        }],
        supports_runes: false,
        supports_spells: false,
        ..Default::default()
    };

    let prefs_pt = UserPreferences {
        language: AppLocale::PtBr,
        ..Default::default()
    };
    let sink_pt = MockInjectionSink::default();

    let (report_pt, applied_pt) =
        safety_reinject_build_with_sink(&sink_pt, 999, QUEUE_ARENA, &build_arena, &prefs_pt).await;

    assert!(!report_pt.runes_injected);
    assert!(!report_pt.spells_injected);
    assert!(report_pt.items_injected);
    assert_eq!(
        report_pt.message,
        "Modo Arena (runas e feitiços desativados pela Riot) | Itens injetados"
    );
    assert!(applied_pt.matches_expected(350, QUEUE_ARENA, &build_arena, &prefs_pt));
    assert!(!applied_pt.supports_runes);
    assert!(!applied_pt.supports_spells);

    {
        assert_eq!(sink_pt.rune_payloads.lock().unwrap().len(), 0);
        assert_eq!(sink_pt.spell_pairs.lock().unwrap().len(), 0);
        let item_payloads = sink_pt.item_payloads.lock().unwrap();
        assert_eq!(item_payloads.len(), 1);
        assert_eq!(
            item_payloads[0]["itemSets"][0]["associatedMaps"],
            serde_json::json!([11, 12, 30])
        );
    }

    let prefs_en = UserPreferences {
        language: AppLocale::EnUs,
        ..Default::default()
    };
    let sink_en = MockInjectionSink::default();
    let (report_en, _) =
        safety_reinject_build_with_sink(&sink_en, 999, QUEUE_ARENA, &build_arena, &prefs_en).await;
    assert_eq!(
        report_en.message,
        "Arena mode (runes and spells disabled by Riot) | Items injected"
    );

    // Mismatched supports_spells flag must invalidate applied state
    let mut build_with_spells = build_arena.clone();
    build_with_spells.supports_spells = true;
    assert!(!applied_pt.matches_expected(350, QUEUE_ARENA, &build_with_spells, &prefs_pt));
}

#[tokio::test]
async fn test_safety_reinject_aram_ignores_stale_ranked_teleport_selection() {
    use crate::lcu_injector::set_selected_spell_pair;
    use crate::models::QUEUE_ARAM;

    let _test_guard = TEST_SELECTIONS_LOCK.lock().await;
    clear_champion_selections();

    // User selected Teleport (12) + Flash (4) on Ahri (103) in Ranked previously
    set_selected_spell_pair(103, 4, 12);

    let aram_build = ChampionBuild {
        champion_id: 103,
        champion_name: "Ahri".to_string(),
        primary_style_id: 8100,
        sub_style_id: 8200,
        selected_perk_ids: vec![8112, 8126, 8138, 8106, 8226, 8210, 5008, 5008, 5001],
        spell1_id: 4,
        spell2_id: 32,
        supports_runes: true,
        supports_spells: true,
        ..Default::default()
    };

    let prefs = UserPreferences::default();
    let sink = MockInjectionSink::default();
    let (report, _) =
        safety_reinject_build_with_sink(&sink, 999, QUEUE_ARAM, &aram_build, &prefs).await;

    assert!(report.spells_injected);
    assert_eq!(sink.spell_pairs.lock().unwrap().as_slice(), &[(4, 32)]);

    clear_champion_selections();
}
