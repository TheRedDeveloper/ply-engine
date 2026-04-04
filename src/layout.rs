use crate::align::{AlignX, AlignY};
use crate::engine;

/// Per-corner border radius for rounded rectangles.
#[derive(Debug, Clone, Copy, Default)]
pub struct CornerRadius {
    pub top_left: f32,
    pub top_right: f32,
    pub bottom_left: f32,
    pub bottom_right: f32,
}

impl CornerRadius {
    /// Returns `true` when all four corners have a radius of zero.
    pub fn is_zero(&self) -> bool {
        self.top_left == 0.0
            && self.top_right == 0.0
            && self.bottom_left == 0.0
            && self.bottom_right == 0.0
    }
}

impl From<f32> for CornerRadius {
    /// Creates a corner radius with the same value for all corners.
    fn from(value: f32) -> Self {
        Self {
            top_left: value,
            top_right: value,
            bottom_left: value,
            bottom_right: value,
        }
    }
}

impl From<(f32, f32, f32, f32)> for CornerRadius {
    /// Creates corner radii from a tuple in CSS order: (top-left, top-right, bottom-right, bottom-left).
    fn from((tl, tr, br, bl): (f32, f32, f32, f32)) -> Self {
        Self {
            top_left: tl,
            top_right: tr,
            bottom_left: bl,
            bottom_right: br,
        }
    }
}

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
    ///
    /// The third argument is the grow weight. A weight of `1.0` is the default behavior.
    /// A weight of `0.0` means the element does not grow (behaves like `Fit` in practice).
    Grow(f32, f32, f32),
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
                grow_weight: 1.0,
            },
            Sizing::Grow(min, max, weight) => {
                assert!(weight >= 0.0, "Grow weight must be non-negative.");

                if weight == 0.0 {
                    Self {
                        type_: engine::SizingType::Fit,
                        min_max: engine::SizingMinMax { min, max },
                        percent: 0.0,
                        grow_weight: 1.0,
                    }
                } else {
                    Self {
                        type_: engine::SizingType::Grow,
                        min_max: engine::SizingMinMax { min, max },
                        percent: 0.0,
                        grow_weight: weight,
                    }
                }
            }
            Sizing::Fixed(size) => Self {
                type_: engine::SizingType::Fixed,
                min_max: engine::SizingMinMax {
                    min: size,
                    max: size,
                },
                percent: 0.0,
                grow_weight: 1.0,
            },
            Sizing::Percent(percent) => Self {
                type_: engine::SizingType::Percent,
                min_max: engine::SizingMinMax { min: 0.0, max: 0.0 },
                percent,
                grow_weight: 1.0,
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

    ($($name:ident : $value:expr),+ $(,)?) => {
        $crate::fit!(@named (0.0, f32::MAX); $($name : $value,)+)
    };

    (@named ($min:expr, $max:expr); ) => {
        $crate::layout::Sizing::Fit($min, $max)
    };
    (@named ($min:expr, $max:expr); min : $value:expr, $($rest:tt)*) => {
        $crate::fit!(@named ($value, $max); $($rest)*)
    };
    (@named ($min:expr, $max:expr); max : $value:expr, $($rest:tt)*) => {
        $crate::fit!(@named ($min, $value); $($rest)*)
    };
    (@named ($min:expr, $max:expr); $unknown:ident : $value:expr, $($rest:tt)*) => {
        compile_error!("Unknown named argument for fit!(). Expected: min, max.");
    };

    ($first:expr, $($rest:tt)+) => {
        compile_error!("Do not mix positional and named arguments in fit!().");
    };
}

/// Shorthand macro for [`Sizing::Grow`]. Defaults max to `f32::MAX` and weight to `1.0` if omitted.
#[macro_export]
macro_rules! grow {
    ($min:expr, $max:expr, $weight:expr) => {
        $crate::layout::Sizing::Grow($min, $max, $weight)
    };
    ($min:expr, $max:expr) => {
        grow!($min, $max, 1.0)
    };
    ($min:expr) => {
        grow!($min, f32::MAX)
    };
    () => {
        grow!(0.0)
    };

    ($($name:ident : $value:expr),+ $(,)?) => {
        $crate::grow!(@named (0.0, f32::MAX, 1.0); $($name : $value,)+)
    };

    (@named ($min:expr, $max:expr, $weight:expr); ) => {
        $crate::layout::Sizing::Grow($min, $max, $weight)
    };
    (@named ($min:expr, $max:expr, $weight:expr); min : $value:expr, $($rest:tt)*) => {
        $crate::grow!(@named ($value, $max, $weight); $($rest)*)
    };
    (@named ($min:expr, $max:expr, $weight:expr); max : $value:expr, $($rest:tt)*) => {
        $crate::grow!(@named ($min, $value, $weight); $($rest)*)
    };
    (@named ($min:expr, $max:expr, $weight:expr); weight : $value:expr, $($rest:tt)*) => {
        $crate::grow!(@named ($min, $max, $value); $($rest)*)
    };
    (@named ($min:expr, $max:expr, $weight:expr); $unknown:ident : $value:expr, $($rest:tt)*) => {
        compile_error!("Unknown named argument for grow!(). Expected: min, max, weight.");
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

        let named_max = fit!(max: 34.0);
        assert!(matches!(named_max, Sizing::Fit(0.0, 34.0)));

        let named_min = fit!(min: 12.0);
        assert!(matches!(named_min, Sizing::Fit(12.0, f32::MAX)));

        let named_both = fit!(max: 34.0, min: 12.0);
        assert!(matches!(named_both, Sizing::Fit(12.0, 34.0)));
    }

    #[test]
    fn grow_macro() {
        let three_args = grow!(12.0, 34.0, 2.5);
        assert!(matches!(three_args, Sizing::Grow(12.0, 34.0, 2.5)));

        let both_args = grow!(12.0, 34.0);
        assert!(matches!(both_args, Sizing::Grow(12.0, 34.0, 1.0)));

        let one_arg = grow!(12.0);
        assert!(matches!(one_arg, Sizing::Grow(12.0, f32::MAX, 1.0)));

        let zero_args = grow!();
        assert!(matches!(zero_args, Sizing::Grow(0.0, f32::MAX, 1.0)));

        let named_weight = grow!(weight: 2.0);
        assert!(matches!(named_weight, Sizing::Grow(0.0, f32::MAX, 2.0)));

        let named_min_weight = grow!(min: 12.0, weight: 2.0);
        assert!(matches!(named_min_weight, Sizing::Grow(12.0, f32::MAX, 2.0)));

        let named_max_weight = grow!(max: 34.0, weight: 3.0);
        assert!(matches!(named_max_weight, Sizing::Grow(0.0, 34.0, 3.0)));

        let named_all = grow!(weight: 2.0, max: 34.0, min: 12.0);
        assert!(matches!(named_all, Sizing::Grow(12.0, 34.0, 2.0)));
    }

    #[test]
    fn zero_weight_grow_converts_to_fit_axis() {
        let axis: engine::SizingAxis = grow!(12.0, 34.0, 0.0).into();
        assert_eq!(axis.type_, engine::SizingType::Fit);
        assert_eq!(axis.min_max.min, 12.0);
        assert_eq!(axis.min_max.max, 34.0);
    }

    #[test]
    #[should_panic(expected = "Grow weight must be non-negative.")]
    fn negative_grow_weight_panics() {
        let _axis: engine::SizingAxis = grow!(0.0, f32::MAX, -1.0).into();
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
