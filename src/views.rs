//! Palette layouts for the Generator: Columns, Rows, Grid and Gradient.
//! All of them share one drag-to-reorder mechanism (`plan`) with a live preview.

use crate::app::{Adjust, App, Drag};
use crate::color::{self, Rgb};
use crate::store::{Swatch, View};
use crate::theme;
use crate::widgets::{c32, icon_at, Icon};
use egui::{
    pos2, vec2, Align2, Color32, Context, CornerRadius, CursorIcon, FontId, Key, Mesh, Pos2, Rect, Sense, Shape,
    Stroke, StrokeKind, Ui,
};

pub enum ColAct {
    Remove(usize),
    Lock(usize),
    Shades(usize),
    Adjust(usize),
    Copy(usize),
    Edit(usize),
    Pick(usize, Rgb),
    Insert(usize),
    CloseShades,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Column,
    Band,
    Card,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Role {
    Normal,
    Placeholder,
    Ghost,
}

fn ease_out_back(t: f32) -> f32 {
    let (c1, c3) = (1.70158_f32, 2.70158_f32);
    1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
}

fn dist(a: Pos2, b: Pos2) -> f32 {
    (a - b).length()
}

impl App {
    pub fn generator(&mut self, ui: &mut Ui) {
        let rect = ui.available_rect_before_wrap();
        ui.allocate_rect(rect, Sense::hover());
        self.sample_morph(ui, rect);
        self.paint_particles(ui);
        let n = self.d.working.len();
        let mut acts: Vec<ColAct> = vec![];
        let can_insert = n < 10 && self.shades_col.is_none() && self.drag.is_none() && self.sample_drag.is_none();

        match self.d.view {
            View::Columns => {
                let w = rect.width() / n as f32;
                let slots: Vec<Rect> = (0..n)
                    .map(|k| {
                        Rect::from_min_max(
                            pos2((rect.min.x + w * k as f32).round(), rect.min.y),
                            pos2((rect.min.x + w * (k + 1) as f32).round(), rect.max.y),
                        )
                    })
                    .collect();
                self.cells(ui, &slots, Kind::Column, &mut acts);
                if can_insert {
                    for (i, s) in slots.iter().enumerate().skip(1) {
                        let hot = Rect::from_center_size(pos2(s.min.x, rect.center().y), vec2(28.0, 120.0));
                        self.insert_button(ui, hot, pos2(s.min.x, rect.center().y), i, &mut acts);
                    }
                }
            }
            View::Rows => {
                let h = rect.height() / n as f32;
                let slots: Vec<Rect> = (0..n)
                    .map(|k| {
                        Rect::from_min_max(
                            pos2(rect.min.x, (rect.min.y + h * k as f32).round()),
                            pos2(rect.max.x, (rect.min.y + h * (k + 1) as f32).round()),
                        )
                    })
                    .collect();
                self.cells(ui, &slots, Kind::Band, &mut acts);
                if can_insert {
                    for (i, s) in slots.iter().enumerate().skip(1) {
                        let c = pos2(rect.center().x, s.min.y);
                        self.insert_button(ui, Rect::from_center_size(c, vec2(160.0, 28.0)), c, i, &mut acts);
                    }
                }
            }
            View::Grid => {
                let (pad, gap) = (22.0, 16.0);
                let cols = match n {
                    1..=3 => n,
                    4 => 2,
                    5 | 6 => 3,
                    7 | 8 => 4,
                    _ => 5,
                };
                let rows = n.div_ceil(cols);
                let cw = (rect.width() - pad * 2.0 - gap * (cols - 1) as f32) / cols as f32;
                let ch = ((rect.height() - pad * 2.0 - gap * (rows - 1) as f32) / rows as f32).min(cw * 1.5);
                let total_h = ch * rows as f32 + gap * (rows - 1) as f32;
                let y0 = rect.min.y + ((rect.height() - total_h) / 2.0).max(pad);
                let slots: Vec<Rect> = (0..n)
                    .map(|k| {
                        let (c, r) = (k % cols, k / cols);
                        Rect::from_min_size(
                            pos2(rect.min.x + pad + c as f32 * (cw + gap), y0 + r as f32 * (ch + gap)),
                            vec2(cw, ch),
                        )
                    })
                    .collect();
                self.cells(ui, &slots, Kind::Card, &mut acts);
            }
            View::Gradient => self.gradient_view(ui, rect, &mut acts),
        }

        let ctx = ui.ctx().clone();
        for a in acts {
            self.apply_col_act(&ctx, a);
        }
    }

