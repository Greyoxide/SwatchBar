//! Painted mockups that show a five-role color scheme on real-looking designs.
//! Everything is drawn in a virtual 1000 x 620 space and scaled to fit.

use crate::color::{self, Rgb};
use crate::widgets::c32;
use egui::{pos2, vec2, Align2, Color32, CornerRadius, FontId, Painter, Rect, Shape, Stroke, StrokeKind};

pub const W: f32 = 1000.0;
pub const H: f32 = 620.0;

/// The five roles a scheme assigns.
#[derive(Clone, Copy)]
pub struct Pal {
    pub bg: Rgb,
    pub surface: Rgb,
    pub text: Rgb,
    pub primary: Rgb,
    pub accent: Rgb,
}

impl Pal {
    pub fn new(r: [Rgb; 5]) -> Self {
        Self { bg: r[0], surface: r[1], text: r[2], primary: r[3], accent: r[4] }
    }
}

fn mx(a: Rgb, b: Rgb, t: f32) -> Color32 {
    c32(color::mix(a, b, t))
}

fn on(c: Rgb) -> Color32 {
    c32(color::text_on(c))
}

const WHITE: Rgb = [255, 255, 255];

pub struct Cv<'a> {
    pub p: &'a Painter,
    pub o: egui::Pos2,
    pub k: f32,
}

impl Cv<'_> {
    fn pt(&self, x: f32, y: f32) -> egui::Pos2 {
        pos2(self.o.x + x * self.k, self.o.y + y * self.k)
    }
    fn r(&self, x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect::from_min_size(self.pt(x, y), vec2(w * self.k, h * self.k))
    }
    fn fill(&self, x: f32, y: f32, w: f32, h: f32, rad: f32, c: Color32) {
        self.p.rect_filled(self.r(x, y, w, h), rad * self.k, c);
    }
    fn fill_arch(&self, x: f32, y: f32, w: f32, h: f32, top: f32, c: Color32) {
        let t = (top * self.k).min(255.0) as u8;
        self.p.rect_filled(self.r(x, y, w, h), CornerRadius { nw: t, ne: t, sw: 0, se: 0 }, c);
    }
    fn outline(&self, x: f32, y: f32, w: f32, h: f32, rad: f32, width: f32, c: Color32) {
        self.p.rect_stroke(self.r(x, y, w, h), rad * self.k, Stroke::new(width * self.k, c), StrokeKind::Inside);
    }
    fn circle(&self, x: f32, y: f32, r: f32, c: Color32) {
        self.p.circle_filled(self.pt(x, y), r * self.k, c);
    }
    fn ring(&self, x: f32, y: f32, r: f32, w: f32, c: Color32) {
        self.p.circle_stroke(self.pt(x, y), r * self.k, Stroke::new(w * self.k, c));
    }
    fn ellipse(&self, x: f32, y: f32, rx: f32, ry: f32, c: Color32) {
        self.p.add(Shape::ellipse_filled(self.pt(x, y), vec2(rx * self.k, ry * self.k), c));
    }
    fn poly(&self, pts: &[(f32, f32)], c: Color32) {
        self.p.add(Shape::convex_polygon(pts.iter().map(|&(x, y)| self.pt(x, y)).collect(), c, Stroke::NONE));
    }
    fn line(&self, a: (f32, f32), b: (f32, f32), w: f32, c: Color32) {
        self.p.line_segment([self.pt(a.0, a.1), self.pt(b.0, b.1)], Stroke::new(w * self.k, c));
    }
    fn text(&self, x: f32, y: f32, s: &str, size: f32, c: Color32, a: Align2) {
        self.p.text(self.pt(x, y), a, s, FontId::proportional(size * self.k), c);
    }
    /// Faux-bold: the same text twice, a hair apart.
    fn bold(&self, x: f32, y: f32, s: &str, size: f32, c: Color32, a: Align2) {
        self.text(x, y, s, size, c, a);
        self.text(x + size * 0.025, y, s, size, c, a);
    }
    fn shadow(&self, x: f32, y: f32, w: f32, h: f32, rad: f32, depth: f32) {
        for (g, a) in [(18.0, 10u8), (11.0, 16), (5.0, 24)] {
            let g = g * depth;
            self.p.rect_filled(self.r(x - g, y - g + depth * 14.0, w + 2.0 * g, h + 2.0 * g), (rad + g) * self.k, Color32::from_black_alpha(a));
        }
    }
    /// A grey-ish bar standing in for a line of text.
    fn skel(&self, x: f32, y: f32, w: f32, h: f32, c: Color32) {
        self.fill(x, y, w, h, h / 2.0, c);
    }
}

