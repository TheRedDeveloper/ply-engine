use crate::align::{AlignX, AlignY};
use crate::id::Id;
use crate::{color::Color, Dimensions, Vector2, engine};

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

    /// Sets the attachment points of the floating element and its parent.
    ///
    /// Each tuple is `(AlignX, AlignY)` — the first for the element, the second for the parent.
    /// ```ignore
    /// .floating(|f| f.anchor((CenterX, Bottom), (CenterX, Top)))
    /// ```
    #[inline]
    pub fn anchor(
        &mut self,
        element: (AlignX, AlignY),
        parent: (AlignX, AlignY),
    ) -> &mut Self {
        self.config.attach_points.element_x = element.0;
        self.config.attach_points.element_y = element.1;
        self.config.attach_points.parent_x = parent.0;
        self.config.attach_points.parent_y = parent.1;
        self
    }

    /// Attaches this floating element to its parent element (default behavior).
    #[inline]
    pub fn attach_parent(&mut self) -> &mut Self {
        self.config.attach_to = FloatingAttachToElement::Parent;
        self
    }

    /// Attaches this floating element to the root of the layout.
    #[inline]
    pub fn attach_root(&mut self) -> &mut Self {
        self.config.attach_to = FloatingAttachToElement::Root;
        self
    }

    /// Attaches this floating element to a specific element by ID.
    #[inline]
    pub fn attach_id(&mut self, id: impl Into<Id>) -> &mut Self {
        self.config.attach_to = FloatingAttachToElement::ElementWithId;
        self.config.parent_id = id.into().id;
        self
    }

    /// Clips this floating element to its parent's bounds.
    #[inline]
    pub fn clip_by_parent(&mut self) -> &mut Self {
        self.config.clip_to = FloatingClipToElement::AttachedParent;
        self
    }

    /// Sets pointer capture mode to Passthrough.
    #[inline]
    pub fn passthrough(&mut self) -> &mut Self {
        self.config.pointer_capture_mode = PointerCaptureMode::Passthrough;
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