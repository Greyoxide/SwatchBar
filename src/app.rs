use crate::capture::{self, Sample};
use crate::color::{self, Rgb};
use crate::hotkeys::{self, Action, Hotkeys};
use crate::store::{Data, Swatch, View};
use crate::tray::{self, Tray};
use egui::{Context, ViewportCommand, ViewportId};
use global_hotkey::{GlobalHotKeyEvent, HotKeyState};
use std::sync::mpsc::{channel, Receiver};
use std::time::{Duration, Instant};
use tray_icon::menu::MenuEvent;

pub const STUDIO: &str = "studio";
pub const PICKER: &str = "picker";
pub const LOUPE: &str = "loupe";
pub const FLASH: &str = "flash";

pub fn vid(name: &str) -> ViewportId {
    ViewportId::from_hash_of(name)
}

pub enum Msg {
    Hotkey(u32),
    Menu(String),
    /// Left click while sampling: pick the color.
    Click { shift: bool },
    /// Right click while sampling: cancel.
    Cancel,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Generator,
    Explore,
    Preview,
    Library,
    Settings,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Slot {
    Sample,
    Picker,
}

pub struct Loupe {
    pub sample: Option<Sample>,
}

pub struct Drag {
    pub idx: usize,
    pub grab: egui::Vec2,
}

pub use crate::adjust::{AdjMode, Adjust};

/// One speck of the splash when a swatch is dropped.
pub struct Particle {
    pub pos: egui::Pos2,
    pub vel: egui::Vec2,
    pub age: f32,
    pub delay: f32,
    pub life: f32,
    pub size: f32,
    pub rgb: Rgb,
}

pub struct App {
    pub d: Data,
    pub dirty: bool,
    pub last_save: Instant,
    pub hk: Option<Hotkeys>,
    pub tray: Option<Tray>,
    pub rx: Receiver<Msg>,
    pub hk_errors: Vec<String>,

    pub studio_open: bool,
    pub picker_open: bool,
    pub picker_pos: egui::Pos2,
    pub picker_fresh: bool,
    pub loupe: Option<Loupe>,
    pub flash: Option<(Rgb, Instant, (f64, f64))>,

    pub tab: Tab,
    pub recording: Option<Slot>,
    pub toast: Option<(String, Instant)>,
    pub undo: Vec<Vec<Swatch>>,
    pub redo: Vec<Vec<Swatch>>,
    pub shades_col: Option<usize>,
    pub drag: Option<Drag>,
    pub particles: Vec<Particle>,
    // Preview tab
    pub preview_scene: usize,
    /// Recolorable packaging photos, loaded on first view.
    pub photos: Vec<Option<crate::transfer::Recolored>>,
    pub role_disp: [[f32; 3]; 5],
    pub role_init: bool,
    pub role_rects: [egui::Rect; 5],
    pub role_drag_from: Option<usize>,
    pub drop_slot: Option<usize>,
    /// Last tab used in the Adjust color window.
    pub adjust_mode: AdjMode,
    /// Where each swatch is currently drawn (eases toward its slot), by data index.
    pub slide: Vec<egui::Rect>,
    pub slide_key: (View, usize),
    /// The swatch that just landed after a drop, for a little pulse.
    pub landed: Option<(usize, Instant)>,
    /// A color being dragged out of the samples list, and where in the palette it would land.
    pub sample_drag: Option<Rgb>,
    /// The dragged sample has been dropped into the palette as a live swatch (and is being dragged there).
    pub sample_in: bool,
    /// After Esc, stay a plain chip until the pointer leaves the palette.
    pub sample_veto: bool,
    pub hex_edit: Option<(usize, String)>,
    pub adjust: Option<Adjust>,
    pub selected_hex: String,
    pub screen_ok: bool,
    pub screen_checked: Instant,
    pub focus_studio: bool,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let ctx = cc.egui_ctx.clone();
        crate::theme::apply(&ctx);

