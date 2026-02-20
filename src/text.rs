use crate::align::AlignX;
use crate::color::Color;
use crate::shaders::{ShaderAsset, ShaderBuilder, ShaderConfig};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum WrapMode {
    /// Wraps on whitespaces not breaking words
    #[default]
    Words,
    /// Only wraps on new line characters
    Newline,
    /// Never wraps, can overflow of parent layout
    None,
}

/// Configuration settings for rendering text elements.
#[derive(Debug, Clone)]
pub struct TextConfig {
    /// Internal engine user data.
    pub(crate) user_data: usize,
    /// The color of the text.
    pub color: Color,
    /// Ply does not manage fonts. It is up to the user to assign a unique ID to each font
    /// and provide it via the [`font_id`](TextConfig::font_id) field.
    pub font_id: u16,
    /// The font size of the text.
    pub font_size: u16,
    /// The spacing between letters.
    pub letter_spacing: u16,
    /// The height of each line of text.
    pub line_height: u16,
    /// Defines the text wrapping behavior.
    pub wrap_mode: WrapMode,
    /// The alignment of the text.
    pub alignment: AlignX,
    /// Per-element shader effects applied to this text.
    pub(crate) effects: Vec<ShaderConfig>,
    /// When true, the text content is exposed to screen readers as static text.
    pub(crate) accessible: bool,
}

impl TextConfig {
    /// Creates a new `TextConfig` instance with default values.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Sets the text color.
    #[inline]
    pub fn color(&mut self, color: impl Into<Color>) -> &mut Self {
        self.color = color.into();
        self
    }

    /// Sets the font ID. The user is responsible for assigning unique font IDs.
    #[inline]
    pub fn font_id(&mut self, id: u16) -> &mut Self {
        self.font_id = id;
        self
    }

    /// Sets the font size.
    #[inline]
    pub fn font_size(&mut self, size: u16) -> &mut Self {
        self.font_size = size;
        self
    }

    /// Sets the letter spacing.
    #[inline]
    pub fn letter_spacing(&mut self, spacing: u16) -> &mut Self {
        self.letter_spacing = spacing;
        self
    }

    /// Sets the line height.
    #[inline]
    pub fn line_height(&mut self, height: u16) -> &mut Self {
        self.line_height = height;
        self
    }

    /// Sets the text wrapping mode.
    #[inline]
    pub fn wrap_mode(&mut self, mode: WrapMode) -> &mut Self {
        self.wrap_mode = mode;
        self
    }

    /// Sets the text alignment.
    #[inline]
    pub fn alignment(&mut self, alignment: AlignX) -> &mut Self {
        self.alignment = alignment;
        self
    }

    /// Adds a per-element shader effect to this text.
    #[inline]
    pub fn effect(&mut self, asset: &ShaderAsset, f: impl FnOnce(&mut ShaderBuilder<'_>)) -> &mut Self {
        let mut builder = ShaderBuilder::new(asset);
        f(&mut builder);
        self.effects.push(builder.into_config());
        self
    }

    /// Makes this text element visible to screen readers.
    ///
    /// The text content is automatically used as the accessible label
    /// with a `StaticText` role. No wrapper element is needed.
    ///
    /// ```ignore
    /// ui.text("Hello world", |t| t.font_size(16).accessible());
    /// ```
    #[inline]
    pub fn accessible(&mut self) -> &mut Self {
        self.accessible = true;
        self
    }
}

impl Default for TextConfig {
    fn default() -> Self {
        Self {
            user_data: 0,
            color: Color::rgba(0., 0., 0., 0.),
            font_id: 0,
            font_size: 0,
            letter_spacing: 0,
            line_height: 0,
            wrap_mode: WrapMode::Words,
            alignment: AlignX::Left,
            effects: Vec::new(),
            accessible: false,
        }
    }
}
