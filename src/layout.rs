use crate::align::{AlignX, AlignY};
use crate::engine;

/// Defines different sizing behaviors for an element.
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum SizingType {
    /// The element's size is determined by its content and constrained by min/max values.
    Fit,
    /// The element expands to fill available space within min/max constraints.
    Grow,
    /// The element's size is fixed to a percentage of its parent.
    Percent,
    /// The element's size is set to a fixed value.
    Fixed,
}

/// Represents different sizing strategies for layout elements.
#[derive(Debug, Clone, Copy)]
pub enum Sizing {
    /// Fits the element’s width/height within a min and max constraint.
    Fit(f32, f32),
    /// Expands the element to fill available space within min/max constraints.
    Grow(f32, f32),
    /// Sets a fixed width/height.
    Fixed(f32),
    /// Sets width/height as a percentage of its parent. Value should be between `0.0` and `1.0`.
    Percent(f32),
}

/// Converts a `Sizing` value into an engine `SizingAxis`.
impl From<Sizing> for engine::SizingAxis {
    fn from(value: Sizing) -> Self {
        match value {
            Sizing::Fit(min, max) => Self {
                type_: engine::SizingType::Fit,
                min_max: engine::SizingMinMax { min, max },
                percent: 0.0,
            },
            Sizing::Grow(min, max) => Self {
                type_: engine::SizingType::Grow,
                min_max: engine::SizingMinMax { min, max },
                percent: 0.0,
            },
            Sizing::Fixed(size) => Self {
                type_: engine::SizingType::Fixed,
                min_max: engine::SizingMinMax {
                    min: size,
                    max: size,
                },
                percent: 0.0,
            },
            Sizing::Percent(percent) => Self {
                type_: engine::SizingType::Percent,
                min_max: engine::SizingMinMax { min: 0.0, max: 0.0 },
                percent,
            },
        }
    }
}

/// Represents padding values for each side of an element.
#[derive(Debug, Default)]
pub struct Padding {
    /// Padding on the left side.
    pub left: u16,
    /// Padding on the right side.
    pub right: u16,
    /// Padding on the top side.
    pub top: u16,
    /// Padding on the bottom side.
    pub bottom: u16,
}

impl Padding {
    /// Creates a new `Padding` with individual values for each side.
    pub fn new(left: u16, right: u16, top: u16, bottom: u16) -> Self {
        Self {
            left,
            right,
            top,
            bottom,
        }
    }

    /// Sets the same padding value for all sides.
    pub fn all(value: u16) -> Self {
        Self::new(value, value, value, value)
    }

    /// Sets the same padding for left and right sides.
    /// Top and bottom are set to `0`.
    pub fn horizontal(value: u16) -> Self {
        Self::new(value, value, 0, 0)
    }

    /// Sets the same padding for top and bottom sides.
    /// Left and right are set to `0`.
    pub fn vertical(value: u16) -> Self {
        Self::new(0, 0, value, value)
    }
}

impl From<u16> for Padding {
    /// Creates padding with the same value for all sides.
    fn from(value: u16) -> Self {
        Self::all(value)
    }
}

impl From<(u16, u16, u16, u16)> for Padding {
    /// Creates padding from a tuple in CSS order: (top, right, bottom, left).
    fn from((top, right, bottom, left): (u16, u16, u16, u16)) -> Self {
        Self { left, right, top, bottom }
    }
}

/// Defines the layout direction for arranging child elements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum LayoutDirection {
    /// Arranges elements from left to right.
    #[default]
    LeftToRight,
    /// Arranges elements from top to bottom.
    TopToBottom,
}

/// Builder for configuring layout properties using a closure.
/// No lifetime parameters — works cleanly with closures.
pub struct LayoutBuilder {
    pub(crate) config: engine::LayoutConfig,
}

impl LayoutBuilder {
    /// Sets the spacing between child elements.
    #[inline]
    pub fn gap(&mut self, gap: u16) -> &mut Self {
        self.config.child_gap = gap;
        self
    }

    /// Sets the alignment of child elements using separate X and Y values.
    #[inline]
    pub fn align(&mut self, x: AlignX, y: AlignY) -> &mut Self {
        self.config.child_alignment.x = x;
        self.config.child_alignment.y = y;
        self
    }

    /// Sets the layout direction.
    #[inline]
    pub fn direction(&mut self, direction: LayoutDirection) -> &mut Self {
        self.config.layout_direction = direction;
        self
    }

    /// Sets padding values for the layout.
    #[inline]
    pub fn padding(&mut self, padding: impl Into<Padding>) -> &mut Self {
        let padding = padding.into();
        self.config.padding.left = padding.left;
        self.config.padding.right = padding.right;
        self.config.padding.top = padding.top;
        self.config.padding.bottom = padding.bottom;
        self
    }
}

/// Shorthand macro for [`Sizing::Fit`]. Defaults max to `f32::MAX` if omitted.
#[macro_export]
macro_rules! fit {
    ($min:expr, $max:expr) => {
        $crate::layout::Sizing::Fit($min, $max)
    };
    ($min:expr) => {
        fit!($min, f32::MAX)
    };
    () => {
        fit!(0.0)
    };
}

/// Shorthand macro for [`Sizing::Grow`]. Defaults max to `f32::MAX` if omitted.
#[macro_export]
macro_rules! grow {
    ($min:expr, $max:expr) => {
        $crate::layout::Sizing::Grow($min, $max)
    };
    ($min:expr) => {
        grow!($min, f32::MAX)
    };
    () => {
        grow!(0.0)
    };
}

/// Shorthand macro for [`Sizing::Fixed`].
#[macro_export]
macro_rules! fixed {
    ($val:expr) => {
        $crate::layout::Sizing::Fixed($val)
    };
}

/// Shorthand macro for [`Sizing::Percent`].
/// The value has to be in range `0.0..=1.0`.
#[macro_export]
macro_rules! percent {
    ($percent:expr) => {{
        const _: () = assert!(
            $percent >= 0.0 && $percent <= 1.0,
            "Percent value must be between 0.0 and 1.0 inclusive!"
        );
        $crate::layout::Sizing::Percent($percent)
    }};
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn fit_macro() {
        let both_args = fit!(12.0, 34.0);
        assert!(matches!(both_args, Sizing::Fit(12.0, 34.0)));

        let one_arg = fit!(12.0);
        assert!(matches!(one_arg, Sizing::Fit(12.0, f32::MAX)));

        let zero_args = fit!();
        assert!(matches!(zero_args, Sizing::Fit(0.0, f32::MAX)));
    }

    #[test]
    fn grow_macro() {
        let both_args = grow!(12.0, 34.0);
        assert!(matches!(both_args, Sizing::Grow(12.0, 34.0)));

        let one_arg = grow!(12.0);
        assert!(matches!(one_arg, Sizing::Grow(12.0, f32::MAX)));

        let zero_args = grow!();
        assert!(matches!(zero_args, Sizing::Grow(0.0, f32::MAX)));
    }

    #[test]
    fn fixed_macro() {
        let value = fixed!(123.0);
        assert!(matches!(value, Sizing::Fixed(123.0)));
    }

    #[test]
    fn percent_macro() {
        let value = percent!(0.5);
        assert!(matches!(value, Sizing::Percent(0.5)));
    }
}
