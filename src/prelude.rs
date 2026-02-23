//! The Ply prelude — a single import for everything you need.
//!
//! ```rust
//! use ply_engine::prelude::*;
//! ```

// Core types
pub use crate::Ply;
pub use crate::Ui;
pub use crate::id::Id;
pub use crate::renderer::GraphicAsset;
pub use crate::renderer::FontAsset;
pub use crate::shaders::ShaderAsset;

// Utility functions
pub use crate::renderer::render_to_texture;
pub use crate::renderer::set_shader_source;

// Macros
pub use crate::{grow, fit, fixed, percent};

// Alignment — globbed
pub use crate::align::AlignX::{self, *};
pub use crate::align::AlignY::{self, *};

// LayoutDirection — globbed
pub use crate::layout::LayoutDirection::{self, *};

// WrapMode — type only, NOT globbed
pub use crate::text::WrapMode;

// AccessibilityRole — type only, NOT globbed
pub use crate::accessibility::AccessibilityRole;

// Built-in shaders — feature-gated, globbed
#[cfg(feature = "built-in-shaders")]
pub use crate::built_in_shaders::*;

// Text styling cursor utilities — feature-gated
#[cfg(feature = "text-styling")]
pub use crate::text_input::styling_cursor;

// Full macroquad prelude, with Color shadowed by ply's version
pub use macroquad::prelude::*;
pub use crate::color::Color;
// Explicit alias for when users need macroquad's Color
pub use macroquad::prelude::Color as MacroquadColor;
