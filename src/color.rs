/// An RGBA color with floating-point components (0.0–255.0 range).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[repr(C)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 255.0 }
    }
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Allows using hex values to build colors
    /// ```
    /// use ply_engine::color::Color;
    /// assert_eq!(Color::rgb(255.0, 255.0, 255.0), Color::u_rgb(0xFF, 0xFF, 0xFF));
    /// ```
    pub const fn u_rgb(r: u8, g: u8, b: u8) -> Self {
        Self::rgb(r as f32, g as f32, b as f32)
    }
    /// Allows using hex values to build colors
    /// ```
    /// use ply_engine::color::Color;
    /// assert_eq!(Color::rgba(255.0, 255.0, 255.0, 255.0), Color::u_rgba(0xFF, 0xFF, 0xFF, 0xFF));
    /// ```
    pub const fn u_rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self::rgba(r as f32, g as f32, b as f32, a as f32)
    }

    /// Allows using OKLCH values to build colors
    /// ```
    /// use ply_engine::color::Color;
    /// let col = Color::oklch(0.7, 0.14, 81.0); 
    /// assert!(col.r.round() == 202.0 && col.g.round() == 148.0 && col.b.round() == 21.0 && col.a == 255.0);
    /// ```
    pub fn oklch(l: f32, c: f32, h: f32) -> Self {
        let l = l.clamp(0.0, 1.0);
        let c = c.max(0.0);
        let h = h.rem_euclid(360.0).to_radians();

        let a = c * h.cos();
        let b = c * h.sin();

        let l_ = l + 0.396_337_78 * a + 0.215_803_76 * b;
        let m_ = l - 0.105_561_346 * a - 0.063_854_17 * b;
        let s_ = l - 0.089_484_18 * a - 1.291_485_5 * b;

        let l3 = l_ * l_ * l_;
        let m3 = m_ * m_ * m_;
        let s3 = s_ * s_ * s_;

        let r_linear = 4.076_741_7 * l3 - 3.307_711_6 * m3 + 0.230_969_94 * s3;
        let g_linear = -1.268_438 * l3 + 2.609_757_4 * m3 - 0.341_319_38 * s3;
        let b_linear = -0.004_196_086_3 * l3 - 0.703_418_6 * m3 + 1.707_614_7 * s3;

        Self::rgba(
            Self::linear_to_srgb(r_linear) * 255.0,
            Self::linear_to_srgb(g_linear) * 255.0,
            Self::linear_to_srgb(b_linear) * 255.0,
            255.0,
        )
    }

    fn linear_to_srgb(v: f32) -> f32 {
        let v = v.max(0.0);
        if v <= 0.003_130_8 {
            (12.92 * v).clamp(0.0, 1.0)
        } else {
            (1.055 * v.powf(1.0 / 2.4) - 0.055).clamp(0.0, 1.0)
        }
    }
}

impl From<(f32, f32, f32)> for Color {
    fn from(value: (f32, f32, f32)) -> Self {
        Self::rgb(value.0, value.1, value.2)
    }
}
impl From<(f32, f32, f32, f32)> for Color {
    fn from(value: (f32, f32, f32, f32)) -> Self {
        Self::rgba(value.0, value.1, value.2, value.3)
    }
}

impl From<(u8, u8, u8)> for Color {
    fn from(value: (u8, u8, u8)) -> Self {
        Self::u_rgb(value.0, value.1, value.2)
    }
}
impl From<(u8, u8, u8, u8)> for Color {
    fn from(value: (u8, u8, u8, u8)) -> Self {
        Self::u_rgba(value.0, value.1, value.2, value.3)
    }
}

impl From<i32> for Color {
    fn from(hex: i32) -> Self {
        let r = ((hex >> 16) & 0xFF) as f32;
        let g = ((hex >> 8) & 0xFF) as f32;
        let b = (hex & 0xFF) as f32;
        Color::rgba(r, g, b, 255.0)
    }
}

impl From<u32> for Color {
    fn from(hex: u32) -> Self {
        let r = ((hex >> 16) & 0xFF) as f32;
        let g = ((hex >> 8) & 0xFF) as f32;
        let b = (hex & 0xFF) as f32;
        Color::rgba(r, g, b, 255.0)
    }
}

impl From<macroquad::color::Color> for Color {
    fn from(c: macroquad::color::Color) -> Self {
        Color::rgba(c.r * 255.0, c.g * 255.0, c.b * 255.0, c.a * 255.0)
    }
}

impl From<Color> for macroquad::color::Color {
    fn from(c: Color) -> Self {
        macroquad::color::Color::new(c.r / 255.0, c.g / 255.0, c.b / 255.0, c.a / 255.0)
    }
}
