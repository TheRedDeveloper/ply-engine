use crate::{color::Color, Dimensions, Vector2, engine};

/// Represents different attachment points for floating elements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum FloatingAttachPointType {
    /// Attaches to the top-left of the parent.
    #[default]
    LeftTop,
    /// Attaches to the center-left of the parent.
    LeftCenter,
    /// Attaches to the bottom-left of the parent.
    LeftBottom,
    /// Attaches to the top-center of the parent.
    CenterTop,
    /// Attaches to the center of the parent.
    CenterCenter,
    /// Attaches to the bottom-center of the parent.
    CenterBottom,
    /// Attaches to the top-right of the parent.
    RightTop,
    /// Attaches to the center-right of the parent.
    RightCenter,
    /// Attaches to the bottom-right of the parent.
    RightBottom,
}

/// Specifies how pointer capture should behave for floating elements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum PointerCaptureMode {
    /// Captures all pointer input.
    #[default]
    Capture,
    /// Allows pointer input to pass through.
    Passthrough,
}

/// Defines how a floating element is attached to other elements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum FloatingAttachToElement {
    /// The floating element is not attached to any other element.
    #[default]
    None,
    /// The floating element is attached to its parent element.
    Parent,
    /// The floating element is attached to a specific element identified by an ID.
    ElementWithId,
    /// The floating element is attached to the root of the layout.
    Root,
}

/// Defines how a floating element is clipped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum FloatingClipToElement {
    /// The floating element is not clipped.
    #[default]
    None,
    /// The floating element is clipped to the attached parent.
    AttachedParent,
}

/// Builder for configuring floating element properties using a closure.
pub struct FloatingBuilder {
    pub(crate) config: engine::FloatingConfig,
}

impl FloatingBuilder {
    /// Sets the floating element's offset.
    #[inline]
    pub fn offset(&mut self, x: f32, y: f32) -> &mut Self {
        self.config.offset = Vector2::new(x, y);
        self
    }

    /// Sets the floating element's dimensions.
    #[inline]
    pub fn dimensions(&mut self, dimensions: Dimensions) -> &mut Self {
        self.config.expand = dimensions;
        self
    }

    /// Sets the floating element's Z-index.
    #[inline]
    pub fn z_index(&mut self, z_index: i16) -> &mut Self {
        self.config.z_index = z_index;
        self
    }

    /// Sets the parent element ID.
    #[inline]
    pub fn parent_id(&mut self, id: u32) -> &mut Self {
        self.config.parent_id = id;
        self
    }

    /// Sets the attachment points of the floating element and its parent.
    #[inline]
    pub fn anchor(
        &mut self,
        element: FloatingAttachPointType,
        parent: FloatingAttachPointType,
    ) -> &mut Self {
        self.config.attach_points.element = element;
        self.config.attach_points.parent = parent;
        self
    }

    /// Sets how the floating element is attached to other elements.
    #[inline]
    pub fn attach(&mut self, attach: FloatingAttachToElement) -> &mut Self {
        self.config.attach_to = attach;
        self
    }

    /// Sets pointer capture mode to Passthrough.
    #[inline]
    pub fn passthrough(&mut self) -> &mut Self {
        self.config.pointer_capture_mode = PointerCaptureMode::Passthrough;
        self
    }

    /// Sets the pointer capture mode.
    #[inline]
    pub fn pointer_capture_mode(&mut self, mode: PointerCaptureMode) -> &mut Self {
        self.config.pointer_capture_mode = mode;
        self
    }
}

/// Builder for configuring border properties using a closure.
pub struct BorderBuilder {
    pub(crate) config: engine::BorderConfig,
}

impl BorderBuilder {
    /// Sets the border color.
    #[inline]
    pub fn color(&mut self, color: impl Into<Color>) -> &mut Self {
        self.config.color = color.into();
        self
    }

    /// Set the same border width for all sides.
    #[inline]
    pub fn all(&mut self, width: u16) -> &mut Self {
        self.config.width.left = width;
        self.config.width.right = width;
        self.config.width.top = width;
        self.config.width.bottom = width;
        self
    }

    /// Sets the left border width.
    #[inline]
    pub fn left(&mut self, width: u16) -> &mut Self {
        self.config.width.left = width;
        self
    }

    /// Sets the right border width.
    #[inline]
    pub fn right(&mut self, width: u16) -> &mut Self {
        self.config.width.right = width;
        self
    }

    /// Sets the top border width.
    #[inline]
    pub fn top(&mut self, width: u16) -> &mut Self {
        self.config.width.top = width;
        self
    }

    /// Sets the bottom border width.
    #[inline]
    pub fn bottom(&mut self, width: u16) -> &mut Self {
        self.config.width.bottom = width;
        self
    }

    /// Sets the spacing between child elements.
    #[inline]
    pub fn between_children(&mut self, width: u16) -> &mut Self {
        self.config.width.between_children = width;
        self
    }
}

/// Builder for configuring visual rotation (render-target based).
pub struct VisualRotationBuilder {
    pub(crate) config: engine::VisualRotationConfig,
}

impl VisualRotationBuilder {
    /// Sets the rotation angle in degrees.
    #[inline]
    pub fn degrees(&mut self, degrees: f32) -> &mut Self {
        self.config.rotation_radians = degrees.to_radians();
        self
    }

    /// Sets the rotation angle in radians.
    #[inline]
    pub fn radians(&mut self, radians: f32) -> &mut Self {
        self.config.rotation_radians = radians;
        self
    }

    /// Sets the rotation pivot as normalized coordinates (0.0–1.0).
    /// Default is (0.5, 0.5) = center of the element.
    /// (0.0, 0.0) = top-left corner.
    #[inline]
    pub fn pivot(&mut self, x: f32, y: f32) -> &mut Self {
        self.config.pivot_x = x;
        self.config.pivot_y = y;
        self
    }

    /// Flips the element horizontally (mirror across the vertical axis).
    #[inline]
    pub fn flip_x(&mut self) -> &mut Self {
        self.config.flip_x = true;
        self
    }

    /// Flips the element vertically (mirror across the horizontal axis).
    #[inline]
    pub fn flip_y(&mut self) -> &mut Self {
        self.config.flip_y = true;
        self
    }
}

/// Builder for configuring shape rotation (vertex-level).
pub struct ShapeRotationBuilder {
    pub(crate) config: engine::ShapeRotationConfig,
}

impl ShapeRotationBuilder {
    /// Sets the rotation angle in degrees.
    #[inline]
    pub fn degrees(&mut self, degrees: f32) -> &mut Self {
        self.config.rotation_radians = degrees.to_radians();
        self
    }

    /// Sets the rotation angle in radians.
    #[inline]
    pub fn radians(&mut self, radians: f32) -> &mut Self {
        self.config.rotation_radians = radians;
        self
    }

    /// Flips the shape horizontally (applied before rotation).
    #[inline]
    pub fn flip_x(&mut self) -> &mut Self {
        self.config.flip_x = true;
        self
    }

    /// Flips the shape vertically (applied before rotation).
    #[inline]
    pub fn flip_y(&mut self) -> &mut Self {
        self.config.flip_y = true;
        self
    }
}