pub fn draw(scene: usize, cv: &Cv, pal: &Pal) {
    match scene {
        0 => landing(cv, pal),
        1 => shop(cv, pal),
        2 => app(cv, pal),
        _ => stationery(cv, pal),
    }
}

// ------------------------------------------------------------------ landing page

fn landing(cv: &Cv, p: &Pal) {
    let (bg, sf, tx, pr, ac) = (c32(p.bg), c32(p.surface), c32(p.text), c32(p.primary), c32(p.accent));
    let muted = mx(p.text, p.bg, 0.42);
    let line = mx(p.bg, p.text, 0.12);
    cv.fill(0.0, 0.0, W, H, 0.0, bg);

    // Browser chrome
    cv.fill(0.0, 0.0, W, 40.0, 0.0, mx(p.bg, p.text, 0.07));
    for (i, c) in [[255, 95, 86], [255, 189, 46], [39, 201, 63]].into_iter().enumerate() {
        cv.circle(22.0 + i as f32 * 18.0, 20.0, 5.0, c32(c));
    }
    cv.fill(120.0, 9.0, 420.0, 22.0, 11.0, bg);
    cv.text(138.0, 20.0, "northwind.app", 11.0, muted, Align2::LEFT_CENTER);

    // Nav
    cv.circle(52.0, 80.0, 13.0, pr);
    cv.circle(52.0, 80.0, 5.0, bg);
    cv.bold(76.0, 80.0, "Northwind", 19.0, tx, Align2::LEFT_CENTER);
    for (i, l) in ["Product", "Pricing", "Customers", "Docs"].into_iter().enumerate() {
        cv.text(560.0 + i as f32 * 92.0, 80.0, l, 14.0, muted, Align2::LEFT_CENTER);
    }
    cv.fill(868.0, 62.0, 104.0, 36.0, 18.0, pr);
    cv.bold(920.0, 80.0, "Get started", 13.0, on(p.primary), Align2::CENTER_CENTER);
    cv.line((0.0, 118.0), (W, 118.0), 1.0, line);

    // Hero copy
    cv.fill(60.0, 158.0, 200.0, 28.0, 14.0, mx(p.accent, p.bg, 0.82));
    cv.circle(78.0, 172.0, 4.0, ac);
    cv.text(92.0, 172.0, "New: smart summaries", 12.0, mx(p.accent, p.text, 0.45), Align2::LEFT_CENTER);
    cv.bold(60.0, 232.0, "Ship colors", 56.0, tx, Align2::LEFT_CENTER);
    cv.bold(60.0, 298.0, "your team", 56.0, tx, Align2::LEFT_CENTER);
    cv.bold(60.0, 364.0, "will love.", 56.0, pr, Align2::LEFT_CENTER);
    cv.text(60.0, 430.0, "One calm place to plan, review and ship", 16.0, muted, Align2::LEFT_CENTER);
    cv.text(60.0, 454.0, "every design decision, together.", 16.0, muted, Align2::LEFT_CENTER);
    cv.fill(60.0, 490.0, 176.0, 52.0, 26.0, pr);
    cv.bold(148.0, 516.0, "Start free trial", 15.0, on(p.primary), Align2::CENTER_CENTER);
    cv.outline(252.0, 490.0, 156.0, 52.0, 26.0, 1.6, mx(p.text, p.bg, 0.3));
    cv.bold(330.0, 516.0, "Watch demo  ▶", 14.0, tx, Align2::CENTER_CENTER);
    cv.text(60.0, 582.0, "★★★★★", 14.0, ac, Align2::LEFT_CENTER);
    cv.text(136.0, 582.0, "Loved by 12,000+ teams", 13.0, muted, Align2::LEFT_CENTER);

    // Hero card
    cv.shadow(540.0, 150.0, 410.0, 340.0, 20.0, 1.0);
    cv.fill(540.0, 150.0, 410.0, 340.0, 20.0, sf);
    cv.outline(540.0, 150.0, 410.0, 340.0, 20.0, 1.0, line);
    cv.bold(566.0, 182.0, "Overview", 15.0, tx, Align2::LEFT_CENTER);
    cv.fill(838.0, 170.0, 88.0, 24.0, 12.0, bg);
    cv.text(882.0, 182.0, "This week", 11.0, muted, Align2::CENTER_CENTER);
    let tiles = [(p.primary, "$48.2k", "Revenue"), (p.accent, "2,481", "Active users"), (p.bg, "94%", "Satisfaction")];
    for (i, (c, big, small)) in tiles.into_iter().enumerate() {
        let x = 566.0 + i as f32 * 122.0;
        cv.fill(x, 212.0, 112.0, 84.0, 14.0, c32(c));
        if c == p.bg {
            cv.outline(x, 212.0, 112.0, 84.0, 14.0, 1.0, line);
        }
        cv.bold(x + 14.0, 242.0, big, 21.0, on(c), Align2::LEFT_CENTER);
        cv.text(x + 14.0, 272.0, small, 11.5, on(c).gamma_multiply(0.75), Align2::LEFT_CENTER);
    }
    let heights = [48.0, 72.0, 58.0, 96.0, 80.0, 118.0, 92.0, 110.0, 138.0];
    for (i, h) in heights.into_iter().enumerate() {
        let hot = i == heights.len() - 1;
        cv.fill(570.0 + i as f32 * 40.0, 462.0 - h, 26.0, h, 7.0, if hot { ac } else { mx(p.primary, p.surface, 0.55 - i as f32 * 0.04) });
    }
    cv.line((566.0, 464.0), (926.0, 464.0), 1.0, line);

    // Floating toast
    cv.shadow(796.0, 118.0, 190.0, 58.0, 14.0, 0.8);
    cv.fill(796.0, 118.0, 190.0, 58.0, 14.0, bg);
    cv.outline(796.0, 118.0, 190.0, 58.0, 14.0, 1.0, line);
    cv.circle(826.0, 147.0, 14.0, ac);
    cv.text(826.0, 147.0, "✓", 14.0, on(p.accent), Align2::CENTER_CENTER);
    cv.bold(850.0, 139.0, "Palette saved", 12.5, tx, Align2::LEFT_CENTER);
    cv.text(850.0, 157.0, "just now", 10.5, muted, Align2::LEFT_CENTER);
}

