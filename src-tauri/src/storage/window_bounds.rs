use super::settings::WindowSettings;

pub const MIN_WINDOW_WIDTH: f64 = 840.0;
pub const MIN_WINDOW_HEIGHT: f64 = 600.0;
pub const DEFAULT_WINDOW_WIDTH: f64 = 980.0;
pub const DEFAULT_WINDOW_HEIGHT: f64 = 720.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MonitorRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ValidatedWindowBounds {
    pub width: f64,
    pub height: f64,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub maximized: bool,
    pub center: bool,
}

/// Validates saved window coordinates and dimensions against currently connected monitors
/// in logical coordinates so disconnected displays never spawn an off-screen window.
pub fn validate_window_bounds(
    win: &WindowSettings,
    monitors: &[MonitorRect],
) -> ValidatedWindowBounds {
    let mut width = if win.window_width.is_finite() {
        win.window_width.max(MIN_WINDOW_WIDTH)
    } else {
        DEFAULT_WINDOW_WIDTH
    };
    let mut height = if win.window_height.is_finite() {
        win.window_height.max(MIN_WINDOW_HEIGHT)
    } else {
        DEFAULT_WINDOW_HEIGHT
    };

    if monitors.is_empty() {
        return ValidatedWindowBounds {
            width,
            height,
            x: None,
            y: None,
            maximized: win.window_maximized,
            center: true,
        };
    }

    if let (Some(wx), Some(wy)) = (win.window_x, win.window_y)
        && wx.is_finite()
        && wy.is_finite()
    {
        // Check if at least 120x60 logical pixels intersect any active monitor
        let mut best_monitor: Option<&MonitorRect> = None;
        let mut best_overlap_area = 0.0f64;

        for m in monitors {
            let ix1 = wx.max(m.x);
            let iy1 = wy.max(m.y);
            let ix2 = (wx + width).min(m.x + m.width);
            let iy2 = (wy + height).min(m.y + m.height);
            let iw = (ix2 - ix1).max(0.0);
            let ih = (iy2 - iy1).max(0.0);
            if iw >= 120.0 && ih >= 60.0 {
                let area = iw * ih;
                if area > best_overlap_area {
                    best_overlap_area = area;
                    best_monitor = Some(m);
                }
            }
        }

        if let Some(m) = best_monitor {
            width = width.min(m.width.max(MIN_WINDOW_WIDTH));
            height = height.min(m.height.max(MIN_WINDOW_HEIGHT));
            // Keep titlebar reachable inside monitor top edge
            let clamped_y = wy.max(m.y);
            return ValidatedWindowBounds {
                width,
                height,
                x: Some(wx),
                y: Some(clamped_y),
                maximized: win.window_maximized,
                center: false,
            };
        }
    }

    let primary = &monitors[0];
    width = width.min(primary.width.max(MIN_WINDOW_WIDTH));
    height = height.min(primary.height.max(MIN_WINDOW_HEIGHT));

    ValidatedWindowBounds {
        width,
        height,
        x: None,
        y: None,
        maximized: win.window_maximized,
        center: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_window_bounds_multi_monitor_and_disconnected_fallback() {
        let monitors = vec![
            MonitorRect {
                x: 0.0,
                y: 0.0,
                width: 1920.0,
                height: 1080.0,
            },
            MonitorRect {
                x: 1920.0,
                y: 0.0,
                width: 1440.0,
                height: 900.0,
            },
        ];

        // Valid position on second monitor
        let on_second = WindowSettings {
            window_width: 1100.0,
            window_height: 760.0,
            window_x: Some(2050.0),
            window_y: Some(60.0),
            window_maximized: false,
        };
        let res = validate_window_bounds(&on_second, &monitors);
        assert_eq!(res.x, Some(2050.0));
        assert_eq!(res.y, Some(60.0));
        assert_eq!(res.width, 1100.0);
        assert_eq!(res.height, 760.0);
        assert!(!res.center);

        // Disconnected third monitor at x = 4000 -> centers on primary
        let offscreen = WindowSettings {
            window_width: 1050.0,
            window_height: 700.0,
            window_x: Some(4000.0),
            window_y: Some(200.0),
            window_maximized: true,
        };
        let res_off = validate_window_bounds(&offscreen, &monitors);
        assert_eq!(res_off.x, None);
        assert_eq!(res_off.y, None);
        assert!(res_off.center);
        assert!(res_off.maximized);
    }
}
