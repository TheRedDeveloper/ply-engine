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
