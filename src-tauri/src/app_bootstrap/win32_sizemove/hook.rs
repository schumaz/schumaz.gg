use std::time::Instant;

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowRect, KillTimer, SIZE_RESTORED, SWP_FRAMECHANGED, SWP_HIDEWINDOW, SWP_NOACTIVATE,
    SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SWP_SHOWWINDOW, SetTimer, WINDOWPOS, WM_CANCELMODE,
    WM_ENTERSIZEMOVE, WM_EXITSIZEMOVE, WM_MOVE, WM_MOVING, WM_NCDESTROY, WM_SIZE, WM_SIZING,
    WM_TIMER, WM_WINDOWPOSCHANGED,
};

use super::{DeferredSizeMoveFlush, RESIZE_THROTTLE_MS, SizeMoveThrottle, WindowPosAction};

const SIZEMOVE_SUBCLASS_ID: usize = 0x5343_485A;
const RESIZE_FLUSH_TIMER_ID: usize = 0x5343_485B;

struct Win32SubclassState {
    throttle: SizeMoveThrottle,
    origin: Instant,
    flushing: bool,
    resize_timer_armed: bool,
}

impl Win32SubclassState {
    fn new() -> Self {
        Self {
            throttle: SizeMoveThrottle::new(RESIZE_THROTTLE_MS),
            origin: Instant::now(),
            flushing: false,
            resize_timer_armed: false,
        }
    }

    fn now_ms(&self) -> u64 {
        self.origin.elapsed().as_millis() as u64
    }
}

fn disarm_resize_timer(hwnd: HWND, state: &mut Win32SubclassState) {
    if state.resize_timer_armed {
        let _ = unsafe { KillTimer(Some(hwnd), RESIZE_FLUSH_TIMER_ID) };
        state.resize_timer_armed = false;
    }
}

fn arm_resize_timer(hwnd: HWND, state: &mut Win32SubclassState) {
    if !state.resize_timer_armed {
        let id = unsafe {
            SetTimer(
                Some(hwnd),
                RESIZE_FLUSH_TIMER_ID,
                RESIZE_THROTTLE_MS as u32,
                None,
            )
        };
        state.resize_timer_armed = id != 0;
    }
}

unsafe fn flush_window_pos_changed(
    hwnd: HWND,
    state_ptr: *mut Win32SubclassState,
    flush: DeferredSizeMoveFlush,
) {
    let mut rect = RECT::default();
    if unsafe { GetWindowRect(hwnd, &mut rect) }.is_err() {
        return;
    }
    let mut flags = SWP_NOZORDER | SWP_NOACTIVATE;
    if !flush.resized {
        flags |= SWP_NOSIZE;
    }
    if !flush.moved {
        flags |= SWP_NOMOVE;
    }
    let wp = WINDOWPOS {
        hwnd,
        hwndInsertAfter: HWND::default(),
        x: rect.left,
        y: rect.top,
        cx: rect.right - rect.left,
        cy: rect.bottom - rect.top,
        flags,
    };
    unsafe { (*state_ptr).flushing = true };
    let _ = unsafe {
        DefSubclassProc(
            hwnd,
            WM_WINDOWPOSCHANGED,
            WPARAM(0),
            LPARAM(&wp as *const WINDOWPOS as isize),
        )
    };
    unsafe { (*state_ptr).flushing = false };
}

unsafe fn handle_window_pos_changed(
    hwnd: HWND,
    wparam: WPARAM,
    lparam: LPARAM,
    state_ptr: *mut Win32SubclassState,
) -> LRESULT {
    let should_intercept = {
        let state = unsafe { &*state_ptr };
        state.throttle.is_in_sizemove() && !state.flushing && lparam.0 != 0
    };
    if !should_intercept {
        return unsafe { DefSubclassProc(hwnd, WM_WINDOWPOSCHANGED, wparam, lparam) };
    }

    let wp = unsafe { &*(lparam.0 as *const WINDOWPOS) };
    let has_lifecycle_flags =
        (wp.flags.0 & (SWP_SHOWWINDOW.0 | SWP_HIDEWINDOW.0 | SWP_FRAMECHANGED.0)) != 0;
    let no_size = (wp.flags.0 & SWP_NOSIZE.0) != 0;
    let no_move = (wp.flags.0 & SWP_NOMOVE.0) != 0;

    let action = {
        let state = unsafe { &mut *state_ptr };
        let now_ms = state.now_ms();
        let action =
            state
                .throttle
                .on_window_pos_changed(has_lifecycle_flags, no_size, no_move, now_ms);
        match action {
            WindowPosAction::ForwardResize => {
                disarm_resize_timer(hwnd, state);
                state.flushing = true;
            }
            WindowPosAction::SwallowAndArmResizeTimer => {
                arm_resize_timer(hwnd, state);
            }
            WindowPosAction::ForwardUnchanged | WindowPosAction::SwallowPureMove => {}
        }
        action
    };

    match action {
        WindowPosAction::ForwardUnchanged => {
            unsafe { DefSubclassProc(hwnd, WM_WINDOWPOSCHANGED, wparam, lparam) }
        }
        WindowPosAction::ForwardResize => {
            let res = unsafe { DefSubclassProc(hwnd, WM_WINDOWPOSCHANGED, wparam, lparam) };
            unsafe { (*state_ptr).flushing = false };
            res
        }
        WindowPosAction::SwallowPureMove | WindowPosAction::SwallowAndArmResizeTimer => LRESULT(0),
    }
}

