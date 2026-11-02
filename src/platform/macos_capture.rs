//! Screen sampling on macOS via CoreGraphics.
//!
//! Pixels are always converted to sRGB so the hex values match what designers
//! see in CSS / Figma, regardless of the display's own profile.

use super::Sample;
use crate::color::Rgb;
use core_graphics::base::kCGImageAlphaPremultipliedLast;
use core_graphics::color_space::{kCGColorSpaceSRGB, CGColorSpace};
use core_graphics::context::CGContext;
use core_graphics::display::{CGDisplay, CGPoint, CGRect, CGSize};
use core_graphics::event::CGEvent;
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

unsafe extern "C" {
    fn CGPreflightScreenCaptureAccess() -> bool;
    fn CGRequestScreenCaptureAccess() -> bool;
    fn CGWarpMouseCursorPosition(p: CGPoint) -> i32;
}

pub const NEEDS_PERMISSION: bool = true;
pub const HAS_SETTINGS_PANE: bool = true;
pub const PERMISSION_NAME: &str = "Screen Recording";
pub const PERMISSION_BANNER: &str = "Swatchbar can't read other apps' pixels yet. Allow Screen Recording, then relaunch.";
pub const PERMISSION_DETAIL: &str = "macOS only lets apps read other apps' pixels with this permission. After granting it, quit and reopen Swatchbar.";

pub fn has_screen_access() -> bool {
    unsafe { CGPreflightScreenCaptureAccess() }
}

/// Triggers the system prompt (first time only). Returns whether access is granted.
pub fn request_screen_access() -> bool {
    unsafe { CGRequestScreenCaptureAccess() }
}

pub fn open_screen_recording_settings() {
    let _ = std::process::Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture")
        .spawn();
}

/// Cursor position in global display points (origin top-left of the main display).
pub fn cursor_pos() -> Option<(f64, f64)> {
    let src = CGEventSource::new(CGEventSourceStateID::CombinedSessionState).ok()?;
    let ev = CGEvent::new(src).ok()?;
    let p = ev.location();
    Some((p.x, p.y))
}

pub fn nudge_cursor(dx: f64, dy: f64) {
    if let Some((x, y)) = cursor_pos() {
        unsafe {
            CGWarpMouseCursorPosition(CGPoint::new(x + dx, y + dy));
        }
    }
}

fn display_for(cx: f64, cy: f64) -> CGDisplay {
    CGDisplay::active_displays()
        .unwrap_or_default()
        .into_iter()
        .map(CGDisplay::new)
        .find(|d| {
            let b = d.bounds();
            cx >= b.origin.x && cx < b.origin.x + b.size.width && cy >= b.origin.y && cy < b.origin.y + b.size.height
        })
        .unwrap_or_else(CGDisplay::main)
}

/// Bounds (x, y, w, h) in points of the display containing the point.
pub fn display_at(cx: f64, cy: f64) -> (f64, f64, f64, f64) {
    let b = display_for(cx, cy).bounds();
    (b.origin.x, b.origin.y, b.size.width, b.size.height)
}

/// Grabs an n*n block of physical pixels centered on the pixel under the cursor.
/// `n` must be odd.
pub fn grab(n: usize) -> Option<Sample> {
    let (cx, cy) = cursor_pos()?;
    let disp = display_for(cx, cy);
    let b = disp.bounds();
    let scale = (disp.pixels_wide() as f64 / b.size.width).max(1.0);

    // Pixel under the cursor, then a pixel-aligned rect around it.
    let px = ((cx - b.origin.x) * scale).floor();
    let py = ((cy - b.origin.y) * scale).floor();
    let half = (n / 2) as f64;
    let rect = CGRect::new(
        &CGPoint::new(b.origin.x + (px - half) / scale, b.origin.y + (py - half) / scale),
        &CGSize::new(n as f64 / scale, n as f64 / scale),
    );
    let image = disp.image_for_rect(rect)?;

    let space = CGColorSpace::create_with_name(unsafe { kCGColorSpaceSRGB })?;
    let mut ctx = CGContext::create_bitmap_context(None, n, n, 8, n * 4, &space, kCGImageAlphaPremultipliedLast);
    ctx.draw_image(CGRect::new(&CGPoint::new(0.0, 0.0), &CGSize::new(n as f64, n as f64)), &image);
    let data = ctx.data();
    let pixels: Vec<Rgb> = data.chunks_exact(4).take(n * n).map(|p| [p[0], p[1], p[2]]).collect();
    if pixels.len() != n * n {
        return None;
    }
    let center = pixels[(n / 2) * n + n / 2];
    Some(Sample {
        pixels,
        n,
        center,
        cursor: (cx, cy),
        display: (b.origin.x, b.origin.y, b.size.width, b.size.height),
    })
}

#[cfg(test)]
mod tests {
    /// Needs a display + Screen Recording permission: `cargo test -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn grabs_pixels_under_cursor() {
        let s = super::grab(17).expect("grab failed");
        println!("access={} cursor={:?} center={:?} display={:?}", super::has_screen_access(), s.cursor, s.center, s.display);
        assert_eq!(s.pixels.len(), 17 * 17);
    }
}
