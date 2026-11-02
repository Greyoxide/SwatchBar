//! The Palette Studio window: generator, explorer (harmonies + shades), library, settings.

use crate::app::{vid, App, Slot, Tab, STUDIO};
use crate::capture;
use crate::color::{self, Rgb};
use crate::hotkeys;
use crate::store::{SavedPalette, Swatch};
use crate::theme;
use crate::widgets::{c32, strip, swatch, SwAct};
use egui::{
    pos2, vec2, Align, Align2, Color32, Context, FontId, Frame, Key, Layout, Margin, Panel, Rect, RichText, Sense,
    Stroke, StrokeKind, Ui, ViewportBuilder, ViewportCommand,
};
use std::time::Duration;

impl App {
    pub fn show_studio(&mut self, ctx: &Context) {
        let builder = ViewportBuilder::default()
            .with_title("Swatchbar")
            .with_inner_size(vec2(1120.0, 740.0))
            .with_min_inner_size(vec2(1000.0, 580.0));
        let focus = std::mem::take(&mut self.focus_studio);
        let mut close = false;
        ctx.show_viewport_immediate(vid(STUDIO), builder, |ui, _| {
            if focus {
                ui.send_viewport_cmd(ViewportCommand::Focus);
            }
            self.shots_focus(ui);
            if ui.input(|i| i.viewport().close_requested()) {
                close = true;
            }
            self.studio_ui(ui);
        });
        if close {
            self.studio_open = false;
            self.stop_recording();
        }
    }

    fn stop_recording(&mut self) {
        self.recording = None;
    }

    pub fn paint_toast(&mut self, ctx: &Context) {
        let Some((msg, t0)) = &self.toast else { return };
        if t0.elapsed() > Duration::from_millis(1800) {
            self.toast = None;
            return;
        }
        ctx.request_repaint_after(Duration::from_millis(100));
        egui::Area::new(egui::Id::new("toast"))
            .anchor(Align2::CENTER_BOTTOM, vec2(0.0, -18.0))
            .order(egui::Order::Tooltip)
            .interactable(false)
            .show(ctx, |ui| {
                Frame::new()
                    .fill(Color32::from_rgba_unmultiplied(12, 12, 14, 235))
                    .corner_radius(10)
                    .inner_margin(Margin::symmetric(14, 8))
                    .stroke(Stroke::new(1.0, theme::LINE))
                    .show(ui, |ui| {
                        ui.label(RichText::new(msg.as_str()).color(Color32::WHITE).size(13.0));
                    });
            });
    }

