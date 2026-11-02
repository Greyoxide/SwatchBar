//! Screen sampling. Each OS has its own backend in `platform/`; all expose the same functions:
//! `has_screen_access`, `request_screen_access`, `open_screen_recording_settings`, `cursor_pos`,
//! `nudge_cursor`, `display_at`, `grab`, plus the permission strings below.
//!
//! Coordinates are "points" in a global space (origin top-left of the main display) that match
//! egui's logical window coordinates. Pixels are always sRGB.

use crate::color::Rgb;

pub struct Sample {
    /// n*n pixels, row-major, sRGB.
    pub pixels: Vec<Rgb>,
    pub n: usize,
    pub center: Rgb,
    pub cursor: (f64, f64),
    /// Bounds of the display the cursor is on: (x, y, w, h) in points.
    pub display: (f64, f64, f64, f64),
}

#[cfg(target_os = "macos")]
#[path = "platform/macos_capture.rs"]
mod imp;
#[cfg(target_os = "windows")]
#[path = "platform/windows_capture.rs"]
mod imp;
#[cfg(target_os = "linux")]
#[path = "platform/linux_capture.rs"]
mod imp;

pub use imp::*;
