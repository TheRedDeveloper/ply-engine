use crate::{color::Color, Declaration, Dimensions, Vector2, renderer::Asset};

/// Builder for configuring border properties of a `Declaration`.
pub struct BorderBuilder<
    'declaration,
    'render,
    CustomElementData: 'render,
> {
    parent: &'declaration mut Declaration<'render, CustomElementData>,
}

impl<'declaration, 'render, CustomElementData: 'render>
    BorderBuilder<'declaration, 'render, CustomElementData>
{
    /// Creates a new `BorderBuilder` with the given parent `Declaration`.
    #[inline]
    pub fn new(
        parent: &'declaration mut Declaration<'render, CustomElementData>,
    ) -> Self {
        BorderBuilder { parent }
    }

    /// Set the same border width for all sides.
    #[inline]
    pub fn all_directions(&mut self, width: u16) -> &mut Self {
        self.parent.inner.border.width.left = width;
        self.parent.inner.border.width.right = width;
        self.parent.inner.border.width.top = width;
        self.parent.inner.border.width.bottom = width;
        self
    }

    /// Sets the left border width.
    #[inline]
    pub fn left(&mut self, width: u16) -> &mut Self {
        self.parent.inner.border.width.left = width;
        self
    }

    /// Sets the right border width.
    #[inline]
    pub fn right(&mut self, width: u16) -> &mut Self {
        self.parent.inner.border.width.right = width;
        self
    }

    /// Sets the top border width.
    #[inline]
    pub fn top(&mut self, width: u16) -> &mut Self {
        self.parent.inner.border.width.top = width;
        self
    }

    /// Sets the bottom border width.
    #[inline]
    pub fn bottom(&mut self, width: u16) -> &mut Self {
        self.parent.inner.border.width.bottom = width;
        self
    }

    /// Sets the spacing between child elements.
    #[inline]
    pub fn between_children(&mut self, width: u16) -> &mut Self {
        self.parent.inner.border.width.between_children = width;
        self
    }

    /// Sets the border color.
    #[inline]
    pub fn color(&mut self, color: Color) -> &mut Self {
        self.parent.inner.border.color = color.into();
        self
    }

    /// Returns the modified `Declaration`.
    #[inline]
    pub fn end(&mut self) -> &mut Declaration<'render, CustomElementData> {
        self.parent
    }
}

/// Builder for configuring image properties in a `Declaration`.
pub struct ImageBuilder<
    'declaration,
    'render,
    CustomElementData: 'render,
> {
    parent: &'declaration mut Declaration<'render, CustomElementData>,
}

impl<'declaration, 'render, CustomElementData: 'render>
    ImageBuilder<'declaration, 'render, CustomElementData>
{
    /// Creates a new `ImageBuilder` with the given parent `Declaration`.
    #[inline]
    pub fn new(
        parent: &'declaration mut Declaration<'render, CustomElementData>,
    ) -> Self {
        ImageBuilder { parent }
    }

    /// Sets the image data.
    /// The data must be created using [`Ply::data`].
    #[inline]
    pub fn data(&mut self, data: &'static Asset) -> &mut Self {
        self.parent.inner.image_data = data as *const Asset as usize;
        self
    }
    /// Returns the modified `Declaration`.
    #[inline]
    pub fn end(&mut self) -> &mut Declaration<'render, CustomElementData> {
        self.parent
    }
}

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

/// Builder for configuring floating element properties in a `Declaration`.
pub struct FloatingBuilder<
    'declaration,
    'render,
    CustomElementData: 'render,
> {
    parent: &'declaration mut Declaration<'render, CustomElementData>,
}

