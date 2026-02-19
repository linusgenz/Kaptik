use std::sync::Arc;
use parking_lot::Mutex;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;
use crate::apm::APMTracker;
use std::thread::{self, JoinHandle};

pub struct InputHook {
    tracker: Arc<Mutex<APMTracker>>,
    state: Arc<Mutex<HookState>>,
}

struct HookState {
    thread_id: Option<u32>,
    thread_handle: Option<JoinHandle<()>>,
}

lazy_static::lazy_static! {
    static ref GLOBAL_TRACKER: Mutex<Option<Arc<Mutex<APMTracker>>>> = Mutex::new(None);
}

use crate::log;

unsafe extern "system" fn keyboard_proc(n_code: i32, w_param: WPARAM, l_param: LPARAM) -> LRESULT { unsafe {
    if n_code >= 0 {
        let kbd = &*(l_param.0 as *const KBDLLHOOKSTRUCT);
        if w_param.0 == WM_KEYDOWN as usize || w_param.0 == WM_SYSKEYDOWN as usize {
            let vk = kbd.vkCode;
            if let Some(tracker) = &*GLOBAL_TRACKER.lock() {
                tracker.lock().record_key(vk);
            }
        }
    }
    CallNextHookEx(None, n_code, w_param, l_param)
}}

unsafe extern "system" fn mouse_proc(n_code: i32, w_param: WPARAM, l_param: LPARAM) -> LRESULT { unsafe {
    if n_code >= 0 {
        let ms = &*(l_param.0 as *const MSLLHOOKSTRUCT);
        match w_param.0 as u32 {
            WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN | WM_XBUTTONDOWN => {
                if let Some(tracker) = &*GLOBAL_TRACKER.lock() {
                    tracker.lock().record_mouse_button();
                }
            }
            WM_MOUSEWHEEL => {
                // high word of mouseData contains wheel delta (signed 16-bit)
                let raw = ((ms.mouseData >> 16) & 0xffff) as i16 as i32;
                if let Some(tracker) = &*GLOBAL_TRACKER.lock() {
                    tracker.lock().record_mouse_wheel(raw);
                }
            }
            _ => {}
        }
    }
    CallNextHookEx(None, n_code, w_param, l_param)
}}

impl InputHook {
    pub fn new(tracker: Arc<Mutex<APMTracker>>) -> Self {
        Self {
            tracker,
            state: Arc::new(Mutex::new(HookState {
                thread_id: None,
                thread_handle: None,
            })),
        }
    }

    pub fn start(&self) {
        let mut state = self.state.lock();

        if state.thread_handle.is_some() {
            return; // Already running
        }

        let tracker = self.tracker.clone();
        let state_arc = self.state.clone();

        let handle = thread::spawn(move || {
            unsafe {
                let tid = GetCurrentThreadId();
                {
                    let mut state = state_arc.lock();
                    state.thread_id = Some(tid);
                }

                *GLOBAL_TRACKER.lock() = Some(tracker.clone());

                let keyboard_hook = SetWindowsHookExW(
                    WH_KEYBOARD_LL,
                    Some(keyboard_proc),
                    None,
                    0
                ).expect("Failed to set keyboard hook");

                let mouse_hook = SetWindowsHookExW(
                    WH_MOUSE_LL,
                    Some(mouse_proc),
                    None,
                    0
                ).expect("Failed to set mouse hook");

                log!("🎮 Input hooks installed (Thread ID: {})", tid);

                let mut msg = MSG::default();
                while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }

                UnhookWindowsHookEx(keyboard_hook).ok();
                UnhookWindowsHookEx(mouse_hook).ok();
                *GLOBAL_TRACKER.lock() = None;

                {
                    let mut state = state_arc.lock();
                    state.thread_id = None;
                }

                log!("✅ Input hooks removed");
            }
        });

        state.thread_handle = Some(handle);

        drop(state);
        thread::sleep(std::time::Duration::from_millis(50));
    }

    pub fn stop(&self) {
        let mut state = self.state.lock();

        if let Some(tid) = state.thread_id {
            unsafe {
                use windows::Win32::UI::WindowsAndMessaging::PostThreadMessageW;
                PostThreadMessageW(tid, WM_QUIT, WPARAM(0), LPARAM(0)).ok();
            }
        }

        if let Some(handle) = state.thread_handle.take() {
            drop(state);
            let _ = handle.join();
        }
    }
}

impl Drop for InputHook {
    fn drop(&mut self) {
        self.stop();
    }
}