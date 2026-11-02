//! While the loupe is up, a click should pick the color instead of reaching the app underneath.
//! A low-level mouse hook (no permission needed) sees every click and can swallow it.

use crate::app::Msg;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Mutex, OnceLock};
use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetKeyState, VK_SHIFT};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage, MSG, WH_MOUSE_LL, WM_LBUTTONDOWN,
    WM_LBUTTONUP, WM_RBUTTONDOWN, WM_RBUTTONUP,
};

static SAMPLING: AtomicBool = AtomicBool::new(false);
static SWALLOW_LEFT_UP: AtomicBool = AtomicBool::new(false);
static SWALLOW_RIGHT_UP: AtomicBool = AtomicBool::new(false);
static SINK: OnceLock<Mutex<(Sender<Msg>, egui::Context)>> = OnceLock::new();

pub fn set_sampling(on: bool) {
    SAMPLING.store(on, Ordering::SeqCst);
}

fn send(msg: Msg) {
    if let Some(sink) = SINK.get() {
        if let Ok(g) = sink.lock() {
            let _ = g.0.send(msg);
            g.1.request_repaint();
        }
    }
}

unsafe extern "system" fn hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 && SAMPLING.load(Ordering::SeqCst) {
        match wparam as u32 {
            WM_LBUTTONDOWN => {
                let shift = unsafe { GetKeyState(VK_SHIFT as i32) } < 0;
                SWALLOW_LEFT_UP.store(true, Ordering::SeqCst);
                send(Msg::Click { shift });
                return 1;
            }
            WM_RBUTTONDOWN => {
                SWALLOW_RIGHT_UP.store(true, Ordering::SeqCst);
                send(Msg::Cancel);
                return 1;
            }
            WM_LBUTTONUP if SWALLOW_LEFT_UP.swap(false, Ordering::SeqCst) => return 1,
            WM_RBUTTONUP if SWALLOW_RIGHT_UP.swap(false, Ordering::SeqCst) => return 1,
            _ => {}
        }
    } else if code >= 0 {
        // Sampling ended between the press and the release: still eat the release so the app
        // underneath doesn't see an orphan mouse-up.
        match wparam as u32 {
            WM_LBUTTONUP if SWALLOW_LEFT_UP.swap(false, Ordering::SeqCst) => return 1,
            WM_RBUTTONUP if SWALLOW_RIGHT_UP.swap(false, Ordering::SeqCst) => return 1,
            _ => {}
        }
    }
    unsafe { CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam) }
}

pub fn install(tx: Sender<Msg>, ctx: egui::Context) {
    let _ = SINK.set(Mutex::new((tx, ctx)));
    std::thread::spawn(|| unsafe {
        let h = SetWindowsHookExW(WH_MOUSE_LL, Some(hook), std::ptr::null_mut(), 0);
        if h.is_null() {
            return;
        }
        // A low-level hook is serviced from this thread's message loop.
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    });
}