// ------------------------------------------------------------------ editorial shop

fn shop(cv: &Cv, p: &Pal) {
    let (bg, sf, tx, pr, ac) = (c32(p.bg), c32(p.surface), c32(p.text), c32(p.primary), c32(p.accent));
    let muted = mx(p.text, p.bg, 0.42);
    let line = mx(p.bg, p.text, 0.14);
    cv.fill(0.0, 0.0, W, H, 0.0, bg);

    for (i, l) in ["Shop", "Journal", "About"].into_iter().enumerate() {
        cv.text(48.0 + i as f32 * 78.0, 38.0, l, 13.0, tx, Align2::LEFT_CENTER);
    }
    cv.bold(500.0, 38.0, "A T E L I E R", 22.0, tx, Align2::CENTER_CENTER);
    cv.text(952.0, 38.0, "Cart (2)", 13.0, tx, Align2::RIGHT_CENTER);
    cv.line((890.0, 50.0), (952.0, 50.0), 1.2, tx);
    cv.line((0.0, 74.0), (W, 74.0), 1.0, line);

    // Left: copy
    cv.fill(0.0, 74.0, 500.0, H - 74.0, 0.0, sf);
    cv.bold(60.0, 172.0, "NEW COLLECTION  ·  AUTUMN", 12.0, mx(p.accent, p.text, 0.2), Align2::LEFT_CENTER);
    cv.bold(60.0, 232.0, "Made slowly,", 54.0, tx, Align2::LEFT_CENTER);
    cv.bold(60.0, 294.0, "worn often.", 54.0, tx, Align2::LEFT_CENTER);
    cv.text(60.0, 358.0, "Small-batch pieces, dyed by hand and", 15.5, muted, Align2::LEFT_CENTER);
    cv.text(60.0, 382.0, "built to outlast the season.", 15.5, muted, Align2::LEFT_CENTER);
    cv.fill(60.0, 424.0, 200.0, 54.0, 0.0, pr);
    cv.bold(160.0, 451.0, "Shop the edit  →", 14.5, on(p.primary), Align2::CENTER_CENTER);
    cv.text(290.0, 451.0, "Our story", 14.0, tx, Align2::LEFT_CENTER);
    cv.line((290.0, 463.0), (360.0, 463.0), 1.2, tx);
    cv.line((60.0, 520.0), (440.0, 520.0), 1.0, line);
    for (i, (n, l)) in [("120+", "Artisans"), ("34", "Countries"), ("100%", "Natural dyes")].into_iter().enumerate() {
        let x = 60.0 + i as f32 * 130.0;
        cv.bold(x, 552.0, n, 26.0, pr, Align2::LEFT_CENTER);
        cv.text(x, 580.0, l, 12.0, muted, Align2::LEFT_CENTER);
    }

    // Right: abstract still life
    cv.fill(500.0, 74.0, 500.0, H - 74.0, 0.0, pr);
    cv.fill_arch(590.0, 150.0, 320.0, 420.0, 160.0, bg);
    let arch = cv.r(590.0, 150.0, 320.0, 420.0);
    let inner = Cv { p: &cv.p.with_clip_rect(arch), o: cv.o, k: cv.k };
    inner.circle(750.0, 300.0, 78.0, ac);
    inner.ellipse(680.0, 600.0, 250.0, 170.0, mx(p.primary, p.bg, 0.35));
    inner.ellipse(830.0, 620.0, 230.0, 150.0, mx(p.text, p.bg, 0.15));
    inner.ellipse(740.0, 650.0, 240.0, 120.0, sf);
    cv.circle(930.0, 150.0, 42.0, ac);
    cv.bold(930.0, 150.0, "-20%", 20.0, on(p.accent), Align2::CENTER_CENTER);
    cv.fill(540.0, 500.0, 150.0, 56.0, 12.0, bg);
    cv.bold(556.0, 520.0, "Linen Overshirt", 12.5, tx, Align2::LEFT_CENTER);
    cv.text(556.0, 540.0, "$128", 12.0, muted, Align2::LEFT_CENTER);
}

