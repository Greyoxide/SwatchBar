//! Global hotkeys: parsing, display, recording from egui key events, registration.

use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::GlobalHotKeyManager;
use std::str::FromStr;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Action {
    Sample,
    Picker,
    // Temporary keys active while the loupe is open.
    LoupeCancel,
    LoupeConfirm,
    Nudge(i8, i8),
}

pub fn parse(s: &str) -> Option<HotKey> {
    HotKey::from_str(s).ok()
}

/// "⌃⌥C" style label on macOS, "Ctrl+Alt+C" elsewhere.
pub fn pretty(s: &str) -> String {
    let Some(hk) = parse(s) else { return "—".into() };
    let mut out = String::new();
    let mut push = |sym: &str, name: &str| {
        if cfg!(target_os = "macos") {
            out.push_str(sym);
        } else {
            out.push_str(name);
            out.push('+');
        }
    };
    if hk.mods.contains(Modifiers::CONTROL) {
        push("⌃", "Ctrl");
    }
    if hk.mods.contains(Modifiers::ALT) {
        push("⌥", "Alt");
    }
    if hk.mods.contains(Modifiers::SHIFT) {
        push("⇧", "Shift");
    }
    if hk.mods.contains(Modifiers::META) {
        push("⌘", if cfg!(windows) { "Win" } else { "Super" });
    }
    let code = format!("{:?}", hk.key);
    let label = code
        .strip_prefix("Key")
        .or_else(|| code.strip_prefix("Digit"))
        .map(str::to_string)
        .unwrap_or_else(|| match code.as_str() {
            "Space" => "Space".into(),
            "ArrowUp" => "↑".into(),
            "ArrowDown" => "↓".into(),
            "ArrowLeft" => "←".into(),
            "ArrowRight" => "→".into(),
            "Escape" => "⎋".into(),
            "Enter" => "↩".into(),
            "Minus" => "-".into(),
            "Equal" => "=".into(),
            "Comma" => ",".into(),
            "Period" => ".".into(),
            "Slash" => "/".into(),
            "Semicolon" => ";".into(),
            "Quote" => "'".into(),
            "Backquote" => "`".into(),
            other => other.to_string(),
        });
    out.push_str(&label);
    out
}

fn code_for(key: egui::Key) -> Option<Code> {
    use egui::Key as K;
    let name = match key {
        K::Space => "Space".to_string(),
        K::Enter => "Enter".to_string(),
        K::Tab => "Tab".to_string(),
        K::ArrowUp => "ArrowUp".into(),
        K::ArrowDown => "ArrowDown".into(),
        K::ArrowLeft => "ArrowLeft".into(),
        K::ArrowRight => "ArrowRight".into(),
        K::Minus => "Minus".into(),
        K::Equals => "Equal".into(),
        K::Comma => "Comma".into(),
        K::Period => "Period".into(),
        K::Slash => "Slash".into(),
        K::Semicolon => "Semicolon".into(),
        K::Backtick => "Backquote".into(),
        other => {
            let n = other.name();
            if n.len() == 1 && n.chars().all(|c| c.is_ascii_alphabetic()) {
                format!("Key{}", n.to_ascii_uppercase())
            } else if n.len() == 1 && n.chars().all(|c| c.is_ascii_digit()) {
                format!("Digit{n}")
            } else if n.starts_with('F') && n[1..].chars().all(|c| c.is_ascii_digit()) {
                n.to_string()
            } else {
                return None;
            }
        }
    };
    Code::from_str(&name).ok()
}

/// Turns an egui key press (+ modifiers) into a hotkey string.
/// Requires at least one modifier unless it's an F-key.
pub fn from_egui(key: egui::Key, m: egui::Modifiers) -> Option<String> {
    let code = code_for(key)?;
    let mut mods = Modifiers::empty();
    if m.ctrl {
        mods |= Modifiers::CONTROL;
    }
    if m.alt {
        mods |= Modifiers::ALT;
    }
    if m.shift {
        mods |= Modifiers::SHIFT;
    }
    if m.mac_cmd {
        mods |= Modifiers::META;
    }
    let is_f = format!("{code:?}").starts_with('F');
    if mods.is_empty() && !is_f {
        return None;
    }
    Some(HotKey::new(Some(mods), code).into_string())
}

