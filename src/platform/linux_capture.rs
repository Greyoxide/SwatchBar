//! Screen sampling on Linux via X11 (XGetImage on the root window). No permission is needed.
//!
//! Wayland doesn't let apps read other windows' pixels or see the global cursor, so on a Wayland
//! session this reports "no access" and the UI asks for an X11 session instead.
//!
//! Coordinates are X pixels divided by the UI scale (`WINIT_X11_SCALE_FACTOR` / `GDK_SCALE`,
//! default 1) so they line up with egui's logical window coordinates.

use super::Sample;
use crate::color::Rgb;
use std::sync::OnceLock;
use x11rb::connection::Connection;
use x11rb::protocol::randr::ConnectionExt as _;
use x11rb::protocol::xproto::{ConnectionExt as _, ImageFormat, ImageOrder, Window};
use x11rb::rust_connection::RustConnection;

pub const NEEDS_PERMISSION: bool = false;
pub const HAS_SETTINGS_PANE: bool = false;
pub const PERMISSION_NAME: &str = "Screen access";
pub const PERMISSION_BANNER: &str =
    "Swatchbar can't read the screen here. It needs an X11 session (Wayland doesn't allow color sampling).";
pub const PERMISSION_DETAIL: &str =
    "Sampling uses X11. On Wayland, log in with an \"X11\" / \"Xorg\" session, then relaunch Swatchbar.";

struct X {
    conn: RustConnection,
    root: Window,
    little_endian: bool,
    bits_per_pixel: u8,
    masks: [u32; 3], // red, green, blue
}

fn x() -> Option<&'static X> {
    static X: OnceLock<Option<X>> = OnceLock::new();
    X.get_or_init(|| {
        let (conn, screen_num) = x11rb::connect(None).ok()?;
        let setup = conn.setup();
        let screen = &setup.roots[screen_num];
        let depth = screen.root_depth;
        let bits_per_pixel = setup.pixmap_formats.iter().find(|f| f.depth == depth)?.bits_per_pixel;
        let visual = screen
            .allowed_depths
            .iter()
            .flat_map(|d| d.visuals.iter())
            .find(|v| v.visual_id == screen.root_visual)?;
        Some(X {
            root: screen.root,
            little_endian: setup.image_byte_order == ImageOrder::LSB_FIRST,
            bits_per_pixel,
            masks: [visual.red_mask, visual.green_mask, visual.blue_mask],
            conn,
        })
    })
    .as_ref()
}

fn scale() -> f64 {
    static SCALE: OnceLock<f64> = OnceLock::new();
    *SCALE.get_or_init(|| {
        ["WINIT_X11_SCALE_FACTOR", "GDK_SCALE"]
            .iter()
            .find_map(|k| std::env::var(k).ok()?.parse::<f64>().ok())
            .filter(|s| *s >= 1.0)
            .unwrap_or(1.0)
    })
}

pub fn has_screen_access() -> bool {
    let wayland = std::env::var("XDG_SESSION_TYPE").is_ok_and(|s| s.eq_ignore_ascii_case("wayland"));
    !wayland && x().is_some()
}

pub fn request_screen_access() -> bool {
    has_screen_access()
}

pub fn open_screen_recording_settings() {}

fn pointer_px() -> Option<(i32, i32)> {
    let x = x()?;
    let r = x.conn.query_pointer(x.root).ok()?.reply().ok()?;
    Some((r.root_x as i32, r.root_y as i32))
}

/// Cursor position in global points.
pub fn cursor_pos() -> Option<(f64, f64)> {
    let (px, py) = pointer_px()?;
    let s = scale();
    Some((px as f64 / s, py as f64 / s))
}

/// Moves the cursor by whole physical pixels.
pub fn nudge_cursor(dx: f64, dy: f64) {
    if let Some(x) = x() {
        let _ = x.conn.warp_pointer(x11rb::NONE, x11rb::NONE, 0, 0, 0, 0, dx.round() as i16, dy.round() as i16);
        let _ = x.conn.flush();
    }
}

