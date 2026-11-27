use super::*;
use crate::models::{QUEUE_ARENA, QUEUE_SOLO_DUO, QueueMode};

#[test]
fn test_arena_reconciliation_recognizes_item_only_injection() {
    let payload = ChampSelectedPayload {
        champion_id: 350,
        queue_id: QUEUE_ARENA,
        queue_mode: QueueMode::Arena,
        locked_in: true,
        assigned_position: Some("support".to_string()),
        ..Default::default()
    };
    assert_eq!(resolve_effective_build_queue_id(&payload), QUEUE_ARENA);
    assert_eq!(
        resolve_effective_build_position(&payload, QUEUE_ARENA),
        None
    );

    let arena_build = ChampionBuild {
        champion_id: 350,
        champion_name: "Yuumi".to_string(),
        supports_runes: false,
        supports_spells: false,
        ..Default::default()
    };
    assert!(build_matches_payload(&arena_build, &payload, QUEUE_ARENA));

    let mut wrong_flags_build = arena_build.clone();
    wrong_flags_build.supports_spells = true;
    assert!(!build_matches_payload(
        &wrong_flags_build,
        &payload,
        QUEUE_ARENA
    ));

    let state = ChampSelectSessionState {
        last_payload: Some(payload.clone()),
        last_build: Some(arena_build),
        last_applied: Some(AppliedInjectionState {
            champion_id: 350,
            queue_id: QUEUE_ARENA,
            supports_runes: false,
            supports_spells: false,
            runes_injected: false,
            spells_injected: false,
            items_injected: true,
        }),
        ui_displayed_champion_id: Some(350),
        reconciled_champion_id: Some(350),
        ..Default::default()
    };
    let prefs = UserPreferences::default();
    assert_eq!(
        evaluate_reconciliation_decision(&state, &payload, &prefs, QUEUE_ARENA),
        ReconciliationDecision::AlreadyReconciled
    );
    assert_eq!(
        resolve_effective_build_queue_id(&ChampSelectedPayload {
            queue_id: crate::models::QUEUE_CLASH,
            ..Default::default()
        }),
        QUEUE_SOLO_DUO
    );
    assert_eq!(
        resolve_effective_build_queue_id(&ChampSelectedPayload {
            queue_id: crate::models::QUEUE_SWIFTPLAY,
            ..Default::default()
        }),
        crate::models::QUEUE_SWIFTPLAY
    );
}
