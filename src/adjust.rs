//! The "Adjust color" window: a color wheel plus HSL, RGB, OKLCH and CMYK sliders.

use crate::app::App;
use crate::color::{self, Rgb};
use crate::theme;
use crate::widgets::c32;
use egui::{pos2, vec2, Align2, Color32, CornerRadius, FontId, Mesh, Rect, RichText, Sense, Shape, Stroke, StrokeKind, Ui};

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum AdjMode {
    #[default]
    Wheel,
    Hsl,
    Rgb,
    Oklch,
    Cmyk,
}

const MODES: [(AdjMode, &str); 5] = [
    (AdjMode::Wheel, "Wheel"),
    (AdjMode::Hsl, "HSL"),
    (AdjMode::Rgb, "RGB"),
    (AdjMode::Oklch, "OKLCH"),
    (AdjMode::Cmyk, "CMYK"),
];

pub struct Adjust {
    pub col: usize,
    /// Editing a Preview role (background, surface, ...) rather than a palette color.
    pub role: bool,
    pub mode: AdjMode,
    /// The color when the window opened (click "was" to go back).
    pub orig: Rgb,
    /// HSV drives the wheel. Hue is kept even when saturation drops to zero.
    pub h: f32,
    pub s: f32,
    pub v: f32,
}

impl Adjust {
    pub fn new(col: usize, rgb: Rgb, mode: AdjMode) -> Self {
        let (h, s, v) = color::to_hsv(rgb);
        Self { col, role: false, mode, orig: rgb, h, s, v }
    }

    pub fn new_role(slot: usize, rgb: Rgb, mode: AdjMode) -> Self {
        Self { role: true, ..Self::new(slot, rgb, mode) }
    }

    fn set_rgb(&mut self, rgb: Rgb) {
        let (h, s, v) = color::to_hsv(rgb);
        if s > 0.005 && v > 0.005 {
            self.h = h;
        }
        self.s = s;
        self.v = v;
    }
}

enum Edit {
    /// Wheel drag: HSV is already updated.
    Wheel(Rgb),
    /// Anything else; optionally the hue to remember.
    Other(Rgb, Option<f32>),
}