impl<'declaration, 'render, CustomElementData: 'render>
    FloatingBuilder<'declaration, 'render, CustomElementData>
{
    /// Creates a new `FloatingBuilder` with the given parent `Declaration`.
    #[inline]
    pub fn new(
        parent: &'declaration mut Declaration<'render, CustomElementData>,
    ) -> Self {
        FloatingBuilder { parent }
    }

    /// Sets the floating element's offset.
    #[inline]
    pub fn offset(&mut self, offset: Vector2) -> &mut Self {
        self.parent.inner.floating.offset = offset;
        self
    }

    /// Sets the floating element's dimensions.
    #[inline]
    pub fn dimensions(&mut self, dimensions: Dimensions) -> &mut Self {
        self.parent.inner.floating.expand = dimensions;
        self
    }

    /// Sets the floating element's Z-index.
    #[inline]
    pub fn z_index(&mut self, z_index: i16) -> &mut Self {
        self.parent.inner.floating.z_index = z_index;
        self
    }

    /// Sets the parent element ID.
    #[inline]
    pub fn parent_id(&mut self, id: u32) -> &mut Self {
        self.parent.inner.floating.parent_id = id;
        self
    }

    /// Sets the attachment points of the floating element and its parent.
    #[inline]
    pub fn attach_points(
        &mut self,
        element: FloatingAttachPointType,
        parent: FloatingAttachPointType,
    ) -> &mut Self {
        self.parent.inner.floating.attach_points.element = element;
        self.parent.inner.floating.attach_points.parent = parent;
        self
    }

    /// Sets how the floating element is attached to other elements.
    ///
    /// - [`FloatingAttachToElement::None`] - The element is not attached to anything.
    /// - [`FloatingAttachToElement::Parent`] - The element is attached to its parent.
    /// - [`FloatingAttachToElement::ElementWithId`] - The element is attached to a specific element by ID.
    /// - [`FloatingAttachToElement::Root`] - The element is attached to the root of the layout.
    #[inline]
    pub fn attach_to(&mut self, attach: FloatingAttachToElement) -> &mut Self {
        self.parent.inner.floating.attach_to = attach;
        self
    }

    /// Sets the pointer capture mode.
    #[inline]
    pub fn pointer_capture_mode(&mut self, mode: PointerCaptureMode) -> &mut Self {
        self.parent.inner.floating.pointer_capture_mode = mode;
        self
    }

    /// Returns the modified `Declaration`.
    #[inline]
    pub fn end(&mut self) -> &mut Declaration<'render, CustomElementData> {
        self.parent
    }
}

/// Builder for configuring corner radius properties in a `Declaration`.
pub struct CornerRadiusBuilder<
    'declaration,
    'render,
    CustomElementData: 'render,
> {
    parent: &'declaration mut Declaration<'render, CustomElementData>,
}

impl<'declaration, 'render, CustomElementData: 'render>
    CornerRadiusBuilder<'declaration, 'render, CustomElementData>
{
    /// Creates a new `CornerRadiusBuilder` with the given parent `Declaration`.
    #[inline]
    pub fn new(
        parent: &'declaration mut Declaration<'render, CustomElementData>,
    ) -> Self {
        CornerRadiusBuilder { parent }
    }

    /// Sets the top-left corner radius.
    #[inline]
    pub fn top_left(&mut self, radius: f32) -> &mut Self {
        self.parent.inner.corner_radius.top_left = radius;
        self
    }

    /// Sets the top-right corner radius.
    #[inline]
    pub fn top_right(&mut self, radius: f32) -> &mut Self {
        self.parent.inner.corner_radius.top_right = radius;
        self
    }

    /// Sets the bottom-left corner radius.
    #[inline]
    pub fn bottom_left(&mut self, radius: f32) -> &mut Self {
        self.parent.inner.corner_radius.bottom_left = radius;
        self
    }

    /// Sets the bottom-right corner radius.
    #[inline]
    pub fn bottom_right(&mut self, radius: f32) -> &mut Self {
        self.parent.inner.corner_radius.bottom_right = radius;
        self
    }

    /// Sets all four corner radii to the same value.
    #[inline]
    pub fn all(&mut self, radius: f32) -> &mut Self {
        self.parent.inner.corner_radius.top_left = radius;
        self.parent.inner.corner_radius.top_right = radius;
        self.parent.inner.corner_radius.bottom_left = radius;
        self.parent.inner.corner_radius.bottom_right = radius;
        self
    }

    /// Returns the modified `Declaration`.
    #[inline]
    pub fn end(&mut self) -> &mut Declaration<'render, CustomElementData> {
        self.parent
    }
}
