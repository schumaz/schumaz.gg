use super::*;

#[test]
fn test_is_window_minimized_or_hidden_preserves_unfocused_visible_window() {
    assert!(!is_window_minimized_or_hidden(false, true));
    assert!(is_window_minimized_or_hidden(true, true));
    assert!(is_window_minimized_or_hidden(false, false));
    assert!(is_window_minimized_or_hidden(true, false));
}

#[test]
fn test_should_allow_exit_on_window_close_respects_close_to_tray() {
    assert!(should_allow_exit_on_window_close(false));
    assert!(!should_allow_exit_on_window_close(true));
}

#[test]
fn test_should_sync_minimized_state_for_event_skips_normal_drag_and_resize() {
    let normal_move = tauri::WindowEvent::Moved(tauri::PhysicalPosition::new(160, 120));
    let normal_resize = tauri::WindowEvent::Resized(tauri::PhysicalSize::new(1280, 720));
    let sentinel_move = tauri::WindowEvent::Moved(tauri::PhysicalPosition::new(-32000, -32000));
    let zero_resize = tauri::WindowEvent::Resized(tauri::PhysicalSize::new(0, 0));

    assert!(!should_sync_minimized_state_for_event(&normal_move, false));
    assert!(!should_sync_minimized_state_for_event(
        &normal_resize,
        false
    ));
    assert!(!should_sync_minimized_state_for_event(
        &tauri::WindowEvent::Destroyed,
        false
    ));

    assert!(should_sync_minimized_state_for_event(&normal_move, true));
    assert!(should_sync_minimized_state_for_event(&normal_resize, true));
    assert!(should_sync_minimized_state_for_event(
        &sentinel_move,
        false
    ));
    assert!(should_sync_minimized_state_for_event(&zero_resize, false));
    assert!(should_sync_minimized_state_for_event(
        &tauri::WindowEvent::Focused(true),
        false
    ));
    assert!(should_sync_minimized_state_for_event(
        &tauri::WindowEvent::Focused(false),
        false
    ));
}

#[test]
fn test_is_minimized_geometry_event_detects_zero_size_and_sentinel_coords() {
    assert!(is_minimized_geometry_event(&tauri::WindowEvent::Resized(
        tauri::PhysicalSize::new(0, 720)
    )));
    assert!(is_minimized_geometry_event(&tauri::WindowEvent::Resized(
        tauri::PhysicalSize::new(1280, 0)
    )));
    assert!(is_minimized_geometry_event(&tauri::WindowEvent::Moved(
        tauri::PhysicalPosition::new(-32000, 100)
    )));
    assert!(is_minimized_geometry_event(&tauri::WindowEvent::Moved(
        tauri::PhysicalPosition::new(100, -30000)
    )));

    assert!(!is_minimized_geometry_event(&tauri::WindowEvent::Resized(
        tauri::PhysicalSize::new(1280, 720)
    )));
    assert!(!is_minimized_geometry_event(&tauri::WindowEvent::Moved(
        tauri::PhysicalPosition::new(-1920, 0)
    )));
    assert!(!is_minimized_geometry_event(
        &tauri::WindowEvent::Destroyed
    ));
}

#[test]
fn test_apply_captured_window_metrics_tracks_monitor_dpi_and_preserves_normal_rect_when_maximized()
{
    let mut settings = storage::WindowSettings::default();

    // Normal capture on 200% DPI secondary monitor
    assert!(apply_captured_window_metrics(
        &mut settings,
        false,
        Some(r"\\.\DISPLAY2".to_string()),
        2.0,
        Some((2560, 1440)),
        Some((2200, 150)),
    ));
    assert_eq!(settings.window_width, 1280.0);
    assert_eq!(settings.window_height, 720.0);
    assert_eq!(settings.window_x, Some(2200.0));
    assert_eq!(settings.window_y, Some(150.0));
    assert!(!settings.window_maximized);
    assert_eq!(settings.window_monitor.as_deref(), Some(r"\\.\DISPLAY2"));

    // Maximizing on DISPLAY3 keeps normal rect intact while updating maximized + monitor
    assert!(apply_captured_window_metrics(
        &mut settings,
        true,
        Some(r"\\.\DISPLAY3".to_string()),
        1.5,
        Some((2560, 1400)),
        Some((0, 0)),
    ));
    assert_eq!(settings.window_width, 1280.0);
    assert_eq!(settings.window_height, 720.0);
    assert_eq!(settings.window_x, Some(2200.0));
    assert_eq!(settings.window_y, Some(150.0));
    assert!(settings.window_maximized);
    assert_eq!(settings.window_monitor.as_deref(), Some(r"\\.\DISPLAY3"));

    // Win32 minimized sentinel (-32000, -32000) and zero-size events are rejected
    assert!(!apply_captured_window_metrics(
        &mut settings,
        false,
        Some(r"\\.\DISPLAY1".to_string()),
        1.0,
        Some((1280, 720)),
        Some((-32000, -32000)),
    ));
    assert!(!apply_captured_window_metrics(
        &mut settings,
        false,
        Some(r"\\.\DISPLAY1".to_string()),
        1.0,
        Some((0, 0)),
        Some((100, 100)),
    ));
    assert_eq!(settings.window_monitor.as_deref(), Some(r"\\.\DISPLAY3"));
}