/// Owns the OS registrations.
pub struct Hotkeys {
    mgr: GlobalHotKeyManager,
    sample: Option<HotKey>,
    picker: Option<HotKey>,
    loupe_keys: Vec<(HotKey, Action)>,
}

impl Hotkeys {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            mgr: GlobalHotKeyManager::new().map_err(|e| e.to_string())?,
            sample: None,
            picker: None,
            loupe_keys: vec![],
        })
    }

    /// (Re)registers both user hotkeys. Returns per-hotkey errors.
    pub fn apply(&mut self, sample: &str, picker: &str) -> Vec<String> {
        let mut errs = vec![];
        for old in [self.sample.take(), self.picker.take()].into_iter().flatten() {
            let _ = self.mgr.unregister(old);
        }
        let mut reg = |label: &str, s: &str| -> Option<HotKey> {
            let hk = parse(s)?;
            match self.mgr.register(hk) {
                Ok(()) => Some(hk),
                Err(e) => {
                    errs.push(format!("{label} hotkey {}: {e}", pretty(s)));
                    None
                }
            }
        };
        self.sample = reg("Sample", sample);
        self.picker = reg("Picker", picker);
        errs
    }

    pub fn action_for(&self, id: u32) -> Option<Action> {
        if self.sample.map(|h| h.id()) == Some(id) {
            return Some(Action::Sample);
        }
        if self.picker.map(|h| h.id()) == Some(id) {
            return Some(Action::Picker);
        }
        self.loupe_keys.iter().find(|(h, _)| h.id() == id).map(|(_, a)| *a)
    }

    /// While the loupe is open: Esc cancels, Return samples, arrows nudge the cursor
    /// by 1px (hold Shift for 10px).
    pub fn set_loupe_keys(&mut self, on: bool) {
        for (hk, _) in self.loupe_keys.drain(..) {
            let _ = self.mgr.unregister(hk);
        }
        if !on {
            return;
        }
        let none = None::<Modifiers>;
        let shift = Some(Modifiers::SHIFT);
        let keys = [
            (HotKey::new(none, Code::Escape), Action::LoupeCancel),
            (HotKey::new(none, Code::Enter), Action::LoupeConfirm),
            (HotKey::new(none, Code::ArrowLeft), Action::Nudge(-1, 0)),
            (HotKey::new(none, Code::ArrowRight), Action::Nudge(1, 0)),
            (HotKey::new(none, Code::ArrowUp), Action::Nudge(0, -1)),
            (HotKey::new(none, Code::ArrowDown), Action::Nudge(0, 1)),
            (HotKey::new(shift, Code::ArrowLeft), Action::Nudge(-10, 0)),
            (HotKey::new(shift, Code::ArrowRight), Action::Nudge(10, 0)),
            (HotKey::new(shift, Code::ArrowUp), Action::Nudge(0, -10)),
            (HotKey::new(shift, Code::ArrowDown), Action::Nudge(0, 10)),
        ];
        for (hk, a) in keys {
            if self.mgr.register(hk).is_ok() {
                self.loupe_keys.push((hk, a));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctrl_alt_c() -> &'static str {
        if cfg!(target_os = "macos") { "⌃⌥C" } else { "Ctrl+Alt+C" }
    }

    #[test]
    fn defaults_parse_and_roundtrip() {
        let s = crate::store::Settings::default();
        for h in [&s.sample_hotkey, &s.picker_hotkey] {
            let hk = parse(h).unwrap_or_else(|| panic!("{h} should parse"));
            assert!(parse(&hk.into_string()).is_some());
        }
        assert_eq!(pretty("control+alt+KeyC"), ctrl_alt_c());
    }

    #[test]
    fn records_from_egui() {
        let m = egui::Modifiers { ctrl: true, alt: true, ..Default::default() };
        let s = from_egui(egui::Key::C, m).unwrap();
        assert_eq!(pretty(&s), ctrl_alt_c());
        assert!(from_egui(egui::Key::C, egui::Modifiers::NONE).is_none());
        assert!(from_egui(egui::Key::F5, egui::Modifiers::NONE).is_some());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn records_command_key() {
        let sp = from_egui(egui::Key::Space, egui::Modifiers { mac_cmd: true, command: true, ..Default::default() }).unwrap();
        assert_eq!(pretty(&sp), "⌘Space");
    }
}