// ------------------------------------------------------------------ mobile app

fn phone(cv: &Cv, x: f32, y: f32, w: f32, h: f32, screen: Rgb) -> (f32, f32, f32, f32) {
    cv.shadow(x, y, w, h, 44.0, 1.2);
    cv.fill(x, y, w, h, 44.0, c32([22, 22, 26]));
    cv.fill(x + 9.0, y + 9.0, w - 18.0, h - 18.0, 36.0, c32(screen));
    cv.fill(x + w / 2.0 - 46.0, y + 16.0, 92.0, 24.0, 12.0, c32([10, 10, 12]));
    (x + 9.0, y + 9.0, w - 18.0, h - 18.0)
}

fn app(cv: &Cv, p: &Pal) {
    let (bg, sf, tx, pr, ac) = (c32(p.bg), c32(p.surface), c32(p.text), c32(p.primary), c32(p.accent));
    let muted = mx(p.text, p.bg, 0.45);
    cv.fill(0.0, 0.0, W, H, 0.0, mx(p.bg, p.text, 0.06));
    cv.circle(200.0, 520.0, 260.0, mx(p.primary, p.bg, 0.86));
    cv.circle(850.0, 120.0, 200.0, mx(p.accent, p.bg, 0.86));

    // Phone 1: home
    let (sx, sy, sw, sh) = phone(cv, 120.0, 30.0, 340.0, 570.0, p.bg);
    cv.bold(sx + 28.0, sy + 20.0, "9:41", 12.0, tx, Align2::LEFT_CENTER);
    cv.text(sx + 24.0, sy + 72.0, "Good morning,", 12.5, muted, Align2::LEFT_CENTER);
    cv.bold(sx + 24.0, sy + 96.0, "Alex", 24.0, tx, Align2::LEFT_CENTER);
    cv.circle(sx + sw - 44.0, sy + 86.0, 20.0, ac);
    cv.bold(sx + sw - 44.0, sy + 86.0, "A", 16.0, on(p.accent), Align2::CENTER_CENTER);
    cv.fill(sx + 20.0, sy + 128.0, sw - 40.0, 138.0, 22.0, pr);
    cv.text(sx + 40.0, sy + 156.0, "Total balance", 12.0, on(p.primary).gamma_multiply(0.75), Align2::LEFT_CENTER);
    cv.bold(sx + 40.0, sy + 192.0, "$12,480", 34.0, on(p.primary), Align2::LEFT_CENTER);
    cv.fill(sx + 40.0, sy + 224.0, 66.0, 24.0, 12.0, on(p.primary).gamma_multiply(0.2));
    cv.bold(sx + 73.0, sy + 236.0, "+4.2%", 11.5, on(p.primary), Align2::CENTER_CENTER);
    for (i, l) in ["Send", "Pay", "Scan", "More"].into_iter().enumerate() {
        let cx = sx + 20.0 + (sw - 40.0) * (0.125 + 0.25 * i as f32);
        cv.circle(cx, sy + 306.0, 25.0, sf);
        cv.circle(cx, sy + 306.0, 8.0, if i % 2 == 0 { ac } else { pr });
        cv.text(cx, sy + 346.0, l, 11.0, muted, Align2::CENTER_CENTER);
    }
    cv.bold(sx + 24.0, sy + 384.0, "Recent", 15.0, tx, Align2::LEFT_CENTER);
    let rows = [("Coffee Lab", "Today, 8:12", "-$6.40", p.primary), ("Design Co.", "Yesterday", "+$820.00", p.accent), ("Metro Pass", "Mon", "-$32.00", p.text)];
    for (i, (t, s, a, c)) in rows.into_iter().enumerate() {
        let y = sy + 412.0 + i as f32 * 52.0;
        cv.circle(sx + 46.0, y + 20.0, 17.0, mx(c, p.bg, 0.8));
        cv.circle(sx + 46.0, y + 20.0, 6.0, c32(c));
        cv.bold(sx + 74.0, y + 12.0, t, 13.0, tx, Align2::LEFT_CENTER);
        cv.text(sx + 74.0, y + 30.0, s, 11.0, muted, Align2::LEFT_CENTER);
        cv.bold(sx + sw - 24.0, y + 20.0, a, 13.0, if a.starts_with('+') { mx(p.accent, p.text, 0.3) } else { tx }, Align2::RIGHT_CENTER);
    }
    cv.fill(sx + 12.0, sy + sh - 62.0, sw - 24.0, 50.0, 25.0, sf);
    for i in 0..4 {
        let cx = sx + 12.0 + (sw - 24.0) * (0.125 + 0.25 * i as f32);
        if i == 0 {
            cv.fill(cx - 22.0, sy + sh - 54.0, 44.0, 34.0, 17.0, mx(p.accent, p.surface, 0.8));
        }
        cv.circle(cx, sy + sh - 37.0, 5.0, if i == 0 { ac } else { muted });
    }

    // Phone 2: onboarding
    let (sx, sy, sw, sh) = phone(cv, 540.0, 30.0, 340.0, 570.0, p.primary);
    let onp = on(p.primary);
    cv.bold(sx + 28.0, sy + 20.0, "9:41", 12.0, onp, Align2::LEFT_CENTER);
    cv.text(sx + sw - 28.0, sy + 72.0, "Skip", 13.0, onp.gamma_multiply(0.75), Align2::RIGHT_CENTER);
    let c = (sx + sw / 2.0, sy + 190.0);
    cv.circle(c.0, c.1, 120.0, mx(p.primary, WHITE, 0.12));
    cv.circle(c.0, c.1, 88.0, mx(p.primary, WHITE, 0.2));
    cv.circle(c.0, c.1, 56.0, ac);
    cv.bold(c.0, c.1, "✦", 36.0, on(p.accent), Align2::CENTER_CENTER);
    cv.circle(c.0 - 100.0, c.1 + 70.0, 14.0, bg);
    cv.circle(c.0 + 104.0, c.1 - 64.0, 9.0, bg);
    cv.bold(sx + 28.0, sy + 356.0, "Save smarter,", 30.0, onp, Align2::LEFT_CENTER);
    cv.bold(sx + 28.0, sy + 392.0, "every day.", 30.0, onp, Align2::LEFT_CENTER);
    cv.text(sx + 28.0, sy + 432.0, "Round-ups, goals and gentle nudges", 13.0, onp.gamma_multiply(0.8), Align2::LEFT_CENTER);
    cv.text(sx + 28.0, sy + 452.0, "that grow your balance on autopilot.", 13.0, onp.gamma_multiply(0.8), Align2::LEFT_CENTER);
    cv.fill(sx + sw / 2.0 - 40.0, sy + 482.0, 30.0, 8.0, 4.0, ac);
    cv.circle(sx + sw / 2.0 - 3.0, sy + 486.0, 4.0, onp.gamma_multiply(0.4));
    cv.circle(sx + sw / 2.0 + 13.0, sy + 486.0, 4.0, onp.gamma_multiply(0.4));
    cv.fill(sx + 24.0, sy + sh - 74.0, sw - 48.0, 54.0, 27.0, ac);
    cv.bold(sx + sw / 2.0, sy + sh - 47.0, "Get started", 15.0, on(p.accent), Align2::CENTER_CENTER);
}

