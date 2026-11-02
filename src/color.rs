//! Color math: conversions, harmonies, shades, and palette generation.

pub type Rgb = [u8; 3];

// ---------------------------------------------------------------- hex

pub fn to_hex(c: Rgb) -> String {
    format!("{:02X}{:02X}{:02X}", c[0], c[1], c[2])
}

pub fn to_hex_hash(c: Rgb) -> String {
    format!("#{}", to_hex(c))
}

pub fn parse_hex(s: &str) -> Option<Rgb> {
    let s = s.trim().trim_start_matches('#');
    let s: String = match s.len() {
        3 => s.chars().flat_map(|c| [c, c]).collect(),
        6 => s.to_string(),
        _ => return None,
    };
    let v = u32::from_str_radix(&s, 16).ok()?;
    Some([(v >> 16) as u8, (v >> 8) as u8, v as u8])
}

// ---------------------------------------------------------------- sRGB <-> linear

fn to_linear(u: u8) -> f32 {
    let c = u as f32 / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn from_linear(l: f32) -> f32 {
    let l = l.clamp(0.0, 1.0);
    if l <= 0.0031308 {
        l * 12.92
    } else {
        1.055 * l.powf(1.0 / 2.4) - 0.055
    }
}

fn to_u8(f: f32) -> u8 {
    (f.clamp(0.0, 1.0) * 255.0).round() as u8
}

// ---------------------------------------------------------------- HSL / HSV

/// Returns (h in 0..360, s 0..1, l 0..1).
pub fn to_hsl(c: Rgb) -> (f32, f32, f32) {
    let (r, g, b) = (c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    let d = max - min;
    if d < 1e-6 {
        return (0.0, 0.0, l);
    }
    let s = d / (1.0 - (2.0 * l - 1.0).abs());
    let h = if max == r {
        ((g - b) / d).rem_euclid(6.0)
    } else if max == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    } * 60.0;
    (h, s.clamp(0.0, 1.0), l)
}

pub fn from_hsl(h: f32, s: f32, l: f32) -> Rgb {
    let h = h.rem_euclid(360.0);
    let s = s.clamp(0.0, 1.0);
    let l = l.clamp(0.0, 1.0);
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;
    let (r, g, b) = match (h / 60.0) as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    [to_u8(r + m), to_u8(g + m), to_u8(b + m)]
}

/// Returns (h 0..360, s 0..1, v 0..1).
pub fn to_hsv(c: Rgb) -> (f32, f32, f32) {
    let (r, g, b) = (c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let (h, _, _) = to_hsl(c);
    let s = if max <= 0.0 { 0.0 } else { d / max };
    (h, s, max)
}

pub fn from_hsv(h: f32, s: f32, v: f32) -> Rgb {
    let h = h.rem_euclid(360.0);
    let (s, v) = (s.clamp(0.0, 1.0), v.clamp(0.0, 1.0));
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match (h / 60.0) as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    [to_u8(r + m), to_u8(g + m), to_u8(b + m)]
}

// ---------------------------------------------------------------- OKLab / OKLCH

pub fn to_oklab(c: Rgb) -> (f32, f32, f32) {
    let (r, g, b) = (to_linear(c[0]), to_linear(c[1]), to_linear(c[2]));
    let l = (0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b).cbrt();
    let m = (0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b).cbrt();
    let s = (0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b).cbrt();
    (
        0.2104542553 * l + 0.7936177850 * m - 0.0040720468 * s,
        1.9779984951 * l - 2.4285922050 * m + 0.4505937099 * s,
        0.0259040371 * l + 0.7827717662 * m - 0.8086757660 * s,
    )
}

/// Linear-light RGB (unclamped) from OKLab.
pub fn oklab_to_linear(l: f32, a: f32, b: f32) -> (f32, f32, f32) {
    let l_ = l + 0.3963377774 * a + 0.2158037573 * b;
    let m_ = l - 0.1055613458 * a - 0.0638541728 * b;
    let s_ = l - 0.0894841775 * a - 1.2914855480 * b;
    let (l, m, s) = (l_ * l_ * l_, m_ * m_ * m_, s_ * s_ * s_);
    (
        4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
        -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
        -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s,
    )
}

pub fn to_oklch(c: Rgb) -> (f32, f32, f32) {
    let (l, a, b) = to_oklab(c);
    let ch = (a * a + b * b).sqrt();
    let h = b.atan2(a).to_degrees().rem_euclid(360.0);
    (l, ch, h)
}

/// OKLCH -> sRGB, reducing chroma until the color is inside the sRGB gamut.
pub fn from_oklch(l: f32, c: f32, h: f32) -> Rgb {
    let l = l.clamp(0.0, 1.0);
    let hr = h.to_radians();
    let in_gamut = |c: f32| {
        let (r, g, b) = oklab_to_linear(l, c * hr.cos(), c * hr.sin());
        let e = 0.0005;
        [r, g, b].iter().all(|v| *v >= -e && *v <= 1.0 + e)
    };
    let mut c = c.max(0.0);
    if !in_gamut(c) {
        let (mut lo, mut hi) = (0.0, c);
        for _ in 0..18 {
            let mid = (lo + hi) / 2.0;
            if in_gamut(mid) {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        c = lo;
    }
    let (r, g, b) = oklab_to_linear(l, c * hr.cos(), c * hr.sin());
    [to_u8(from_linear(r)), to_u8(from_linear(g)), to_u8(from_linear(b))]
}

// ---------------------------------------------------------------- contrast

pub fn luminance(c: Rgb) -> f32 {
    0.2126 * to_linear(c[0]) + 0.7152 * to_linear(c[1]) + 0.0722 * to_linear(c[2])
}

pub fn contrast_ratio(a: Rgb, b: Rgb) -> f32 {
    let (la, lb) = (luminance(a), luminance(b));
    let (hi, lo) = if la > lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

/// Black or white, whichever reads better on `bg`.
pub fn text_on(bg: Rgb) -> Rgb {
    if contrast_ratio(bg, [255, 255, 255]) >= contrast_ratio(bg, [20, 20, 20]) {
        [255, 255, 255]
    } else {
        [20, 20, 20]
    }
}

/// Perceptually even interpolation (used for gradients).
pub fn mix_oklab(a: Rgb, b: Rgb, t: f32) -> Rgb {
    let (l1, a1, b1) = to_oklab(a);
    let (l2, a2, b2) = to_oklab(b);
    let (r, g, bl) = oklab_to_linear(l1 + (l2 - l1) * t, a1 + (a2 - a1) * t, b1 + (b2 - b1) * t);
    [to_u8(from_linear(r)), to_u8(from_linear(g)), to_u8(from_linear(bl))]
}

pub fn mix(a: Rgb, b: Rgb, t: f32) -> Rgb {
    let f = |i: usize| to_u8(from_linear(to_linear(a[i]) * (1.0 - t) + to_linear(b[i]) * t));
    [f(0), f(1), f(2)]
}

// ---------------------------------------------------------------- scheme roles

fn hue_gap(a: f32, b: f32) -> f32 {
    let d = (a - b).abs() % 360.0;
    d.min(360.0 - d)
}

/// Picks [background, surface, text, primary, accent] from a palette, for a light or dark page.
pub fn auto_roles(palette: &[Rgb], dark: bool) -> [Rgb; 5] {
    let mut pool: Vec<Rgb> = vec![];
    for c in palette {
        if !pool.contains(c) {
            pool.push(*c);
        }
    }
    if pool.is_empty() {
        pool = vec![[0x3D, 0x7E, 0xF5], [0xF5, 0x9E, 0x0B]];
    }
    let by_lum = |a: &Rgb, b: &Rgb| luminance(*a).total_cmp(&luminance(*b));
    let src = if dark { *pool.iter().min_by(|a, b| by_lum(a, b)).unwrap() } else { *pool.iter().max_by(|a, b| by_lum(a, b)).unwrap() };
    let bg = if dark {
        if luminance(src) > 0.12 { mix(src, [10, 10, 14], 0.85) } else { src }
    } else if luminance(src) < 0.55 {
        mix(src, [255, 255, 255], 0.9)
    } else {
        src
    };
    pool.retain(|c| *c != src);

    let text = pool
        .iter()
        .copied()
        .max_by(|a, b| contrast_ratio(*a, bg).total_cmp(&contrast_ratio(*b, bg)))
        .filter(|c| contrast_ratio(*c, bg) >= 4.5)
        .unwrap_or_else(|| mix(bg, text_on(bg), 0.92));
    pool.retain(|c| *c != text);

    let sat = |c: Rgb| to_hsv(c).1;
    let primary = pool
        .iter()
        .copied()
        .max_by(|a, b| {
            let score = |c: Rgb| sat(c) * 0.7 + (contrast_ratio(c, bg).min(4.5) / 4.5) * 0.3;
            score(*a).total_cmp(&score(*b))
        })
        .unwrap_or([0x3D, 0x7E, 0xF5]);
    pool.retain(|c| *c != primary);

    let ph = to_hsl(primary).0;
    let accent = pool
        .iter()
        .copied()
        .max_by(|a, b| {
            let score = |c: Rgb| hue_gap(to_hsl(c).0, ph) / 180.0 * 0.6 + sat(c) * 0.4;
            score(*a).total_cmp(&score(*b))
        })
        .unwrap_or_else(|| {
            let (h, s, l) = to_hsl(primary);
            from_hsl(h + 150.0, s.max(0.6), l.clamp(0.4, 0.6))
        });

    let surface = if dark { mix(bg, [255, 255, 255], 0.07) } else { mix(bg, [0, 0, 0], 0.045) };
    [bg, mix(surface, primary, 0.05), text, primary, accent]
}

// ---------------------------------------------------------------- harmonies

pub struct Harmony {
    pub name: &'static str,
    pub blurb: &'static str,
    pub colors: Vec<Rgb>,
}

pub fn harmonies(base: Rgb) -> Vec<Harmony> {
    let (h, s, l) = to_hsl(base);
    let rot = |deg: f32| from_hsl(h + deg, s, l);
    let rotc = |deg: f32| if deg == 0.0 { base } else { rot(deg) };
    let set = |degs: &[f32]| degs.iter().map(|d| rotc(*d)).collect::<Vec<_>>();
    let mono: Vec<Rgb> = [-0.30f32, -0.15, 0.0, 0.15, 0.30]
        .iter()
        .map(|d| {
            if *d == 0.0 {
                base
            } else {
                from_hsl(h, s, (l + d).clamp(0.05, 0.95))
            }
        })
        .collect();
    vec![
        Harmony { name: "Complementary", blurb: "Opposite on the wheel. Maximum contrast.", colors: set(&[0.0, 180.0]) },
        Harmony { name: "Analogous", blurb: "Neighbors. Calm and cohesive.", colors: set(&[-60.0, -30.0, 0.0, 30.0, 60.0]) },
        Harmony { name: "Triadic", blurb: "Three evenly spaced hues. Vibrant balance.", colors: set(&[0.0, 120.0, 240.0]) },
        Harmony { name: "Split complementary", blurb: "Base plus the two neighbors of its complement.", colors: set(&[0.0, 150.0, 210.0]) },
        Harmony { name: "Tetradic", blurb: "Two complementary pairs (rectangle).", colors: set(&[0.0, 60.0, 180.0, 240.0]) },
        Harmony { name: "Square", blurb: "Four evenly spaced hues.", colors: set(&[0.0, 90.0, 180.0, 270.0]) },
        Harmony { name: "Monochromatic", blurb: "One hue, stepped lightness.", colors: mono },
    ]
}

// ---------------------------------------------------------------- shades

/// A light-to-dark ladder in OKLCH keeping hue and (gamut-limited) chroma.
/// The step nearest to `base` is replaced by `base` itself; its index is returned.
pub fn shade_ladder(base: Rgb, steps: usize) -> (Vec<Rgb>, usize) {
    let (bl, bc, bh) = to_oklch(base);
    let (top, bottom) = (0.97f32, 0.14f32);
    let mut out = Vec::with_capacity(steps);
    let mut nearest = 0;
    let mut best = f32::MAX;
    for i in 0..steps {
        let t = i as f32 / (steps - 1) as f32;
        let l = top + (bottom - top) * t;
        if (l - bl).abs() < best {
            best = (l - bl).abs();
            nearest = i;
        }
        out.push(from_oklch(l, bc, bh));
    }
    out[nearest] = base;
    (out, nearest)
}

pub fn tints(base: Rgb, n: usize) -> Vec<Rgb> {
    (0..n).map(|i| mix(base, [255, 255, 255], i as f32 / (n - 1) as f32)).collect()
}
pub fn shades(base: Rgb, n: usize) -> Vec<Rgb> {
    (0..n).map(|i| mix(base, [0, 0, 0], i as f32 / (n - 1) as f32)).collect()
}
pub fn tones(base: Rgb, n: usize) -> Vec<Rgb> {
    (0..n).map(|i| mix(base, [128, 128, 128], i as f32 / (n - 1) as f32)).collect()
}

// ---------------------------------------------------------------- generator

/// Coolors-style generation: unlocked slots get fresh colors that work together
/// with whatever is locked.
pub fn generate(current: &[Rgb], locked: &[bool]) -> Vec<Rgb> {
    let n = current.len();
    let free: Vec<usize> = (0..n).filter(|i| !locked[*i]).collect();
    if free.is_empty() {
        return current.to_vec();
    }

    // Anchor hue: from a random chromatic locked color, otherwise random.
    let anchors: Vec<f32> = (0..n)
        .filter(|i| locked[*i])
        .map(|i| to_oklch(current[i]))
        .filter(|(_, c, _)| *c > 0.03)
        .map(|(_, _, h)| h)
        .collect();
    let base_h = if anchors.is_empty() {
        fastrand::f32() * 360.0
    } else {
        anchors[fastrand::usize(..anchors.len())]
    };

    let offsets: Vec<f32> = match fastrand::u8(..7) {
        0 => vec![0.0, 25.0, -25.0, 50.0],         // analogous
        1 => vec![0.0, 180.0, 0.0, 180.0],         // complementary
        2 => vec![0.0, 120.0, 240.0],              // triad
        3 => vec![0.0, 150.0, 210.0],              // split
        4 => vec![0.0, 6.0, -6.0],                 // monochrome-ish
        5 => vec![0.0, 90.0, 180.0, 270.0],        // square
        _ => vec![0.0, 40.0, 190.0, 230.0],        // loose
    };

    // Stratified lightness so a palette always has range.
    let k = free.len();
    let mut ls: Vec<f32> = (0..k)
        .map(|i| {
            let t = if k == 1 { fastrand::f32() } else { i as f32 / (k - 1) as f32 };
            0.30 + 0.64 * t + (fastrand::f32() - 0.5) * 0.08
        })
        .collect();
    match fastrand::u8(..3) {
        0 => ls.reverse(),
        1 => fastrand::shuffle(&mut ls),
        _ => {}
    }

    let base_c = 0.05 + fastrand::f32() * 0.13;
    let mut out = current.to_vec();
    for (j, slot) in free.iter().enumerate() {
        let off = offsets[j % offsets.len()] + (fastrand::f32() - 0.5) * 16.0;
        let l = ls[j].clamp(0.2, 0.97);
        let mut c = base_c * (0.55 + fastrand::f32() * 0.6);
        if !(0.32..=0.9).contains(&l) {
            c *= 0.55;
        }
        out[*slot] = from_oklch(l, c, base_h + off);
    }
    out
}

pub fn random_color() -> Rgb {
    from_oklch(0.4 + fastrand::f32() * 0.5, 0.05 + fastrand::f32() * 0.15, fastrand::f32() * 360.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_roundtrip() {
        assert_eq!(parse_hex("#3D7EF5"), Some([0x3D, 0x7E, 0xF5]));
        assert_eq!(parse_hex("fa0"), Some([0xFF, 0xAA, 0x00]));
        assert_eq!(parse_hex("zzz"), None);
        assert_eq!(to_hex([1, 2, 255]), "0102FF");
    }

    #[test]
    fn oklch_roundtrip() {
        for c in [[200u8, 30, 90], [10, 10, 10], [250, 250, 250], [30, 160, 220]] {
            let (l, ch, h) = to_oklch(c);
            let back = from_oklch(l, ch, h);
            for i in 0..3 {
                assert!((back[i] as i32 - c[i] as i32).abs() <= 2, "{c:?} -> {back:?}");
            }
        }
    }

    #[test]
    fn hsl_roundtrip() {
        let c = [200u8, 30, 90];
        let (h, s, l) = to_hsl(c);
        let b = from_hsl(h, s, l);
        for i in 0..3 {
            assert!((b[i] as i32 - c[i] as i32).abs() <= 1);
        }
    }

    #[test]
    fn generator_respects_locks() {
        let cur = [[10, 20, 30], [40, 50, 60], [70, 80, 90], [100, 110, 120], [1, 2, 3]];
        let lock = [true, false, true, false, false];
        for _ in 0..50 {
            let n = generate(&cur, &lock);
            assert_eq!(n[0], cur[0]);
            assert_eq!(n[2], cur[2]);
        }
    }

    #[test]
    fn ladder_contains_base() {
        let (l, i) = shade_ladder([61, 126, 245], 21);
        assert_eq!(l[i], [61, 126, 245]);
        assert_eq!(l.len(), 21);
    }
}
