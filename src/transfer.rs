//! Recolors a photo so its design colors become a scheme's roles, keeping the lighting.
//!
//! The photos are generated with five known flat colors (backdrop, cream label, navy ink,
//! teal body, orange accent). We find where each of those actually landed in the picture
//! (k-means in OKLab), soft-assign every pixel to them, then move each pixel to the matching
//! role color while keeping how much lighter or darker than its cluster it was. Shadows,
//! folds and texture survive; only the "paint" changes.

use crate::color::{self, Rgb};
use egui::{ColorImage, Context, TextureHandle, TextureOptions};

/// Photos bundled in the app, in Preview order.
pub const PHOTOS: [(&str, &[u8]); 4] = [
    ("Box", &[]), // layered scene, see `Layers`
    ("Pouch", &[]),
    ("Bottle", &[]), // layered scene, see `Layers`
    ("Tote", &[]),
];

/// The colors the photos were prompted with, in role order:
/// background (cream label), surface (backdrop, sampled), text (navy), primary (teal), accent (orange).
const NOMINAL: [Rgb; 5] = [[0xF3, 0xE9, 0xD2], [0xC9, 0xD4, 0xE6], [0x14, 0x21, 0x3D], [0x0F, 0x6B, 0x6B], [0xF2, 0x6B, 0x21]];

type Lab = [f32; 3];

const MAX_W: u32 = 2240;
const SIGMA: f32 = 0.03;
const LW: f32 = 0.5; // how much lightness counts when matching a pixel to a cluster

pub struct Recolored {
    w: usize,
    h: usize,
    lab: Vec<Lab>,
    weights: Vec<[f32; 5]>,
    centers: [Lab; 5],
    pub tex: TextureHandle,
    last: Option<([Rgb; 5], bool)>,
    layers: Option<Layers>,
}

fn dist2(p: Lab, c: Lab) -> f32 {
    let dl = (p[0] - c[0]) * LW;
    let (da, db) = (p[1] - c[1], p[2] - c[2]);
    dl * dl + da * da + db * db
}

fn lab_of(c: Rgb) -> Lab {
    let (l, a, b) = color::to_oklab(c);
    [l, a, b]
}

impl Recolored {
    pub fn load(ctx: &Context, name: &str, bytes: &[u8]) -> Option<Self> {
        if bytes.is_empty() {
            return Self::load_layers(ctx, name);
        }
        let img = image::load_from_memory(bytes).ok()?.to_rgb8();
        let (w0, h0) = img.dimensions();
        let (w, h) = if w0 > MAX_W { (MAX_W, (h0 as f32 * MAX_W as f32 / w0 as f32).round() as u32) } else { (w0, h0) };
        let img = image::imageops::resize(&img, w, h, image::imageops::FilterType::Lanczos3);
        let (w, h) = (w as usize, h as usize);
        let lab: Vec<Lab> = img.pixels().map(|p| lab_of(p.0)).collect();

        // Start from the colors we asked for; the backdrop comes from the picture's corners.
        let mut centers: [Lab; 5] = NOMINAL.map(lab_of);
        let mut corner = [0.0f32; 3];
        let mut n = 0.0;
        for (cx, cy) in [(0, 0), (w - 12, 0), (0, h - 12), (w - 12, h - 12)] {
            for y in cy..cy + 12 {
                for x in cx..cx + 12 {
                    let p = lab[y * w + x];
                    for k in 0..3 {
                        corner[k] += p[k];
                    }
                    n += 1.0;
                }
            }
        }
        centers[1] = corner.map(|v| v / n);

        // A few rounds of k-means on a subsample.
        let step = (lab.len() / 24_000).max(1);
        for _ in 0..8 {
            let mut sum = [[0.0f32; 3]; 5];
            let mut cnt = [0.0f32; 5];
            for p in lab.iter().step_by(step) {
                let k = (0..5).min_by(|&a, &b| dist2(*p, centers[a]).total_cmp(&dist2(*p, centers[b]))).unwrap();
                for c in 0..3 {
                    sum[k][c] += p[c];
                }
                cnt[k] += 1.0;
            }
            for k in 0..5 {
                if cnt[k] > 30.0 {
                    centers[k] = sum[k].map(|v| v / cnt[k]);
                }
            }
        }

        // Soft assignment of every pixel. Anti-aliased edges sit between two colors, so for those
        // we find the two colors that blend to them (otherwise a third color fringes the edge).
        let ab = |p: Lab, c: Lab| ((p[1] - c[1]).powi(2) + (p[2] - c[2]).powi(2)).sqrt();
        let weights: Vec<[f32; 5]> = lab
            .iter()
            .map(|p| {
                let d: [f32; 5] = std::array::from_fn(|k| dist2(*p, centers[k]));
                let m = d.iter().cloned().fold(f32::MAX, f32::min);
                let mut w: [f32; 5] = d.map(|d| (-(d - m) / (2.0 * SIGMA * SIGMA)).exp());
                let s: f32 = w.iter().sum();
                w.iter_mut().for_each(|v| *v /= s);

                let single = (0..5).map(|k| ab(*p, centers[k])).fold(f32::MAX, f32::min);
                if single > 0.012 {
                    let mut best: Option<(f32, usize, usize, f32)> = None;
                    for i in 0..5 {
                        for j in i + 1..5 {
                            let (ci, cj) = (centers[i], centers[j]);
                            let (vx, vy) = (cj[1] - ci[1], cj[2] - ci[2]);
                            let len2 = vx * vx + vy * vy;
                            if len2 < 1e-5 {
                                continue;
                            }
                            let t = (((p[1] - ci[1]) * vx + (p[2] - ci[2]) * vy) / len2).clamp(0.0, 1.0);
                            let (qx, qy) = (ci[1] + t * vx, ci[2] + t * vy);
                            let dist = ((p[1] - qx).powi(2) + (p[2] - qy).powi(2)).sqrt();
                            if best.is_none_or(|b| dist < b.0) {
                                best = Some((dist, i, j, t));
                            }
                        }
                    }
                    if let Some((dist, i, j, t)) = best {
                        if dist < 0.5 * single && t > 0.06 && t < 0.94 {
                            w = [0.0; 5];
                            w[i] = 1.0 - t;
                            w[j] = t;
                        }
                    }
                }
                w
            })
            .collect();

        let mut weights = smooth_weights(&lab, weights, w, h);
        // Smoothing blurs the paint boundaries a little; pull each pixel back toward its dominant
        // paint so flat areas are one paint, not a blend of neighbours.
        for wt in weights.iter_mut() {
            let mut s = 0.0;
            for v in wt.iter_mut() {
                *v = v.powf(2.5);
                s += *v;
            }
            wt.iter_mut().for_each(|v| *v /= s);
        }

        let tex = ctx.load_texture(
            format!("photo-{name}"),
            ColorImage::from_rgb([w, h], &img.into_raw()),
            TextureOptions::LINEAR,
        );
        Some(Self { w, h, lab, weights, centers, tex, last: None, layers: None })
    }

