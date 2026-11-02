//! Menu bar icon (macOS), notification-area icon (Windows) or status-notifier icon (Linux) + menu.

use crate::color::{self, Rgb};
use tray_icon::menu::{IconMenuItem, Menu, MenuId, MenuItem, PredefinedMenuItem, Submenu};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

pub const ID_SAMPLE: &str = "sample";
pub const ID_PICKER: &str = "picker";
pub const ID_STUDIO: &str = "studio";
pub const ID_SETTINGS: &str = "settings";
pub const ID_QUIT: &str = "quit";
pub const ID_CLEAR: &str = "clear-history";
pub const RECENT_PREFIX: &str = "recent:";

/// A 22pt-ish glyph: ring with a center dot and tick marks (a loupe reticle).
/// macOS draws it as a template (black + alpha, tinted by the system); elsewhere it gets the accent color.
fn glyph() -> Icon {
    let rgb: [u8; 3] = if cfg!(target_os = "macos") { [0, 0, 0] } else { [0x5B, 0x9B, 0xFF] };
    let n = 44u32;
    let c = (n as f32 - 1.0) / 2.0;
    let mut px = vec![0u8; (n * n * 4) as usize];
    for y in 0..n {
        for x in 0..n {
            let (dx, dy) = (x as f32 - c, y as f32 - c);
            let d = (dx * dx + dy * dy).sqrt();
            let ring = (d - 12.5).abs();
            let mut a: f32 = (1.0 - (ring - 1.2).max(0.0)).clamp(0.0, 1.0) * if ring < 2.4 { 1.0 } else { 0.0 };
            // dot
            a = a.max((1.0 - (d - 3.6).max(0.0)).clamp(0.0, 1.0));
            // ticks
            let on_axis = dx.abs() < 1.1 || dy.abs() < 1.1;
            if on_axis && d > 8.0 && d < 19.0 {
                a = a.max(0.95);
            }
            // hollow between dot and ticks
            if d > 3.6 + 1.0 && d < 8.0 {
                a = a.min(if ring < 2.4 { a } else { 0.0 });
            }
            let i = ((y * n + x) * 4) as usize;
            px[i..i + 3].copy_from_slice(&rgb);
            px[i + 3] = (a * 255.0) as u8;
        }
    }
    Icon::from_rgba(px, n, n).expect("icon")
}

fn chip(c: Rgb) -> tray_icon::menu::Icon {
    let n = 28u32;
    let mut px = vec![0u8; (n * n * 4) as usize];
    for y in 0..n {
        for x in 0..n {
            let (dx, dy) = (x as f32 - 13.5, y as f32 - 13.5);
            let r = (dx.abs().max(dy.abs())) as f32;
            let i = ((y * n + x) * 4) as usize;
            if r <= 12.0 {
                let edge = r > 11.0;
                let col = if edge { [0x80, 0x80, 0x80] } else { c };
                px[i..i + 3].copy_from_slice(&col);
                px[i + 3] = 255;
            }
        }
    }
    tray_icon::menu::Icon::from_rgba(px, n, n).expect("chip")
}

fn build_icon(sample_label: &str, picker_label: &str, recent: &[Rgb]) -> Result<TrayIcon, String> {
    let builder = TrayIconBuilder::new()
        .with_menu(Box::new(build_menu(sample_label, picker_label, recent)))
        .with_tooltip("Swatchbar");
    #[cfg(target_os = "macos")]
    let builder = builder.with_icon_templated(glyph());
    #[cfg(not(target_os = "macos"))]
    let builder = builder.with_icon(glyph());
    builder.build().map_err(|e| e.to_string())
}

#[cfg(not(target_os = "linux"))]
pub struct Tray {
    icon: TrayIcon,
}

#[cfg(not(target_os = "linux"))]
impl Tray {
    pub fn new(sample_label: &str, picker_label: &str, recent: &[Rgb]) -> Result<Self, String> {
        Ok(Self { icon: build_icon(sample_label, picker_label, recent)? })
    }

    pub fn rebuild(&self, sample_label: &str, picker_label: &str, recent: &[Rgb]) {
        self.icon
            .set_menu(Some(Box::new(build_menu(sample_label, picker_label, recent))));
    }
}

/// On Linux the tray icon needs a running GTK main loop, which winit doesn't provide, and its
/// objects aren't `Send`. So the icon lives on its own GTK thread and we send it menu updates.
#[cfg(target_os = "linux")]
pub struct Tray {
    tx: std::sync::mpsc::Sender<(String, String, Vec<Rgb>)>,
}

#[cfg(target_os = "linux")]
impl Tray {
    pub fn new(sample_label: &str, picker_label: &str, recent: &[Rgb]) -> Result<Self, String> {
        let (tx, rx) = std::sync::mpsc::channel::<(String, String, Vec<Rgb>)>();
        let first = (sample_label.to_string(), picker_label.to_string(), recent.to_vec());
        let (ready_tx, ready_rx) = std::sync::mpsc::channel::<Result<(), String>>();
        std::thread::spawn(move || {
            if let Err(e) = gtk::init() {
                let _ = ready_tx.send(Err(format!("gtk: {e}")));
                return;
            }
            let icon = match build_icon(&first.0, &first.1, &first.2) {
                Ok(i) => i,
                Err(e) => {
                    let _ = ready_tx.send(Err(e));
                    return;
                }
            };
            let _ = ready_tx.send(Ok(()));
            glib::timeout_add_local(std::time::Duration::from_millis(150), move || {
                while let Ok((s, p, r)) = rx.try_recv() {
                    icon.set_menu(Some(Box::new(build_menu(&s, &p, &r))));
                }
                glib::ControlFlow::Continue
            });
            gtk::main();
        });
        ready_rx.recv().map_err(|e| e.to_string())??;
        Ok(Self { tx })
    }

    pub fn rebuild(&self, sample_label: &str, picker_label: &str, recent: &[Rgb]) {
        let _ = self.tx.send((sample_label.to_string(), picker_label.to_string(), recent.to_vec()));
    }
}

fn build_menu(sample_label: &str, picker_label: &str, recent: &[Rgb]) -> Menu {
    let menu = Menu::new();
    let item = |id: &str, text: String| MenuItem::with_id(MenuId::new(id), text, true, None);
    let _ = menu.append(&item(ID_SAMPLE, format!("Sample Color…    {sample_label}")));
    let _ = menu.append(&item(ID_PICKER, format!("Quick Picker    {picker_label}")));
    let _ = menu.append(&PredefinedMenuItem::separator());
    let _ = menu.append(&item(ID_STUDIO, "Open Palette Studio".into()));

    let sub = Submenu::new("Recent Colors", !recent.is_empty());
    for c in recent.iter().take(12) {
        let it = IconMenuItem::with_id(
            MenuId::new(format!("{RECENT_PREFIX}{}", color::to_hex(*c))),
            color::to_hex_hash(*c),
            true,
            Some(chip(*c)),
            None,
        );
        let _ = sub.append(&it);
    }
    if !recent.is_empty() {
        let _ = sub.append(&PredefinedMenuItem::separator());
        let _ = sub.append(&item(ID_CLEAR, "Clear History".into()));
    }
    let _ = menu.append(&sub);

    let _ = menu.append(&PredefinedMenuItem::separator());
    let _ = menu.append(&item(ID_SETTINGS, "Settings…".into()));
    let _ = menu.append(&item(ID_QUIT, "Quit Swatchbar".into()));
    menu
}
