// Release builds on Windows are GUI apps: no console window.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod adjust;
mod app;
mod capture;
mod clicktap;
mod color;
mod hotkeys;
mod loupe;
mod mockups;
mod picker;
mod preview;
mod store;
mod studio;
mod theme;
mod transfer;
mod tray;
mod views;
mod widgets;


fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        // The root window is only a hidden host; real windows are child viewports.
        viewport: egui::ViewportBuilder::default()
            .with_title("Swatchbar")
            .with_visible(false)
            .with_decorations(false)
            .with_transparent(true)
            .with_mouse_passthrough(true)
            .with_position([-200.0, -200.0])
            .with_inner_size([1.0, 1.0])
            .with_taskbar(false),
        event_loop_builder: Some(Box::new(|b| {
            // Menu-bar app: no Dock icon, no app menu.
            #[cfg(target_os = "macos")]
            {
                use winit::platform::macos::{ActivationPolicy, EventLoopBuilderExtMacOS};
                b.with_activation_policy(ActivationPolicy::Accessory);
            }
            // Window positioning, global hotkeys and color sampling all need X11 (XWayland is fine).
            #[cfg(target_os = "linux")]
            {
                use winit::platform::x11::EventLoopBuilderExtX11;
                b.with_x11();
            }
            let _ = b;
        })),
        persist_window: false,
        ..Default::default()
    };
    eframe::run_native("Swatchbar", options, Box::new(|cc| Ok(Box::new(app::App::new(cc)))))
}