unsafe extern "system" fn sizemove_subclass_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _uid: usize,
    ref_data: usize,
) -> LRESULT {
    if ref_data == 0 {
        return unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) };
    }
    let state_ptr = ref_data as *mut Win32SubclassState;

    match msg {
        WM_NCDESTROY => {
            {
                let state = unsafe { &mut *state_ptr };
                disarm_resize_timer(hwnd, state);
            }
            let _ = unsafe {
                RemoveWindowSubclass(hwnd, Some(sizemove_subclass_proc), SIZEMOVE_SUBCLASS_ID)
            };
            let res = unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) };
            drop(unsafe { Box::from_raw(state_ptr) });
            res
        }
        WM_ENTERSIZEMOVE => {
            unsafe { (*state_ptr).throttle.on_enter_sizemove() };
            unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
        }
        WM_EXITSIZEMOVE | WM_CANCELMODE => {
            let pending = {
                let state = unsafe { &mut *state_ptr };
                if state.throttle.is_in_sizemove() {
                    disarm_resize_timer(hwnd, state);
                    state.throttle.on_exit_sizemove()
                } else {
                    None
                }
            };
            let res = unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) };
            if let Some(flush) = pending {
                unsafe { flush_window_pos_changed(hwnd, state_ptr, flush) };
            }
            res
        }
        WM_MOVING => {
            let swallow = {
                let state = unsafe { &mut *state_ptr };
                if state.throttle.is_in_sizemove() && !state.flushing {
                    state.throttle.note_deferred_move();
                    true
                } else {
                    false
                }
            };
            if swallow {
                LRESULT(1)
            } else {
                unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
            }
        }
        WM_SIZING => {
            let swallow = {
                let state = unsafe { &*state_ptr };
                state.throttle.is_in_sizemove() && !state.flushing
            };
            if swallow {
                LRESULT(1)
            } else {
                unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
            }
        }
        WM_WINDOWPOSCHANGED => unsafe { handle_window_pos_changed(hwnd, wparam, lparam, state_ptr) },
        WM_MOVE => {
            let swallow = {
                let state = unsafe { &mut *state_ptr };
                if state.throttle.is_in_sizemove() && !state.flushing {
                    state.throttle.note_deferred_move();
                    true
                } else {
                    false
                }
            };
            if swallow {
                LRESULT(0)
            } else {
                unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
            }
        }
        WM_SIZE if wparam.0 == SIZE_RESTORED as usize => {
            let swallow = {
                let state = unsafe { &mut *state_ptr };
                if state.throttle.is_in_sizemove() && !state.flushing {
                    state.throttle.note_deferred_size();
                    arm_resize_timer(hwnd, state);
                    true
                } else {
                    false
                }
            };
            if swallow {
                LRESULT(0)
            } else {
                unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
            }
        }
        WM_TIMER if wparam.0 == RESIZE_FLUSH_TIMER_ID => {
            let flush = {
                let state = unsafe { &mut *state_ptr };
                disarm_resize_timer(hwnd, state);
                let now_ms = state.now_ms();
                state.throttle.take_timer_flush(now_ms)
            };
            if let Some(flush) = flush {
                unsafe { flush_window_pos_changed(hwnd, state_ptr, flush) };
            }
            LRESULT(0)
        }
        _ => unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) },
    }
}

pub fn install(window: &tauri::WebviewWindow) {
    let Ok(raw_hwnd) = window.hwnd() else {
        return;
    };
    let hwnd = HWND(raw_hwnd.0);
    let state_ptr = Box::into_raw(Box::new(Win32SubclassState::new()));
    let installed = unsafe {
        SetWindowSubclass(
            hwnd,
            Some(sizemove_subclass_proc),
            SIZEMOVE_SUBCLASS_ID,
            state_ptr as usize,
        )
    };
    if !installed.as_bool() {
        drop(unsafe { Box::from_raw(state_ptr) });
    }
}