    /// A scene rendered in Blender as layers (see `Layers`).
    fn load_layers(ctx: &Context, name: &str) -> Option<Self> {
        let layers = Layers::load(name)?;
        let (w, h) = (layers.w, layers.h);
        let tex = ctx.load_texture(format!("photo-{name}"), ColorImage::filled([w, h], egui::Color32::BLACK), TextureOptions::LINEAR);
        Some(Self { w, h, lab: Vec::new(), weights: Vec::new(), centers: [[0.0; 3]; 5], tex, last: None, layers: Some(layers) })
    }

    /// Aspect ratio (width / height) of the photo.
    pub fn aspect(&self) -> f32 {
        self.w as f32 / self.h as f32
    }

    /// Repaints the photo with a scheme [bg, surface, text, primary, accent], if it changed.
    /// While colors are still easing (`fast`), paint at half size so it keeps up; the full-size
    /// picture follows as soon as they settle.
    pub fn apply(&mut self, roles: [Rgb; 5], fast: bool) {
        if self.last == Some((roles, fast)) {
            return;
        }
        self.last = Some((roles, fast));
        let step = if fast && self.layers.is_none() { 3 } else { 1 };
        let out = self.render(roles, step);
        self.tex.set(ColorImage::from_rgba_unmultiplied([self.w / step, self.h / step], &out), TextureOptions::LINEAR);
    }

    /// RGBA bytes of the photo repainted with the scheme, sampling every `step`th pixel.
    fn render(&self, roles: [Rgb; 5], step: usize) -> Vec<u8> {
        if let Some(l) = &self.layers {
            return l.render(roles);
        }
        let target: [Lab; 5] = roles.map(lab_of);
        let ratio: [f32; 5] = std::array::from_fn(|k| {
            let (t, c) = (&target[k], &self.centers[k]);
            (((t[1] * t[1] + t[2] * t[2]).sqrt() + 0.03) / ((c[1] * c[1] + c[2] * c[2]).sqrt() + 0.03)).clamp(0.25, 3.0)
        });

        let (ow, oh) = (self.w / step, self.h / step);
        let mut out = Vec::with_capacity(ow * oh * 4);
        for i in 0..ow * oh {
            let idx = (i / ow) * step * self.w + (i % ow) * step;
            let (p, w) = (&self.lab[idx], &self.weights[idx]);
            // Blend the role colors by weight, then add back only the pixel's variation *within*
            // that blend (shading, texture), never the gap between two different paints.
            let (mut tl, mut ta, mut tb) = (0.0, 0.0, 0.0);
            let (mut cl, mut ca, mut cb) = (0.0, 0.0, 0.0);
            let mut r = 0.0;
            for k in 0..5 {
                if w[k] < 1e-4 {
                    continue;
                }
                let (t, c) = (&target[k], &self.centers[k]);
                tl += w[k] * t[0];
                ta += w[k] * t[1];
                tb += w[k] * t[2];
                cl += w[k] * c[0];
                ca += w[k] * c[1];
                cb += w[k] * c[2];
                r += w[k] * ratio[k];
            }
            let (l, a, b) = (tl + (p[0] - cl), ta + (p[1] - ca) * r, tb + (p[2] - cb) * r);
            let (r, g, bl) = color::oklab_to_linear(l.clamp(0.0, 1.0), a, b);
            out.extend_from_slice(&[encode(r), encode(g), encode(bl), 255]);
        }
        out
    }
}