    fn studio_ui(&mut self, ui: &mut Ui) {
        self.record_hotkey(ui);
        self.drop_slot = None;

        // Global-to-window shortcuts.
        let typing = ui.ctx().egui_wants_keyboard_input();
        let (space, undo, redo) = ui.input_mut(|i| {
            let redo = i.consume_key(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, Key::Z);
            let undo = i.consume_key(egui::Modifiers::COMMAND, Key::Z);
            (!typing && i.key_pressed(Key::Space), undo, redo)
        });
        if undo {
            self.do_undo();
        }
        if redo {
            self.do_redo();
        }
        if space && self.tab == Tab::Generator && self.recording.is_none() {
            self.generate();
        }

        Panel::top("top")
            .frame(Frame::new().fill(theme::BG).inner_margin(Margin::symmetric(16, 10)).stroke(Stroke::new(1.0, theme::LINE)))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // Brand mark: the working palette in miniature.
                    let (mark, _) = ui.allocate_exact_size(vec2(22.0, 22.0), Sense::hover());
                    for (k, sw) in self.d.working.iter().take(4).enumerate() {
                        let (x, y) = ((k % 2) as f32 * 11.0, (k / 2) as f32 * 11.0);
                        let r = Rect::from_min_size(mark.min + vec2(x, y), vec2(10.0, 10.0));
                        ui.painter().rect_filled(r, 3.0, c32(sw.rgb));
                    }
                    ui.label(RichText::new("Swatchbar").strong().size(15.0));
                    ui.add_space(18.0);

                    // Segmented tab control
                    Frame::new()
                        .fill(theme::PANEL)
                        .stroke(Stroke::new(1.0, theme::LINE))
                        .corner_radius(10)
                        .inner_margin(Margin::same(3))
                        .show(ui, |ui| {
                            ui.spacing_mut().item_spacing.x = 2.0;
                            for (t, label) in [
                                (Tab::Generator, "Generator"),
                                (Tab::Explore, "Explore"),
                                (Tab::Preview, "Preview"),
                                (Tab::Library, "Library"),
                                (Tab::Settings, "Settings"),
                            ] {
                                let on = self.tab == t;
                                let txt = RichText::new(label).size(13.0).color(if on { Color32::WHITE } else { Color32::from_gray(170) });
                                let b = egui::Button::new(if on { txt.strong() } else { txt })
                                    .fill(if on { theme::ACCENT } else { Color32::TRANSPARENT })
                                    .stroke(Stroke::NONE)
                                    .corner_radius(7)
                                    .min_size(vec2(78.0, 26.0));
                                if ui.add(b).clicked() {
                                    self.tab = t;
                                }
                            }
                        });

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let (s, _) = self.labels();
                        if ui.add(theme::primary("Pick from screen")).clicked() {
                            let c = ui.ctx().clone();
                            self.start_loupe(&c);
                        }
                        theme::kbd(ui, &s);
                        ui.add_space(10.0);
                        ui.add_enabled_ui(!self.redo.is_empty(), |ui| {
                            if ui.add(theme::ghost("Redo")).clicked() {
                                self.do_redo();
                            }
                        });
                        ui.add_enabled_ui(!self.undo.is_empty(), |ui| {
                            if ui.add(theme::ghost("Undo")).clicked() {
                                self.do_undo();
                            }
                        });
                    });
                });
            });

        if !self.screen_ok {
            Panel::top("perm")
                .frame(Frame::new().fill(Color32::from_rgb(52, 40, 18)).inner_margin(Margin::symmetric(16, 7)))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(capture::PERMISSION_BANNER).color(theme::AMBER).size(12.5));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if capture::HAS_SETTINGS_PANE && ui.button("Open System Settings").clicked() {
                                capture::request_screen_access();
                                capture::open_screen_recording_settings();
                            }
                        });
                    });
                });
        }

        Panel::left("samples")
            .exact_size(196.0)
            .resizable(false)
            .frame(Frame::new().fill(theme::PANEL).inner_margin(Margin::same(14)).stroke(Stroke::new(1.0, theme::LINE)))
            .show(ui, |ui| self.samples_panel(ui));

        if self.tab == Tab::Generator {
            Panel::bottom("genbar")
                .frame(Frame::new().fill(theme::BG).inner_margin(Margin::symmetric(16, 10)).stroke(Stroke::new(1.0, theme::LINE)))
                .show(ui, |ui| self.generator_bar(ui));
        }

        egui::CentralPanel::default()
            .frame(Frame::new().fill(theme::BG))
            .show(ui, |ui| match self.tab {
                Tab::Generator => self.generator(ui),
                Tab::Explore => self.explore(ui),
                Tab::Preview => self.preview(ui),
                Tab::Library => self.library(ui),
                Tab::Settings => self.settings(ui),
            });

        self.adjust_window(ui);
        let ctx = ui.ctx().clone();
        self.sample_drag_overlay(ui, &ctx);
        self.paint_toast(&ctx);
    }

    /// The color chip that follows the pointer while dragging a sample. Over the palette it
    /// becomes a real swatch instead (see `sample_morph`), which also handles the drop.
    fn sample_drag_overlay(&mut self, ui: &Ui, ctx: &Context) {
        let Some(c) = self.sample_drag else { return };
        ctx.request_repaint();
        if self.sample_in {
            return;
        }
        ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
        if let Some(p) = ctx.input(|i| i.pointer.latest_pos()) {
            let layer = egui::LayerId::new(egui::Order::Tooltip, ui.scope_id().with("sample-drag"));
            let pl = ctx.layer_painter(layer);
            let r = Rect::from_center_size(p, vec2(44.0, 44.0));
            for (grow, a) in [(12.0, 16), (7.0, 28), (3.0, 46)] {
                pl.rect_filled(r.translate(vec2(0.0, 8.0)).expand(grow), 8.0 + grow, Color32::from_black_alpha(a));
            }
            pl.rect_filled(r, 9.0, c32(c));
            pl.rect_stroke(r, 9.0, Stroke::new(2.0, Color32::WHITE), StrokeKind::Inside);
        }
        if !ctx.input(|i| i.pointer.any_down()) {
            self.sample_drag = None;
            self.sample_veto = false;
            if let Some(slot) = self.drop_slot.take() {
                self.assign_role(slot, c);
            }
            self.role_drag_from = None;
        }
    }

    // ---------------------------------------------------------------- samples

    fn samples_panel(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            theme::section(ui, "Samples");
            ui.label(RichText::new(self.d.history.len().to_string()).size(11.0).color(theme::MUTED));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if !self.d.history.is_empty() && ui.add(theme::ghost("Clear")).clicked() {
                    self.d.history_clear();
                    self.rebuild_tray();
                    self.touch();
                }
            });
        });

        let sel = self.d.selected;
        let mut act = None;
        let avail = ui.available_height();
        egui::ScrollArea::vertical()
            .max_height((avail - 176.0).max(100.0))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if self.d.history.is_empty() {
                    let (s, _) = self.labels();
                    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 96.0), Sense::hover());
                    ui.painter().rect_stroke(rect, 10.0, Stroke::new(1.0, theme::LINE), StrokeKind::Inside);
                    ui.painter().text(
                        rect.center() - vec2(0.0, 10.0),
                        Align2::CENTER_CENTER,
                        "No samples yet",
                        FontId::proportional(13.0),
                        Color32::from_gray(190),
                    );
                    ui.painter().text(
                        rect.center() + vec2(0.0, 10.0),
                        Align2::CENTER_CENTER,
                        format!("Press {s} anywhere"),
                        FontId::proportional(11.5),
                        theme::MUTED,
                    );
                }
                let now = chrono::Local::now();
                let mut group = String::new();
                let mut k = 0;
                let n = self.d.history.len();
                while k < n {
                    let label = time_label(self.d.history_at.get(k).copied().unwrap_or(0), now);
                    let mut end = k;
                    while end < n && time_label(self.d.history_at.get(end).copied().unwrap_or(0), now) == label {
                        end += 1;
                    }
                    if label != group {
                        group = label.clone();
                        ui.add_space(if k == 0 { 2.0 } else { 8.0 });
                        let (hr, _) = ui.allocate_exact_size(vec2(ui.available_width(), 16.0), Sense::hover());
                        let galley = ui.painter().layout_no_wrap(label, FontId::proportional(11.0), theme::MUTED);
                        let tw = galley.size().x;
                        let y = hr.center().y;
                        ui.painter().galley(pos2(hr.min.x, y - galley.size().y / 2.0), galley, theme::MUTED);
                        ui.painter().line_segment(
                            [pos2(hr.min.x + tw + 8.0, y), pos2(hr.max.x, y)],
                            Stroke::new(1.0, theme::LINE),
                        );
                        ui.add_space(2.0);
                    }
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
                        for c in self.d.history[k..end].to_vec() {
                            if let Some(a) = swatch(ui, c, vec2(34.0, 34.0), c == sel, None, true) {
                                act = Some(a);
                            }
                        }
                    });
                    k = end;
                }
            });

        ui.add_space(4.0);
        theme::section(ui, "Selected");
        let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 64.0), Sense::click());
        ui.painter().rect_filled(rect, 10.0, c32(sel));
        ui.painter().rect_stroke(rect, 10.0, Stroke::new(1.0, Color32::from_white_alpha(28)), StrokeKind::Inside);
        ui.painter().text(
            rect.center(),
            Align2::CENTER_CENTER,
            self.d.settings.fmt_hex(sel),
            FontId::monospace(17.0),
            c32(color::text_on(sel)),
        );
        if resp.on_hover_text("Click to copy").clicked() {
            act = Some(SwAct::Copy(sel));
        }
        ui.horizontal(|ui| {
            let w = (ui.available_width() - ui.spacing().item_spacing.x) / 2.0;
            if ui.add_sized([w, 28.0], egui::Button::new("+ Palette")).clicked() {
                act = Some(SwAct::Add(sel));
            }
            if ui.add_sized([w, 28.0], egui::Button::new("Explore")).clicked() {
                self.tab = Tab::Explore;
            }
        });
        let ctx = ui.ctx().clone();
        if let Some(a) = act {
            self.apply_swatch_action(&ctx, a);
        }
    }

    // ---------------------------------------------------------------- generator

    fn generator_bar(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        // The "or press Space" hint only fits in a wide window.
        let wide = ui.available_width() > 1120.0;
        ui.horizontal(|ui| {
            let r = ui.add(
                egui::TextEdit::singleline(&mut self.d.working_name)
                    .hint_text("Palette name")
                    .margin(Margin::symmetric(10, 6))
                    .desired_width(170.0),
            );
            if r.changed() {
                self.touch();
            }
            if ui.button("Save").on_hover_text("Save to Library").clicked() {
                let colors: Vec<Rgb> = self.d.working.iter().map(|s| s.rgb).collect();
                let name = self.d.working_name.trim().to_string();
                if let Some(p) = self.d.saved.iter_mut().find(|p| p.name == name) {
                    p.colors = colors;
                } else {
                    self.d.saved.insert(0, SavedPalette { name, colors });
                }
                self.toast("Saved to Library");
                self.touch();
            }
            ui.menu_button("Export", |ui| {
                let cols: Vec<Rgb> = self.d.working.iter().map(|s| s.rgb).collect();
                if ui.button("Copy hex list").clicked() {
                    ctx.copy_text(cols.iter().map(|c| self.d.settings.fmt_hex(*c)).collect::<Vec<_>>().join(", "));
                    self.toast("Copied hex list");
                    ui.close();
                }
                if ui.button("Copy CSS variables").clicked() {
                    ctx.copy_text(css_vars(&cols));
                    self.toast("Copied CSS variables");
                    ui.close();
                }
                if ui.button("Copy CSS gradient").clicked() {
                    ctx.copy_text(css_gradient(&cols));
                    self.toast("Copied CSS gradient");
                    ui.close();
                }
                if ui.button("Copy JSON").clicked() {
                    ctx.copy_text(serde_json::to_string(&cols.iter().map(|c| color::to_hex_hash(*c)).collect::<Vec<_>>()).unwrap_or_default());
                    self.toast("Copied JSON");
                    ui.close();
                }
            });
            ui.add_space(8.0);
            Frame::new()
                .fill(theme::PANEL)
                .stroke(Stroke::new(1.0, theme::LINE))
                .corner_radius(9)
                .inner_margin(Margin::same(2))
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    for (v, label) in crate::store::View::ALL {
                        let on = self.d.view == v;
                        let (rect, resp) = ui.allocate_exact_size(vec2(34.0, 26.0), Sense::click());
                        let fill = if on {
                            theme::ACCENT
                        } else if resp.hovered() {
                            Color32::from_white_alpha(16)
                        } else {
                            Color32::TRANSPARENT
                        };
                        ui.painter().rect_filled(rect, 7.0, fill);
                        view_glyph(ui, rect.center(), v, if on { Color32::WHITE } else { Color32::from_gray(170) });
                        if resp.on_hover_text(label).clicked() {
                            self.d.view = v;
                            self.shades_col = None;
                            self.touch();
                        }
                    }
                });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if wide {
                    theme::kbd(ui, "Space");
                    ui.label(RichText::new("or press").color(theme::MUTED).size(12.0));
                    ui.add_space(6.0);
                }
                if ui.add(theme::primary("Generate").min_size(vec2(104.0, 32.0))).clicked() {
                    self.generate();
                }
                ui.add_space(12.0);
                if ui.add(theme::ghost("+ Add color")).clicked() {
                    let c = color::random_color();
                    self.add_color(c);
                }
            });
        });
    }

    // ---------------------------------------------------------------- explore

    fn explore(&mut self, ui: &mut Ui) {
        let base = self.d.selected;
        let ctx = ui.ctx().clone();
        let mut act: Option<SwAct> = None;
        let mut use_palette: Option<Vec<Rgb>> = None;

        {
            // Left: base color
            Panel::left("explore_base")
                .exact_size(300.0)
                .resizable(false)
                .frame(Frame::new().fill(theme::BG).inner_margin(Margin::same(18)).stroke(Stroke::new(1.0, theme::LINE)))
                .show(ui, |ui| {
                ui.label(RichText::new("Base color").strong().size(15.0));
                ui.spacing_mut().slider_width = 232.0;
                let mut col = c32(base);
                if egui::color_picker::color_picker_color32(ui, &mut col, egui::color_picker::Alpha::Opaque) {
                    let n = [col.r(), col.g(), col.b()];
                    if n != base {
                        self.select(n);
                    }
                }
                ui.horizontal(|ui| {
                    let r = ui.add(
                        egui::TextEdit::singleline(&mut self.selected_hex)
                            .font(FontId::monospace(14.0))
                            .desired_width(100.0),
                    );
                    if r.changed() {
                        if let Some(c) = color::parse_hex(&self.selected_hex) {
                            self.d.selected = c;
                            self.touch();
                        }
                    }
                    if r.lost_focus() {
                        self.selected_hex = color::to_hex_hash(self.d.selected);
                    }
                    if ui.button("Copy").clicked() {
                        act = Some(SwAct::Copy(self.d.selected));
                    }
                    if ui.button("+ Palette").clicked() {
                        act = Some(SwAct::Add(self.d.selected));
                    }
                });
                let b = self.d.selected;
                let (h, s, l) = color::to_hsl(b);
                let (_, sv, v) = color::to_hsv(b);
                let (ol, oc, oh) = color::to_oklch(b);
                ui.add_space(4.0);
                egui::Grid::new("vals").num_columns(2).spacing([14.0, 4.0]).show(ui, |ui| {
                    let mut row = |k: &str, v: String| {
                        ui.label(RichText::new(k).color(theme::MUTED).size(12.0));
                        ui.label(RichText::new(v).monospace().size(12.0));
                        ui.end_row();
                    };
                    row("RGB", format!("{}, {}, {}", b[0], b[1], b[2]));
                    row("HSL", format!("{:.0}°, {:.0}%, {:.0}%", h, s * 100.0, l * 100.0));
                    row("HSV", format!("{:.0}°, {:.0}%, {:.0}%", h, sv * 100.0, v * 100.0));
                    row("OKLCH", format!("{:.2} {:.3} {:.0}°", ol, oc, oh));
                });
                ui.add_space(6.0);
                ui.label(RichText::new("Contrast").color(theme::MUTED).size(12.0));
                ui.horizontal(|ui| {
                    for (fg, name) in [([255u8, 255, 255], "White"), ([0, 0, 0], "Black")] {
                        let ratio = color::contrast_ratio(b, fg);
                        let (rect, _) = ui.allocate_exact_size(vec2(118.0, 54.0), Sense::hover());
                        ui.painter().rect_filled(rect, 8.0, c32(b));
                        ui.painter().text(rect.center() - vec2(0.0, 8.0), Align2::CENTER_CENTER, "Aa", FontId::proportional(20.0), c32(fg));
                        let grade = if ratio >= 7.0 { "AAA" } else if ratio >= 4.5 { "AA" } else if ratio >= 3.0 { "AA Large" } else { "Fail" };
                        ui.painter().text(
                            rect.center() + vec2(0.0, 14.0),
                            Align2::CENTER_CENTER,
                            format!("{name} {ratio:.1}  {grade}"),
                            FontId::proportional(10.5),
                            c32(fg),
                        );
                    }
                });
            });

            // Right: harmonies + shade viewer
            Frame::new().inner_margin(Margin::symmetric(22, 0)).show(ui, |ui| {
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                ui.add_space(16.0);
                ui.label(RichText::new("Harmonies").strong().size(15.0));
                for h in color::harmonies(base) {
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(h.name).strong());
                        ui.label(RichText::new(h.blurb).color(theme::MUTED).size(12.0));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.add_space(16.0);
                            if ui.small_button("Use as palette").clicked() {
                                use_palette = Some(h.colors.clone());
                            }
                        });
                    });
                    ui.horizontal(|ui| {
                        ui.set_width(ui.available_width());
                        if let Some(a) = strip(ui, &h.colors, 60.0, Some(base), true) {
                            act = Some(a);
                        }
                    });
                }

                ui.add_space(18.0);
                ui.label(RichText::new("Shade viewer").strong().size(15.0));
                let (ladder, _) = color::shade_ladder(base, 21);
                for (name, colors, labels) in [
                    ("Lightness ladder", ladder, false),
                    ("Tints (toward white)", color::tints(base, 11), true),
                    ("Shades (toward black)", color::shades(base, 11), true),
                    ("Tones (toward gray)", color::tones(base, 11), true),
                ] {
                    ui.add_space(6.0);
                    ui.label(RichText::new(name).color(theme::MUTED).size(12.0));
                    ui.horizontal(|ui| {
                        ui.set_width(ui.available_width());
                        if let Some(a) = strip(ui, &colors, 52.0, Some(base), labels) {
                            act = Some(a);
                        }
                    });
                }
                ui.add_space(24.0);
            });
            });
        }

        if let Some(cols) = use_palette {
            self.snapshot();
            self.d.working = cols.into_iter().map(|rgb| Swatch { rgb, locked: false }).collect();
            self.tab = Tab::Generator;
            self.touch();
        }
        if let Some(a) = act {
            self.apply_swatch_action(&ctx, a);
        }
    }

    // ---------------------------------------------------------------- library

    fn library(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        let mut open: Option<usize> = None;
        let mut delete: Option<usize> = None;
        let mut dup: Option<usize> = None;
        let mut act = None;
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.add_space(14.0);
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                ui.label(RichText::new("Saved palettes").strong().size(15.0));
            });
            if self.d.saved.is_empty() {
                ui.horizontal(|ui| {
                    ui.add_space(16.0);
                    ui.label(
                        RichText::new("Nothing saved yet. Build a palette in the Generator and press Save.")
                            .color(theme::MUTED),
                    );
                });
            }
            for (i, p) in self.d.saved.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.add_space(16.0);
                    Frame::new()
                        .fill(theme::PANEL)
                        .corner_radius(10)
                        .inner_margin(Margin::same(12))
                        .show(ui, |ui| {
                          // The frame inherits the row's horizontal layout; stack its contents instead.
                          ui.vertical(|ui| {
                            ui.set_width((ui.available_width() - 16.0).max(10.0));
                            ui.horizontal(|ui| {
                                ui.add(egui::TextEdit::singleline(&mut p.name).desired_width(220.0));
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    if ui.button("Delete").clicked() {
                                        delete = Some(i);
                                    }
                                    if ui.button("Duplicate").clicked() {
                                        dup = Some(i);
                                    }
                                    if ui.button("Copy CSS").clicked() {
                                        ctx.copy_text(css_vars(&p.colors));
                                    }
                                    if ui
                                        .add(egui::Button::new("Open in Generator").fill(Color32::from_rgb(61, 126, 245)))
                                        .clicked()
                                    {
                                        open = Some(i);
                                    }
                                });
                            });
                            if let Some(a) = strip(ui, &p.colors, 62.0, None, true) {
                                act = Some(a);
                            }
                          });
                        });
                });
            }
        });
        if let Some(i) = open {
            self.snapshot();
            self.d.working = self.d.saved[i].colors.iter().map(|rgb| Swatch { rgb: *rgb, locked: false }).collect();
            self.d.working_name = self.d.saved[i].name.clone();
            self.tab = Tab::Generator;
        }
        if let Some(i) = dup {
            let mut p = self.d.saved[i].clone();
            p.name.push_str(" copy");
            self.d.saved.insert(i + 1, p);
        }
        if let Some(i) = delete {
            self.d.saved.remove(i);
        }
        if let Some(a) = act {
            self.apply_swatch_action(&ctx, a);
        }
        self.touch();
    }

    // ---------------------------------------------------------------- settings

    fn record_hotkey(&mut self, ui: &mut Ui) {
        let Some(slot) = self.recording else { return };
        let events = ui.input(|i| i.events.clone());
        for e in events {
            if let egui::Event::Key { key, pressed: true, modifiers, .. } = e {
                if key == Key::Escape {
                    self.recording = None;
                } else if let Some(s) = hotkeys::from_egui(key, modifiers) {
                    match slot {
                        Slot::Sample => self.d.settings.sample_hotkey = s,
                        Slot::Picker => self.d.settings.picker_hotkey = s,
                    }
                    self.recording = None;
                    self.apply_hotkeys();
                }
            }
        }
    }

    fn settings(&mut self, ui: &mut Ui) {
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            Frame::new().inner_margin(Margin::same(20)).show(ui, |ui| {
                ui.set_max_width(640.0);
                ui.label(RichText::new("Hotkeys").strong().size(15.0));
                ui.label(
                    RichText::new("Click a shortcut, then press the new combination (at least one modifier). Esc cancels.")
                        .color(theme::MUTED)
                        .size(12.0),
                );
                ui.add_space(4.0);
                for (slot, title, desc) in [
                    (Slot::Sample, "Sample color", "Opens the zoom loupe at the cursor. Press again (or Return) to take the color under it."),
                    (Slot::Picker, "Quick picker", "Toggles the floating color picker window."),
                ] {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.set_width(380.0);
                            ui.label(RichText::new(title).strong());
                            ui.label(RichText::new(desc).color(theme::MUTED).size(12.0));
                        });
                        let rec = self.recording == Some(slot);
                        let cur = match slot {
                            Slot::Sample => &self.d.settings.sample_hotkey,
                            Slot::Picker => &self.d.settings.picker_hotkey,
                        };
                        let text = if rec { "Press shortcut…".to_string() } else { hotkeys::pretty(cur) };
                        let b = egui::Button::new(RichText::new(text).monospace().size(15.0))
                            .min_size(vec2(150.0, 32.0))
                            .stroke(if rec { Stroke::new(1.5, Color32::from_rgb(61, 126, 245)) } else { Stroke::NONE });
                        if ui.add(b).clicked() {
                            self.recording = if rec { None } else { Some(slot) };
                        }
                    });
                    ui.add_space(4.0);
                }
                for e in &self.hk_errors {
                    ui.colored_label(Color32::from_rgb(240, 140, 90), e);
                }
                ui.horizontal(|ui| {
                    if ui.button("Reset to defaults").clicked() {
                        let d = crate::store::Settings::default();
                        self.d.settings.sample_hotkey = d.sample_hotkey;
                        self.d.settings.picker_hotkey = d.picker_hotkey;
                        self.apply_hotkeys();
                    }
                });

                ui.add_space(18.0);
                ui.label(RichText::new("Loupe").strong().size(15.0));
                let mut k = (self.d.settings.loupe_cells.max(9) - 1) / 2;
                ui.horizontal(|ui| {
                    ui.label("Zoom area");
                    if ui
                        .add(egui::Slider::new(&mut k, 4..=20).custom_formatter(|v, _| format!("{0}×{0} px", v as usize * 2 + 1)))
                        .changed()
                    {
                        self.d.settings.loupe_cells = k * 2 + 1;
                        self.touch();
                    }
                });
                ui.label(RichText::new("Smaller area = stronger magnification.").color(theme::MUTED).size(12.0));
                let mut changed = false;
                changed |= ui.checkbox(&mut self.d.settings.copy_on_sample, "Copy the hex to the clipboard when sampling").changed();
                changed |= ui.checkbox(&mut self.d.settings.close_after_sample, "Close the loupe after each sample").changed();

                ui.add_space(14.0);
                ui.label(RichText::new("Format").strong().size(15.0));
                changed |= ui.checkbox(&mut self.d.settings.hash_prefix, "Include # prefix").changed();
                changed |= ui.checkbox(&mut self.d.settings.uppercase_hex, "Uppercase hex").changed();
                if changed {
                    self.touch();
                }

                if capture::NEEDS_PERMISSION || !self.screen_ok {
                    ui.add_space(18.0);
                    ui.label(RichText::new("Permissions").strong().size(15.0));
                    ui.horizontal(|ui| {
                        let (txt, col): (String, Color32) = if self.screen_ok {
                            (format!("{}: granted", capture::PERMISSION_NAME), Color32::from_rgb(110, 200, 130))
                        } else {
                            (format!("{}: not granted", capture::PERMISSION_NAME), Color32::from_rgb(240, 140, 90))
                        };
                        ui.colored_label(col, txt);
                        if capture::HAS_SETTINGS_PANE && !self.screen_ok && ui.button("Request").clicked() {
                            capture::request_screen_access();
                        }
                        if capture::HAS_SETTINGS_PANE && ui.button("Open System Settings").clicked() {
                            capture::open_screen_recording_settings();
                        }
                    });
                    ui.label(
                        RichText::new(capture::PERMISSION_DETAIL)
                            .color(theme::MUTED)
                            .size(12.0),
                    );
                }
            });
        });
        let _ = StrokeKind::Inside;
    }
}

