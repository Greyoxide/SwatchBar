//! The magnifier that follows the cursor, and the little "copied" flash.

use crate::app::{vid, App, FLASH, LOUPE};
use crate::capture;
use crate::color;
use crate::widgets::c32;
use egui::{pos2, vec2, Align2, Color32, Context, FontId, Rect, Stroke, StrokeKind, ViewportBuilder, ViewportCommand};
use std::time::Duration;

const CELL: f32 = 10.0;
const PAD: f32 = 8.0;
const LABEL_H: f32 = 58.0;
const OFFSET: f32 = 26.0;

impl App {
    pub fn show_loupe(&mut self, ctx: &Context) {
        let n = self.d.settings.loupe_cells | 1;
        let size = vec2(n as f32 * CELL + PAD * 2.0, n as f32 * CELL + PAD * 2.0 + LABEL_H);

        if let Some(s) = capture::grab(n) {
            if let Some(l) = &mut self.loupe {
                l.sample = Some(s);
            }
        }
        let Some(sample) = self.loupe.as_ref().and_then(|l| l.sample.as_ref()) else {
            return;
        };

        // Sit down-right of the cursor; flip when near the display edge.
        let (cx, cy) = (sample.cursor.0 as f32, sample.cursor.1 as f32);
        let (dx, dy, dw, dh) = sample.display;
        let mut x = cx + OFFSET;
        let mut y = cy + OFFSET;
        if x + size.x > (dx + dw) as f32 {
            x = cx - OFFSET - size.x;
        }
        if y + size.y > (dy + dh) as f32 {
            y = cy - OFFSET - size.y;
        }
        let pos = pos2(x, y);

        let builder = ViewportBuilder::default()
            .with_title("Swatchbar Loupe")
            .with_inner_size(size)
            .with_position(pos)
            .with_decorations(false)
            .with_transparent(true)
            .with_always_on_top()
            .with_resizable(false)
            .with_mouse_passthrough(true)
            .with_active(false)
            .with_taskbar(false)
            .with_has_shadow(false);

        let center = sample.center;
        let fmt = self.d.settings.fmt_hex(center);
        let pixels = &sample.pixels;
        let (px, py) = (sample.cursor.0, sample.cursor.1);

        ctx.show_viewport_immediate(vid(LOUPE), builder, |ui, _class| {
            ui.send_viewport_cmd(ViewportCommand::OuterPosition(pos));
            let p = ui.painter();
            let full = Rect::from_min_size(pos2(0.0, 0.0), size);
            p.rect_filled(full.shrink(1.0), 14.0, Color32::from_rgba_unmultiplied(24, 24, 27, 238));
            p.rect_stroke(full.shrink(1.0), 14.0, Stroke::new(1.0, Color32::from_white_alpha(60)), StrokeKind::Inside);

            let grid = Rect::from_min_size(pos2(PAD, PAD), vec2(n as f32 * CELL, n as f32 * CELL));
            let clip = p.with_clip_rect(grid);
            for row in 0..n {
                for col in 0..n {
                    let r = Rect::from_min_size(
                        grid.min + vec2(col as f32 * CELL, row as f32 * CELL),
                        vec2(CELL, CELL),
                    );
                    clip.rect_filled(r, 0.0, c32(pixels[row * n + col]));
                }
            }
            // pixel grid
            let line = Stroke::new(1.0, Color32::from_black_alpha(34));
            for i in 1..n {
                let o = i as f32 * CELL;
                p.line_segment([grid.min + vec2(o, 0.0), grid.min + vec2(o, grid.height())], line);
                p.line_segment([grid.min + vec2(0.0, o), grid.min + vec2(grid.width(), o)], line);
            }
            p.rect_stroke(grid, 3.0, Stroke::new(1.0, Color32::from_white_alpha(70)), StrokeKind::Outside);

            // center pixel reticle
            let m = (n / 2) as f32;
            let cell = Rect::from_min_size(grid.min + vec2(m * CELL, m * CELL), vec2(CELL, CELL));
            p.rect_stroke(cell.expand(1.0), 0.0, Stroke::new(2.0, Color32::BLACK), StrokeKind::Outside);
            p.rect_stroke(cell, 0.0, Stroke::new(1.5, Color32::WHITE), StrokeKind::Inside);

            // readout
            let y0 = grid.max.y + 10.0;
            let chip = Rect::from_min_size(pos2(PAD + 2.0, y0), vec2(30.0, 30.0));
            p.rect_filled(chip, 7.0, c32(center));
            p.rect_stroke(chip, 7.0, Stroke::new(1.0, Color32::from_white_alpha(80)), StrokeKind::Inside);
            p.text(pos2(chip.max.x + 10.0, y0 + 1.0), Align2::LEFT_TOP, &fmt, FontId::monospace(16.0), Color32::WHITE);
            p.text(
                pos2(chip.max.x + 10.0, y0 + 19.0),
                Align2::LEFT_TOP,
                format!("rgb({}, {}, {})   {:.0}, {:.0}", center[0], center[1], center[2], px, py),
                FontId::proportional(10.5),
                Color32::from_gray(170),
            );
            p.text(
                pos2(size.x / 2.0, size.y - 9.0),
                Align2::CENTER_BOTTOM,
                "click pick  ·  ⇧click add  ·  ⎋ done",
                FontId::proportional(9.5),
                Color32::from_gray(120),
            );
        });
    }

