//! Small painted widgets (icons are drawn with shapes so we don't depend on font glyphs).

use crate::color::{self, Rgb};
use egui::{pos2, vec2, Color32, FontId, Pos2, Rect, Response, Sense, Shape, Stroke, StrokeKind, Ui};

pub fn c32(c: Rgb) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

#[derive(Clone, Copy, PartialEq)]
pub enum Icon {
    Lock(bool),
    Copy,
    Shades,
    Adjust,
    Left,
    Right,
    Close,
    Plus,
}

/// Round icon button centered at `center`. Returns the click response.
pub fn icon_at(ui: &mut Ui, center: Pos2, icon: Icon, fg: Color32, active: bool, salt: impl std::hash::Hash + std::fmt::Debug) -> Response {
    let rect = Rect::from_center_size(center, vec2(34.0, 34.0));
    let id = ui.scope_id().with(("icon", salt));
    let resp = ui.interact(rect, id, Sense::click_and_drag());
    let p = ui.painter();
    if resp.hovered() || active {
        p.rect_filled(rect, 17.0, fg.gamma_multiply(if resp.is_pointer_button_down_on() { 0.28 } else { 0.16 }));
    }
    let s = Stroke::new(1.8, fg);
    let c = center;
    match icon {
        Icon::Lock(locked) => {
            let body = Rect::from_center_size(c + vec2(0.0, 3.5), vec2(13.0, 9.5));
            p.rect_filled(body, 2.0, fg);
            let r = 4.2;
            let top = c.y - 1.0;
            let mut pts = vec![];
            for i in 0..=12 {
                let a = std::f32::consts::PI * (i as f32 / 12.0);
                pts.push(pos2(c.x - r * a.cos(), top - r * a.sin() - if locked { 0.0 } else { 3.0 }));
            }
            // left leg
            let mut path = vec![pos2(c.x - r, top + 1.5)];
            path.extend(pts.iter().copied());
            if locked {
                path.push(pos2(c.x + r, top + 1.5));
            }
            p.add(Shape::line(path, s));
        }
        Icon::Copy => {
            let a = Rect::from_center_size(c + vec2(-1.5, 1.5), vec2(11.0, 12.0));
            let b = Rect::from_center_size(c + vec2(2.5, -2.5), vec2(11.0, 12.0));
            p.rect_stroke(a, 2.0, s, StrokeKind::Middle);
            p.rect_stroke(b, 2.0, s, StrokeKind::Middle);
        }
        Icon::Shades => {
            for i in 0..4 {
                let y = c.y - 7.5 + i as f32 * 5.0;
                let r = Rect::from_min_size(pos2(c.x - 7.0, y), vec2(14.0, 3.6));
                p.rect_filled(r, 1.0, fg.gamma_multiply(1.0 - i as f32 * 0.22));
            }
        }
        Icon::Adjust => {
            for (i, k) in [0.3f32, 0.7, 0.45].iter().enumerate() {
                let y = c.y - 6.0 + i as f32 * 6.0;
                p.line_segment([pos2(c.x - 8.0, y), pos2(c.x + 8.0, y)], Stroke::new(1.4, fg));
                p.circle_filled(pos2(c.x - 8.0 + 16.0 * k, y), 2.8, fg);
            }
        }
        Icon::Left => {
            p.add(Shape::line(vec![c + vec2(2.5, -6.0), c + vec2(-3.5, 0.0), c + vec2(2.5, 6.0)], s));
        }
        Icon::Right => {
            p.add(Shape::line(vec![c + vec2(-2.5, -6.0), c + vec2(3.5, 0.0), c + vec2(-2.5, 6.0)], s));
        }
        Icon::Close => {
            p.line_segment([c + vec2(-5.0, -5.0), c + vec2(5.0, 5.0)], s);
            p.line_segment([c + vec2(-5.0, 5.0), c + vec2(5.0, -5.0)], s);
        }
        Icon::Plus => {
            p.line_segment([c + vec2(-6.0, 0.0), c + vec2(6.0, 0.0)], s);
            p.line_segment([c + vec2(0.0, -6.0), c + vec2(0.0, 6.0)], s);
        }
    }
    resp
}

#[derive(Clone, Copy, Debug)]
pub enum SwAct {
    Select(Rgb),
    Add(Rgb),
    Copy(Rgb),
    Remove(Rgb),
    /// Started dragging this color (e.g. into the palette).
    Drag(Rgb),
}

/// A clickable color chip. Left click selects, right click opens a menu.
pub fn swatch(ui: &mut Ui, rgb: Rgb, size: egui::Vec2, selected: bool, label: Option<&str>, removable: bool) -> Option<SwAct> {
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click_and_drag());
    let p = ui.painter();
    p.rect_filled(rect, 6.0, c32(rgb));
    p.rect_stroke(rect, 6.0, Stroke::new(1.0, Color32::from_black_alpha(50)), StrokeKind::Inside);
    if selected {
        p.rect_stroke(rect.expand(1.5), 7.0, Stroke::new(2.0, Color32::WHITE), StrokeKind::Outside);
        p.rect_stroke(rect.expand(3.0), 8.0, Stroke::new(1.0, Color32::from_black_alpha(160)), StrokeKind::Outside);
    }
    if let Some(l) = label {
        let t = c32(color::text_on(rgb));
        p.text(rect.center(), egui::Align2::CENTER_CENTER, l, FontId::monospace(11.5), t);
    }
    let resp = resp.on_hover_text(format!("{}\nClick: select · Drag: into palette · Right-click: more", color::to_hex_hash(rgb)));
    let mut act = None;
    if resp.clicked() {
        act = Some(SwAct::Select(rgb));
    }
    if resp.drag_started() {
        act = Some(SwAct::Drag(rgb));
    }
    resp.context_menu(|ui| {
        if ui.button("Select").clicked() {
            act = Some(SwAct::Select(rgb));
            ui.close();
        }
        if ui.button("Add to palette").clicked() {
            act = Some(SwAct::Add(rgb));
            ui.close();
        }
        if ui.button(format!("Copy {}", color::to_hex_hash(rgb))).clicked() {
            act = Some(SwAct::Copy(rgb));
            ui.close();
        }
        if removable && ui.button("Remove").clicked() {
            act = Some(SwAct::Remove(rgb));
            ui.close();
        }
    });
    act
}

/// A horizontal strip of equal-width swatches filling the available width.
pub fn strip(ui: &mut Ui, colors: &[Rgb], height: f32, selected: Option<Rgb>, labels: bool) -> Option<SwAct> {
    let gap = 4.0;
    let n = colors.len().max(1) as f32;
    let w = ((ui.available_width() - gap * (n - 1.0)) / n).floor().max(10.0);
    let mut act = None;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        for c in colors {
            let l = to_label(*c, labels && w > 66.0);
            if let Some(a) = swatch(ui, *c, vec2(w, height), selected == Some(*c), l.as_deref(), false) {
                act = Some(a);
            }
        }
    });
    act
}

fn to_label(c: Rgb, on: bool) -> Option<String> {
    on.then(|| color::to_hex(c))
}
