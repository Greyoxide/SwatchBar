//! The Preview tab: drop colors onto five roles and watch real-looking designs update.

use crate::app::App;
use crate::color::{self, Rgb};
use crate::mockups::{self, Cv, Pal};
use crate::theme;
use crate::widgets::c32;
use egui::{pos2, vec2, Align, Align2, Color32, FontId, Frame, Layout, Margin, Rect, RichText, Sense, Stroke, StrokeKind, Ui};

/// The first four are painted; the rest are photos recolored to the scheme.
const SCENES: [&str; 8] = ["Landing", "Shop", "App", "Stationery", "Box", "Pouch", "Bottle", "Tote"];
const PAINTED: usize = 4;
pub const ROLES: [(&str, &str); 5] = [
    ("Background", "page"),
    ("Surface", "cards & panels"),
    ("Text", "copy"),
    ("Primary", "buttons, brand"),
    ("Accent", "highlights"),
];

fn contrast_label(r: f32) -> (&'static str, Color32) {
    if r >= 7.0 {
        ("AAA", Color32::from_rgb(90, 200, 130))
    } else if r >= 4.5 {
        ("AA", Color32::from_rgb(90, 200, 130))
    } else if r >= 3.0 {
        ("AA large", theme::AMBER)
    } else {
        ("Low", Color32::from_rgb(240, 100, 90))
    }
}

impl App {
    /// A color dropped on a role (or swapped in from another role).
    pub fn assign_role(&mut self, slot: usize, c: Rgb) {
        if slot >= 5 || self.d.roles.len() != 5 {
            return;
        }
        match self.role_drag_from.take() {
            Some(from) if from < 5 && from != slot => self.d.roles.swap(from, slot),
            Some(_) => return,
            None => self.d.roles[slot] = c,
        }
        let landed = self.d.roles[slot];
        self.spawn_splash(self.role_rects[slot], landed);
        self.touch();
    }

