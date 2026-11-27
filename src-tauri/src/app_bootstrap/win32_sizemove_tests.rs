use super::*;

#[test]
fn test_window_pos_passes_through_outside_sizemove_and_for_lifecycle_or_noop_flags() {
    let mut throttle = SizeMoveThrottle::new(RESIZE_THROTTLE_MS);
    assert!(!throttle.is_in_sizemove());
    assert_eq!(
        throttle.on_window_pos_changed(false, true, false, 10),
        WindowPosAction::ForwardUnchanged
    );

    throttle.on_enter_sizemove();
    assert!(throttle.is_in_sizemove());
    assert_eq!(
        throttle.on_window_pos_changed(true, true, false, 10),
        WindowPosAction::ForwardUnchanged
    );
    assert_eq!(
        throttle.on_window_pos_changed(false, true, true, 10),
        WindowPosAction::ForwardUnchanged
    );
    assert_eq!(throttle.on_exit_sizemove(), None);
}

#[test]
fn test_pure_drag_coalesces_all_moves_until_sizemove_exit() {
    let mut throttle = SizeMoveThrottle::new(RESIZE_THROTTLE_MS);
    throttle.on_enter_sizemove();

    throttle.note_deferred_move();
    for tick_ms in 0..20 {
        assert_eq!(
            throttle.on_window_pos_changed(false, true, false, tick_ms),
            WindowPosAction::SwallowPureMove
        );
    }
    assert_eq!(throttle.take_timer_flush(20), None);
    assert_eq!(
        throttle.on_exit_sizemove(),
        Some(DeferredSizeMoveFlush {
            moved: true,
            resized: false,
        })
    );
    assert!(!throttle.is_in_sizemove());
    assert_eq!(throttle.on_exit_sizemove(), None);
}

#[test]
fn test_resize_throttles_to_cadence_and_timer_flushes_pending_resize_and_move() {
    let mut throttle = SizeMoveThrottle::new(32);
    throttle.on_enter_sizemove();

    assert_eq!(
        throttle.on_window_pos_changed(false, false, true, 100),
        WindowPosAction::ForwardResize
    );
    assert_eq!(
        throttle.on_window_pos_changed(false, false, true, 110),
        WindowPosAction::SwallowAndArmResizeTimer
    );
    assert_eq!(
        throttle.on_window_pos_changed(false, false, true, 120),
        WindowPosAction::SwallowAndArmResizeTimer
    );
    assert_eq!(
        throttle.on_window_pos_changed(false, false, true, 132),
        WindowPosAction::ForwardResize
    );
    assert_eq!(throttle.take_timer_flush(132), None);

    // Top-left border resize changes both position and size, then stalls before exit.
    assert_eq!(
        throttle.on_window_pos_changed(false, false, false, 140),
        WindowPosAction::SwallowAndArmResizeTimer
    );
    assert_eq!(
        throttle.take_timer_flush(164),
        Some(DeferredSizeMoveFlush {
            moved: true,
            resized: true,
        })
    );
    assert_eq!(throttle.on_exit_sizemove(), None);

    // Trailing un-flushed resize flushes once on exit.
    throttle.on_enter_sizemove();
    assert_eq!(
        throttle.on_window_pos_changed(false, false, true, 200),
        WindowPosAction::ForwardResize
    );
    assert_eq!(
        throttle.on_window_pos_changed(false, false, false, 205),
        WindowPosAction::SwallowAndArmResizeTimer
    );
    assert_eq!(
        throttle.on_exit_sizemove(),
        Some(DeferredSizeMoveFlush {
            moved: true,
            resized: true,
        })
    );
}
