//! Docs tool: `SWATCHBAR_SHOTS=<dir> swatchbar` steps through the windows for an external
//! screenshot tool. It writes the current step's name to `<dir>/current`, keeps that window in
//! front, and advances when `<dir>/next` appears (it deletes the file); after the last step it
//! quits. egui can't capture its own secondary windows, hence the external tool.
//!
//! Point `HOME` / `APPDATA` / `XDG_CONFIG_HOME` at a scratch folder with a seeded `state.json`
//! so the windows show demo data instead of your own palettes.

use crate::app::{App, Tab, STUDIO};
use egui::{Context, ViewportCommand};
use std::path::PathBuf;
use std::time::{Duration, Instant};

pub struct Shots {
    dir: PathBuf,
    step: usize,
    entered: bool,
    since: Instant,
}

const STEPS: [&str; 7] = ["generator", "explore", "preview", "library", "settings", "picker", "loupe"];

impl Shots {
    pub fn from_env() -> Option<Self> {
        let dir = PathBuf::from(std::env::var_os("SWATCHBAR_SHOTS")?);
        std::fs::create_dir_all(&dir).ok()?;
        let _ = std::fs::remove_file(dir.join("next"));
        Some(Self { dir, step: 0, entered: false, since: Instant::now() })
    }
}

impl App {
    /// Called from inside the Studio's frame: keeps it in front while shooting.
    pub fn shots_focus(&self, ui: &egui::Ui) {
        if self.shots.is_some() {
            ui.send_viewport_cmd(ViewportCommand::Focus);
        }
    }

    pub fn drive_shots(&mut self, ctx: &Context) {
        let Some(mut s) = self.shots.take() else { return };
        self.screen_ok = true;
        self.screen_checked = Instant::now();
        ctx.request_repaint_after(Duration::from_millis(100));

        let Some(&name) = STEPS.get(s.step) else {
            self.d.save();
            std::process::exit(0);
        };
        if !s.entered {
            match name {
                "generator" => self.tab = Tab::Generator,
                "explore" => self.tab = Tab::Explore,
                "preview" => self.tab = Tab::Preview,
                "library" => self.tab = Tab::Library,
                "settings" => self.tab = Tab::Settings,
                "picker" => {
                    if !self.picker_open {
                        self.toggle_picker(ctx);
                    }
                }
                _ => {
                    // Only the loupe window itself: no global keys or click capture while shooting.
                    self.picker_open = false;
                    self.tab = Tab::Generator;
                    self.loupe = Some(crate::app::Loupe { sample: None });
                }
            }
            s.entered = true;
            s.since = Instant::now();
        } else if s.since.elapsed() > Duration::from_millis(1500) {
            // Settled: announce it, then wait to be told to move on.
            let _ = std::fs::write(s.dir.join("current"), name);
            if std::fs::remove_file(s.dir.join("next")).is_ok() {
                s.step += 1;
                s.entered = false;
                let _ = std::fs::remove_file(s.dir.join("current"));
            }
        }
        let _ = STUDIO;
        self.shots = Some(s);
    }
}