fn css_vars(cols: &[Rgb]) -> String {
    let mut s = String::from(":root {\n");
    for (i, c) in cols.iter().enumerate() {
        s.push_str(&format!("  --color-{}: {};\n", i + 1, color::to_hex_hash(*c)));
    }
    s.push('}');
    s
}

fn css_gradient(cols: &[Rgb]) -> String {
    let n = cols.len().max(2) - 1;
    let stops: Vec<String> = cols
        .iter()
        .enumerate()
        .map(|(i, c)| format!("{} {}%", color::to_hex_hash(*c), (i * 100) / n))
        .collect();
    format!("linear-gradient(90deg, {})", stops.join(", "))
}

/// "9am" for today (one group per hour), "Yesterday", a weekday for the past week, else a date.
fn time_label(ts: i64, now: chrono::DateTime<chrono::Local>) -> String {
    use chrono::{Datelike, TimeZone};
    if ts <= 0 {
        return "Earlier".into();
    }
    let Some(t) = chrono::Local.timestamp_opt(ts, 0).single() else { return "Earlier".into() };
    let days = (now.date_naive() - t.date_naive()).num_days();
    match days {
        d if d <= 0 => t.format("%-I%P").to_string(),
        1 => "Yesterday".into(),
        2..=6 => t.format("%A").to_string(),
        _ if t.year() == now.year() => t.format("%b %-d").to_string(),
        _ => t.format("%b %-d, %Y").to_string(),
    }
}