    /// A sample dragged over the palette becomes a real swatch (the same drag as any other),
    /// making room for itself. Leaving the palette or pressing Esc turns it back into a chip.
    fn sample_morph(&mut self, ui: &Ui, area: Rect) {
        let Some(c) = self.sample_drag else { return };
        let (pos, esc) = ui.input(|i| (i.pointer.latest_pos(), i.key_pressed(Key::Escape)));
        let inside = pos.is_some_and(|p| area.contains(p));
        if !inside {
            self.sample_veto = false;
        }
        if esc {
            self.unmorph_sample();
            self.sample_veto = inside;
            return;
        }
        if self.sample_in && !inside {
            self.unmorph_sample();
        } else if !self.sample_in && inside && !self.sample_veto && self.d.working.len() < 10 {
            self.snapshot();
            self.d.working.push(Swatch { rgb: c, locked: false });
            self.shades_col = None;
            self.hex_edit = None;
            self.adjust = None;
            self.sample_in = true;
            self.drag = Some(Drag { idx: self.d.working.len() - 1, grab: egui::Vec2::ZERO });
        }
    }

    /// A mild burst of color specks thrown off the edges of the swatch where it lands.
    pub(crate) fn spawn_splash(&mut self, slot: Rect, rgb: Rgb) {
        let (w, h) = (slot.width(), slot.height());
        let scale = (w.min(h) / 220.0).clamp(0.8, 1.3);
        let perimeter = 2.0 * (w + h);
        for k in 0..32 {
            // A random point on the outline, and the direction pointing away from the swatch there.
            let mut d = fastrand::f32() * perimeter;
            let (from, normal, tangent) = if d < w {
                (pos2(slot.min.x + d, slot.min.y), vec2(0.0, -1.0), vec2(1.0, 0.0))
            } else if { d -= w; d < h } {
                (pos2(slot.max.x, slot.min.y + d), vec2(1.0, 0.0), vec2(0.0, 1.0))
            } else if { d -= h; d < w } {
                (pos2(slot.max.x - d, slot.max.y), vec2(0.0, 1.0), vec2(-1.0, 0.0))
            } else {
                d -= w;
                (pos2(slot.min.x, slot.max.y - d), vec2(-1.0, 0.0), vec2(0.0, -1.0))
            };
            let big = k < 8;
            let speed = (if big { 120.0 + fastrand::f32() * 130.0 } else { 70.0 + fastrand::f32() * 210.0 }) * scale;
            let sideways = (fastrand::f32() - 0.5) * speed * 0.7;
            let tone = match fastrand::u8(0..4) {
                0 => color::mix(rgb, [255, 255, 255], 0.35 + fastrand::f32() * 0.4),
                1 => color::mix(rgb, [0, 0, 0], 0.15 + fastrand::f32() * 0.2),
                2 => [255, 255, 255],
                _ => rgb,
            };
            self.particles.push(crate::app::Particle {
                pos: from + normal * 1.0,
                vel: normal * speed + tangent * sideways,
                age: 0.0,
                delay: 0.1 + fastrand::f32() * 0.05,
                life: 0.5 + fastrand::f32() * 0.4,
                size: if big { 3.5 + fastrand::f32() * 3.0 } else { 1.8 + fastrand::f32() * 2.0 },
                rgb: tone,
            });
        }
    }

    pub(crate) fn paint_particles(&mut self, ui: &Ui) {
        if self.particles.is_empty() {
            return;
        }
        let ctx = ui.ctx().clone();
        let dt = ctx.input(|i| i.stable_dt).min(0.05);
        let layer = egui::LayerId::new(egui::Order::Foreground, ui.scope_id().with("splash"));
        let p = ctx.layer_painter(layer);
        for q in &mut self.particles {
            if q.delay > 0.0 {
                q.delay -= dt;
                continue;
            }
            q.age += dt;
            q.vel.y += 420.0 * dt;
            q.vel *= (1.0 - 2.6 * dt).max(0.0);
            q.pos += q.vel * dt;
            let t = (q.age / q.life).clamp(0.0, 1.0);
            let a = ((1.0 - t).powf(1.4) * 255.0) as u8;
            let r = q.size * (1.0 - 0.55 * t);
            p.circle_filled(q.pos, r, Color32::from_rgba_unmultiplied(q.rgb[0], q.rgb[1], q.rgb[2], a));
        }
        self.particles.retain(|q| q.delay > 0.0 || q.age < q.life);
        ctx.request_repaint();
    }

