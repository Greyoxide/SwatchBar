//! While the loupe is up, a click should pick the color instead of reaching the app underneath.
//! That needs a global event tap that can swallow the click (macOS asks for Accessibility access).
//! Without that access we fall back to a listen-only tap: the click still picks, but also passes through.

use crate::app::Msg;
use core_foundation::runloop::{kCFRunLoopCommonModes, CFRunLoop};
use core_graphics::event::{
    CGEventFlags, CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement, CGEventType, CallbackResult,
};
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::mpsc::Sender;
use std::sync::OnceLock;
use std::thread::Thread;

static SAMPLING: AtomicBool = AtomicBool::new(false);
static SWALLOW_UP: AtomicBool = AtomicBool::new(false);
/// 0 = no tap, 1 = listen-only, 2 = can swallow clicks.
static MODE: AtomicU8 = AtomicU8::new(0);
static PROMPTED: AtomicBool = AtomicBool::new(false);
/// The button poller, parked whenever we aren't sampling.
static POLLER: OnceLock<Thread> = OnceLock::new();

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn AXIsProcessTrustedWithOptions(options: *const c_void) -> bool;
    fn CGEventSourceButtonState(state: i32, button: u32) -> bool;
    fn CGEventSourceFlagsState(state: i32) -> u64;
}

pub fn set_sampling(on: bool) {
    SAMPLING.store(on, Ordering::SeqCst);
    if let Some(t) = POLLER.get() {
        t.unpark();
    }
    if on && MODE.load(Ordering::SeqCst) < 2 {
        prompt_accessibility();
    }
}

/// Shows the system Accessibility prompt once per launch.
fn prompt_accessibility() {
    if PROMPTED.swap(true, Ordering::SeqCst) {
        return;
    }
    use core_foundation::base::TCFType;
    use core_foundation::boolean::CFBoolean;
    use core_foundation::dictionary::CFDictionary;
    use core_foundation::string::CFString;
    let key = CFString::new("AXTrustedCheckOptionPrompt");
    let opts = CFDictionary::from_CFType_pairs(&[(key.as_CFType(), CFBoolean::true_value().as_CFType())]);
    unsafe { AXIsProcessTrustedWithOptions(opts.as_concrete_TypeRef() as *const c_void) };
}

pub fn install(tx: Sender<Msg>, ctx: egui::Context) {
    // Needs no permission: watch the physical button so a click always picks, even if the tap is unavailable.
    {
        let (tx, ctx) = (tx.clone(), ctx.clone());
        let h = std::thread::spawn(move || {
            let mut prev = false;
            loop {
                // Idle without waking the CPU until sampling starts.
                if !SAMPLING.load(Ordering::SeqCst) {
                    while !SAMPLING.load(Ordering::SeqCst) {
                        std::thread::park();
                    }
                    // A button already held when sampling began isn't a click.
                    prev = unsafe { CGEventSourceButtonState(1, 0) };
                }
                std::thread::sleep(std::time::Duration::from_millis(8));
                let down = unsafe { CGEventSourceButtonState(1, 0) };
                // The tap reports clicks itself when it can; this covers the no-permission case.
                if down && !prev && SAMPLING.load(Ordering::SeqCst) && MODE.load(Ordering::SeqCst) != 2 {
                    let shift = unsafe { CGEventSourceFlagsState(1) } & 0x2_0000 != 0;
                    let _ = tx.send(Msg::Click { shift });
                    ctx.request_repaint();
                }
                prev = down;
            }
        });
        let _ = POLLER.set(h.thread().clone());
    }
    std::thread::spawn(move || {
        let events = vec![
            CGEventType::LeftMouseDown,
            CGEventType::LeftMouseUp,
            CGEventType::RightMouseDown,
            CGEventType::RightMouseUp,
        ];
        let make = |opts: CGEventTapOptions| {
            let (tx, ctx) = (tx.clone(), ctx.clone());
            CGEventTap::new(
                CGEventTapLocation::Session,
                CGEventTapPlacement::HeadInsertEventTap,
                opts,
                events.clone(),
                move |_p, ty, e| {
                    if !SAMPLING.load(Ordering::SeqCst) {
                        return CallbackResult::Keep;
                    }
                    let swallow = MODE.load(Ordering::SeqCst) == 2;
                    match ty {
                        CGEventType::LeftMouseDown | CGEventType::RightMouseDown => {
                            let left = matches!(ty, CGEventType::LeftMouseDown);
                            // Listen-only taps leave left clicks to the poller above.
                            if !left || swallow {
                                let shift = e.get_flags().contains(CGEventFlags::CGEventFlagShift);
                                let _ = tx.send(if left { Msg::Click { shift } } else { Msg::Cancel });
                                ctx.request_repaint();
                            }
                            if swallow {
                                SWALLOW_UP.store(true, Ordering::SeqCst);
                                return CallbackResult::Drop;
                            }
                            CallbackResult::Keep
                        }
                        _ => {
                            if swallow && SWALLOW_UP.swap(false, Ordering::SeqCst) {
                                CallbackResult::Drop
                            } else {
                                CallbackResult::Keep
                            }
                        }
                    }
                },
            )
        };
        let (tap, mode) = match make(CGEventTapOptions::Default) {
            Ok(t) => (t, 2),
            Err(()) => match make(CGEventTapOptions::ListenOnly) {
                Ok(t) => (t, 1),
                Err(()) => return,
            },
        };
        MODE.store(mode, Ordering::SeqCst);
        let Ok(src) = tap.mach_port().create_runloop_source(0) else { return };
        CFRunLoop::get_current().add_source(&src, unsafe { kCFRunLoopCommonModes });
        tap.enable();
        CFRunLoop::run_current();
    });
}
