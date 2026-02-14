use crate::{bindings::*, Declaration};

/// Defines different sizing behaviors for an element.
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum SizingType {
    /// The element's size is determined by its content and constrained by min/max values.
    Fit = Clay__SizingType_CLAY__SIZING_TYPE_FIT,
    /// The element expands to fill available space within min/max constraints.
    Grow = Clay__SizingType_CLAY__SIZING_TYPE_GROW,
    /// The element's size is fixed to a percentage of its parent.
    Percent = Clay__SizingType_CLAY__SIZING_TYPE_PERCENT,
    /// The element's size is set to a fixed value.
    Fixed = Clay__SizingType_CLAY__SIZING_TYPE_FIXED,
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

/// Converts a `Sizing` value into a `Clay_SizingAxis` representation.
impl From<Sizing> for Clay_SizingAxis {
    fn from(value: Sizing) -> Self {
        match value {
            Sizing::Fit(min, max) => Self {
                type_: SizingType::Fit as _,
                size: Clay_SizingAxis__bindgen_ty_1 {
                    minMax: Clay_SizingMinMax { min, max },
                },
            },
            Sizing::Grow(min, max) => Self {
                type_: SizingType::Grow as _,
                size: Clay_SizingAxis__bindgen_ty_1 {
                    minMax: Clay_SizingMinMax { min, max },
                },
            },
            Sizing::Fixed(size) => Self {
                type_: SizingType::Fixed as _,
                size: Clay_SizingAxis__bindgen_ty_1 {
                    minMax: Clay_SizingMinMax {
                        min: size,
                        max: size,
                    },
                },
            },
            Sizing::Percent(percent) => Self {
                type_: SizingType::Percent as _,
                size: Clay_SizingAxis__bindgen_ty_1 { percent },
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

/// Represents horizontal alignment options for layout elements.
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum LayoutAlignmentX {
    /// Aligns to the left.
    Left = Clay_LayoutAlignmentX_CLAY_ALIGN_X_LEFT,
    /// Centers the element.
    Center = Clay_LayoutAlignmentX_CLAY_ALIGN_X_CENTER,
    /// Aligns to the right.
    Right = Clay_LayoutAlignmentX_CLAY_ALIGN_X_RIGHT,
}

/// Represents vertical alignment options for layout elements.
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum LayoutAlignmentY {
    /// Aligns to the top.
    Top = Clay_LayoutAlignmentY_CLAY_ALIGN_Y_TOP,
    /// Centers the element.
    Center = Clay_LayoutAlignmentY_CLAY_ALIGN_Y_CENTER,
    /// Aligns to the bottom.
    Bottom = Clay_LayoutAlignmentY_CLAY_ALIGN_Y_BOTTOM,
}

/// Controls child alignment within a layout.
#[derive(Debug, Copy, Clone)]
pub struct Alignment {
    pub x: LayoutAlignmentX,
    pub y: LayoutAlignmentY,
}

impl Alignment {
    /// Creates a new alignment setting for a layout.
    pub fn new(x: LayoutAlignmentX, y: LayoutAlignmentY) -> Self {
        Self { x, y }
    }
}

/// Defines the layout direction for arranging child elements.
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum LayoutDirection {
    /// Arranges elements from left to right.
    LeftToRight = Clay_LayoutDirection_CLAY_LEFT_TO_RIGHT,
    /// Arranges elements from top to bottom.
    TopToBottom = Clay_LayoutDirection_CLAY_TOP_TO_BOTTOM,
}

/// Builder for configuring layout properties in a `Declaration`.
pub struct LayoutBuilder<
    'declaration,
    'render,
    ImageElementData: 'render,
    CustomElementData: 'render,
> {
    parent: &'declaration mut Declaration<'render, ImageElementData, CustomElementData>,
}

impl<'declaration, 'render, ImageElementData: 'render, CustomElementData: 'render>
    LayoutBuilder<'declaration, 'render, ImageElementData, CustomElementData>
{
    /// Creates a new `LayoutBuilder` with the given parent `Declaration`.
    #[inline]
    pub fn new(
        parent: &'declaration mut Declaration<'render, ImageElementData, CustomElementData>,
    ) -> Self {
        LayoutBuilder { parent }
    }

    /// Sets the width of the layout.
    #[inline]
    pub fn width(&mut self, width: Sizing) -> &mut Self {
        self.parent.inner.layout.sizing.width = width.into();
        self
    }

    /// Sets the height of the layout.
    #[inline]
    pub fn height(&mut self, height: Sizing) -> &mut Self {
        self.parent.inner.layout.sizing.height = height.into();
        self
    }

    /// Sets padding values for the layout.
    #[inline]
    pub fn padding(&mut self, padding: Padding) -> &mut Self {
        self.parent.inner.layout.padding.left = padding.left;
        self.parent.inner.layout.padding.right = padding.right;
        self.parent.inner.layout.padding.top = padding.top;
        self.parent.inner.layout.padding.bottom = padding.bottom;
        self
    }

    /// Sets the spacing between child elements.
    #[inline]
    pub fn child_gap(&mut self, child_gap: u16) -> &mut Self {
        self.parent.inner.layout.childGap = child_gap;
        self
    }

    /// Sets the alignment of child elements.
    #[inline]
    pub fn child_alignment(&mut self, child_alignment: Alignment) -> &mut Self {
        self.parent.inner.layout.childAlignment.x = child_alignment.x as _;
        self.parent.inner.layout.childAlignment.y = child_alignment.y as _;
        self
    }

    /// Sets the layout direction.
    #[inline]
    pub fn direction(&mut self, direction: LayoutDirection) -> &mut Self {
        self.parent.inner.layout.layoutDirection = direction as _;
        self
    }

    /// Returns the modified `Declaration`.
    #[inline]
    pub fn end(&mut self) -> &mut Declaration<'render, ImageElementData, CustomElementData> {
        self.parent
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
