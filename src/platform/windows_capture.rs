//! Screen sampling on Windows via GDI. No permission is needed.
//!
//! The process is per-monitor DPI aware (winit sets that up), so cursor and screen coordinates
//! are physical pixels. We report "points" by dividing by the monitor's scale so positions line
//! up with egui's logical window coordinates.

use super::Sample;
use crate::color::Rgb;
use std::mem::{size_of, zeroed};
use std::ptr::null_mut;
use windows_sys::Win32::Foundation::{LPARAM, POINT, RECT};
use windows_sys::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, EnumDisplayMonitors, GetDC, GetDIBits,
    GetMonitorInfoW, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, CAPTUREBLT, DIB_RGB_COLORS, HDC,
    HMONITOR, MONITORINFO, SRCCOPY,
};
use windows_sys::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use windows_sys::Win32::UI::WindowsAndMessaging::{GetCursorPos, SetCursorPos};

pub const NEEDS_PERMISSION: bool = false;
pub const HAS_SETTINGS_PANE: bool = false;
pub const PERMISSION_NAME: &str = "Screen capture";
pub const PERMISSION_BANNER: &str = "";
pub const PERMISSION_DETAIL: &str = "";

pub fn has_screen_access() -> bool {
    true
}

pub fn request_screen_access() -> bool {
    true
}

pub fn open_screen_recording_settings() {}

struct Monitor {
    /// Physical pixel rect (left, top, right, bottom).
    px: (i32, i32, i32, i32),
    scale: f64,
}

impl Monitor {
    fn contains_px(&self, x: i32, y: i32) -> bool {
        x >= self.px.0 && x < self.px.2 && y >= self.px.1 && y < self.px.3
    }
    /// Bounds in points: (x, y, w, h).
    fn points(&self) -> (f64, f64, f64, f64) {
        let s = self.scale;
        (
            self.px.0 as f64 / s,
            self.px.1 as f64 / s,
            (self.px.2 - self.px.0) as f64 / s,
            (self.px.3 - self.px.1) as f64 / s,
        )
    }
}

unsafe extern "system" fn collect(h: HMONITOR, _dc: HDC, _r: *mut RECT, data: LPARAM) -> i32 {
    let out = unsafe { &mut *(data as *mut Vec<Monitor>) };
    let mut info: MONITORINFO = unsafe { zeroed() };
    info.cbSize = size_of::<MONITORINFO>() as u32;
    if unsafe { GetMonitorInfoW(h, &mut info) } == 0 {
        return 1;
    }
    let (mut dx, mut dy) = (96u32, 96u32);
    if unsafe { GetDpiForMonitor(h, MDT_EFFECTIVE_DPI, &mut dx, &mut dy) } != 0 || dx == 0 {
        dx = 96;
    }
    let r = info.rcMonitor;
    out.push(Monitor { px: (r.left, r.top, r.right, r.bottom), scale: dx as f64 / 96.0 });
    1
}

fn monitors() -> Vec<Monitor> {
    let mut v: Vec<Monitor> = vec![];
    unsafe { EnumDisplayMonitors(null_mut(), std::ptr::null(), Some(collect), &mut v as *mut _ as LPARAM) };
    v
}

fn cursor_px() -> Option<(i32, i32)> {
    let mut p = POINT { x: 0, y: 0 };
    (unsafe { GetCursorPos(&mut p) } != 0).then_some((p.x, p.y))
}

fn monitor_at_px(mons: Vec<Monitor>, x: i32, y: i32) -> Option<Monitor> {
    let mut first = None;
    for m in mons {
        if m.contains_px(x, y) {
            return Some(m);
        }
        first.get_or_insert(m);
    }
    first
}

/// Cursor position in global points.
pub fn cursor_pos() -> Option<(f64, f64)> {
    let (x, y) = cursor_px()?;
    let scale = monitor_at_px(monitors(), x, y).map(|m| m.scale).unwrap_or(1.0);
    Some((x as f64 / scale, y as f64 / scale))
}

/// Moves the cursor by whole physical pixels.
pub fn nudge_cursor(dx: f64, dy: f64) {
    if let Some((x, y)) = cursor_px() {
        unsafe { SetCursorPos(x + dx.round() as i32, y + dy.round() as i32) };
    }
}

/// Bounds (x, y, w, h) in points of the display containing the point.
pub fn display_at(cx: f64, cy: f64) -> (f64, f64, f64, f64) {
    let mons = monitors();
    let hit = mons.iter().position(|m| {
        let (x, y, w, h) = m.points();
        cx >= x && cx < x + w && cy >= y && cy < y + h
    });
    match hit.map(|i| &mons[i]).or(mons.first()) {
        Some(m) => m.points(),
        None => (0.0, 0.0, 1920.0, 1080.0),
    }
}

/// Grabs an n*n block of physical pixels centered on the pixel under the cursor.
/// `n` must be odd. Areas that fall off-screen read as black.
pub fn grab(n: usize) -> Option<Sample> {
    let (px, py) = cursor_px()?;
    let mon = monitor_at_px(monitors(), px, py)?;
    let half = (n / 2) as i32;

    let mut bgra = vec![0u8; n * n * 4];
    unsafe {
        let screen = GetDC(null_mut());
        if screen.is_null() {
            return None;
        }
        let mem = CreateCompatibleDC(screen);
        let bmp = CreateCompatibleBitmap(screen, n as i32, n as i32);
        let old = SelectObject(mem, bmp);
        let ok = BitBlt(mem, 0, 0, n as i32, n as i32, screen, px - half, py - half, SRCCOPY | CAPTUREBLT) != 0;
        let mut bi: BITMAPINFO = zeroed();
        bi.bmiHeader = BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: n as i32,
            biHeight: -(n as i32), // top-down
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
            ..zeroed()
        };
        let lines = if ok {
            GetDIBits(mem, bmp, 0, n as u32, bgra.as_mut_ptr().cast(), &mut bi, DIB_RGB_COLORS)
        } else {
            0
        };
        SelectObject(mem, old);
        DeleteObject(bmp);
        DeleteDC(mem);
        ReleaseDC(null_mut(), screen);
        if lines as usize != n {
            return None;
        }
    }

    // Windows has no per-display color management for screen reads; treat the output as sRGB.
    let pixels: Vec<Rgb> = bgra.chunks_exact(4).map(|p| [p[2], p[1], p[0]]).collect();
    let center = pixels[(n / 2) * n + n / 2];
    let (dx, dy, dw, dh) = mon.points();
    Some(Sample { pixels, n, center, cursor: (px as f64 / mon.scale, py as f64 / mon.scale), display: (dx, dy, dw, dh) })
}

#[cfg(test)]
mod tests {
    /// Needs a desktop session: `cargo test -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn grabs_pixels_under_cursor() {
        let s = super::grab(17).expect("grab failed");
        println!("cursor={:?} center={:?} display={:?}", s.cursor, s.center, s.display);
        assert_eq!(s.pixels.len(), 17 * 17);
    }
}