// ------------------------------------------------------------------ stationery

fn stationery(cv: &Cv, p: &Pal) {
    let (bg, tx, pr, ac) = (c32(p.bg), c32(p.text), c32(p.primary), c32(p.accent));
    let muted = mx(p.text, p.bg, 0.45);
    let skel = mx(p.bg, p.text, 0.14);
    cv.fill(0.0, 0.0, W, H, 0.0, c32(p.surface));

    // Letterhead
    cv.shadow(90.0, 40.0, 390.0, 540.0, 4.0, 1.0);
    cv.fill(90.0, 40.0, 390.0, 540.0, 4.0, bg);
    cv.circle(136.0, 94.0, 16.0, pr);
    cv.circle(136.0, 94.0, 6.0, bg);
    cv.bold(162.0, 94.0, "Northwind", 20.0, tx, Align2::LEFT_CENTER);
    cv.fill(120.0, 132.0, 44.0, 4.0, 2.0, ac);
    for (i, l) in ["12 Harbour Street", "Portland, OR 97201", "hello@northwind.app"].into_iter().enumerate() {
        cv.text(452.0, 80.0 + i as f32 * 14.0, l, 9.5, muted, Align2::RIGHT_CENTER);
    }
    cv.text(120.0, 180.0, "March 14, 2026", 11.0, muted, Align2::LEFT_CENTER);
    cv.bold(120.0, 214.0, "Dear Alex,", 14.0, tx, Align2::LEFT_CENTER);
    let widths = [330.0, 318.0, 326.0, 240.0, 0.0, 332.0, 322.0, 300.0, 0.0, 328.0, 180.0];
    for (i, w) in widths.into_iter().enumerate() {
        if w > 0.0 {
            cv.skel(120.0, 244.0 + i as f32 * 20.0, w, 7.0, skel);
        }
    }
    cv.text(120.0, 490.0, "Warm regards,", 12.0, tx, Align2::LEFT_CENTER);
    cv.bold(120.0, 516.0, "Alex Morgan", 15.0, pr, Align2::LEFT_CENTER);
    cv.fill(90.0, 566.0, 390.0, 14.0, 0.0, pr);
    cv.fill(90.0, 566.0, 70.0, 14.0, 0.0, ac);

    // Card front
    cv.shadow(540.0, 70.0, 380.0, 220.0, 12.0, 0.9);
    cv.fill(540.0, 70.0, 380.0, 220.0, 12.0, pr);
    cv.poly(&[(920.0, 190.0), (920.0, 278.0), (850.0, 290.0), (880.0, 190.0)], ac);
    cv.circle(600.0, 130.0, 24.0, bg);
    cv.circle(600.0, 130.0, 9.0, pr);
    cv.bold(640.0, 130.0, "Northwind", 24.0, on(p.primary), Align2::LEFT_CENTER);
    cv.text(572.0, 262.0, "northwind.app", 12.0, on(p.primary).gamma_multiply(0.85), Align2::LEFT_CENTER);

    // Card back
    cv.shadow(580.0, 330.0, 380.0, 220.0, 12.0, 0.9);
    cv.fill(580.0, 330.0, 380.0, 220.0, 12.0, bg);
    cv.fill(580.0, 330.0, 16.0, 220.0, 0.0, pr);
    cv.fill(580.0, 330.0, 16.0, 70.0, 0.0, ac);
    cv.outline(580.0, 330.0, 380.0, 220.0, 12.0, 1.0, mx(p.bg, p.text, 0.12));
    cv.bold(628.0, 392.0, "Alex Morgan", 24.0, tx, Align2::LEFT_CENTER);
    cv.text(628.0, 420.0, "Design Lead", 13.0, mx(p.primary, p.text, 0.2), Align2::LEFT_CENTER);
    for (i, l) in ["alex@northwind.app", "+1 (503) 555 0142", "northwind.app/alex"].into_iter().enumerate() {
        let y = 468.0 + i as f32 * 24.0;
        cv.circle(634.0, y, 4.0, if i % 2 == 0 { pr } else { ac });
        cv.text(650.0, y, l, 12.5, muted, Align2::LEFT_CENTER);
    }
}
