//! While the loupe is up, a click should pick the color instead of reaching the app underneath.
//! On X11 we take an active pointer grab on the root window, which delivers every click to us and
//! none to the app below. If another client already holds a grab (an open menu, say) we fall back
//! to watching the button state, so the click still picks but also passes through.

use crate::app::Msg;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, OnceLock};
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{
    ButtonIndex, ConnectionExt as _, EventMask, GrabMode, GrabStatus, KeyButMask, Window,
};
use x11rb::protocol::Event;
use x11rb::rust_connection::RustConnection;

static SAMPLING: AtomicBool = AtomicBool::new(false);
static GRABBED: AtomicBool = AtomicBool::new(false);
static LINK: OnceLock<(Arc<RustConnection>, Window)> = OnceLock::new();

pub fn set_sampling(on: bool) {
    SAMPLING.store(on, Ordering::SeqCst);
    let Some((conn, root)) = LINK.get() else { return };
    if on {
        let mask = EventMask::BUTTON_PRESS | EventMask::BUTTON_RELEASE;
        let status = conn
            .grab_pointer(false, *root, mask, GrabMode::ASYNC, GrabMode::ASYNC, x11rb::NONE, x11rb::NONE, x11rb::CURRENT_TIME)
            .ok()
            .and_then(|c| c.reply().ok())
            .map(|r| r.status);
        GRABBED.store(status == Some(GrabStatus::SUCCESS), Ordering::SeqCst);
    } else {
        GRABBED.store(false, Ordering::SeqCst);
        let _ = conn.ungrab_pointer(x11rb::CURRENT_TIME);
    }
    let _ = conn.flush();
}

fn shift_held(state: KeyButMask) -> bool {
    u16::from(state) & u16::from(KeyButMask::SHIFT) != 0
}

pub fn install(tx: Sender<Msg>, ctx: egui::Context) {
    let Ok((conn, screen_num)) = x11rb::connect(None) else { return };
    let root = conn.setup().roots[screen_num].root;
    let conn = Arc::new(conn);
    let _ = LINK.set((conn.clone(), root));

    // Receives the grabbed clicks.
    {
        let (conn, tx, ctx) = (conn.clone(), tx.clone(), ctx.clone());
        std::thread::spawn(move || {
            while let Ok(ev) = conn.wait_for_event() {
                if let Event::ButtonPress(b) = ev {
                    if !SAMPLING.load(Ordering::SeqCst) {
                        continue;
                    }
                    let msg = match ButtonIndex::from(b.detail) {
                        ButtonIndex::M1 => Msg::Click { shift: shift_held(b.state) },
                        ButtonIndex::M3 => Msg::Cancel,
                        _ => continue,
                    };
                    let _ = tx.send(msg);
                    ctx.request_repaint();
                }
            }
        });
    }

    // Fallback when the grab was refused: watch the physical left button.
    std::thread::spawn(move || {
        let mut prev = false;
        loop {
            std::thread::sleep(std::time::Duration::from_millis(8));
            let Some(r) = conn.query_pointer(root).ok().and_then(|c| c.reply().ok()) else { continue };
            let down = u16::from(r.mask) & u16::from(KeyButMask::BUTTON1) != 0;
            if down && !prev && SAMPLING.load(Ordering::SeqCst) && !GRABBED.load(Ordering::SeqCst) {
                let _ = tx.send(Msg::Click { shift: shift_held(r.mask) });
                ctx.request_repaint();
            }
            prev = down;
        }
    });
}
