#[cfg(windows)]
mod hook;

pub const RESIZE_THROTTLE_MS: u64 = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowPosAction {
    ForwardUnchanged,
    ForwardResize,
    SwallowPureMove,
    SwallowAndArmResizeTimer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeferredSizeMoveFlush {
    pub moved: bool,
    pub resized: bool,
}

#[derive(Debug, Clone)]
pub struct SizeMoveThrottle {
    in_sizemove: bool,
    deferred_move: bool,
    deferred_size: bool,
    last_resize_forward_ms: Option<u64>,
    resize_throttle_ms: u64,
}

impl SizeMoveThrottle {
    pub fn new(resize_throttle_ms: u64) -> Self {
        Self {
            in_sizemove: false,
            deferred_move: false,
            deferred_size: false,
            last_resize_forward_ms: None,
            resize_throttle_ms,
        }
    }

    pub fn is_in_sizemove(&self) -> bool {
        self.in_sizemove
    }

    pub fn on_enter_sizemove(&mut self) {
        self.in_sizemove = true;
        self.deferred_move = false;
        self.deferred_size = false;
        self.last_resize_forward_ms = None;
    }

    pub fn on_exit_sizemove(&mut self) -> Option<DeferredSizeMoveFlush> {
        self.in_sizemove = false;
        self.last_resize_forward_ms = None;
        let moved = self.deferred_move;
        let resized = self.deferred_size;
        self.deferred_move = false;
        self.deferred_size = false;
        (moved || resized).then_some(DeferredSizeMoveFlush { moved, resized })
    }

    pub fn note_deferred_move(&mut self) {
        self.deferred_move = true;
    }

    pub fn note_deferred_size(&mut self) {
        self.deferred_size = true;
    }

    pub fn on_window_pos_changed(
        &mut self,
        has_lifecycle_flags: bool,
        no_size: bool,
        no_move: bool,
        now_ms: u64,
    ) -> WindowPosAction {
        if !self.in_sizemove || has_lifecycle_flags || (no_size && no_move) {
            return WindowPosAction::ForwardUnchanged;
        }
        if no_size {
            self.deferred_move = true;
            return WindowPosAction::SwallowPureMove;
        }

        let due = self
            .last_resize_forward_ms
            .is_none_or(|prev| now_ms.saturating_sub(prev) >= self.resize_throttle_ms);
        if due {
            self.last_resize_forward_ms = Some(now_ms);
            self.deferred_size = false;
            if !no_move {
                self.deferred_move = false;
            }
            WindowPosAction::ForwardResize
        } else {
            self.deferred_size = true;
            if !no_move {
                self.deferred_move = true;
            }
            WindowPosAction::SwallowAndArmResizeTimer
        }
    }

    pub fn take_timer_flush(&mut self, now_ms: u64) -> Option<DeferredSizeMoveFlush> {
        if !self.deferred_size {
            return None;
        }
        let moved = self.deferred_move;
        self.deferred_move = false;
        self.deferred_size = false;
        self.last_resize_forward_ms = Some(now_ms);
        Some(DeferredSizeMoveFlush {
            moved,
            resized: true,
        })
    }
}

/// Installs a Win32 window subclass on Windows that coalesces high-frequency move/resize modal-loop
/// messages so `wry`/`tao`/WebView2 do not commit 1000Hz `NotifyParentWindowPositionChanged`/`SetBounds` calls.
pub fn install_sizemove_subclass(window: &tauri::WebviewWindow) {
    #[cfg(windows)]
    hook::install(window);
    #[cfg(not(windows))]
    let _ = window;
}

#[cfg(test)]
#[path = "win32_sizemove_tests.rs"]
mod tests;
