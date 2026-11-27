pub use crate::models::{
    DEFAULT_WINDOW_HEIGHT, DEFAULT_WINDOW_WIDTH, MIN_WINDOW_HEIGHT, MIN_WINDOW_WIDTH,
    WindowSettings,
};

const MIN_VISIBLE_LOGICAL_WIDTH: f64 = 120.0;
const MIN_VISIBLE_LOGICAL_HEIGHT: f64 = 60.0;

#[derive(Debug, Clone, PartialEq)]
pub struct MonitorRect {
    pub name: Option<String>,
    /// Physical work-area origin X in virtual desktop coordinates.
    pub x: f64,
    /// Physical work-area origin Y in virtual desktop coordinates.
    pub y: f64,
    /// Physical work-area width in pixels.
    pub width: f64,
    /// Physical work-area height in pixels.
    pub height: f64,
    /// Monitor DPI scale factor (e.g. `1.0`, `1.25`, `1.5`, `2.0`).
    pub scale_factor: f64,
    /// Whether this monitor is the OS primary display.
    pub is_primary: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedWindowBounds {
    /// Target inner width in logical pixels for the target monitor's DPI.
    pub width: f64,
    /// Target inner height in logical pixels for the target monitor's DPI.
    pub height: f64,
    /// Target outer X in physical virtual-desktop pixels (`None` only when no monitors are available).
    pub x: Option<i32>,
    /// Target outer Y in physical virtual-desktop pixels (`None` only when no monitors are available).
    pub y: Option<i32>,
    /// Scale factor of the resolved target monitor (`1.0` when unavailable).
    pub scale_factor: f64,
    /// Name of the resolved target monitor when known.
    pub monitor_name: Option<String>,
    pub maximized: bool,
    pub center: bool,
}

fn sanitize_scale(scale: f64) -> f64 {
    if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    }
}

fn clamp_logical_size(width: f64, height: f64, monitor: &MonitorRect) -> (f64, f64) {
    let scale = sanitize_scale(monitor.scale_factor);
    let mon_logical_w = (monitor.width / scale).max(MIN_WINDOW_WIDTH);
    let mon_logical_h = (monitor.height / scale).max(MIN_WINDOW_HEIGHT);
    (
        width.max(MIN_WINDOW_WIDTH).min(mon_logical_w),
        height.max(MIN_WINDOW_HEIGHT).min(mon_logical_h),
    )
}

fn centered_on_monitor(
    width: f64,
    height: f64,
    monitor: &MonitorRect,
    maximized: bool,
) -> ValidatedWindowBounds {
    let scale = sanitize_scale(monitor.scale_factor);
    let (clamped_w, clamped_h) = clamp_logical_size(width, height, monitor);
    let phys_w = (clamped_w * scale).round();
    let phys_h = (clamped_h * scale).round();
    let center_x = (monitor.x + ((monitor.width - phys_w) / 2.0).max(0.0)).round() as i32;
    let center_y = (monitor.y + ((monitor.height - phys_h) / 2.0).max(0.0)).round() as i32;

    ValidatedWindowBounds {
        width: clamped_w,
        height: clamped_h,
        x: Some(center_x),
        y: Some(center_y),
        scale_factor: scale,
        monitor_name: monitor.name.clone(),
        maximized,
        center: true,
    }
}

fn overlap_logical_area(
    wx: f64,
    wy: f64,
    logical_w: f64,
    logical_h: f64,
    monitor: &MonitorRect,
) -> Option<f64> {
    let scale = sanitize_scale(monitor.scale_factor);
    let phys_w = logical_w * scale;
    let phys_h = logical_h * scale;
    let ix1 = wx.max(monitor.x);
    let iy1 = wy.max(monitor.y);
    let ix2 = (wx + phys_w).min(monitor.x + monitor.width);
    let iy2 = (wy + phys_h).min(monitor.y + monitor.height);
    let iw = (ix2 - ix1).max(0.0);
    let ih = (iy2 - iy1).max(0.0);
    let min_w = MIN_VISIBLE_LOGICAL_WIDTH * scale;
    let min_h = MIN_VISIBLE_LOGICAL_HEIGHT * scale;
    if iw >= min_w && ih >= min_h {
        Some((iw / scale) * (ih / scale))
    } else {
        None
    }
}

fn find_best_overlapping_monitor(
    wx: f64,
    wy: f64,
    logical_w: f64,
    logical_h: f64,
    monitors: &[MonitorRect],
) -> Option<&MonitorRect> {
    let mut best_monitor: Option<&MonitorRect> = None;
    let mut best_area = 0.0f64;

    for monitor in monitors {
        if let Some(area) = overlap_logical_area(wx, wy, logical_w, logical_h, monitor)
            && area > best_area
        {
            best_area = area;
            best_monitor = Some(monitor);
        }
    }

    best_monitor
}

