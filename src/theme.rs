use egui::{Button, Color32, Context, CornerRadius, FontFamily, FontId, Frame, Margin, RichText, Stroke, TextStyle, Ui, Visuals};

pub const BG: Color32 = Color32::from_rgb(17, 17, 20);
pub const PANEL: Color32 = Color32::from_rgb(25, 25, 29);
pub const RAISED: Color32 = Color32::from_rgb(38, 38, 44);
pub const LINE: Color32 = Color32::from_rgb(46, 46, 53);
pub const MUTED: Color32 = Color32::from_rgb(132, 132, 146);
pub const TEXT: Color32 = Color32::from_rgb(230, 230, 236);
pub const ACCENT: Color32 = Color32::from_rgb(76, 130, 247);
pub const AMBER: Color32 = Color32::from_rgb(244, 190, 100);

pub fn apply(ctx: &Context) {
    // Child windows would otherwise follow the system theme (light), leaving dark text on dark panels.
    ctx.set_theme(egui::ThemePreference::Dark);

    let mut v = Visuals::dark();
    v.panel_fill = PANEL;
    v.window_fill = PANEL;
    v.extreme_bg_color = Color32::from_rgb(12, 12, 14);
    v.faint_bg_color = RAISED;
    v.override_text_color = Some(TEXT);
    v.window_stroke = Stroke::new(1.0, LINE);
    v.window_corner_radius = CornerRadius::same(12);
    v.menu_corner_radius = CornerRadius::same(10);
    v.selection.bg_fill = ACCENT.gamma_multiply(0.55);
    v.selection.stroke = Stroke::new(1.0, Color32::WHITE);

    let r = CornerRadius::same(8);
    let w = &mut v.widgets;
    w.noninteractive.bg_stroke = Stroke::new(1.0, LINE);
    w.noninteractive.fg_stroke = Stroke::new(1.0, TEXT);
    for (s, fill, stroke) in [
        (&mut w.inactive, RAISED, Color32::TRANSPARENT),
        (&mut w.hovered, Color32::from_rgb(52, 52, 60), Color32::from_rgb(70, 70, 80)),
        (&mut w.active, Color32::from_rgb(64, 64, 74), Color32::from_rgb(90, 90, 104)),
    ] {
        s.corner_radius = r;
        s.weak_bg_fill = fill;
        s.bg_fill = fill;
        s.bg_stroke = Stroke::new(1.0, stroke);
        s.fg_stroke = Stroke::new(1.0, TEXT);
    }
    w.noninteractive.corner_radius = r;
    ctx.set_visuals_of(egui::Theme::Dark, v);

    ctx.all_styles_mut(|s| {
        s.spacing.item_spacing = egui::vec2(8.0, 8.0);
        s.spacing.button_padding = egui::vec2(12.0, 6.0);
        s.spacing.interact_size.y = 28.0;
        s.text_styles.insert(TextStyle::Body, FontId::new(13.5, FontFamily::Proportional));
        s.text_styles.insert(TextStyle::Button, FontId::new(13.0, FontFamily::Proportional));
        s.text_styles.insert(TextStyle::Small, FontId::new(11.0, FontFamily::Proportional));
        s.text_styles.insert(TextStyle::Heading, FontId::new(17.0, FontFamily::Proportional));
    });
}

/// Small-caps style section label.
pub fn section(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text.to_uppercase()).size(10.5).strong().color(MUTED));
}

pub fn primary(text: impl Into<String>) -> Button<'static> {
    Button::new(RichText::new(text.into()).strong().color(Color32::WHITE))
        .fill(ACCENT)
        .min_size(egui::vec2(0.0, 30.0))
}

/// Borderless button for secondary actions.
pub fn ghost(text: impl Into<String>) -> Button<'static> {
    Button::new(RichText::new(text.into()).color(Color32::from_gray(200)))
        .fill(Color32::TRANSPARENT)
        .stroke(Stroke::NONE)
}

/// Keyboard-key chip, e.g. "Space".
pub fn kbd(ui: &mut Ui, text: &str) {
    Frame::new()
        .fill(RAISED)
        .stroke(Stroke::new(1.0, LINE))
        .corner_radius(5)
        .inner_margin(Margin::symmetric(6, 1))
        .show(ui, |ui| {
            ui.label(RichText::new(text).monospace().size(11.0).color(Color32::from_gray(190)));
        });
}
