use crate::color::Color;
use crate::math::Vector2;

/// Linear interpolation trait.
pub trait Lerp {
    fn lerp(self, other: Self, t: f32) -> Self;
}

#[inline]
fn clamp01(t: f32) -> f32 {
    t.clamp(0.0, 1.0)
}

impl Lerp for f32 {
    #[inline]
    fn lerp(self, other: Self, t: f32) -> Self {
        let t = clamp01(t);
        if self == other {
            return self;
        }
        self + (other - self) * t
    }
}

impl Lerp for u16 {
    #[inline]
    fn lerp(self, other: Self, t: f32) -> Self {
        let t = clamp01(t);
        if self == other {
            return self;
        }
        let v = (self as f32)
            .lerp(other as f32, t)
            .round()
            .clamp(u16::MIN as f32, u16::MAX as f32);
        v as u16
    }
}

impl Lerp for Vector2 {
    #[inline]
    fn lerp(self, other: Self, t: f32) -> Self {
        let t = clamp01(t);
        if self == other {
            return self;
        }
        Self {
            x: self.x.lerp(other.x, t),
            y: self.y.lerp(other.y, t),
        }
    }
}

impl Lerp for macroquad::prelude::Vec2 {
    #[inline]
    fn lerp(self, other: Self, t: f32) -> Self {
        let t = clamp01(t);
        if self == other {
            return self;
        }
        Self::new(
            self.x.lerp(other.x, t),
            self.y.lerp(other.y, t),
        )
    }
}

impl Lerp for (f32, f32, f32, f32) {
    #[inline]
    fn lerp(self, other: Self, t: f32) -> Self {
        let t = clamp01(t);
        if self == other {
            return self;
        }
        (
            self.0.lerp(other.0, t),
            self.1.lerp(other.1, t),
            self.2.lerp(other.2, t),
            self.3.lerp(other.3, t),
        )
    }
}

impl Lerp for (u16, u16, u16, u16) {
    #[inline]
    fn lerp(self, other: Self, t: f32) -> Self {
        let t = clamp01(t);
        if self == other {
            return self;
        }
        (
            self.0.lerp(other.0, t),
            self.1.lerp(other.1, t),
            self.2.lerp(other.2, t),
            self.3.lerp(other.3, t),
        )
    }
}

impl Lerp for Color {
    #[inline]
    fn lerp(self, other: Self, t: f32) -> Self {
        let t = clamp01(t);
        if self == other {
            return self;
        }
        Color {
            r: self.r.lerp(other.r, t),
            g: self.g.lerp(other.g, t),
            b: self.b.lerp(other.b, t),
            a: self.a.lerp(other.a, t),
        }
    }
}

impl Color {
    /// Interpolate in sRGB transfer space (gamma-aware).
    #[inline]
    pub fn lerp_srgb(self, other: Self, t: f32) -> Self {
        if self == other {
            return self;
        }

        let t = clamp01(t);
        if t == 0.0 {
            return self;
        }
        if t == 1.0 {
            return other;
        }
        let a = self.a.lerp(other.a, t);

        let r0 = srgb_to_linear(channel_to_unit(self.r));
        let g0 = srgb_to_linear(channel_to_unit(self.g));
        let b0 = srgb_to_linear(channel_to_unit(self.b));

        let r1 = srgb_to_linear(channel_to_unit(other.r));
        let g1 = srgb_to_linear(channel_to_unit(other.g));
        let b1 = srgb_to_linear(channel_to_unit(other.b));

        let r = linear_to_srgb(r0.lerp(r1, t));
        let g = linear_to_srgb(g0.lerp(g1, t));
        let b = linear_to_srgb(b0.lerp(b1, t));

        Color::rgba(unit_to_channel(r), unit_to_channel(g), unit_to_channel(b), a)
    }

