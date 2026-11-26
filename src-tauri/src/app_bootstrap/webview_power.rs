use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

use tauri::Manager;

use super::window_events::WindowRuntimeState;

pub const UNSET_POWER_POLICY: u8 = u8::MAX;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowGraphicsTier {
    Focused,
    Unfocused,
    ZeroGpu,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WebviewPowerPolicy {
    pub controller_visible: bool,
    pub should_suspend: bool,
    pub low_memory_target: bool,
}

impl WebviewPowerPolicy {
    pub const fn encode(self) -> u8 {
        (self.controller_visible as u8)
            | ((self.should_suspend as u8) << 1)
            | ((self.low_memory_target as u8) << 2)
    }
}

/// Returns `true` when the main window is minimized to the taskbar or hidden from view.
pub const fn is_window_minimized_or_hidden(is_minimized: bool, is_visible: bool) -> bool {
    is_minimized || !is_visible
}

/// Resolves the 3-tier graphics orchestration mode for the main window.
/// Side-by-side focus loss stays in `Unfocused` so loading bars and glass surfaces stay intact;
/// minimizing, hiding, or entering an active League match switches to `ZeroGpu`.
pub const fn resolve_window_graphics_tier(
    is_minimized: bool,
    is_visible: bool,
    is_focused: bool,
    is_in_game: bool,
) -> WindowGraphicsTier {
    if is_window_minimized_or_hidden(is_minimized, is_visible) || is_in_game {
        WindowGraphicsTier::ZeroGpu
    } else if !is_focused {
        WindowGraphicsTier::Unfocused
    } else {
        WindowGraphicsTier::Focused
    }
}

/// Computes the native WebView2 visibility, suspension, and memory target policy.
/// `TrySuspend` is requested only when minimized/hidden during an active match so side-by-side
/// windows and normal desktop browsing keep live updates intact.
pub const fn evaluate_webview_power_policy(
    is_minimized: bool,
    is_visible: bool,
    is_focused: bool,
    is_in_game: bool,
) -> WebviewPowerPolicy {
    let minimized_or_hidden = is_window_minimized_or_hidden(is_minimized, is_visible);
    WebviewPowerPolicy {
        controller_visible: !minimized_or_hidden,
        should_suspend: minimized_or_hidden && is_in_game,
        low_memory_target: minimized_or_hidden || !is_focused || is_in_game,
    }
}

pub fn apply_webview_power_policy_if_changed(
    window: &tauri::WebviewWindow,
    last_policy_slot: &Arc<AtomicU8>,
    policy: WebviewPowerPolicy,
) {
    let target_encoded = policy.encode();
    let prev = last_policy_slot.swap(target_encoded, Ordering::SeqCst);
    if prev == target_encoded {
        return;
    }
    apply_platform_webview_power(window, last_policy_slot, target_encoded, policy);
}

/// Synchronizes native WebView2 visibility/suspend/memory policy for `"main"` using live window & gameflow state.
pub fn sync_main_webview_power_state(app_handle: &tauri::AppHandle) {
    let Some(window) = app_handle.get_webview_window("main") else {
        return;
    };
    let Some(runtime_state) = app_handle.try_state::<WindowRuntimeState>() else {
        return;
    };

    let is_minimized = window.is_minimized().unwrap_or(false);
    let is_visible = window.is_visible().unwrap_or(true);
    let is_focused = window.is_focused().unwrap_or(true);
    let is_in_game = crate::ws_listener::is_in_game_phase_active();

    let policy = evaluate_webview_power_policy(is_minimized, is_visible, is_focused, is_in_game);
    apply_webview_power_policy_if_changed(&window, &runtime_state.last_power_policy, policy);
}

#[cfg(not(windows))]
fn apply_platform_webview_power(
    _window: &tauri::WebviewWindow,
    _last_policy_slot: &Arc<AtomicU8>,
    _target_encoded: u8,
    _policy: WebviewPowerPolicy,
) {
}

#[cfg(windows)]
fn apply_platform_webview_power(
    window: &tauri::WebviewWindow,
    last_policy_slot: &Arc<AtomicU8>,
    target_encoded: u8,
    policy: WebviewPowerPolicy,
) {
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW, COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL,
        ICoreWebView2_3, ICoreWebView2_19,
    };
    use webview2_com::TrySuspendCompletedHandler;
    use windows_core::{BOOL, Interface};

    let policy_slot = Arc::clone(last_policy_slot);
    if let Err(err) = window.with_webview(move |platform_webview| {
        let controller = platform_webview.controller();
        let Ok(core) = (unsafe { controller.CoreWebView2() }) else {
            let _ = policy_slot.compare_exchange(
                target_encoded,
                UNSET_POWER_POLICY,
                Ordering::SeqCst,
                Ordering::SeqCst,
            );
            return;
        };

        if let Ok(webview19) = core.cast::<ICoreWebView2_19>() {
            let level = if policy.low_memory_target {
                COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW
            } else {
                COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL
            };
            let _ = unsafe { webview19.SetMemoryUsageTargetLevel(level) };
        }

        if policy.should_suspend {
            if let Err(err) = unsafe { controller.SetIsVisible(false) } {
                crate::logging::log_debug(
                    "app.webview_power",
                    format!("event=set_invisible_failed err={err}"),
                );
                let _ = policy_slot.compare_exchange(
                    target_encoded,
                    UNSET_POWER_POLICY,
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                );
                return;
            }
            if let Ok(webview3) = core.cast::<ICoreWebView2_3>() {
                let mut is_suspended = BOOL::default();
                let already_suspended = unsafe { webview3.IsSuspended(&mut is_suspended) }.is_ok()
                    && is_suspended.as_bool();
                if !already_suspended {
                    let retry_slot = Arc::clone(&policy_slot);
                    let webview3_cb = webview3.clone();
                    let handler = TrySuspendCompletedHandler::create(Box::new(
                        move |result, is_successful| {
                            let current = retry_slot.load(Ordering::SeqCst);
                            if result.is_err() || !is_successful {
                                crate::logging::log_debug(
                                    "app.webview_power",
                                    format!(
                                        "event=try_suspend_rejected ok={} err={:?}",
                                        is_successful,
                                        result.err()
                                    ),
                                );
                                let _ = retry_slot.compare_exchange(
                                    target_encoded,
                                    UNSET_POWER_POLICY,
                                    Ordering::SeqCst,
                                    Ordering::SeqCst,
                                );
                            } else if current != target_encoded {
                                let _ = unsafe { webview3_cb.Resume() };
                                crate::logging::log_debug(
                                    "app.webview_power",
                                    "event=webview_resumed_after_stale_suspend",
                                );
                            } else {
                                crate::logging::log_debug(
                                    "app.webview_power",
                                    "event=webview_suspended",
                                );
                            }
                            Ok(())
                        },
                    ));
                    if let Err(err) = unsafe { webview3.TrySuspend(&handler) } {
                        crate::logging::log_debug(
                            "app.webview_power",
                            format!("event=try_suspend_invoke_failed err={err}"),
                        );
                        let _ = policy_slot.compare_exchange(
                            target_encoded,
                            UNSET_POWER_POLICY,
                            Ordering::SeqCst,
                            Ordering::SeqCst,
                        );
                    }
                }
            }
        } else {
            if let Ok(webview3) = core.cast::<ICoreWebView2_3>() {
                let _ = unsafe { webview3.Resume() };
            }
            let _ = unsafe { controller.SetIsVisible(policy.controller_visible) };
        }
    }) {
        crate::logging::log_debug(
            "app.webview_power",
            format!("event=with_webview_failed err={err}"),
        );
        let _ = last_policy_slot.compare_exchange(
            target_encoded,
            UNSET_POWER_POLICY,
            Ordering::SeqCst,
            Ordering::SeqCst,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_window_graphics_tier_separates_three_modes() {
        assert_eq!(
            resolve_window_graphics_tier(false, true, true, false),
            WindowGraphicsTier::Focused
        );
        assert_eq!(
            resolve_window_graphics_tier(false, true, false, false),
            WindowGraphicsTier::Unfocused
        );
        assert_eq!(
            resolve_window_graphics_tier(true, true, true, false),
            WindowGraphicsTier::ZeroGpu
        );
        assert_eq!(
            resolve_window_graphics_tier(false, false, true, false),
            WindowGraphicsTier::ZeroGpu
        );
        assert_eq!(
            resolve_window_graphics_tier(false, true, true, true),
            WindowGraphicsTier::ZeroGpu
        );
        assert_eq!(
            resolve_window_graphics_tier(false, true, false, true),
            WindowGraphicsTier::ZeroGpu
        );
    }

    #[test]
    fn test_evaluate_webview_power_policy_suspends_only_when_minimized_in_game() {
        assert_eq!(
            evaluate_webview_power_policy(false, true, true, false),
            WebviewPowerPolicy {
                controller_visible: true,
                should_suspend: false,
                low_memory_target: false,
            }
        );
        assert_eq!(
            evaluate_webview_power_policy(false, true, false, false),
            WebviewPowerPolicy {
                controller_visible: true,
                should_suspend: false,
                low_memory_target: true,
            }
        );
        assert_eq!(
            evaluate_webview_power_policy(true, true, false, false),
            WebviewPowerPolicy {
                controller_visible: false,
                should_suspend: false,
                low_memory_target: true,
            }
        );
        assert_eq!(
            evaluate_webview_power_policy(false, true, true, true),
            WebviewPowerPolicy {
                controller_visible: true,
                should_suspend: false,
                low_memory_target: true,
            }
        );
        assert_eq!(
            evaluate_webview_power_policy(true, true, false, true),
            WebviewPowerPolicy {
                controller_visible: false,
                should_suspend: true,
                low_memory_target: true,
            }
        );
        assert_eq!(
            evaluate_webview_power_policy(false, false, false, true),
            WebviewPowerPolicy {
                controller_visible: false,
                should_suspend: true,
                low_memory_target: true,
            }
        );
    }
}