/// Tiny icon for a palette layout, drawn centered on `c`.
fn view_glyph(ui: &Ui, c: egui::Pos2, v: crate::store::View, col: Color32) {
    use crate::store::View;
    let p = ui.painter();
    let r = |x: f32, y: f32, w: f32, h: f32| Rect::from_min_size(pos2(c.x + x, c.y + y), vec2(w, h));
    match v {
        View::Columns => {
            for k in 0..3 {
                p.rect_filled(r(-8.0 + k as f32 * 6.0, -6.0, 4.0, 12.0), 1.2, col);
            }
        }
        View::Rows => {
            for k in 0..3 {
                p.rect_filled(r(-8.0, -6.0 + k as f32 * 5.0, 16.0, 3.0), 1.2, col);
            }
        }
        View::Gradient => {
            for k in 0..8 {
                p.rect_filled(r(-8.0 + k as f32 * 2.0, -5.0, 2.0, 10.0), 0.0, col.gamma_multiply(0.2 + 0.8 * k as f32 / 7.0));
            }
        }
        View::Grid => {
            for (x, y) in [(-7.0, -7.0), (0.5, -7.0), (-7.0, 0.5), (0.5, 0.5)] {
                p.rect_filled(r(x, y, 6.5, 6.5), 1.5, col);
            }
        }
    }
}
