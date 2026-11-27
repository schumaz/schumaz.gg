use super::*;

fn primary_1080p() -> MonitorRect {
    MonitorRect {
        name: Some(r"\\.\DISPLAY1".to_string()),
        x: 0.0,
        y: 0.0,
        width: 1920.0,
        height: 1040.0,
        scale_factor: 1.0,
        is_primary: true,
    }
}

fn secondary_4k_200pct() -> MonitorRect {
    MonitorRect {
        name: Some(r"\\.\DISPLAY2".to_string()),
        x: 1920.0,
        y: 0.0,
        width: 3840.0,
        height: 2080.0,
        scale_factor: 2.0,
        is_primary: false,
    }
}

#[test]
fn test_first_launch_centers_default_1280x720_on_primary_monitor() {
    let monitors = vec![secondary_4k_200pct(), primary_1080p()];
    let res = validate_window_bounds(&WindowSettings::default(), &monitors);

    assert_eq!(res.width, 1280.0);
    assert_eq!(res.height, 720.0);
    assert_eq!(res.x, Some(320));
    assert_eq!(res.y, Some(160));
    assert_eq!(res.scale_factor, 1.0);
    assert_eq!(res.monitor_name.as_deref(), Some(r"\\.\DISPLAY1"));
    assert!(!res.maximized);
    assert!(res.center);
}

#[test]
fn test_restores_exact_position_size_and_mixed_dpi_secondary_monitor() {
    let monitors = vec![primary_1080p(), secondary_4k_200pct()];
    let saved = WindowSettings {
        window_width: 1320.0,
        window_height: 760.0,
        window_x: Some(2200.0),
        window_y: Some(150.0),
        window_maximized: false,
        window_monitor: Some(r"\\.\DISPLAY2".to_string()),
    };

    let res = validate_window_bounds(&saved, &monitors);
    assert_eq!(res.width, 1320.0);
    assert_eq!(res.height, 760.0);
    assert_eq!(res.x, Some(2200));
    assert_eq!(res.y, Some(150));
    assert_eq!(res.scale_factor, 2.0);
    assert_eq!(res.monitor_name.as_deref(), Some(r"\\.\DISPLAY2"));
    assert!(!res.maximized);
    assert!(!res.center);
}

#[test]
fn test_maximized_window_moved_to_secondary_monitor_centers_normal_rect_on_saved_monitor() {
    let monitors = vec![primary_1080p(), secondary_4k_200pct()];
    // Window was unmaximized on primary (100, 100), then moved/maximized on DISPLAY2.
    let saved = WindowSettings {
        window_width: 1280.0,
        window_height: 720.0,
        window_x: Some(100.0),
        window_y: Some(100.0),
        window_maximized: true,
        window_monitor: Some(r"\\.\DISPLAY2".to_string()),
    };

    let res = validate_window_bounds(&saved, &monitors);
    // 1280x720 at 2.0x = 2560x1440 physical; centered on [1920..5760, 0..2080] => (2560, 320).
    assert_eq!(res.width, 1280.0);
    assert_eq!(res.height, 720.0);
    assert_eq!(res.x, Some(2560));
    assert_eq!(res.y, Some(320));
    assert_eq!(res.scale_factor, 2.0);
    assert_eq!(res.monitor_name.as_deref(), Some(r"\\.\DISPLAY2"));
    assert!(res.maximized);
    assert!(res.center);
}

#[test]
fn test_disconnected_saved_monitor_falls_back_to_centered_1280x720_clamped_on_primary() {
    // Primary laptop screen: 1920x1080 @ 150% DPI with taskbar => 1920x1020 physical (1280x680 logical).
    let laptop_primary = MonitorRect {
        name: Some(r"\\.\DISPLAY1".to_string()),
        x: 0.0,
        y: 0.0,
        width: 1920.0,
        height: 1020.0,
        scale_factor: 1.5,
        is_primary: true,
    };
    let monitors = vec![laptop_primary];

    let disconnected = WindowSettings {
        window_width: 1600.0,
        window_height: 950.0,
        window_x: Some(2500.0),
        window_y: Some(180.0),
        window_maximized: true,
        window_monitor: Some(r"\\.\DISPLAY3".to_string()),
    };

    let res = validate_window_bounds(&disconnected, &monitors);
    // Fallback starts at 1280x720 and clamps safely to the 1280x680 logical work area.
    assert_eq!(res.width, 1280.0);
    assert_eq!(res.height, 680.0);
    assert_eq!(res.x, Some(0));
    assert_eq!(res.y, Some(0));
    assert_eq!(res.scale_factor, 1.5);
    assert_eq!(res.monitor_name.as_deref(), Some(r"\\.\DISPLAY1"));
    assert!(res.maximized);
    assert!(res.center);
}

#[test]
fn test_titlebar_clamping_and_negative_x_left_monitor() {
    let left_monitor = MonitorRect {
        name: Some(r"\\.\DISPLAY2".to_string()),
        x: -1920.0,
        y: 0.0,
        width: 1920.0,
        height: 1040.0,
        scale_factor: 1.0,
        is_primary: false,
    };
    let monitors = vec![primary_1080p(), left_monitor];

    let saved = WindowSettings {
        window_width: 1200.0,
        window_height: 700.0,
        window_x: Some(-1600.0),
        window_y: Some(-45.0),
        window_maximized: false,
        window_monitor: Some(r"\\.\DISPLAY2".to_string()),
    };

    let res = validate_window_bounds(&saved, &monitors);
    assert_eq!(res.x, Some(-1600));
    assert_eq!(res.y, Some(0));
    assert_eq!(res.monitor_name.as_deref(), Some(r"\\.\DISPLAY2"));
    assert!(!res.center);
}