impl App {
    pub fn adjust_window(&mut self, ui: &mut Ui) {
        let Some(mut a) = self.adjust.take() else { return };
        if a.role && (a.col >= 5 || self.d.roles.len() != 5 || self.tab != crate::app::Tab::Preview) {
            return;
        }
        if !a.role && a.col >= self.d.working.len() {
            return;
        }
        let rgb = if a.role { self.d.roles[a.col] } else { self.d.working[a.col].rgb };
        if color::from_hsv(a.h, a.s, a.v) != rgb {
            a.set_rgb(rgb); // changed elsewhere (undo, hex edit)
        }
        let mut open = true;
        let mut edit: Option<Edit> = None;
        let mut mode = a.mode;
        let (title, anchor, offset) = if a.role {
            (format!("Adjust {}", crate::preview::ROLES[a.col].0.to_lowercase()), Align2::RIGHT_TOP, vec2(-24.0, 190.0))
        } else {
            ("Adjust color".to_string(), Align2::CENTER_BOTTOM, vec2(0.0, -80.0))
        };
        egui::Window::new(title)
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(anchor, offset)
            .show(ui.ctx(), |ui| {
                ui.set_width(300.0);
                ui.spacing_mut().item_spacing = vec2(6.0, 8.0);

                // New color, with the original beside it.
                let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 42.0), Sense::hover());
                let split = rect.max.x - 64.0;
                let (l, r) = (Rect::from_min_max(rect.min, pos2(split, rect.max.y)), Rect::from_min_max(pos2(split, rect.min.y), rect.max));
                let rad = 10;
                ui.painter().rect_filled(l, CornerRadius { nw: rad, sw: rad, ne: 0, se: 0 }, c32(rgb));
                ui.painter().text(
                    l.center(),
                    Align2::CENTER_CENTER,
                    color::to_hex_hash(rgb),
                    FontId::monospace(14.0),
                    c32(color::text_on(rgb)),
                );
                ui.painter().rect_filled(r, CornerRadius { ne: rad, se: rad, nw: 0, sw: 0 }, c32(a.orig));
                ui.painter().text(r.center(), Align2::CENTER_CENTER, "was", FontId::proportional(11.5), c32(color::text_on(a.orig)));
                let was = ui.interact(r, ui.scope_id().with("was"), Sense::click());
                if was.on_hover_text("Go back to the original").clicked() {
                    edit = Some(Edit::Other(a.orig, None));
                }

                // Mode tabs.
                egui::Frame::new()
                    .fill(theme::PANEL)
                    .stroke(Stroke::new(1.0, theme::LINE))
                    .corner_radius(9)
                    .inner_margin(egui::Margin::same(2))
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.x = 2.0;
                        ui.horizontal(|ui| {
                            let w = (ui.available_width() - 2.0 * 4.0) / 5.0;
                            for (m, label) in MODES {
                                let on = mode == m;
                                let t = RichText::new(label).size(12.0).color(if on { Color32::WHITE } else { Color32::from_gray(165) });
                                let b = egui::Button::new(t)
                                    .fill(if on { theme::ACCENT } else { Color32::TRANSPARENT })
                                    .stroke(Stroke::NONE)
                                    .corner_radius(7)
                                    .min_size(vec2(w, 24.0));
                                if ui.add(b).clicked() {
                                    mode = m;
                                }
                            }
                        });
                    });

                match mode {
                    AdjMode::Wheel => {
                        if let Some(c) = wheel(ui, &mut a, rgb) {
                            edit = Some(Edit::Wheel(c));
                        }
                    }
                    AdjMode::Hsl => {
                        let (_, s0, l0) = color::to_hsl(rgb);
                        let (mut h, mut s, mut l) = (a.h, s0 * 100.0, l0 * 100.0);
                        let mut ch = false;
                        ch |= track_slider(ui, "H", &mut h, 0.0, 360.0, 0, "°", |t| color::from_hsl(t * 360.0, 1.0, 0.5));
                        ch |= track_slider(ui, "S", &mut s, 0.0, 100.0, 0, "%", |t| color::from_hsl(a.h, t, l0));
                        ch |= track_slider(ui, "L", &mut l, 0.0, 100.0, 0, "%", |t| color::from_hsl(a.h, s0, t));
                        if ch {
                            edit = Some(Edit::Other(color::from_hsl(h, s / 100.0, l / 100.0), Some(h)));
                        }
                    }
                    AdjMode::Rgb => {
                        let mut v = [rgb[0] as f32, rgb[1] as f32, rgb[2] as f32];
                        let mut ch = false;
                        for (k, name) in ["R", "G", "B"].into_iter().enumerate() {
                            ch |= track_slider(ui, name, &mut v[k], 0.0, 255.0, 0, "", |t| {
                                let mut c = rgb;
                                c[k] = (t * 255.0).round() as u8;
                                c
                            });
                        }
                        if ch {
                            edit = Some(Edit::Other([v[0] as u8, v[1] as u8, v[2] as u8], None));
                        }
                    }
                    AdjMode::Oklch => {
                        let (l0, c0, h0) = color::to_oklch(rgb);
                        let (mut l, mut c, mut h) = (l0 * 100.0, c0 * 100.0, h0);
                        let mut ch = false;
                        ch |= track_slider(ui, "L", &mut l, 0.0, 100.0, 0, "%", |t| color::from_oklch(t, c0, h0));
                        ch |= track_slider(ui, "C", &mut c, 0.0, 40.0, 1, "", |t| color::from_oklch(l0, t * 0.4, h0));
                        ch |= track_slider(ui, "H", &mut h, 0.0, 360.0, 0, "°", |t| color::from_oklch(l0.clamp(0.45, 0.85), 0.13, t * 360.0));
                        if ch {
                            edit = Some(Edit::Other(color::from_oklch(l / 100.0, c / 100.0, h), None));
                        }
                    }
                    AdjMode::Cmyk => {
                        let cmyk = to_cmyk(rgb);
                        let mut v = cmyk.map(|x| x * 100.0);
                        let mut ch = false;
                        for (k, name) in ["C", "M", "Y", "K"].into_iter().enumerate() {
                            ch |= track_slider(ui, name, &mut v[k], 0.0, 100.0, 0, "%", |t| {
                                let mut x = cmyk;
                                x[k] = t;
                                from_cmyk(x)
                            });
                        }
                        if ch {
                            edit = Some(Edit::Other(from_cmyk(v.map(|x| x / 100.0)), None));
                        }
                    }
                }
            });

        if mode != a.mode {
            a.mode = mode;
            self.adjust_mode = mode;
        }
        if let Some(e) = edit {
            let c = match e {
                Edit::Wheel(c) => c,
                Edit::Other(c, hue) => {
                    a.set_rgb(c);
                    if let Some(h) = hue {
                        a.h = h;
                    }
                    c
                }
            };
            // Roles and the palette are one set of colors: an edit lands in both.
            let old = rgb;
            let in_palette = self.d.working.iter().any(|s| s.rgb == old);
            if a.role {
                self.d.roles[a.col] = c;
            } else {
                self.d.working[a.col].rgb = c;
            }
            for r in self.d.roles.iter_mut().filter(|r| **r == old) {
                *r = c;
            }
            for s in self.d.working.iter_mut().filter(|s| s.rgb == old) {
                s.rgb = c;
            }
            if a.role && !in_palette && self.d.working.len() < 10 {
                self.d.working.push(crate::store::Swatch { rgb: c, locked: false });
            }
            self.touch();
        }
        if open {
            self.adjust = Some(a);
        }
    }
}