fn coords_belong_to_monitor(
    wx: f64,
    wy: f64,
    width: f64,
    height: f64,
    target: &MonitorRect,
    monitors: &[MonitorRect],
) -> bool {
    let Some(target_area) = overlap_logical_area(wx, wy, width, height, target) else {
        return false;
    };
    let Some(best_mon) = find_best_overlapping_monitor(wx, wy, width, height, monitors) else {
        return false;
    };
    std::ptr::eq(best_mon, target)
        || overlap_logical_area(wx, wy, width, height, best_mon)
            .is_some_and(|best_area| target_area + 1.0 >= best_area)
}

fn clamped_on_monitor(
    wx: f64,
    wy: f64,
    width: f64,
    height: f64,
    monitor: &MonitorRect,
    maximized: bool,
) -> ValidatedWindowBounds {
    let scale = sanitize_scale(monitor.scale_factor);
    let (clamped_w, clamped_h) = clamp_logical_size(width, height, monitor);
    let phys_w = (clamped_w * scale).round();
    let min_visible_w = (MIN_VISIBLE_LOGICAL_WIDTH * scale).min(monitor.width);
    let min_visible_h = (MIN_VISIBLE_LOGICAL_HEIGHT * scale).min(monitor.height);

    let min_x = monitor.x - phys_w + min_visible_w;
    let max_x = (monitor.x + monitor.width - min_visible_w).max(min_x);
    let min_y = monitor.y;
    let max_y = (monitor.y + monitor.height - min_visible_h).max(min_y);

    let clamped_x = wx.clamp(min_x, max_x).round() as i32;
    let clamped_y = wy.clamp(min_y, max_y).round() as i32;

    ValidatedWindowBounds {
        width: clamped_w,
        height: clamped_h,
        x: Some(clamped_x),
        y: Some(clamped_y),
        scale_factor: scale,
        monitor_name: monitor.name.clone(),
        maximized,
        center: false,
    }
}

/// Validates saved window coordinates, logical size, and target monitor against currently connected
/// displays (in physical work-area coordinates with per-monitor DPI scale factors), falling back
/// to a centered `1280×720` window safely clamped on the primary monitor if the saved display is disconnected.
pub fn validate_window_bounds(
    win: &WindowSettings,
    monitors: &[MonitorRect],
) -> ValidatedWindowBounds {
    let width = if win.window_width.is_finite() {
        win.window_width.max(MIN_WINDOW_WIDTH)
    } else {
        DEFAULT_WINDOW_WIDTH
    };
    let height = if win.window_height.is_finite() {
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
            scale_factor: 1.0,
            monitor_name: None,
            maximized: win.window_maximized,
            center: true,
        };
    }

    let primary = monitors
        .iter()
        .find(|m| m.is_primary)
        .unwrap_or(&monitors[0]);

    let valid_coords = match (win.window_x, win.window_y) {
        (Some(wx), Some(wy)) if wx.is_finite() && wy.is_finite() => Some((wx, wy)),
        _ => None,
    };

    let saved_monitor_name = win
        .window_monitor
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty());

    // 1. Prefer the explicitly saved monitor when it is still connected.
    if let Some(target_name) = saved_monitor_name {
        if let Some(saved_monitor) = monitors
            .iter()
            .find(|m| m.name.as_deref() == Some(target_name))
        {
            if let Some((wx, wy)) = valid_coords
                && coords_belong_to_monitor(wx, wy, width, height, saved_monitor, monitors)
            {
                return clamped_on_monitor(
                    wx,
                    wy,
                    width,
                    height,
                    saved_monitor,
                    win.window_maximized,
                );
            }
            return centered_on_monitor(width, height, saved_monitor, win.window_maximized);
        }

        // Saved monitor name not found; if unmaximized and coordinates still overlap a connected
        // display (e.g. Windows renamed display indices), preserve the exact rect there.
        if !win.window_maximized
            && let Some((wx, wy)) = valid_coords
            && let Some(best) = find_best_overlapping_monitor(wx, wy, width, height, monitors)
        {
            return clamped_on_monitor(wx, wy, width, height, best, false);
        }

        // Saved monitor was disconnected: fall back to centered 1280×720 safely clamped on primary.
        return centered_on_monitor(
            DEFAULT_WINDOW_WIDTH,
            DEFAULT_WINDOW_HEIGHT,
            primary,
            win.window_maximized,
        );
    }

    // 2. No saved monitor name (first launch or legacy INI): check coordinate overlap.
    if let Some((wx, wy)) = valid_coords {
        if let Some(best) = find_best_overlapping_monitor(wx, wy, width, height, monitors) {
            return clamped_on_monitor(wx, wy, width, height, best, win.window_maximized);
        }
        return centered_on_monitor(
            DEFAULT_WINDOW_WIDTH,
            DEFAULT_WINDOW_HEIGHT,
            primary,
            win.window_maximized,
        );
    }

    // 3. First launch with no saved coordinates/monitor: center on primary (defaults to 1280×720).
    centered_on_monitor(width, height, primary, win.window_maximized)
}

#[cfg(test)]
#[path = "window_bounds_tests.rs"]
mod tests;