    fn unmorph_sample(&mut self) {
        if self.sample_in {
            self.d.working.pop();
            self.undo.pop();
            self.sample_in = false;
            self.drag = None;
        }
    }

    fn insert_button(&mut self, ui: &mut Ui, hot: Rect, c: Pos2, at: usize, acts: &mut Vec<ColAct>) {
        if ui.rect_contains_pointer(hot) {
            let p = ui.painter();
            p.circle_filled(c, 15.0, Color32::from_rgba_unmultiplied(255, 255, 255, 245));
            p.circle_stroke(c, 15.0, Stroke::new(1.0, Color32::from_black_alpha(60)));
            if icon_at(ui, c, Icon::Plus, Color32::from_gray(30), false, ("ins", at))
                .on_hover_text("Insert a color here")
                .clicked()
            {
                acts.push(ColAct::Insert(at));
            }
        }
    }

    // ------------------------------------------------------------ drag & drop

    /// Works out what to draw where. While a swatch is being dragged, the others
    /// glide aside to show where it will land, and the dragged one follows the pointer,
    /// lifted. On release the data is reordered and the swatch glides into its slot.
    fn plan(&mut self, ui: &Ui, slots: &[Rect]) -> Vec<(usize, Rect, Role)> {
        let n = slots.len();
        let ctx = ui.ctx().clone();
        // New layout: snap. Colors added or removed: keep what's there so it glides.
        if self.slide_key.0 != self.d.view {
            self.slide = slots.to_vec();
        }
        self.slide_key = (self.d.view, n);
        if self.slide.len() < n {
            let from = self.slide.len();
            self.slide.extend_from_slice(&slots[from..]);
        }
        self.slide.truncate(n);
        // A sample just turned into a live swatch: hold it by its middle.
        if self.sample_in {
            if let Some(d) = &mut self.drag {
                if d.grab == egui::Vec2::ZERO && d.idx < n {
                    d.grab = slots[d.idx].size() / 2.0;
                }
            }
        }
        let lift = ctx.animate_bool_with_time(ui.scope_id().with("lift"), self.drag.is_some(), 0.16);

        let mut ghost: Option<Rect> = None;
        let mut targets: Vec<(usize, Rect, Role)> = (0..n).map(|k| (k, slots[k], Role::Normal)).collect();
        if let Some(d) = &self.drag {
            let (di, grab) = (d.idx, d.grab);
            let (pos, down) = ui.input(|i| (i.pointer.latest_pos(), i.pointer.any_down()));
            match pos {
                Some(p) if di < n => {
                    let t = slots
                        .iter()
                        .enumerate()
                        .min_by(|a, b| dist(a.1.center(), p).total_cmp(&dist(b.1.center(), p)))
                        .map(|x| x.0)
                        .unwrap_or(di);
                    let size = slots[di].size() * (1.0 + 0.06 * ease_out_back(lift));
                    let held = Rect::from_center_size(p - grab + slots[di].size() / 2.0, size);
                    if !down {
                        // Drop: start the glide from where the swatch was held.
                        self.drag = None;
                        self.slide[di] = held;
                        if t != di {
                            if !self.sample_in {
                                self.snapshot();
                            }
                            let item = self.d.working.remove(di);
                            self.d.working.insert(t, item);
                            let r = self.slide.remove(di);
                            self.slide.insert(t, r);
                            self.shades_col = None;
                            self.hex_edit = None;
                            self.adjust = None;
                            self.touch();
                        }
                        if self.sample_in {
                            // The sample was inserted (and snapshotted) when it entered the palette.
                            self.sample_in = false;
                            self.sample_drag = None;
                            self.touch();
                        }
                        self.landed = Some((t, std::time::Instant::now()));
                        let rgb = self.d.working[t].rgb;
                        self.spawn_splash(slots[t], rgb);
                    } else {
                        ctx.set_cursor_icon(CursorIcon::Grabbing);
                        let mut order: Vec<usize> = (0..n).collect();
                        order.remove(di);
                        order.insert(t, di);
                        targets = order
                            .iter()
                            .enumerate()
                            .map(|(k, &i)| (i, slots[k], if i == di { Role::Placeholder } else { Role::Normal }))
                            .collect();
                        ghost = Some(held);
                    }
                }
                _ => self.drag = None,
            }
        }

        // Ease every swatch toward its target.
        let dt = ctx.input(|i| i.stable_dt).min(0.05);
        let k = 1.0 - (-dt * 24.0).exp();
        let mut moving = false;
        let mut out: Vec<(usize, Rect, Role)> = Vec::with_capacity(n + 1);
        for (i, tgt, role) in targets {
            let cur = self.slide[i];
            let far = (cur.min - tgt.min).length().max((cur.max - tgt.max).length());
            let next = if far < 0.4 {
                tgt
            } else {
                moving = true;
                Rect::from_min_max(cur.min + (tgt.min - cur.min) * k, cur.max + (tgt.max - cur.max) * k)
            };
            self.slide[i] = next;
            out.push((i, next, role));
        }
        if let Some(g) = ghost {
            if let Some(d) = &self.drag {
                out.push((d.idx, g, Role::Ghost));
            }
            moving = true;
        }
        if let Some((_, t0)) = self.landed {
            if t0.elapsed().as_secs_f32() > 0.6 {
                self.landed = None;
            } else {
                moving = true;
            }
        }
        if moving {
            ctx.request_repaint();
        }
        out
    }