fn to_cmyk(c: Rgb) -> [f32; 4] {
    let (r, g, b) = (c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0);
    let k = 1.0 - r.max(g).max(b);
    if k >= 0.999 {
        return [0.0, 0.0, 0.0, 1.0];
    }
    [(1.0 - r - k) / (1.0 - k), (1.0 - g - k) / (1.0 - k), (1.0 - b - k) / (1.0 - k), k]
}

fn from_cmyk(x: [f32; 4]) -> Rgb {
    let f = |v: f32| (255.0 * (1.0 - v.clamp(0.0, 1.0)) * (1.0 - x[3].clamp(0.0, 1.0))).round() as u8;
    [f(x[0]), f(x[1]), f(x[2])]
}

/// Hue ring with a saturation/value square inside. Returns the new color when dragged.
fn wheel(ui: &mut Ui, a: &mut Adjust, rgb: Rgb) -> Option<Rgb> {
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 232.0), Sense::click_and_drag());
    let center = rect.center();
    let (outer, inner) = (108.0_f32, 84.0_f32);
    let half = inner * 0.70;
    let sq = Rect::from_center_size(center, vec2(half * 2.0, half * 2.0));

    let mut changed = false;
    if resp.is_pointer_button_down_on() {
        if let (Some(o), Some(p)) = ui.input(|i| (i.pointer.press_origin(), i.pointer.latest_pos())) {
            let d0 = (o - center).length();
            if sq.expand(6.0).contains(o) {
                a.s = ((p.x - sq.min.x) / sq.width()).clamp(0.0, 1.0);
                a.v = (1.0 - (p.y - sq.min.y) / sq.height()).clamp(0.0, 1.0);
                changed = true;
            } else if d0 >= inner - 4.0 && d0 <= outer + 8.0 {
                let d = p - center;
                a.h = d.y.atan2(d.x).to_degrees().rem_euclid(360.0);
                changed = true;
            }
        }
    }

    let p = ui.painter();
    // Hue ring.
    let mut mesh = Mesh::default();
    let segs = 120;
    for k in 0..segs {
        let (t0, t1) = (k as f32 / segs as f32, (k + 1) as f32 / segs as f32);
        let (a0, a1) = (t0 * std::f32::consts::TAU, t1 * std::f32::consts::TAU);
        let (c0, c1) = (c32(color::from_hsv(t0 * 360.0, 1.0, 1.0)), c32(color::from_hsv(t1 * 360.0, 1.0, 1.0)));
        let at = |ang: f32, r: f32| center + vec2(ang.cos(), ang.sin()) * r;
        let base = mesh.vertices.len() as u32;
        mesh.colored_vertex(at(a0, inner), c0);
        mesh.colored_vertex(at(a0, outer), c0);
        mesh.colored_vertex(at(a1, outer), c1);
        mesh.colored_vertex(at(a1, inner), c1);
        mesh.add_triangle(base, base + 1, base + 2);
        mesh.add_triangle(base, base + 2, base + 3);
    }
    p.add(Shape::mesh(mesh));
    p.circle_stroke(center, outer, Stroke::new(1.0, Color32::from_black_alpha(70)));
    p.circle_stroke(center, inner, Stroke::new(1.0, Color32::from_black_alpha(70)));

    // Saturation (right) / value (up) square.
    let hue = c32(color::from_hsv(a.h, 1.0, 1.0));
    let mut sqm = Mesh::default();
    sqm.colored_vertex(sq.left_top(), Color32::WHITE);
    sqm.colored_vertex(sq.right_top(), hue);
    sqm.colored_vertex(sq.right_bottom(), Color32::BLACK);
    sqm.colored_vertex(sq.left_bottom(), Color32::BLACK);
    sqm.add_triangle(0, 1, 2);
    sqm.add_triangle(0, 2, 3);
    p.add(Shape::mesh(sqm));
    p.rect_stroke(sq, 2.0, Stroke::new(1.0, Color32::from_black_alpha(90)), StrokeKind::Outside);

    // Markers.
    let ang = a.h.to_radians();
    let hp = center + vec2(ang.cos(), ang.sin()) * ((outer + inner) / 2.0);
    p.circle_filled(hp, 10.5, Color32::from_black_alpha(80));
    p.circle_filled(hp, 9.0, hue);
    p.circle_stroke(hp, 9.0, Stroke::new(3.0, Color32::WHITE));
    let sp = pos2(sq.min.x + a.s * sq.width(), sq.min.y + (1.0 - a.v) * sq.height());
    p.circle_filled(sp, 9.0, Color32::from_black_alpha(80));
    p.circle_filled(sp, 7.5, c32(rgb));
    p.circle_stroke(sp, 7.5, Stroke::new(2.5, Color32::WHITE));

    changed.then(|| color::from_hsv(a.h, a.s, a.v))
}