/// Edge-aware smoothing of the paint weights: each pixel averages the weights of nearby pixels
/// that look like it, so shading noise can't flip a pixel to another paint (blotches) while real
/// edges, whose neighbors differ a lot, stay put.
fn smooth_weights(lab: &[Lab], weights: Vec<[f32; 5]>, w: usize, h: usize) -> Vec<[f32; 5]> {
    const R: isize = 4;
    const RANGE: f32 = 0.035;
    let mut cur = weights;
    for _ in 0..2 {
        let mut out = cur.clone();
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                let p = lab[i];
                let mut acc = [0.0f32; 5];
                let mut tot = 0.0;
                for dy in (-R..=R).step_by(2) {
                    let yy = y as isize + dy;
                    if yy < 0 || yy >= h as isize {
                        continue;
                    }
                    for dx in (-R..=R).step_by(2) {
                        let xx = x as isize + dx;
                        if xx < 0 || xx >= w as isize {
                            continue;
                        }
                        let j = yy as usize * w + xx as usize;
                        let q = lab[j];
                        let d = (p[0] - q[0]).powi(2) * 0.5 + (p[1] - q[1]).powi(2) + (p[2] - q[2]).powi(2);
                        let k = (-d / (2.0 * RANGE * RANGE)).exp();
                        for c in 0..5 {
                            acc[c] += k * cur[j][c];
                        }
                        tot += k;
                    }
                }
                out[i] = acc.map(|v| v / tot);
            }
        }
        cur = out;
    }
    cur
}

/// A packaging scene rendered as separate layers, the way mockup files work:
///  - `light`: shading with a white product (shadows, form), no color;
///  - `gloss`: the reflections, added on top untinted;
///  - masks: how much of each pixel is each paint, in photo order [label, backdrop, ink, body, accent]
///    (stored as [backdrop, label, ink] and [body, accent]).
/// Pixel = (sum of mask x role color) x light + gloss, in linear light. Nothing is guessed.
/// Kept as the PNGs' own bytes (11 per pixel) and decoded while painting.
struct Layers {
    w: usize,
    h: usize,
    light: Vec<u8>,
    gloss: Vec<u8>,
    m1: Vec<u8>,
    m2: Vec<u8>,
    lin: [f32; 256],
    to_srgb: Vec<u8>,
}

/// The studio is lit below full brightness so nothing clips; this brings a flat paint back up to
/// roughly its swatch color (whites simply reach white).
const GAIN: f32 = 1.22;

/// [light, gloss, mask1, mask2] PNGs for a scene.
fn scene_layers(name: &str) -> Option<[&'static [u8]; 4]> {
    Some(match name {
        "Bottle" => [
            include_bytes!("../assets/blender/duo_light.png"),
            include_bytes!("../assets/blender/duo_gloss.png"),
            include_bytes!("../assets/blender/duo_mask1.png"),
            include_bytes!("../assets/blender/duo_mask2.png"),
        ],
        "Pouch" => [
            include_bytes!("../assets/blender/pouch_light.png"),
            include_bytes!("../assets/blender/pouch_gloss.png"),
            include_bytes!("../assets/blender/pouch_mask1.png"),
            include_bytes!("../assets/blender/pouch_mask2.png"),
        ],
        "Box" => [
            include_bytes!("../assets/blender/box_light.png"),
            include_bytes!("../assets/blender/box_gloss.png"),
            include_bytes!("../assets/blender/box_mask1.png"),
            include_bytes!("../assets/blender/box_mask2.png"),
        ],
        "Tote" => [
            include_bytes!("../assets/blender/tote_light.png"),
            include_bytes!("../assets/blender/tote_gloss.png"),
            include_bytes!("../assets/blender/tote_mask1.png"),
            include_bytes!("../assets/blender/tote_mask2.png"),
        ],
        _ => return None,
    })
}

