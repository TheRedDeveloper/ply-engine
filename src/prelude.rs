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
pub use crate::shaders::ShaderAsset;

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

// Full macroquad prelude, with Color shadowed by ply's version
pub use macroquad::prelude::*;
pub use crate::color::Color;
// Explicit alias for when users need macroquad's Color
pub use macroquad::prelude::Color as MacroquadColor;