        let (tx, rx) = channel();
        {
            let (tx, ctx) = (tx.clone(), ctx.clone());
            GlobalHotKeyEvent::set_event_handler(Some(move |e: GlobalHotKeyEvent| {
                if e.state() == HotKeyState::Pressed {
                    let _ = tx.send(Msg::Hotkey(e.id()));
                    ctx.request_repaint();
                }
            }));
        }
        crate::clicktap::install(tx.clone(), ctx.clone());
        {
            let ctx = ctx.clone();
            MenuEvent::set_event_handler(Some(move |e: MenuEvent| {
                let _ = tx.send(Msg::Menu(e.id.0.clone()));
                ctx.request_repaint();
            }));
        }

        let d = Data::load();
        let mut hk_errors = vec![];
        let hk = match Hotkeys::new() {
            Ok(mut h) => {
                hk_errors = h.apply(&d.settings.sample_hotkey, &d.settings.picker_hotkey);
                Some(h)
            }
            Err(e) => {
                hk_errors.push(format!("Global hotkeys unavailable: {e}"));
                None
            }
        };
        let tray = match Tray::new(
            &hotkeys::pretty(&d.settings.sample_hotkey),
            &hotkeys::pretty(&d.settings.picker_hotkey),
            &d.history,
        ) {
            Ok(t) => Some(t),
            Err(e) => {
                eprintln!("tray: {e}");
                None
            }
        };

        let selected_hex = color::to_hex_hash(d.selected);
        Self {
            d,
            dirty: false,
            last_save: Instant::now(),
            hk,
            tray,
            rx,
            hk_errors,
            studio_open: true,
            picker_open: false,
            picker_pos: egui::pos2(200.0, 200.0),
            picker_fresh: false,
            loupe: None,
            flash: None,
            tab: Tab::Generator,
            recording: None,
            toast: None,
            undo: vec![],
            redo: vec![],
            shades_col: None,
            drag: None,
            particles: vec![],
            preview_scene: 0,
            photos: (0..4).map(|_| None).collect(),
            role_disp: [[0.0; 3]; 5],
            role_init: false,
            role_rects: [egui::Rect::NOTHING; 5],
            role_drag_from: None,
            drop_slot: None,
            adjust_mode: AdjMode::default(),
            slide: vec![],
            slide_key: (View::Columns, 0),
            landed: None,
            sample_drag: None,
            sample_in: false,
            sample_veto: false,
            hex_edit: None,
            adjust: None,
            selected_hex,
            screen_ok: capture::has_screen_access(),
            screen_checked: Instant::now(),
            focus_studio: false,
        }
    }

    // ------------------------------------------------------------ helpers

    pub fn touch(&mut self) {
        self.dirty = true;
    }

    pub fn toast(&mut self, msg: impl Into<String>) {
        self.toast = Some((msg.into(), Instant::now()));
    }

    pub fn copy(&mut self, ctx: &Context, c: Rgb) {
        let s = self.d.settings.fmt_hex(c);
        ctx.copy_text(s.clone());
        self.toast(format!("Copied {s}"));
    }

    pub fn select(&mut self, c: Rgb) {
        self.d.selected = c;
        self.selected_hex = color::to_hex_hash(c);
        self.touch();
    }

    pub fn labels(&self) -> (String, String) {
        (
            hotkeys::pretty(&self.d.settings.sample_hotkey),
            hotkeys::pretty(&self.d.settings.picker_hotkey),
        )
    }

    pub fn rebuild_tray(&self) {
        if let Some(t) = &self.tray {
            let (a, b) = self.labels();
            t.rebuild(&a, &b, &self.d.history);
        }
    }

    pub fn apply_hotkeys(&mut self) {
        if let Some(h) = &mut self.hk {
            self.hk_errors = h.apply(&self.d.settings.sample_hotkey, &self.d.settings.picker_hotkey);
        }
        self.rebuild_tray();
        self.touch();
    }

    // ------------------------------------------------------------ palette editing