    /// Interpolate in Oklab color space (perceptually uniform).
    #[inline]
    pub fn lerp_oklab(self, other: Self, t: f32) -> Self {
        if self == other {
            return self;
        }

        let t = clamp01(t);
        if t == 0.0 {
            return self;
        }
        if t == 1.0 {
            return other;
        }
        let a = self.a.lerp(other.a, t);

        let c0 = rgb_to_oklab(self);
        let c1 = rgb_to_oklab(other);

        let l = c0.0.lerp(c1.0, t);
        let a_lab = c0.1.lerp(c1.1, t);
        let b_lab = c0.2.lerp(c1.2, t);

        let (r, g, b) = oklab_to_rgb(l, a_lab, b_lab);
        Color::rgba(unit_to_channel(r), unit_to_channel(g), unit_to_channel(b), a)
    }
}

#[inline]
fn channel_to_unit(v: f32) -> f32 {
    (v / 255.0).clamp(0.0, 1.0)
}

#[inline]
fn unit_to_channel(v: f32) -> f32 {
    (v.clamp(0.0, 1.0) * 255.0).clamp(0.0, 255.0)
}

#[inline]
fn srgb_to_linear(v: f32) -> f32 {
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

#[inline]
fn linear_to_srgb(v: f32) -> f32 {
    if v <= 0.003_130_8 {
        12.92 * v
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    }
}

#[inline]
fn rgb_to_oklab(color: Color) -> (f32, f32, f32) {
    let r = srgb_to_linear(channel_to_unit(color.r));
    let g = srgb_to_linear(channel_to_unit(color.g));
    let b = srgb_to_linear(channel_to_unit(color.b));

    let l = 0.412_221_46 * r + 0.536_332_55 * g + 0.051_445_995 * b;
    let m = 0.211_903_5 * r + 0.680_699_5 * g + 0.107_396_96 * b;
    let s = 0.088_302_46 * r + 0.281_718_85 * g + 0.629_978_7 * b;

    let l_ = l.cbrt();
    let m_ = m.cbrt();
    let s_ = s.cbrt();

    (
        0.210_454_26 * l_ + 0.793_617_8 * m_ - 0.004_072_047 * s_,
        1.977_998_5 * l_ - 2.428_592_2 * m_ + 0.450_593_7 * s_,
        0.025_904_037 * l_ + 0.782_771_77 * m_ - 0.808_675_77 * s_,
    )
}

#[inline]
fn oklab_to_rgb(l: f32, a: f32, b: f32) -> (f32, f32, f32) {
    let l_ = l + 0.396_337_78 * a + 0.215_803_76 * b;
    let m_ = l - 0.105_561_346 * a - 0.063_854_17 * b;
    let s_ = l - 0.089_484_18 * a - 1.291_485_5 * b;

    let l3 = l_ * l_ * l_;
    let m3 = m_ * m_ * m_;
    let s3 = s_ * s_ * s_;

    let r = 4.076_741_7 * l3 - 3.307_711_6 * m3 + 0.230_969_94 * s3;
    let g = -1.268_438 * l3 + 2.609_757_4 * m3 - 0.341_319_38 * s3;
    let b = -0.004_196_086_3 * l3 - 0.703_418_6 * m3 + 1.707_614_7 * s3;

    (
        linear_to_srgb(r).clamp(0.0, 1.0),
        linear_to_srgb(g).clamp(0.0, 1.0),
        linear_to_srgb(b).clamp(0.0, 1.0),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(a: f32, b: f32) {
        assert!(
            (a - b).abs() <= 0.001,
            "expected {} ~= {} (delta {})",
            a,
            b,
            (a - b).abs()
        );
    }

    fn generic_lerp<T: Lerp>(a: T, b: T, t: f32) -> T {
        a.lerp(b, t)
    }

    #[test]
    fn test_f32_lerp_clamps_and_identity() {
        assert_eq!(generic_lerp(10.0_f32, 20.0_f32, -2.0), 10.0);
        assert_eq!(generic_lerp(10.0_f32, 20.0_f32, 2.0), 20.0);
        assert_eq!(generic_lerp(7.5_f32, 7.5_f32, 0.25), 7.5);
        assert_close(generic_lerp(10.0_f32, 20.0_f32, 0.25), 12.5);
    }

    #[test]
    fn test_u16_lerp_clamps_rounds_and_identity() {
        assert_eq!(10_u16.lerp(20, -1.0), 10);
        assert_eq!(10_u16.lerp(20, 2.0), 20);
        assert_eq!(10_u16.lerp(20, 0.49), 15);
        assert_eq!(42_u16.lerp(42, 0.66), 42);
    }

    #[test]
    fn test_vector2_and_vec2_lerp() {
        let a = Vector2::new(0.0, 10.0);
        let b = Vector2::new(20.0, 30.0);
        let v = a.lerp(b, 0.5);
        assert_close(v.x, 10.0);
        assert_close(v.y, 20.0);

        let mq_a = macroquad::prelude::Vec2::new(0.0, 10.0);
        let mq_b = macroquad::prelude::Vec2::new(20.0, 30.0);
        let mq_v = mq_a.lerp(mq_b, 0.5);
        assert_close(mq_v.x, 10.0);
        assert_close(mq_v.y, 20.0);
    }

    #[test]
    fn test_tuple_lerp() {
        let tf = (0.0_f32, 1.0_f32, 2.0_f32, 3.0_f32).lerp((4.0, 5.0, 6.0, 7.0), 0.5);
        assert_close(tf.0, 2.0);
        assert_close(tf.1, 3.0);
        assert_close(tf.2, 4.0);
        assert_close(tf.3, 5.0);

        let tu = (0_u16, 10_u16, 20_u16, 30_u16).lerp((10, 20, 30, 40), 0.5);
        assert_eq!(tu, (5, 15, 25, 35));
    }

    #[test]
    fn test_color_lerp_linear_and_alpha() {
        let a = Color::rgba(10.0, 20.0, 30.0, 40.0);
        let b = Color::rgba(110.0, 220.0, 130.0, 240.0);
        let mid = a.lerp(b, 0.5);

        assert_close(mid.r, 60.0);
        assert_close(mid.g, 120.0);
        assert_close(mid.b, 80.0);
        assert_close(mid.a, 140.0);

        assert_eq!(a.lerp(b, -1.0), a);
        assert_eq!(a.lerp(b, 2.0), b);
    }

    #[test]
    fn test_color_lerp_srgb_clamps_and_is_brighter_midpoint_than_linear() {
        let black = Color::rgba(0.0, 0.0, 0.0, 10.0);
        let white = Color::rgba(255.0, 255.0, 255.0, 210.0);

        let linear_mid = black.lerp(white, 0.5);
        let srgb_mid = black.lerp_srgb(white, 0.5);

        assert!(srgb_mid.r > linear_mid.r, "sRGB midpoint should be perceptually brighter");
        assert_close(srgb_mid.a, 110.0);

        assert_eq!(black.lerp_srgb(white, -0.5), black);
        assert_eq!(black.lerp_srgb(white, 1.5), white);
        assert_eq!(black.lerp_srgb(black, 0.42), black);
    }

    #[test]
    fn test_color_lerp_oklab_clamps_alpha_and_identity() {
        let a = Color::rgba(255.0, 0.0, 0.0, 10.0);
        let b = Color::rgba(0.0, 0.0, 255.0, 250.0);
        let mid = a.lerp_oklab(b, 0.5);

        assert_close(mid.a, 130.0);
        assert!(mid.r >= 0.0 && mid.r <= 255.0);
        assert!(mid.g >= 0.0 && mid.g <= 255.0);
        assert!(mid.b >= 0.0 && mid.b <= 255.0);

        assert_eq!(a.lerp_oklab(b, -0.5), a);
        assert_eq!(a.lerp_oklab(b, 1.5), b);
        assert_eq!(a.lerp_oklab(a, 0.42), a);
    }
}