use crate::{color::Color, engine, math::BoundingBox, renderer::GraphicAsset, shaders::ShaderConfig};

/// Represents a rectangle with a specified color and corner radii.
#[derive(Debug, Clone)]
pub struct Rectangle {
    /// The fill color of the rectangle.
    pub color: Color,
    /// The corner radii for rounded edges.
    pub corner_radii: CornerRadii,
}

/// Represents a text element with styling attributes.
#[derive(Debug, Clone)]
pub struct Text {
    /// The text content.
    pub text: String,
    /// The color of the text.
    pub color: Color,
    /// The ID of the font used.
    pub font_id: u16,
    /// The font size.
    pub font_size: u16,
    /// The spacing between letters.
    pub letter_spacing: u16,
    /// The line height.
    pub line_height: u16,
}

/// Defines individual corner radii for an element.
#[derive(Debug, Clone)]
pub struct CornerRadii {
    /// The radius for the top-left corner.
    pub top_left: f32,
    /// The radius for the top-right corner.
    pub top_right: f32,
    /// The radius for the bottom-left corner.
    pub bottom_left: f32,
    /// The radius for the bottom-right corner.
    pub bottom_right: f32,
}

/// Defines the border width for each side of an element.
#[derive(Debug, Clone)]
pub struct BorderWidth {
    /// Border width on the left side.
    pub left: u16,
    /// Border width on the right side.
    pub right: u16,
    /// Border width on the top side.
    pub top: u16,
    /// Border width on the bottom side.
    pub bottom: u16,
    /// Border width between child elements.
    pub between_children: u16,
}

/// Represents a border with a specified color, width, and corner radii.
#[derive(Debug, Clone)]
pub struct Border {
    /// The border color.
    pub color: Color,
    /// The corner radii for rounded border edges.
    pub corner_radii: CornerRadii,
    /// The width of the border on each side.
    pub width: BorderWidth,
}

/// Represents an image with defined dimensions and data.
#[derive(Debug, Clone)]
pub struct Image {
    /// Background color
    pub background_color: Color,
    /// The corner radii for rounded border edges.
    pub corner_radii: CornerRadii,
    /// A reference to the asset data.
    pub data: &'static GraphicAsset,
}

/// Represents a custom element with a background color, corner radii, and associated data.
#[derive(Debug, Clone)]
pub struct Custom<CustomElementData> {
    /// The background color of the custom element.
    pub background_color: Color,
    /// The corner radii for rounded edges.
    pub corner_radii: CornerRadii,
    /// The custom element data.
    pub data: CustomElementData,
}

impl From<engine::CornerRadius> for CornerRadii {
    fn from(value: engine::CornerRadius) -> Self {
        Self {
            top_left: value.top_left,
            top_right: value.top_right,
            bottom_left: value.bottom_left,
            bottom_right: value.bottom_right,
        }
    }
}

#[derive(Debug, Clone)]
pub enum RenderCommandConfig<CustomElementData> {
    None(),
    Rectangle(Rectangle),
    Border(Border),
    Text(Text),
    Image(Image),
    ScissorStart(),
    ScissorEnd(),
    Custom(Custom<CustomElementData>),
    /// Begin a group shader — renders children to an offscreen buffer,
    /// then applies a fragment shader as a post-process.
    ShaderBegin(ShaderConfig),
    /// End a group shader — pops the render target and composites.
    ShaderEnd,
}

impl<CustomElementData: Clone + Default + std::fmt::Debug>
    RenderCommandConfig<CustomElementData>
{
    pub(crate) fn from_engine_render_command(value: &engine::InternalRenderCommand<CustomElementData>) -> Self {
        match value.command_type {
            engine::RenderCommandType::None => Self::None(),
            engine::RenderCommandType::Rectangle => {
                if let engine::InternalRenderData::Rectangle { background_color, corner_radius } = &value.render_data {
                    Self::Rectangle(Rectangle {
                        color: *background_color,
                        corner_radii: (*corner_radius).into(),
                    })
                } else {
                    Self::None()
                }
            }
            engine::RenderCommandType::Text => {
                if let engine::InternalRenderData::Text { text, text_color, font_id, font_size, letter_spacing, line_height } = &value.render_data {
                    Self::Text(Text {
                        text: text.clone(),
                        color: *text_color,
                        font_id: *font_id,
                        font_size: *font_size,
                        letter_spacing: *letter_spacing,
                        line_height: *line_height,
                    })
                } else {
                    Self::None()
                }
            }
            engine::RenderCommandType::Border => {
                if let engine::InternalRenderData::Border { color, corner_radius, width } = &value.render_data {
                    Self::Border(Border {
                        color: *color,
                        corner_radii: (*corner_radius).into(),
                        width: BorderWidth {
                            left: width.left,
                            right: width.right,
                            top: width.top,
                            bottom: width.bottom,
                            between_children: width.between_children,
                        },
                    })
                } else {
                    Self::None()
                }
            }
            engine::RenderCommandType::Image => {
                if let engine::InternalRenderData::Image { background_color, corner_radius, image_data } = &value.render_data {
                    Self::Image(Image {
                        data: *image_data,
                        corner_radii: (*corner_radius).into(),
                        background_color: *background_color,
                    })
                } else {
                    Self::None()
                }
            }
            engine::RenderCommandType::ScissorStart => Self::ScissorStart(),
            engine::RenderCommandType::ScissorEnd => Self::ScissorEnd(),
            engine::RenderCommandType::ShaderBegin => {
                // ShaderBegin uses the first effect from the render command as its config
                if let Some(config) = value.effects.first() {
                    Self::ShaderBegin(config.clone())
                } else {
                    Self::None()
                }
            }
            engine::RenderCommandType::ShaderEnd => Self::ShaderEnd,
            engine::RenderCommandType::Custom => {
                if let engine::InternalRenderData::Custom { background_color, corner_radius, custom_data } = &value.render_data {
                    Self::Custom(Custom {
                        background_color: *background_color,
                        corner_radii: (*corner_radius).into(),
                        data: custom_data.clone(),
                    })
                } else {
                    Self::None()
                }
            }
        }
    }
}

/// Represents a render command for drawing an element on the screen.
#[derive(Debug, Clone)]
pub struct RenderCommand<CustomElementData> {
    /// The bounding box defining the area occupied by the element.
    pub bounding_box: BoundingBox,
    /// The specific configuration for rendering this command.
    pub config: RenderCommandConfig<CustomElementData>,
    /// A unique identifier for the render command.
    pub id: u32,
    /// The z-index determines the stacking order of elements.
    /// Higher values are drawn above lower values.
    pub z_index: i16,
    /// Per-element shader effects (chained in order).
    pub effects: Vec<ShaderConfig>,
}

impl<CustomElementData: Clone + Default + std::fmt::Debug> RenderCommand<CustomElementData> {
    pub(crate) fn from_engine_render_command(value: &engine::InternalRenderCommand<CustomElementData>) -> Self {
        Self {
            id: value.id,
            z_index: value.z_index,
            bounding_box: value.bounding_box,
            config: RenderCommandConfig::from_engine_render_command(value),
            effects: value.effects.clone(),
        }
    }
}
