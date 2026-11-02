//! The always-on-top Quick Picker window (summoned by its own hotkey).

use crate::app::{vid, App, PICKER};
use crate::color;
use crate::theme;
use crate::widgets::{c32, swatch, SwAct};
use egui::{vec2, Align, Color32, Context, FontId, Frame, Key, Layout, Margin, RichText, Sense, ViewportBuilder, ViewportCommand};

impl App {
    pub fn show_picker(&mut self, ctx: &Context) {
        let builder = ViewportBuilder::default()
            .with_title("Quick Picker")
            .with_inner_size(vec2(300.0, 520.0))
            .with_position(self.picker_pos)
            .with_always_on_top()
            .with_resizable(false)
            .with_minimize_button(false)
            .with_maximize_button(false);

        let fresh = std::mem::take(&mut self.picker_fresh);
        let mut act: Option<SwAct> = None;
        let mut eyedrop = false;
        let mut done = false;
        let mut close = false;

        ctx.show_viewport_immediate(vid(PICKER), builder, |ui, _| {
            if fresh {
                ui.send_viewport_cmd(ViewportCommand::Focus);
            }
            if ui.input(|i| i.viewport().close_requested() || i.key_pressed(Key::Escape)) {
                close = true;
            }

            egui::CentralPanel::default()
                .frame(Frame::new().fill(theme::PANEL).inner_margin(Margin::same(14)))
                .show(ui, |ui| {
                    ui.spacing_mut().slider_width = 252.0;
                    let sel = self.d.selected;

                    // Preview
                    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 56.0), Sense::click());
                    ui.painter().rect_filled(rect, 10.0, c32(sel));
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        self.d.settings.fmt_hex(sel),
                        FontId::monospace(22.0),
                        c32(color::text_on(sel)),
                    );
                    if resp.on_hover_text("Click to copy").clicked() {
                        act = Some(SwAct::Copy(sel));
                    }

                    // HSV picker
                    let mut col = c32(sel);
                    if egui::color_picker::color_picker_color32(ui, &mut col, egui::color_picker::Alpha::Opaque) {
                        let n = [col.r(), col.g(), col.b()];
                        if n != sel {
                            self.select(n);
                        }
                    }

                    // Hex entry
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("HEX").color(theme::MUTED).size(11.0));
                        let r = ui.add(
                            egui::TextEdit::singleline(&mut self.selected_hex)
                                .font(FontId::monospace(14.0))
                                .desired_width(110.0),
                        );
                        if r.changed() {
                            if let Some(c) = color::parse_hex(&self.selected_hex) {
                                self.d.selected = c;
                                self.dirty = true;
                            }
                        }
                        if r.lost_focus() {
                            self.selected_hex = color::to_hex_hash(self.d.selected);
                        }
                        let (h, s, l) = color::to_hsl(self.d.selected);
                        ui.label(
                            RichText::new(format!("{:.0}° {:.0}% {:.0}%", h, s * 100.0, l * 100.0))
                                .color(theme::MUTED)
                                .size(11.0),
                        );
                    });

                    ui.horizontal(|ui| {
                        if ui.button("Eyedropper").clicked() {
                            eyedrop = true;
                        }
                        if ui.button("+ Palette").on_hover_text("Add to the working palette").clicked() {
                            act = Some(SwAct::Add(self.d.selected));
                        }
                        if ui.button("Save").on_hover_text("Keep in Samples").clicked() {
                            let c = self.d.selected;
                            self.d.push_history(c);
                            self.rebuild_tray();
                            self.toast("Saved to Samples");
                        }
                    });
                    ui.with_layout(Layout::top_down(Align::Min), |ui| {
                        if ui
                            .add_sized([ui.available_width(), 30.0], egui::Button::new("Copy & close").fill(Color32::from_rgb(61, 126, 245)))
                            .clicked()
                        {
                            done = true;
                        }
                    });

                    ui.separator();
                    ui.label(RichText::new("Recent samples").color(theme::MUTED).size(11.0));
                    let hist: Vec<_> = self.d.history.iter().take(16).copied().collect();
                    if hist.is_empty() {
                        ui.label(RichText::new("Nothing sampled yet").color(theme::MUTED).size(11.0));
                    }
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = vec2(5.0, 5.0);
                        for c in hist {
                            if let Some(a) = swatch(ui, c, vec2(28.0, 28.0), c == self.d.selected, None, false) {
                                act = Some(a);
                            }
                        }
                    });
                    ui.label(RichText::new("Working palette").color(theme::MUTED).size(11.0));
                    let pal: Vec<_> = self.d.working.iter().map(|s| s.rgb).collect();
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = vec2(5.0, 5.0);
                        for c in pal {
                            if let Some(a) = swatch(ui, c, vec2(28.0, 28.0), c == self.d.selected, None, false) {
                                act = Some(a);
                            }
                        }
                    });
                });

            self.paint_toast(ui.ctx());
        });

        if let Some(a) = act {
            self.apply_swatch_action(ctx, a);
        }
        if eyedrop {
            self.picker_open = false;
            self.start_loupe(ctx);
        }
        if done {
            let c = self.d.selected;
            self.d.push_history(c);
            self.rebuild_tray();
            ctx.copy_text(self.d.settings.fmt_hex(c));
            close = true;
        }
        if close {
            self.picker_open = false;
        }
    }

    pub fn apply_swatch_action(&mut self, ctx: &Context, a: SwAct) {
        match a {
            SwAct::Select(c) => self.select(c),
            SwAct::Drag(c) => self.sample_drag = Some(c),
            SwAct::Add(c) => {
                self.add_color(c);
                self.select(c);
            }
            SwAct::Copy(c) => self.copy(ctx, c),
            SwAct::Remove(c) => {
                self.d.history_remove(c);
                self.rebuild_tray();
                self.touch();
            }
        }
    }
}