    fn begin_drag(&mut self, ui: &Ui, i: usize, origin: Rect) {
        if let Some(p) = ui.input(|i| i.pointer.press_origin()) {
            self.drag = Some(Drag { idx: i, grab: p - origin.min });
        }
    }

    fn cells(&mut self, ui: &mut Ui, slots: &[Rect], kind: Kind, acts: &mut Vec<ColAct>) {
        let plan = self.plan(ui, slots);
        let dragging = self.drag.is_some();
        for (i, r, role) in plan {
            self.cell(ui, i, r, kind, role, dragging, acts);
        }
    }

    // ------------------------------------------------------------ one swatch

    #[allow(clippy::too_many_arguments)]
    fn cell(&mut self, ui: &mut Ui, i: usize, r: Rect, kind: Kind, role: Role, dragging: bool, acts: &mut Vec<ColAct>) {
        let sw = self.d.working[i].clone();
        let fg = c32(color::text_on(sw.rgb));
        let rad = if kind == Kind::Card { 16.0 } else { 0.0 };

        {
            let p = ui.painter();
            if role == Role::Ghost {
                // Soft layered shadow: the swatch looks lifted off the page.
                for (grow, a) in [(18.0, 14), (11.0, 24), (5.0, 40)] {
                    p.rect_filled(r.translate(vec2(0.0, 12.0)).expand(grow), rad + grow, Color32::from_black_alpha(a));
                }
            }
            let fill = if role == Role::Placeholder { c32(sw.rgb).gamma_multiply(0.28) } else { c32(sw.rgb) };
            p.rect_filled(r, rad, fill);
            if role == Role::Ghost {
                p.rect_stroke(r, rad, Stroke::new(2.0, Color32::from_white_alpha(200)), StrokeKind::Inside);
            }
        }
        if role == Role::Placeholder {
            ui.painter().rect_stroke(r.shrink(3.0), rad, Stroke::new(1.5, Color32::from_white_alpha(60)), StrokeKind::Inside);
            return;
        }
        // A quick flash as a dropped swatch settles into place.
        if role == Role::Normal {
            if let Some((li, t0)) = self.landed {
                let age = t0.elapsed().as_secs_f32() - 0.1;
                if li == i && (0.0..0.45).contains(&age) {
                    let a = (1.0 - age / 0.45).powi(2);
                    ui.painter().rect_filled(r, rad, Color32::from_white_alpha((a * 90.0) as u8));
                }
            }
        }

        let (label_rect, centered) = match kind {
            Kind::Column => (
                Rect::from_center_size(pos2(r.center().x, r.max.y - 78.0), vec2((r.width() - 12.0).min(190.0), 44.0)),
                true,
            ),
            Kind::Band => (Rect::from_min_size(pos2(r.min.x + 40.0, r.center().y - 24.0), vec2(190.0, 48.0)), false),
            Kind::Card => (
                Rect::from_min_size(pos2(r.min.x + 12.0, r.max.y - 64.0), vec2((r.width() - 24.0).max(40.0), 50.0)),
                false,
            ),
        };
        let interactive = role == Role::Normal && !dragging;
        let hex_size = if kind == Kind::Column { 25.0 } else { 22.0 };
        self.hex_label(ui, i, label_rect, centered, fg, hex_size, interactive, acts);

        // Grip dots hint that the swatch can be dragged (rows and cards).
        if kind != Kind::Column {
            let g = if kind == Kind::Band { pos2(r.min.x + 18.0, r.center().y) } else { pos2(r.min.x + 16.0, r.min.y + 24.0) };
            for (dx, dy) in [(-2.5, -5.0), (2.5, -5.0), (-2.5, 0.0), (2.5, 0.0), (-2.5, 5.0), (2.5, 5.0)] {
                ui.painter().circle_filled(g + vec2(dx, dy), 1.4, fg.gamma_multiply(0.5));
            }
        }
        if !interactive {
            return;
        }

        if self.shades_col == Some(i) {
            self.shade_overlay(ui, i, r, kind, acts);
            return;
        }

        // Dragging from anywhere on the swatch reorders it.
        let bg = ui.interact(r, ui.scope_id().with(("bg", i)), Sense::drag());
        if bg.drag_started() {
            self.begin_drag(ui, i, r);
        }
        if bg.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::Grab);
        }