/// A labelled slider whose track shows what the color will be along it, plus a numeric field.
/// Values are in display units (e.g. 0..100 for a percentage).
fn track_slider(
    ui: &mut Ui,
    label: &str,
    v: &mut f32,
    min: f32,
    max: f32,
    decimals: usize,
    suffix: &str,
    track: impl Fn(f32) -> Rgb,
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.add_sized([16.0, 20.0], egui::Label::new(RichText::new(label).size(12.0).color(theme::MUTED)));
        let w = (ui.available_width() - 74.0).max(60.0);
        let (rect, resp) = ui.allocate_exact_size(vec2(w, 22.0), Sense::click_and_drag());
        let bar = Rect::from_center_size(rect.center(), vec2(rect.width() - 8.0, 12.0));

        if resp.is_pointer_button_down_on() {
            if let Some(p) = ui.input(|i| i.pointer.latest_pos()) {
                let t = ((p.x - bar.min.x) / bar.width()).clamp(0.0, 1.0);
                let mut nv = min + t * (max - min);
                let m = 10f32.powi(decimals as i32);
                nv = (nv * m).round() / m;
                if nv != *v {
                    *v = nv;
                    changed = true;
                }
            }
        }

        let p = ui.painter();
        let t_of = |x: f32| ((x - bar.min.x) / bar.width()).clamp(0.0, 1.0);
        let cap = 6.0;
        p.rect_filled(Rect::from_min_max(bar.min, pos2(bar.min.x + cap * 2.0, bar.max.y)), cap, c32(track(0.0)));
        p.rect_filled(Rect::from_min_max(pos2(bar.max.x - cap * 2.0, bar.min.y), bar.max), cap, c32(track(1.0)));
        let (x0, x1) = (bar.min.x + cap, bar.max.x - cap);
        let mut mesh = Mesh::default();
        let segs = 36;
        for s in 0..segs {
            let (xa, xb) = (x0 + (x1 - x0) * s as f32 / segs as f32, x0 + (x1 - x0) * (s + 1) as f32 / segs as f32);
            let (ca, cb) = (c32(track(t_of(xa))), c32(track(t_of(xb))));
            let base = mesh.vertices.len() as u32;
            mesh.colored_vertex(pos2(xa, bar.min.y), ca);
            mesh.colored_vertex(pos2(xb, bar.min.y), cb);
            mesh.colored_vertex(pos2(xb, bar.max.y), cb);
            mesh.colored_vertex(pos2(xa, bar.max.y), ca);
            mesh.add_triangle(base, base + 1, base + 2);
            mesh.add_triangle(base, base + 2, base + 3);
        }
        p.add(Shape::mesh(mesh));
        p.rect_stroke(bar, cap, Stroke::new(1.0, Color32::from_white_alpha(24)), StrokeKind::Inside);
        let tv = ((*v - min) / (max - min)).clamp(0.0, 1.0);
        let hx = bar.min.x + tv * bar.width();
        p.circle_filled(pos2(hx, bar.center().y + 1.0), 9.5, Color32::from_black_alpha(80));
        p.circle_filled(pos2(hx, bar.center().y), 8.5, c32(track(tv)));
        p.circle_stroke(pos2(hx, bar.center().y), 8.5, Stroke::new(2.5, Color32::WHITE));

        let dv = egui::DragValue::new(v).range(min..=max).speed((max - min) / 200.0).max_decimals(decimals).min_decimals(decimals).suffix(suffix);
        if ui.add_sized([62.0, 22.0], dv).changed() {
            changed = true;
        }
    });
    changed
}