    pub fn preview(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        if self.d.roles.len() != 5 {
            let cols: Vec<Rgb> = self.d.working.iter().map(|s| s.rgb).collect();
            self.d.roles = color::auto_roles(&cols, self.d.roles_dark).to_vec();
        }

        // Ease the on-screen colors toward their roles so changes melt in.
        let dt = ctx.input(|i| i.stable_dt).min(0.05);
        let k = 1.0 - (-dt * 14.0).exp();
        let mut moving = false;
        for i in 0..5 {
            for ch in 0..3 {
                let target = self.d.roles[i][ch] as f32;
                let cur = &mut self.role_disp[i][ch];
                if !self.role_init || (target - *cur).abs() < 0.5 {
                    *cur = target;
                } else {
                    *cur += (target - *cur) * k;
                    moving = true;
                }
            }
        }
        self.role_init = true;
        if moving {
            ctx.request_repaint();
        }
        let disp: [Rgb; 5] = std::array::from_fn(|i| std::array::from_fn(|c| self.role_disp[i][c].round().clamp(0.0, 255.0) as u8));

        self.paint_particles(ui);

        Frame::new().inner_margin(Margin::symmetric(24, 16)).show(ui, |ui| {
            // Scene picker + scheme tools
            ui.horizontal(|ui| {
                Frame::new()
                    .fill(theme::PANEL)
                    .stroke(Stroke::new(1.0, theme::LINE))
                    .corner_radius(9)
                    .inner_margin(Margin::same(2))
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.x = 2.0;
                        for (i, name) in SCENES.into_iter().enumerate() {
                            let on = self.preview_scene == i;
                            let t = RichText::new(name).size(12.5).color(if on { Color32::WHITE } else { Color32::from_gray(170) });
                            let b = egui::Button::new(t)
                                .fill(if on { theme::ACCENT } else { Color32::TRANSPARENT })
                                .stroke(Stroke::NONE)
                                .corner_radius(7)
                                .min_size(vec2(70.0, 24.0));
                            if ui.add(b).clicked() {
                                self.preview_scene = i;
                            }
                        }
                    });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.add(theme::ghost("Auto-assign")).on_hover_text("Pick roles from your palette").clicked() {
                        self.auto_assign();
                    }
                    for (dark, label) in [(true, "Dark"), (false, "Light")] {
                        let on = self.d.roles_dark == dark;
                        let b = egui::Button::new(RichText::new(label).size(12.0).color(if on { Color32::WHITE } else { Color32::from_gray(170) }))
                            .fill(if on { theme::RAISED } else { Color32::TRANSPARENT })
                            .stroke(Stroke::NONE)
                            .corner_radius(7)
                            .min_size(vec2(52.0, 24.0));
                        if ui.add(b).clicked() && !on {
                            self.d.roles_dark = dark;
                            self.auto_assign();
                        }
                    }
                });
            });
            ui.add_space(4.0);

            self.role_slots(ui, &ctx, &disp);
            self.palette_row(ui);
            ui.add_space(2.0);

            // The mockup, as large as fits.
            let avail = ui.available_rect_before_wrap();
            let aspect = mockups::W / mockups::H;
            let (w, h) = if avail.width() / avail.height().max(1.0) > aspect {
                (avail.height() * aspect, avail.height())
            } else {
                (avail.width(), avail.width() / aspect)
            };
            if w < 200.0 || h < 120.0 {
                return;
            }
            let canvas = Rect::from_center_size(avail.center(), vec2(w, h));
            ui.allocate_rect(avail, Sense::hover());
            let pl = ui.painter().with_clip_rect(canvas);
            ui.painter().rect_filled(canvas.expand(1.0), 4.0, Color32::from_black_alpha(90));
            if self.preview_scene < PAINTED {
                mockups::draw(self.preview_scene, &Cv { p: &pl, o: canvas.min, k: w / mockups::W }, &Pal::new(disp));
            } else {
                let i = self.preview_scene - PAINTED;
                if self.photos[i].is_none() {
                    let (name, bytes) = crate::transfer::PHOTOS[i];
                    self.photos[i] = crate::transfer::Recolored::load(&ctx, name, bytes);
                }
                if let Some(ph) = &mut self.photos[i] {
                    // Photos index their paints as [label, backdrop, ink, body, accent]; roles are [bg, surface, ...].
                    ph.apply([disp[1], disp[0], disp[2], disp[3], disp[4]], moving);
                    // Fill the canvas, trimming the photo's sides to fit.
                    let f = (canvas.aspect_ratio() / ph.aspect()).min(1.0);
                    let uv = Rect::from_min_max(pos2((1.0 - f) / 2.0, 0.0), pos2(1.0 - (1.0 - f) / 2.0, 1.0));
                    pl.image(ph.tex.id(), canvas, uv, Color32::WHITE);
                }
            }
            ui.painter().rect_stroke(canvas, 0.0, Stroke::new(1.0, theme::LINE), StrokeKind::Outside);
        });
    }

    fn auto_assign(&mut self) {
        let cols: Vec<Rgb> = self.d.working.iter().map(|s| s.rgb).collect();
        self.d.roles = color::auto_roles(&cols, self.d.roles_dark).to_vec();
        self.touch();
    }

    /// Five role cards. Drop a color on one to assign it; drag one onto another to swap.
    fn role_slots(&mut self, ui: &mut Ui, ctx: &egui::Context, disp: &[Rgb; 5]) {
        let gap = 10.0;
        let (strip, _) = ui.allocate_exact_size(vec2(ui.available_width(), 72.0), Sense::hover());
        let w = (strip.width() - gap * 4.0) / 5.0;
        let ptr = ctx.input(|i| i.pointer.latest_pos());
        let bg = self.d.roles[0];
        for i in 0..5 {
            let r = Rect::from_min_size(pos2(strip.min.x + i as f32 * (w + gap), strip.min.y), vec2(w, 72.0));
            self.role_rects[i] = r;
            let resp = ui.interact(r, ui.scope_id().with(("role", i)), Sense::click_and_drag());
            let dragging = self.sample_drag.is_some();
            let hot = dragging && self.role_drag_from != Some(i) && ptr.is_some_and(|p| r.contains(p));
            if hot {
                self.drop_slot = Some(i);
            }
            let p = ui.painter();
            p.rect_filled(r, 12.0, if hot { theme::ACCENT.gamma_multiply(0.22) } else { theme::RAISED });
            let stroke = if hot {
                Stroke::new(2.0, theme::ACCENT)
            } else if dragging {
                Stroke::new(1.0, theme::ACCENT.gamma_multiply(0.5))
            } else {
                Stroke::new(1.0, theme::LINE)
            };
            p.rect_stroke(r, 12.0, stroke, StrokeKind::Inside);
            let chip = Rect::from_min_size(r.min + vec2(10.0, 10.0), vec2(52.0, 52.0));
            let chip = if hot { chip.expand(3.0) } else { chip };
            p.rect_filled(chip, 10.0, c32(disp[i]));
            p.rect_stroke(chip, 10.0, Stroke::new(1.0, Color32::from_white_alpha(36)), StrokeKind::Inside);
            let x = chip.max.x + 12.0;
            p.text(pos2(x, r.min.y + 17.0), Align2::LEFT_CENTER, ROLES[i].0, FontId::proportional(13.0), theme::TEXT);
            p.text(pos2(x, r.min.y + 34.0), Align2::LEFT_CENTER, color::to_hex_hash(self.d.roles[i]), FontId::monospace(11.0), theme::MUTED);
            // Readability badges where they matter.
            let badge = match i {
                2 => Some(color::contrast_ratio(self.d.roles[2], bg)),
                3 => Some(color::contrast_ratio(self.d.roles[3], bg)),
                _ => None,
            };
            if let Some(ratio) = badge {
                let (label, col) = contrast_label(ratio);
                p.circle_filled(pos2(x + 3.0, r.min.y + 53.0), 3.0, col);
                p.text(pos2(x + 11.0, r.min.y + 53.0), Align2::LEFT_CENTER, format!("{label} · {ratio:.1}:1"), FontId::proportional(10.5), theme::MUTED);
            } else {
                p.text(pos2(x, r.min.y + 53.0), Align2::LEFT_CENTER, ROLES[i].1, FontId::proportional(10.5), theme::MUTED);
            }
            if resp.drag_started() {
                self.sample_drag = Some(self.d.roles[i]);
                self.role_drag_from = Some(i);
            }
            if resp.clicked() {
                self.snapshot();
                self.adjust = Some(crate::adjust::Adjust::new_role(i, self.d.roles[i], self.adjust_mode));
            }
            if self.adjust.as_ref().is_some_and(|a| a.role && a.col == i) {
                ui.painter().rect_stroke(r, 12.0, Stroke::new(2.0, theme::ACCENT), StrokeKind::Inside);
            }
            if resp.hovered() && !dragging {
                ctx.set_cursor_icon(egui::CursorIcon::Grab);
            }
        }
        if self.sample_drag.is_some() {
            ctx.request_repaint();
        }
    }

    /// The working palette as draggable chips.
    fn palette_row(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            theme::section(ui, "Palette");
            ui.add_space(4.0);
            for s in self.d.working.clone() {
                let (r, resp) = ui.allocate_exact_size(vec2(36.0, 36.0), Sense::click_and_drag());
                let r = if resp.hovered() { r.expand(2.0) } else { r };
                ui.painter().rect_filled(r, 9.0, c32(s.rgb));
                ui.painter().rect_stroke(r, 9.0, Stroke::new(1.0, Color32::from_white_alpha(40)), StrokeKind::Inside);
                let resp = resp.on_hover_text(format!("{}\nClick to adjust · drag onto a role", color::to_hex_hash(s.rgb)));
                if resp.drag_started() {
                    self.sample_drag = Some(s.rgb);
                    self.role_drag_from = None;
                }
                if resp.clicked() {
                    if let Some(i) = self.d.working.iter().position(|w| w.rgb == s.rgb) {
                        self.snapshot();
                        self.adjust = Some(crate::adjust::Adjust::new(i, s.rgb, self.adjust_mode));
                    }
                }
            }
            ui.add_space(8.0);
            ui.label(RichText::new("Click a color to adjust it, or drag it onto a role. Samples work too.").size(12.0).color(theme::MUTED));
        });
    }
}