        let hovered = ui.rect_contains_pointer(r) && self.adjust.is_none() && self.sample_drag.is_none();
        let len = self.d.working.len();
        let step = match kind {
            Kind::Column => 42.0,
            Kind::Band => 40.0,
            Kind::Card => ((r.width() - 48.0) / 4.0).clamp(26.0, 34.0),
        };
        let pos = |k: usize| -> Pos2 {
            match kind {
                Kind::Column => pos2(r.center().x, r.center().y - 84.0 + step * k as f32),
                Kind::Band => pos2(r.max.x - 30.0 - step * (4 - k) as f32, r.center().y),
                Kind::Card => pos2(r.max.x - 24.0 - step * (4 - k) as f32, r.min.y + 26.0),
            }
        };
        let lock_tip = if sw.locked { "Unlock color" } else { "Lock color" };
        let items: [(Icon, &str, ColAct, bool); 5] = [
            (Icon::Close, "Remove color", ColAct::Remove(i), false),
            (Icon::Shades, "View shades", ColAct::Shades(i), false),
            (Icon::Adjust, "Adjust color", ColAct::Adjust(i), false),
            (Icon::Copy, "Copy hex", ColAct::Copy(i), false),
            (Icon::Lock(sw.locked), lock_tip, ColAct::Lock(i), sw.locked),
        ];
        for (k, (icon, tip, act, active)) in items.into_iter().enumerate() {
            let is_lock = k == 4;
            if !hovered && !(is_lock && sw.locked) {
                continue;
            }
            if k == 0 && len <= 2 {
                continue;
            }
            if is_lock && !hovered {
                ui.painter().circle_filled(pos(k), 17.0, fg.gamma_multiply(0.12));
            }
            if icon_at(ui, pos(k), icon, fg, active, (i, tip)).on_hover_text(tip).clicked() {
                acts.push(act);
            }
        }
    }

    /// Hex text; click to edit in place. When not interactive it just paints.
    #[allow(clippy::too_many_arguments)]
    fn hex_label(
        &mut self,
        ui: &mut Ui,
        i: usize,
        rect: Rect,
        centered: bool,
        fg: Color32,
        size: f32,
        interactive: bool,
        acts: &mut Vec<ColAct>,
    ) {
        let rgb = self.d.working[i].rgb;
        let align = if centered { Align2::CENTER_CENTER } else { Align2::LEFT_CENTER };
        let editing = interactive && self.hex_edit.as_ref().map(|(c, _)| *c) == Some(i);
        let text_pos = if centered { rect.center() - vec2(0.0, 9.0) } else { pos2(rect.min.x + 8.0, rect.center().y - 9.0) };

        if editing {
            if let Some((_, buf)) = &mut self.hex_edit {
                let te = egui::TextEdit::singleline(buf)
                    .font(FontId::monospace(size - 3.0))
                    .align(align)
                    .text_color(fg)
                    .desired_width(rect.width());
                let edit_rect = Rect::from_center_size(rect.center() - vec2(0.0, 9.0), vec2(rect.width(), 34.0));
                let resp = ui.put(edit_rect, te);
                resp.request_focus();
                let (enter, esc) = ui.input(|i| (i.key_pressed(Key::Enter), i.key_pressed(Key::Escape)));
                if enter || esc || resp.lost_focus() {
                    if !esc {
                        if let Some(c) = color::parse_hex(buf) {
                            acts.push(ColAct::Pick(i, c));
                        }
                    }
                    self.hex_edit = None;
                }
            }
            return;
        }

        let hover = interactive && {
            let resp = ui.interact(rect, ui.scope_id().with(("hex", i)), Sense::click_and_drag());
            if resp.hovered() {
                ui.painter().rect_filled(rect, 8.0, fg.gamma_multiply(0.12));
            }
            if resp.on_hover_text("Click to edit hex").clicked() {
                acts.push(ColAct::Edit(i));
            }
            true
        };
        let _ = hover;
        let p = ui.painter();
        p.text(text_pos, align, color::to_hex(rgb), FontId::monospace(size), fg);
        let (h, s, l) = color::to_hsl(rgb);
        let sub = if centered { pos2(rect.center().x, rect.center().y + 16.0) } else { pos2(rect.min.x + 8.0, rect.center().y + 16.0) };
        p.text(
            sub,
            align,
            format!("{:.0}°  {:.0}%  {:.0}%", h, s * 100.0, l * 100.0),
            FontId::proportional(11.0),
            fg.gamma_multiply(0.65),
        );
    }

    fn shade_overlay(&mut self, ui: &mut Ui, i: usize, r: Rect, kind: Kind, acts: &mut Vec<ColAct>) {
        let rgb = self.d.working[i].rgb;
        let (ladder, cur) = color::shade_ladder(rgb, 21);
        let count = ladder.len();
        for (k, c) in ladder.iter().enumerate() {
            let row = if kind == Kind::Band {
                let w = r.width() / count as f32;
                Rect::from_min_max(
                    pos2((r.min.x + w * k as f32).round(), r.min.y),
                    pos2((r.min.x + w * (k + 1) as f32).round(), r.max.y),
                )
            } else {
                let h = r.height() / count as f32;
                Rect::from_min_max(
                    pos2(r.min.x, (r.min.y + h * k as f32).round()),
                    pos2(r.max.x, (r.min.y + h * (k + 1) as f32).round()),
                )
            };
            let cr = if kind == Kind::Card {
                let q = 16u8;
                CornerRadius {
                    nw: if k == 0 { q } else { 0 },
                    ne: if k == 0 { q } else { 0 },
                    sw: if k + 1 == count { q } else { 0 },
                    se: if k + 1 == count { q } else { 0 },
                }
            } else {
                CornerRadius::ZERO
            };
            let resp = ui.interact(row, ui.scope_id().with(("shade", i, k)), Sense::click());
            let t = c32(color::text_on(*c));
            let p = ui.painter();
            p.rect_filled(row, cr, c32(*c));
            if resp.hovered() {
                p.text(row.center(), Align2::CENTER_CENTER, color::to_hex(*c), FontId::monospace(13.0), t);
            }
            if k == cur {
                let dot = if kind == Kind::Band { pos2(row.center().x, row.max.y - 10.0) } else { pos2(row.min.x + 14.0, row.center().y) };
                p.circle_filled(dot, 3.5, t);
            }
            if resp.clicked() {
                acts.push(ColAct::Pick(i, *c));
            }
        }
        if ui.input(|i| i.key_pressed(Key::Escape)) {
            acts.push(ColAct::CloseShades);
        }
        let close = match kind {
            Kind::Column => pos2(r.center().x, r.min.y + 24.0),
            Kind::Band => pos2(r.max.x - 24.0, r.center().y),
            Kind::Card => pos2(r.max.x - 22.0, r.min.y + 22.0),
        };
        if icon_at(ui, close, Icon::Close, Color32::from_gray(20), true, ("shclose", i))
            .on_hover_text("Close shades")
            .clicked()
        {
            acts.push(ColAct::CloseShades);
        }
    }

    // ------------------------------------------------------------ gradient

    fn gradient_view(&mut self, ui: &mut Ui, rect: Rect, acts: &mut Vec<ColAct>) {
        let n = self.d.working.len();
        let pad = 44.0;
        let gh = (rect.height() * 0.40).clamp(130.0, 300.0);
        let grad = Rect::from_min_size(pos2(rect.min.x + pad, rect.min.y + pad), vec2(rect.width() - pad * 2.0, gh));
        let slot_w = grad.width() / n as f32;
        let slots: Vec<Rect> = (0..n)
            .map(|k| {
                let x = grad.min.x + grad.width() * k as f32 / (n - 1) as f32;
                Rect::from_center_size(pos2(x, grad.max.y + 110.0), vec2(slot_w, 190.0))
            })
            .collect();
        let plan = self.plan(ui, &slots);
        let dragging = self.drag.is_some();

        // Gradient bar, using the live (preview) order while dragging.
        let cols: Vec<Rgb> = plan.iter().filter(|(_, _, r)| *r != Role::Ghost).map(|(i, _, _)| self.d.working[*i].rgb).collect();
        let at = |t: f32| -> Rgb {
            let f = (t.clamp(0.0, 1.0)) * (cols.len() - 1) as f32;
            let k = (f.floor() as usize).min(cols.len() - 2);
            color::mix_oklab(cols[k], cols[k + 1], f - k as f32)
        };
        let radius = 18.0;
        {
            let p = ui.painter();
            p.rect_filled(grad.translate(vec2(0.0, 10.0)).shrink2(vec2(10.0, 0.0)), radius, Color32::from_black_alpha(60));
            let (x0, x1) = (grad.min.x + radius, grad.max.x - radius);
            p.rect_filled(Rect::from_min_max(grad.min, pos2(x0 + 2.0, grad.max.y)), radius, c32(at(radius / grad.width())));
            p.rect_filled(Rect::from_min_max(pos2(x1 - 2.0, grad.min.y), grad.max), radius, c32(at(1.0 - radius / grad.width())));
            let mut mesh = Mesh::default();
            let segs = 180;
            for s in 0..segs {
                let (ta, tb) = (s as f32 / segs as f32, (s + 1) as f32 / segs as f32);
                let (xa, xb) = (x0 + (x1 - x0) * ta, x0 + (x1 - x0) * tb);
                let tc = |x: f32| (x - grad.min.x) / grad.width();
                let (ca, cb) = (c32(at(tc(xa))), c32(at(tc(xb))));
                let base = mesh.vertices.len() as u32;
                mesh.colored_vertex(pos2(xa, grad.min.y), ca);
                mesh.colored_vertex(pos2(xb, grad.min.y), cb);
                mesh.colored_vertex(pos2(xb, grad.max.y), cb);
                mesh.colored_vertex(pos2(xa, grad.max.y), ca);
                mesh.add_triangle(base, base + 1, base + 2);
                mesh.add_triangle(base, base + 2, base + 3);
            }
            p.add(Shape::mesh(mesh));
            p.rect_stroke(grad, radius, Stroke::new(1.0, Color32::from_white_alpha(26)), StrokeKind::Inside);
        }

        for (i, r, role) in plan {
            if role == Role::Placeholder {
                continue;
            }
            let ghost = role == Role::Ghost;
            let rgb = self.d.working[i].rgb;
            let locked = self.d.working[i].locked;
            let x = r.center().x.clamp(grad.min.x, grad.max.x);
            let handle = pos2(x, grad.max.y);
            let card_w = (slot_w - 10.0).clamp(64.0, 156.0);
            let wide = card_w >= 128.0;
            let card = Rect::from_min_size(
                pos2(r.center().x - card_w / 2.0, grad.max.y + 34.0 + if ghost { r.center().y - slots[i].center().y } else { 0.0 }),
                vec2(card_w, if wide { 150.0 } else { 118.0 }),
            );

            // Handle on the gradient
            let hr = Rect::from_center_size(handle, vec2(30.0, 30.0));
            let hresp = if dragging || ghost { None } else { Some(ui.interact(hr, ui.scope_id().with(("handle", i)), Sense::drag())) };
            {
                let p = ui.painter();
                p.line_segment([handle, pos2(card.center().x, card.min.y)], Stroke::new(1.5, Color32::from_white_alpha(40)));
                p.circle_filled(handle + vec2(0.0, 2.0), 13.0, Color32::from_black_alpha(90));
                p.circle_filled(handle, 12.0, c32(rgb));
                p.circle_stroke(handle, 12.0, Stroke::new(3.0, Color32::WHITE));
            }
            if let Some(h) = hresp {
                if h.hovered() {
                    ui.ctx().set_cursor_icon(CursorIcon::Grab);
                }
                if h.drag_started() {
                    self.begin_drag(ui, i, slots[i]);
                }
            }

            // Stop card
            {
                let p = ui.painter();
                if ghost {
                    p.rect_filled(card.translate(vec2(0.0, 8.0)).expand(2.0), 14.0, Color32::from_black_alpha(100));
                }
                p.rect_filled(card, 14.0, theme::RAISED);
                p.rect_stroke(card, 14.0, Stroke::new(1.0, if ghost { Color32::from_white_alpha(160) } else { theme::LINE }), StrokeKind::Inside);
            }
            let chip = Rect::from_min_size(card.min + vec2(8.0, 8.0), vec2(card_w - 16.0, 48.0));
            ui.painter().rect_filled(chip, 9.0, c32(rgb));
            let hex_rect = Rect::from_min_size(card.min + vec2(4.0, 60.0), vec2(card_w - 8.0, 30.0));
            let interactive = !ghost && !dragging;
            self.hex_label_plain(ui, i, hex_rect, interactive, acts);

            if interactive {
                let cresp = ui.interact(chip, ui.scope_id().with(("chip", i)), Sense::click_and_drag());
                if cresp.hovered() {
                    ui.ctx().set_cursor_icon(CursorIcon::Grab);
                }
                if cresp.drag_started() {
                    self.begin_drag(ui, i, slots[i]);
                }
                if cresp.on_hover_text("Click to copy · drag to reorder").clicked() {
                    acts.push(ColAct::Copy(i));
                }
                let len = self.d.working.len();
                let mut list: Vec<(Icon, &str, ColAct, bool)> = vec![
                    (Icon::Lock(locked), if locked { "Unlock" } else { "Lock" }, ColAct::Lock(i), locked),
                    (Icon::Copy, "Copy hex", ColAct::Copy(i), false),
                ];
                if wide {
                    list.push((Icon::Adjust, "Adjust color", ColAct::Adjust(i), false));
                    if len > 2 {
                        list.push((Icon::Close, "Remove color", ColAct::Remove(i), false));
                    }
                }
                let step = if wide { 34.0 } else { 30.0 };
                let total = step * list.len() as f32;
                let y = card.max.y - 22.0;
                for (k, (icon, tip, act, active)) in list.into_iter().enumerate() {
                    let c = pos2(card.center().x - total / 2.0 + step * (k as f32 + 0.5), y);
                    if icon_at(ui, c, icon, Color32::from_gray(210), active, (i, "g", tip)).on_hover_text(tip).clicked() {
                        acts.push(act);
                    }
                }
            }
        }
    }

    /// Hex text on a dark card (white text), click to edit.
    fn hex_label_plain(&mut self, ui: &mut Ui, i: usize, rect: Rect, interactive: bool, acts: &mut Vec<ColAct>) {
        let rgb = self.d.working[i].rgb;
        let editing = interactive && self.hex_edit.as_ref().map(|(c, _)| *c) == Some(i);
        if editing {
            if let Some((_, buf)) = &mut self.hex_edit {
                let te = egui::TextEdit::singleline(buf)
                    .font(FontId::monospace(14.0))
                    .align(Align2::CENTER_CENTER)
                    .desired_width(rect.width());
                let resp = ui.put(rect, te);
                resp.request_focus();
                let (enter, esc) = ui.input(|i| (i.key_pressed(Key::Enter), i.key_pressed(Key::Escape)));
                if enter || esc || resp.lost_focus() {
                    if !esc {
                        if let Some(c) = color::parse_hex(buf) {
                            acts.push(ColAct::Pick(i, c));
                        }
                    }
                    self.hex_edit = None;
                }
            }
            return;
        }
        if interactive {
            let resp = ui.interact(rect, ui.scope_id().with(("hexp", i)), Sense::click());
            if resp.hovered() {
                ui.painter().rect_filled(rect, 7.0, Color32::from_white_alpha(14));
            }
            if resp.on_hover_text("Click to edit hex").clicked() {
                acts.push(ColAct::Edit(i));
            }
        }
        ui.painter().text(rect.center(), Align2::CENTER_CENTER, color::to_hex(rgb), FontId::monospace(15.0), theme::TEXT);
    }

    // ------------------------------------------------------------ actions

    pub fn apply_col_act(&mut self, ctx: &Context, a: ColAct) {
        match a {
            ColAct::Remove(i) => {
                if self.d.working.len() > 2 {
                    self.snapshot();
                    self.d.working.remove(i);
                    if i < self.slide.len() {
                        self.slide.remove(i);
                    }
                    self.shades_col = None;
                    self.touch();
                }
            }
            ColAct::Lock(i) => {
                self.d.working[i].locked ^= true;
                self.touch();
            }
            ColAct::Shades(i) => {
                self.shades_col = if self.shades_col == Some(i) { None } else { Some(i) };
            }
            ColAct::CloseShades => self.shades_col = None,
            ColAct::Adjust(i) => {
                self.snapshot();
                self.adjust = Some(Adjust::new(i, self.d.working[i].rgb, self.adjust_mode));
            }
            ColAct::Copy(i) => {
                let c = self.d.working[i].rgb;
                self.copy(ctx, c);
                self.select(c);
            }
            ColAct::Edit(i) => {
                self.hex_edit = Some((i, color::to_hex(self.d.working[i].rgb)));
            }
            ColAct::Pick(i, c) => {
                self.snapshot();
                self.d.working[i].rgb = c;
                self.shades_col = None;
                self.select(c);
            }
            ColAct::Insert(i) => {
                self.snapshot();
                let c = color::mix(self.d.working[i - 1].rgb, self.d.working[i].rgb, 0.5);
                self.d.working.insert(i, Swatch { rgb: c, locked: false });
                self.touch();
            }
        }
    }
}
