#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[repr(C)]
pub struct Vector2 {
    pub x: f32,
    pub y: f32,
}

impl Vector2 {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

impl From<(f32, f32)> for Vector2 {
    fn from(value: (f32, f32)) -> Self {
        Self::new(value.0, value.1)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[repr(C)]
pub struct Dimensions {
    pub width: f32,
    pub height: f32,
}

impl Dimensions {
    pub fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}

impl From<(f32, f32)> for Dimensions {
    fn from(value: (f32, f32)) -> Self {
        Self::new(value.0, value.1)
    }
}

/// An axis-aligned rectangle defined by its top-left position and dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[repr(C)]
pub struct BoundingBox {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl BoundingBox {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

/// Classifies a rotation angle into common fast-path categories.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AngleType {
    /// 0° (or 360°) — no rotation needed.
    Zero,
    /// 90° clockwise.
    Right90,
    /// 180°.
    Straight180,
    /// 270° clockwise (= 90° counter-clockwise).
    Right270,
    /// An angle that doesn't match any fast-path.
    Arbitrary(f32),
}

/// Classifies a rotation in radians into an [`AngleType`].
/// Normalises to `[0, 2π)` first, then checks within `EPS` of each cardinal.
pub fn classify_angle(radians: f32) -> AngleType {
    let normalized = radians.rem_euclid(std::f32::consts::TAU);
    const EPS: f32 = 0.001;
    if normalized < EPS || (std::f32::consts::TAU - normalized) < EPS {
        AngleType::Zero
    } else if (normalized - std::f32::consts::FRAC_PI_2).abs() < EPS {
        AngleType::Right90
    } else if (normalized - std::f32::consts::PI).abs() < EPS {
        AngleType::Straight180
    } else if (normalized - 3.0 * std::f32::consts::FRAC_PI_2).abs() < EPS {
        AngleType::Right270
    } else {
        AngleType::Arbitrary(normalized)
    }
}

use crate::layout::CornerRadius;

/// Computes the axis-aligned bounding box of a rounded rectangle after rotation.
///
/// Uses the Minkowski-sum approach for equal corner radii:
///   `AABB_w = |(w-2r)·cosθ| + |(h-2r)·sinθ| + 2r`
///   `AABB_h = |(w-2r)·sinθ| + |(h-2r)·cosθ| + 2r`
///
/// For non-uniform radii, uses the maximum radius as a conservative approximation.
/// Returns `(effective_width, effective_height)`.
pub fn compute_rotated_aabb(
    width: f32,
    height: f32,
    corner_radius: &CornerRadius,
    rotation_radians: f32,
) -> (f32, f32) {
    let angle = classify_angle(rotation_radians);
    match angle {
        AngleType::Zero => (width, height),
        AngleType::Straight180 => (width, height),
        AngleType::Right90 | AngleType::Right270 => (height, width),
        AngleType::Arbitrary(theta) => {
            let r = corner_radius
                .top_left
                .max(corner_radius.top_right)
                .max(corner_radius.bottom_left)
                .max(corner_radius.bottom_right)
                .min(width / 2.0)
                .min(height / 2.0);

            let cos_t = theta.cos().abs();
            let sin_t = theta.sin().abs();
            let inner_w = (width - 2.0 * r).max(0.0);
            let inner_h = (height - 2.0 * r).max(0.0);

            let eff_w = inner_w * cos_t + inner_h * sin_t + 2.0 * r;
            let eff_h = inner_w * sin_t + inner_h * cos_t + 2.0 * r;
            (eff_w, eff_h)
        }
    }
}