fn srgb_to_linear(u: u8) -> f32 {
    let v = u as f32 / 255.0;
    if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
}

impl Layers {
    fn load(name: &str) -> Option<Self> {
        let files = scene_layers(name)?;
        let dec = |b: &[u8]| image::load_from_memory(b).ok().map(|i| i.to_rgb8());
        let (light, gloss, m1, m2) = (dec(files[0])?, dec(files[1])?, dec(files[2])?, dec(files[3])?);
        let (w, h) = light.dimensions();
        if [gloss.dimensions(), m1.dimensions(), m2.dimensions()].iter().any(|d| *d != (w, h)) {
            return None;
        }
        Some(Self {
            w: w as usize,
            h: h as usize,
            light: light.into_raw(),
            gloss: gloss.into_raw(),
            m1: m1.into_raw(),
            m2: m2.into_raw(),
            lin: std::array::from_fn(|i| srgb_to_linear(i as u8)),
            to_srgb: (0..4096).map(|i| encode(i as f32 / 4095.0)).collect(),
        })
    }

    fn render(&self, roles: [Rgb; 5]) -> Vec<u8> {
        let role: [[f32; 3]; 5] = roles.map(|c| c.map(srgb_to_linear));
        let mut out = Vec::with_capacity(self.w * self.h * 4);
        for i in 0..self.w * self.h {
            let (a, b) = (&self.m1[i * 3..i * 3 + 3], &self.m2[i * 3..i * 3 + 2]);
            // Masks were saved raw: the byte is the coverage. Photo order is [label, backdrop, ...].
            let m = [a[1] as f32, a[0] as f32, a[2] as f32, b[0] as f32, b[1] as f32].map(|v| v / 255.0);
            let s = (m[0] + m[1] + m[2] + m[3] + m[4]).max(1e-3);
            for c in 0..3 {
                let alb = (m[0] * role[0][c] + m[1] * role[1][c] + m[2] * role[2][c] + m[3] * role[3][c] + m[4] * role[4][c]) / s;
                let gl = self.lin[self.gloss[i * 3 + c] as usize];
                let diffuse = (self.lin[self.light[i * 3 + c] as usize] - gl).max(0.0); // white-product render minus its reflections
                let v = alb * diffuse * GAIN + gl;
                out.push(self.to_srgb[(v.clamp(0.0, 1.0) * 4095.0 + 0.5) as usize]);
            }
            out.push(255);
        }
        out
    }
}

fn encode(v: f32) -> u8 {
    let v = v.clamp(0.0, 1.0);
    let s = if v <= 0.003_130_8 { v * 12.92 } else { 1.055 * v.powf(1.0 / 2.4) - 0.055 };
    (s * 255.0 + 0.5) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Writes recolored photos to $SWATCH_OUT for eyeballing: cargo test --release dump -- --ignored
    #[test]
    #[ignore]
    fn dump() {
        let Ok(dir) = std::env::var("SWATCH_OUT") else { return };
        let ctx = Context::default();
        let schemes: [(&str, [Rgb; 5]); 4] = [
            ("user", [[0x3A, 0x80, 0xC2], [0xFF, 0xEB, 0xF6], [0x4A, 0x5F, 0x6B], [0x62, 0x00, 0x54], [0x3A, 0x80, 0xC2]]),
            ("dark", color::auto_roles(&[[0x12, 0x14, 0x1C], [0xF2, 0xC1, 0x4E], [0xE8, 0x5D, 0x75], [0x5B, 0x8E, 0x7D], [0xEE, 0xEE, 0xE8]], true)),
            ("pink", color::auto_roles(&[[0xFF, 0xF1, 0xF2], [0xFF, 0x8F, 0xA3], [0xC9, 0x18, 0x4A], [0x59, 0x0D, 0x22], [0xFF, 0xB3, 0xC1]], false)),
            ("forest", color::auto_roles(&[[0xF1, 0xEF, 0xE6], [0x2D, 0x4A, 0x32], [0x8A, 0x9A, 0x5B], [0xD9, 0x7B, 0x29], [0x1B, 0x26, 0x1F]], false)),
        ];
        for (name, bytes) in PHOTOS {
            let mut r = Recolored::load(&ctx, name, bytes).unwrap();
            for (sn, roles) in &schemes {
                let o = *roles;
                let out = r.render([o[1], o[0], o[2], o[3], o[4]], 1);
                let rgb: Vec<u8> = out.chunks(4).flat_map(|p| [p[0], p[1], p[2]]).collect();
                image::RgbImage::from_raw(r.w as u32, r.h as u32, rgb).unwrap().save(format!("{dir}/{name}-{sn}.png")).unwrap();
            }
        }
    }
}