    pub fn snapshot(&mut self) {
        self.undo.push(self.d.working.clone());
        if self.undo.len() > 100 {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    pub fn do_undo(&mut self) {
        if let Some(prev) = self.undo.pop() {
            self.redo.push(std::mem::replace(&mut self.d.working, prev));
            self.shades_col = None;
            self.adjust = None;
            self.touch();
        }
    }

    pub fn do_redo(&mut self) {
        if let Some(next) = self.redo.pop() {
            self.undo.push(std::mem::replace(&mut self.d.working, next));
            self.shades_col = None;
            self.adjust = None;
            self.touch();
        }
    }

    pub fn generate(&mut self) {
        self.snapshot();
        let cur: Vec<Rgb> = self.d.working.iter().map(|s| s.rgb).collect();
        let lock: Vec<bool> = self.d.working.iter().map(|s| s.locked).collect();
        let new = color::generate(&cur, &lock);
        for (s, c) in self.d.working.iter_mut().zip(new) {
            s.rgb = c;
        }
        self.shades_col = None;
        self.touch();
    }

    pub fn add_color(&mut self, c: Rgb) {
        if self.d.working.len() >= 10 {
            self.toast("Palettes hold up to 10 colors");
            return;
        }
        self.snapshot();
        self.d.working.push(Swatch { rgb: c, locked: false });
        self.touch();
    }

    // ------------------------------------------------------------ sampling

    pub fn start_loupe(&mut self, ctx: &Context) {
        if self.loupe.is_some() {
            return;
        }
        if !capture::has_screen_access() {
            capture::request_screen_access();
            self.screen_ok = false;
            self.toast(if capture::HAS_SETTINGS_PANE {
                "Grant Screen Recording permission, then restart Swatchbar"
            } else {
                "Swatchbar can't read the screen in this session (needs X11)"
            });
            self.studio_open = true;
            self.tab = Tab::Settings;
            self.focus_studio = true;
        }
        self.loupe = Some(Loupe { sample: None });
        crate::clicktap::set_sampling(true);
        if let Some(h) = &mut self.hk {
            h.set_loupe_keys(true);
        }
        ctx.request_repaint();
    }

    pub fn stop_loupe(&mut self) {
        crate::clicktap::set_sampling(false);
        if self.loupe.take().is_some() {
            if let Some(h) = &mut self.hk {
                h.set_loupe_keys(false);
            }
        }
    }

    /// Commit the color under the cursor.
    pub fn commit_sample(&mut self, ctx: &Context) {
        let n = self.d.settings.loupe_cells;
        let Some(s) = capture::grab(n).or_else(|| self.loupe.as_mut().and_then(|l| l.sample.take())) else {
            return;
        };
        let c = s.center;
        self.d.push_history(c);
        self.select(c);
        if self.d.settings.copy_on_sample {
            ctx.copy_text(self.d.settings.fmt_hex(c));
        }
        self.flash = Some((c, Instant::now(), s.cursor));
        self.rebuild_tray();
        if self.d.settings.close_after_sample {
            self.stop_loupe();
        }
        self.touch();
        ctx.request_repaint();
    }

    /// Shift-click: sample the color under the cursor into the palette and keep the loupe open.
    pub fn sample_add(&mut self, ctx: &Context) {
        let n = self.d.settings.loupe_cells;
        let Some(s) = capture::grab(n).or_else(|| self.loupe.as_mut().and_then(|l| l.sample.take())) else {
            return;
        };
        let c = s.center;
        self.d.push_history(c);
        self.select(c);
        if self.d.settings.copy_on_sample {
            ctx.copy_text(self.d.settings.fmt_hex(c));
        }
        self.flash = Some((c, Instant::now(), s.cursor));
        self.add_color(c);
        self.rebuild_tray();
        self.touch();
        ctx.request_repaint();
    }

    pub fn toggle_picker(&mut self, ctx: &Context) {
        if self.picker_open {
            self.picker_open = false;
            return;
        }
        // Open near the cursor, kept on the display it's on.
        let (cx, cy) = capture::cursor_pos().unwrap_or((300.0, 300.0));
        let size = egui::vec2(300.0, 520.0);
        let (dx, dy, dw, dh) = capture::display_at(cx, cy);
        let x = (cx as f32 - size.x / 2.0).clamp(dx as f32 + 8.0, (dx + dw) as f32 - size.x - 8.0);
        let y = (cy as f32 - 40.0).clamp(dy as f32 + 30.0, (dy + dh) as f32 - size.y - 8.0);
        self.picker_pos = egui::pos2(x, y);
        self.picker_open = true;
        self.picker_fresh = true;
        ctx.request_repaint();
    }

    // ------------------------------------------------------------ events

    fn handle(&mut self, msg: Msg, ctx: &Context) {
        // A click picks the color under the cursor and always dismisses the loupe.
        if let Msg::Click { shift } = msg {
            if self.loupe.is_some() {
                if shift {
                    self.sample_add(ctx);
                } else {
                    self.commit_sample(ctx);
                    self.stop_loupe();
                }
            }
            return;
        }
        let action = match msg {
            Msg::Hotkey(id) => self.hk.as_ref().and_then(|h| h.action_for(id)),
            Msg::Click { .. } => None,
            Msg::Cancel => self.loupe.is_some().then_some(Action::LoupeCancel),
            Msg::Menu(id) => {
                match id.as_str() {
                    tray::ID_SAMPLE => Some(Action::Sample),
                    tray::ID_PICKER => Some(Action::Picker),
                    tray::ID_STUDIO => {
                        self.studio_open = true;
                        self.focus_studio = true;
                        None
                    }
                    tray::ID_SETTINGS => {
                        self.studio_open = true;
                        self.focus_studio = true;
                        self.tab = Tab::Settings;
                        None
                    }
                    tray::ID_CLEAR => {
                        self.d.history_clear();
                        self.rebuild_tray();
                        self.touch();
                        None
                    }
                    tray::ID_QUIT => {
                        self.d.save();
                        std::process::exit(0);
                    }
                    other => {
                        if let Some(hex) = other.strip_prefix(tray::RECENT_PREFIX) {
                            if let Some(c) = color::parse_hex(hex) {
                                ctx.copy_text(self.d.settings.fmt_hex(c));
                                self.select(c);
                                self.flash = Some((c, Instant::now(), capture::cursor_pos().unwrap_or((200.0, 200.0))));
                            }
                        }
                        None
                    }
                }
            }
        };
        let Some(action) = action else { return };
        if self.recording.is_some() {
            return;
        }
        match action {
            Action::Sample => {
                if self.loupe.is_some() {
                    self.commit_sample(ctx);
                } else {
                    self.start_loupe(ctx);
                }
            }
            Action::Picker => self.toggle_picker(ctx),
            Action::LoupeCancel => self.stop_loupe(),
            Action::LoupeConfirm => self.commit_sample(ctx),
            Action::Nudge(dx, dy) => capture::nudge_cursor(dx as f64, dy as f64),
        }
    }
}

impl eframe::App for App {
    fn clear_color(&self, _v: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn logic(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        while let Ok(m) = self.rx.try_recv() {
            self.handle(m, ctx);
        }

        if self.screen_checked.elapsed() > Duration::from_secs(2) {
            self.screen_ok = capture::has_screen_access();
            self.screen_checked = Instant::now();
        }

        if self.dirty && self.last_save.elapsed() > Duration::from_millis(800) {
            self.d.save();
            self.dirty = false;
            self.last_save = Instant::now();
        }

        // The root window is hidden; keep it painting so child windows are driven.
        let any = self.studio_open || self.picker_open || self.loupe.is_some() || self.flash.is_some();
        if any {
            ctx.send_viewport_cmd(ViewportCommand::RequestPaintWhileHidden);
        }
        if self.loupe.is_some() || self.flash.is_some() {
            ctx.request_repaint();
        } else if any {
            ctx.request_repaint_after(Duration::from_millis(250));
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if self.studio_open {
            self.show_studio(&ctx);
        }
        if self.picker_open {
            self.show_picker(&ctx);
        }
        if self.loupe.is_some() {
            self.show_loupe(&ctx);
        }
        if self.flash.is_some() {
            self.show_flash(&ctx);
        }
    }

    fn on_exit(&mut self) {
        self.d.save();
    }
}