    /// A short-lived chip near the cursor confirming what was sampled.
    pub fn show_flash(&mut self, ctx: &Context) {
        let Some((c, t0, cur)) = self.flash else { return };
        let age = t0.elapsed();
        if age > Duration::from_millis(1400) {
            self.flash = None;
            return;
        }
        let size = vec2(168.0, 38.0);
        let (dx, dy, dw, dh) = capture::display_at(cur.0, cur.1);
        let mut x = cur.0 as f32 + 18.0;
        let mut y = cur.1 as f32 + 18.0;
        if x + size.x > (dx + dw) as f32 {
            x = cur.0 as f32 - 18.0 - size.x;
        }
        if y + size.y > (dy + dh) as f32 {
            y = cur.1 as f32 - 18.0 - size.y;
        }
        let fade = if age > Duration::from_millis(1000) {
            1.0 - (age.as_millis() as f32 - 1000.0) / 400.0
        } else {
            1.0
        };
        let label = if self.d.settings.copy_on_sample {
            format!("{} copied", self.d.settings.fmt_hex(c))
        } else {
            self.d.settings.fmt_hex(c)
        };
        let builder = ViewportBuilder::default()
            .with_title("Swatchbar Flash")
            .with_inner_size(size)
            .with_position(pos2(x, y))
            .with_decorations(false)
            .with_transparent(true)
            .with_always_on_top()
            .with_resizable(false)
            .with_mouse_passthrough(true)
            .with_active(false)
            .with_taskbar(false)
            .with_has_shadow(false);
        ctx.show_viewport_immediate(vid(FLASH), builder, |ui, _| {
            let p = ui.painter();
            let r = Rect::from_min_size(pos2(0.0, 0.0), size).shrink(1.0);
            p.rect_filled(r, 12.0, Color32::from_rgba_unmultiplied(24, 24, 27, (235.0 * fade) as u8));
            let chip = Rect::from_min_size(pos2(8.0, 7.0), vec2(24.0, 24.0));
            p.rect_filled(chip, 6.0, c32(c).gamma_multiply(fade));
            p.text(
                pos2(42.0, 19.0),
                Align2::LEFT_CENTER,
                &label,
                FontId::monospace(13.0),
                Color32::from_white_alpha((255.0 * fade) as u8),
            );
            let _ = color::to_hex(c);
        });
    }
}
