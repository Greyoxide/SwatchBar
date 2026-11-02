//! Settings + user data, persisted as JSON in Application Support.

use crate::color::{self, Rgb};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Settings {
    /// global-hotkey string, e.g. "control+alt+KeyC"
    pub sample_hotkey: String,
    pub picker_hotkey: String,
    /// Loupe grid size in pixels (odd).
    pub loupe_cells: usize,
    pub copy_on_sample: bool,
    pub uppercase_hex: bool,
    pub hash_prefix: bool,
    /// Close the loupe after one sample (otherwise stay in sampling mode).
    pub close_after_sample: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            sample_hotkey: "control+alt+KeyC".into(),
            picker_hotkey: "control+alt+KeyP".into(),
            loupe_cells: 17,
            copy_on_sample: true,
            uppercase_hex: true,
            hash_prefix: true,
            close_after_sample: true,
        }
    }
}

impl Settings {
    pub fn fmt_hex(&self, c: Rgb) -> String {
        let mut s = color::to_hex(c);
        if !self.uppercase_hex {
            s = s.to_lowercase();
        }
        if self.hash_prefix {
            format!("#{s}")
        } else {
            s
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum View {
    #[default]
    Columns,
    Rows,
    Gradient,
    Grid,
}

impl View {
    pub const ALL: [(View, &'static str); 4] = [
        (View::Columns, "Columns"),
        (View::Rows, "Rows"),
        (View::Gradient, "Gradient"),
        (View::Grid, "Grid"),
    ];
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Swatch {
    pub rgb: Rgb,
    pub locked: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SavedPalette {
    pub name: String,
    pub colors: Vec<Rgb>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Data {
    pub settings: Settings,
    /// Most recent first.
    pub history: Vec<Rgb>,
    /// Unix seconds each history entry was sampled (0 = unknown); parallel to `history`.
    pub history_at: Vec<i64>,
    pub saved: Vec<SavedPalette>,
    pub working: Vec<Swatch>,
    /// Preview roles: background, surface, text, primary, accent.
    pub roles: Vec<Rgb>,
    pub roles_dark: bool,
    pub working_name: String,
    pub selected: Rgb,
    pub view: View,
}

impl Default for Data {
    fn default() -> Self {
        let cols = color::generate(&[[0; 3]; 5], &[false; 5]);
        let roles = color::auto_roles(&cols, false).to_vec();
        Self {
            roles,
            roles_dark: false,
            settings: Settings::default(),
            history: vec![],
            history_at: vec![],
            saved: vec![],
            working: cols.into_iter().map(|rgb| Swatch { rgb, locked: false }).collect(),
            working_name: "Untitled palette".into(),
            selected: [0x3D, 0x7E, 0xF5],
            view: View::Columns,
        }
    }
}

fn path() -> Option<PathBuf> {
    let mut p = dirs::config_dir()?;
    p.push("Swatchbar");
    std::fs::create_dir_all(&p).ok()?;
    p.push("state.json");
    Some(p)
}

impl Data {
    pub fn load() -> Self {
        path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str::<Data>(&s).ok())
            .map(|mut d| {
                d.history_at.resize(d.history.len(), 0);
                if d.roles.len() != 5 {
                    let cols: Vec<Rgb> = d.working.iter().map(|s| s.rgb).collect();
                    d.roles = color::auto_roles(&cols, d.roles_dark).to_vec();
                }
                if d.working.is_empty() {
                    d.working = Data::default().working;
                }
                d
            })
            .unwrap_or_default()
    }

    pub fn save(&self) {
        if let (Some(p), Ok(s)) = (path(), serde_json::to_string_pretty(self)) {
            let _ = std::fs::write(p, s);
        }
    }

    pub fn push_history(&mut self, c: Rgb) {
        self.history_remove(c);
        self.history.insert(0, c);
        self.history_at.insert(0, chrono::Local::now().timestamp());
        self.history.truncate(80);
        self.history_at.truncate(80);
    }

    pub fn history_remove(&mut self, c: Rgb) {
        self.history_at.resize(self.history.len(), 0);
        if let Some(k) = self.history.iter().position(|h| *h == c) {
            self.history.remove(k);
            self.history_at.remove(k);
        }
    }

    pub fn history_clear(&mut self) {
        self.history.clear();
        self.history_at.clear();
    }
}
