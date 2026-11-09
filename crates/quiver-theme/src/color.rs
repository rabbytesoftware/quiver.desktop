//! Colour types and the OKLCH -> sRGB conversion the CSS tokens need.

/// An sRGB colour with straight alpha, each channel in `0.0..=1.0`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const TRANSPARENT: Color = Color { r: 0.0, g: 0.0, b: 0.0, a: 0.0 };

    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// `0xRRGGBBAA`, rounded.
    pub fn to_rgba8(self) -> u32 {
        let q = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u32;
        (q(self.r) << 24) | (q(self.g) << 16) | (q(self.b) << 8) | q(self.a)
    }

    pub fn with_alpha(self, a: f32) -> Self {
        Self { a, ..self }
    }

    /// CSS `color-mix(in srgb, self pct%, other)`: `pct` is how much of `self` is kept.
    pub fn mix_srgb(self, pct: f32, other: Color) -> Self {
        let m = |a: f32, b: f32| a * pct + b * (1.0 - pct);
        Self { r: m(self.r, other.r), g: m(self.g, other.g), b: m(self.b, other.b), a: m(self.a, other.a) }
    }

    pub const BLACK: Color = Color::rgba(0.0, 0.0, 0.0, 1.0);
}

/// A CSS `oklch(L C H / alpha)` value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Oklch {
    pub l: f32,
    pub c: f32,
    /// Hue in degrees.
    pub h: f32,
    pub a: f32,
}

impl Oklch {
    pub const fn new(l: f32, c: f32, h: f32) -> Self {
        Self { l, c, h, a: 1.0 }
    }

    pub const fn alpha(l: f32, c: f32, h: f32, a: f32) -> Self {
        Self { l, c, h, a }
    }

    /// Converts to gamut-clipped sRGB (Björn Ottosson's reference matrices).
    pub fn to_color(self) -> Color {
        let hr = self.h.to_radians();
        let (a, b) = (self.c * hr.cos(), self.c * hr.sin());
        let l_ = self.l + 0.396_337_78 * a + 0.215_803_76 * b;
        let m_ = self.l - 0.105_561_346 * a - 0.063_854_17 * b;
        let s_ = self.l - 0.089_484_18 * a - 1.291_485_5 * b;
        let (l, m, s) = (l_ * l_ * l_, m_ * m_ * m_, s_ * s_ * s_);
        let lin_r = 4.076_741_7 * l - 3.307_711_6 * m + 0.230_969_94 * s;
        let lin_g = -1.268_438 * l + 2.609_757_4 * m - 0.341_319_4 * s;
        let lin_b = -0.004_196_086_3 * l - 0.703_418_6 * m + 1.707_614_7 * s;
        Color { r: encode(lin_r), g: encode(lin_g), b: encode(lin_b), a: self.a }
    }
}

fn encode(linear: f32) -> f32 {
    let v = linear.clamp(0.0, 1.0);
    if v <= 0.003_130_8 {
        12.92 * v
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn white_and_black_round_trip() {
        assert_eq!(Oklch::new(1.0, 0.0, 0.0).to_color().to_rgba8(), 0xFFFF_FFFF);
        assert_eq!(Oklch::new(0.0, 0.0, 0.0).to_color().to_rgba8(), 0x0000_00FF);
    }

    #[test]
    fn neutral_grey_matches_tailwind_neutral() {
        // oklch(0.145 0 0) is Tailwind's neutral-950 (#0a0a0a).
        let c = Oklch::new(0.145, 0.0, 0.0).to_color().to_rgba8();
        assert_eq!(c >> 8, 0x0A_0A_0A);
    }

    #[test]
    fn alpha_is_carried() {
        assert_eq!(Oklch::alpha(0.0, 0.0, 0.0, 0.08).to_color().to_rgba8() & 0xFF, 20);
    }

    #[test]
    fn mix_keeps_the_requested_share() {
        let c = Color::rgba(1.0, 1.0, 1.0, 1.0).mix_srgb(0.6, Color::BLACK);
        assert!((c.r - 0.6).abs() < 1e-6);
    }
}