/// Monitor rectangles in physical pixels: (x, y, w, h).
fn monitors_px() -> Vec<(i32, i32, i32, i32)> {
    let Some(x) = x() else { return vec![] };
    let from_randr = || {
        let r = x.conn.randr_get_monitors(x.root, true).ok()?.reply().ok()?;
        let v: Vec<_> = r
            .monitors
            .iter()
            .map(|m| (m.x as i32, m.y as i32, m.width as i32, m.height as i32))
            .collect();
        (!v.is_empty()).then_some(v)
    };
    from_randr().unwrap_or_else(|| {
        let g = x.conn.get_geometry(x.root).ok().and_then(|c| c.reply().ok());
        g.map(|g| vec![(0, 0, g.width as i32, g.height as i32)]).unwrap_or_default()
    })
}

/// Bounds (x, y, w, h) in points of the display containing the point.
pub fn display_at(cx: f64, cy: f64) -> (f64, f64, f64, f64) {
    let s = scale();
    let (px, py) = ((cx * s) as i32, (cy * s) as i32);
    let mons = monitors_px();
    let m = mons
        .iter()
        .find(|(x, y, w, h)| px >= *x && px < x + w && py >= *y && py < y + h)
        .or(mons.first())
        .copied()
        .unwrap_or((0, 0, 1920, 1080));
    (m.0 as f64 / s, m.1 as f64 / s, m.2 as f64 / s, m.3 as f64 / s)
}

/// Grabs an n*n block of physical pixels centered on the pixel under the cursor.
/// `n` must be odd. Areas that fall off-screen read as black.
pub fn grab(n: usize) -> Option<Sample> {
    let x = x()?;
    if x.bits_per_pixel != 32 {
        return None;
    }
    let (px, py) = pointer_px()?;
    let half = (n / 2) as i32;
    let geo = x.conn.get_geometry(x.root).ok()?.reply().ok()?;
    let (sw, sh) = (geo.width as i32, geo.height as i32);

    // GetImage fails if the rect leaves the root window, so fetch the visible part only.
    let (x0, y0) = ((px - half).max(0), (py - half).max(0));
    let (x1, y1) = ((px - half + n as i32).min(sw), (py - half + n as i32).min(sh));
    let (w, h) = (x1 - x0, y1 - y0);
    let mut pixels: Vec<Rgb> = vec![[0, 0, 0]; n * n];
    if w > 0 && h > 0 {
        let img = x
            .conn
            .get_image(ImageFormat::Z_PIXMAP, x.root, x0 as i16, y0 as i16, w as u16, h as u16, !0)
            .ok()?
            .reply()
            .ok()?;
        let channel = |p: u32, mask: u32| ((p & mask) >> mask.trailing_zeros()) as u8;
        for (i, b) in img.data.chunks_exact(4).take((w * h) as usize).enumerate() {
            let raw = [b[0], b[1], b[2], b[3]];
            let p = if x.little_endian { u32::from_le_bytes(raw) } else { u32::from_be_bytes(raw) };
            let (ix, iy) = (i as i32 % w, i as i32 / w);
            let (ox, oy) = ((x0 + ix) - (px - half), (y0 + iy) - (py - half));
            pixels[oy as usize * n + ox as usize] =
                [channel(p, x.masks[0]), channel(p, x.masks[1]), channel(p, x.masks[2])];
        }
    }

    let center = pixels[(n / 2) * n + n / 2];
    let s = scale();
    let (dx, dy, dw, dh) = display_at(px as f64 / s, py as f64 / s);
    Some(Sample { pixels, n, center, cursor: (px as f64 / s, py as f64 / s), display: (dx, dy, dw, dh) })
}

#[cfg(test)]
mod tests {
    /// Needs an X11 display: `cargo test -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn grabs_pixels_under_cursor() {
        let s = super::grab(17).expect("grab failed");
        println!("cursor={:?} center={:?} display={:?}", s.cursor, s.center, s.display);
        assert_eq!(s.pixels.len(), 17 * 17);
    }
}
