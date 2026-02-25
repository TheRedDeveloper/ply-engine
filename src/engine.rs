//! Pure Rust implementation of the Ply layout engine.
//! A UI layout engine inspired by Clay.

use rustc_hash::FxHashMap;

use crate::align::{AlignX, AlignY};
use crate::color::Color;
use crate::renderer::ImageSource;
use crate::shaders::ShaderConfig;
use crate::elements::{
    FloatingAttachToElement, FloatingClipToElement, PointerCaptureMode,
};
use crate::layout::{LayoutDirection, CornerRadius};
use crate::math::{BoundingBox, Dimensions, Vector2};
use crate::text::{TextConfig, WrapMode};

const DEFAULT_MAX_ELEMENT_COUNT: i32 = 8192;
const DEFAULT_MAX_MEASURE_TEXT_WORD_CACHE_COUNT: i32 = 16384;
const MAXFLOAT: f32 = 3.40282346638528859812e+38;
const EPSILON: f32 = 0.01;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum SizingType {
    #[default]
    Fit,
    Grow,
    Percent,
    Fixed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum RenderCommandType {
    #[default]
    None,
    Rectangle,
    Border,
    Text,
    Image,
    ScissorStart,
    ScissorEnd,
    Custom,
    GroupBegin,
    GroupEnd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum PointerDataInteractionState {
    PressedThisFrame,
    Pressed,
    ReleasedThisFrame,
    #[default]
    Released,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrowDirection {
    Left,
    Right,
    Up,
    Down,
}

/// Actions that can be performed on a focused text input.
#[derive(Debug, Clone)]
pub enum TextInputAction {
    MoveLeft { shift: bool },
    MoveRight { shift: bool },
    MoveWordLeft { shift: bool },
    MoveWordRight { shift: bool },
    MoveHome { shift: bool },
    MoveEnd { shift: bool },
    MoveUp { shift: bool },
    MoveDown { shift: bool },
    Backspace,
    Delete,
    BackspaceWord,
    DeleteWord,
    SelectAll,
    Copy,
    Cut,
    Paste { text: String },
    Submit,
    Undo,
    Redo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ElementConfigType {
    Shared,
    Text,
    Image,
    Floating,
    Custom,
    Clip,
    Border,
    Aspect,
    TextInput,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SizingMinMax {
    pub min: f32,
    pub max: f32,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SizingAxis {
    pub type_: SizingType,
    pub min_max: SizingMinMax,
    pub percent: f32,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SizingConfig {
    pub width: SizingAxis,
    pub height: SizingAxis,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct PaddingConfig {
    pub left: u16,
    pub right: u16,
    pub top: u16,
    pub bottom: u16,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ChildAlignmentConfig {
    pub x: AlignX,
    pub y: AlignY,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct LayoutConfig {
    pub sizing: SizingConfig,
    pub padding: PaddingConfig,
    pub child_gap: u16,
    pub child_alignment: ChildAlignmentConfig,
    pub layout_direction: LayoutDirection,
}


#[derive(Debug, Clone, Copy)]
pub struct VisualRotationConfig {
    /// Rotation angle in radians.
    pub rotation_radians: f32,
    /// Normalized pivot X (0.0 = left, 0.5 = center, 1.0 = right). Default 0.5.
    pub pivot_x: f32,
    /// Normalized pivot Y (0.0 = top, 0.5 = center, 1.0 = bottom). Default 0.5.
    pub pivot_y: f32,
    /// Mirror horizontally.
    pub flip_x: bool,
    /// Mirror vertically.
    pub flip_y: bool,
}

impl Default for VisualRotationConfig {
    fn default() -> Self {
        Self {
            rotation_radians: 0.0,
            pivot_x: 0.5,
            pivot_y: 0.5,
            flip_x: false,
            flip_y: false,
        }
    }
}

impl VisualRotationConfig {
    /// Returns `true` when the config is effectively a no-op.
    pub fn is_noop(&self) -> bool {
        self.rotation_radians == 0.0 && !self.flip_x && !self.flip_y
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ShapeRotationConfig {
    /// Rotation angle in radians.
    pub rotation_radians: f32,
    /// Mirror horizontally (applied before rotation).
    pub flip_x: bool,
    /// Mirror vertically (applied before rotation).
    pub flip_y: bool,
}

impl Default for ShapeRotationConfig {
    fn default() -> Self {
        Self {
            rotation_radians: 0.0,
            flip_x: false,
            flip_y: false,
        }
    }
}

impl ShapeRotationConfig {
    /// Returns `true` when the config is effectively a no-op.
    pub fn is_noop(&self) -> bool {
        self.rotation_radians == 0.0 && !self.flip_x && !self.flip_y
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct FloatingAttachPoints {
    pub element_x: AlignX,
    pub element_y: AlignY,
    pub parent_x: AlignX,
    pub parent_y: AlignY,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct FloatingConfig {
    pub offset: Vector2,
    pub parent_id: u32,
    pub z_index: i16,
    pub attach_points: FloatingAttachPoints,
    pub pointer_capture_mode: PointerCaptureMode,
    pub attach_to: FloatingAttachToElement,
    pub clip_to: FloatingClipToElement,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ClipConfig {
    pub horizontal: bool,
    pub vertical: bool,
    pub scroll_x: bool,
    pub scroll_y: bool,
    pub child_offset: Vector2,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct BorderWidth {
    pub left: u16,
    pub right: u16,
    pub top: u16,
    pub bottom: u16,
    pub between_children: u16,
}

impl BorderWidth {
    pub fn is_zero(&self) -> bool {
        self.left == 0
            && self.right == 0
            && self.top == 0
            && self.bottom == 0
            && self.between_children == 0
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct BorderConfig {
    pub color: Color,
    pub width: BorderWidth,
}

/// The top-level element declaration.
#[derive(Debug, Clone)]
pub struct ElementDeclaration<CustomElementData: Clone + Default + std::fmt::Debug = ()> {
    pub layout: LayoutConfig,
    pub background_color: Color,
    pub corner_radius: CornerRadius,
    pub aspect_ratio: f32,
    pub image_data: Option<ImageSource>,
    pub floating: FloatingConfig,
    pub custom_data: Option<CustomElementData>,
    pub clip: ClipConfig,
    pub border: BorderConfig,
    pub user_data: usize,
    pub effects: Vec<ShaderConfig>,
    pub shaders: Vec<ShaderConfig>,
    pub visual_rotation: Option<VisualRotationConfig>,
    pub shape_rotation: Option<ShapeRotationConfig>,
    pub accessibility: Option<crate::accessibility::AccessibilityConfig>,
    pub text_input: Option<crate::text_input::TextInputConfig>,
    pub preserve_focus: bool,
}

impl<CustomElementData: Clone + Default + std::fmt::Debug> Default for ElementDeclaration<CustomElementData> {
    fn default() -> Self {
        Self {
            layout: LayoutConfig::default(),
            background_color: Color::rgba(0.0, 0.0, 0.0, 0.0),
            corner_radius: CornerRadius::default(),
            aspect_ratio: 0.0,
            image_data: None,
            floating: FloatingConfig::default(),
            custom_data: None,
            clip: ClipConfig::default(),
            border: BorderConfig::default(),
            user_data: 0,
            effects: Vec::new(),
            shaders: Vec::new(),
            visual_rotation: None,
            shape_rotation: None,
            accessibility: None,
            text_input: None,
            preserve_focus: false,
        }
    }
}

use crate::id::{Id, StringId};

#[derive(Debug, Clone, Copy, Default)]
struct SharedElementConfig {
    background_color: Color,
    corner_radius: CornerRadius,
    user_data: usize,
}

#[derive(Debug, Clone, Copy)]
struct ElementConfig {
    config_type: ElementConfigType,
    config_index: usize,
}

#[derive(Debug, Clone, Copy, Default)]
struct ElementConfigSlice {
    start: usize,
    length: i32,
}

#[derive(Debug, Clone, Copy, Default)]
struct WrappedTextLine {
    dimensions: Dimensions,
    start: usize,
    length: usize,
}

#[derive(Debug, Clone)]
struct TextElementData {
    text: String,
    preferred_dimensions: Dimensions,
    element_index: i32,
    wrapped_lines_start: usize,
    wrapped_lines_length: i32,
}

#[derive(Debug, Clone, Copy, Default)]
struct LayoutElement {
    // Children data (for non-text elements)
    children_start: usize,
    children_length: u16,
    // Text data (for text elements)
    text_data_index: i32, // -1 means no text, >= 0 is index
    dimensions: Dimensions,
    min_dimensions: Dimensions,
    layout_config_index: usize,
    element_configs: ElementConfigSlice,
    id: u32,
    floating_children_count: u16,
}

#[derive(Default)]
struct LayoutElementHashMapItem {
    bounding_box: BoundingBox,
    element_id: Id,
    layout_element_index: i32,
    on_hover_fn: Option<Box<dyn FnMut(Id, PointerData)>>,
    on_press_fn: Option<Box<dyn FnMut(Id, PointerData)>>,
    on_release_fn: Option<Box<dyn FnMut(Id, PointerData)>>,
    on_focus_fn: Option<Box<dyn FnMut(Id)>>,
    on_unfocus_fn: Option<Box<dyn FnMut(Id)>>,
    on_text_changed_fn: Option<Box<dyn FnMut(&str)>>,
    on_text_submit_fn: Option<Box<dyn FnMut(&str)>>,
    is_text_input: bool,
    preserve_focus: bool,
    generation: u32,
    collision: bool,
    collapsed: bool,
}

impl Clone for LayoutElementHashMapItem {
    fn clone(&self) -> Self {
        Self {
            bounding_box: self.bounding_box,
            element_id: self.element_id.clone(),
            layout_element_index: self.layout_element_index,
            on_hover_fn: None, // Callbacks are not cloneable
            on_press_fn: None,
            on_release_fn: None,
            on_focus_fn: None,
            on_unfocus_fn: None,
            on_text_changed_fn: None,
            on_text_submit_fn: None,
            is_text_input: self.is_text_input,
            preserve_focus: self.preserve_focus,
            generation: self.generation,
            collision: self.collision,
            collapsed: self.collapsed,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct MeasuredWord {
    start_offset: i32,
    length: i32,
    width: f32,
    next: i32,
}

#[derive(Debug, Clone, Copy, Default)]
#[allow(dead_code)]
struct MeasureTextCacheItem {
    unwrapped_dimensions: Dimensions,
    measured_words_start_index: i32,
    min_width: f32,
    contains_newlines: bool,
    id: u32,
    generation: u32,
}

#[derive(Debug, Clone, Copy, Default)]
#[allow(dead_code)]
struct ScrollContainerDataInternal {
    bounding_box: BoundingBox,
    content_size: Dimensions,
    scroll_origin: Vector2,
    pointer_origin: Vector2,
    scroll_momentum: Vector2,
    scroll_position: Vector2,
    previous_delta: Vector2,
    element_id: u32,
    layout_element_index: i32,
    open_this_frame: bool,
    pointer_scroll_active: bool,
}

#[derive(Debug, Clone, Copy, Default)]
struct LayoutElementTreeNode {
    layout_element_index: i32,
    position: Vector2,
    next_child_offset: Vector2,
}

#[derive(Debug, Clone, Copy, Default)]
struct LayoutElementTreeRoot {
    layout_element_index: i32,
    parent_id: u32,
    clip_element_id: u32,
    z_index: i16,
    pointer_offset: Vector2,
}

#[derive(Debug, Clone, Copy)]
struct FocusableEntry {
    element_id: u32,
    tab_index: Option<i32>,
    insertion_order: u32,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct PointerData {
    pub position: Vector2,
    pub state: PointerDataInteractionState,
}

#[derive(Debug, Clone, Copy, Default)]
#[allow(dead_code)]
struct BooleanWarnings {
    max_elements_exceeded: bool,
    text_measurement_fn_not_set: bool,
    max_text_measure_cache_exceeded: bool,
    max_render_commands_exceeded: bool,
}

#[derive(Debug, Clone)]
pub struct InternalRenderCommand<CustomElementData: Clone + Default + std::fmt::Debug = ()> {
    pub bounding_box: BoundingBox,
    pub command_type: RenderCommandType,
    pub render_data: InternalRenderData<CustomElementData>,
    pub user_data: usize,
    pub id: u32,
    pub z_index: i16,
    pub effects: Vec<ShaderConfig>,
    pub visual_rotation: Option<VisualRotationConfig>,
    pub shape_rotation: Option<ShapeRotationConfig>,
}

#[derive(Debug, Clone)]
pub enum InternalRenderData<CustomElementData: Clone + Default + std::fmt::Debug = ()> {
    None,
    Rectangle {
        background_color: Color,
        corner_radius: CornerRadius,
    },
    Text {
        text: String,
        text_color: Color,
        font_size: u16,
        letter_spacing: u16,
        line_height: u16,
        font_asset: Option<&'static crate::renderer::FontAsset>,
    },
    Image {
        background_color: Color,
        corner_radius: CornerRadius,
        image_data: ImageSource,
    },
    Custom {
        background_color: Color,
        corner_radius: CornerRadius,
        custom_data: CustomElementData,
    },
    Border {
        color: Color,
        corner_radius: CornerRadius,
        width: BorderWidth,
    },
    Clip {
        horizontal: bool,
        vertical: bool,
    },
}

impl<CustomElementData: Clone + Default + std::fmt::Debug> Default for InternalRenderData<CustomElementData> {
    fn default() -> Self {
        Self::None
    }
}

impl<CustomElementData: Clone + Default + std::fmt::Debug> Default for InternalRenderCommand<CustomElementData> {
    fn default() -> Self {
        Self {
            bounding_box: BoundingBox::default(),
            command_type: RenderCommandType::None,
            render_data: InternalRenderData::None,
            user_data: 0,
            id: 0,
            z_index: 0,
            effects: Vec::new(),
            visual_rotation: None,
            shape_rotation: None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ScrollContainerData {
    pub scroll_position: Vector2,
    pub scroll_container_dimensions: Dimensions,
    pub content_dimensions: Dimensions,
    pub horizontal: bool,
    pub vertical: bool,
    pub found: bool,
}

impl Default for ScrollContainerData {
    fn default() -> Self {
        Self {
            scroll_position: Vector2::default(),
            scroll_container_dimensions: Dimensions::default(),
            content_dimensions: Dimensions::default(),
            horizontal: false,
            vertical: false,
            found: false,
        }
    }
}

pub struct PlyContext<CustomElementData: Clone + Default + std::fmt::Debug = ()> {
    // Settings
    pub max_element_count: i32,
    pub max_measure_text_cache_word_count: i32,
    pub debug_mode_enabled: bool,
    pub culling_disabled: bool,
    pub external_scroll_handling_enabled: bool,
    pub debug_selected_element_id: u32,
    pub generation: u32,

    // Warnings
    boolean_warnings: BooleanWarnings,

    // Pointer info
    pointer_info: PointerData,
    pub layout_dimensions: Dimensions,

    // Dynamic element tracking
    dynamic_element_index: u32,

    // Measure text callback
    measure_text_fn: Option<Box<dyn Fn(&str, &TextConfig) -> Dimensions>>,

    // Layout elements
    layout_elements: Vec<LayoutElement>,
    render_commands: Vec<InternalRenderCommand<CustomElementData>>,
    open_layout_element_stack: Vec<i32>,
    layout_element_children: Vec<i32>,
    layout_element_children_buffer: Vec<i32>,
    text_element_data: Vec<TextElementData>,
    aspect_ratio_element_indexes: Vec<i32>,
    reusable_element_index_buffer: Vec<i32>,
    layout_element_clip_element_ids: Vec<i32>,

    // Configs
    layout_configs: Vec<LayoutConfig>,
    element_configs: Vec<ElementConfig>,
    text_element_configs: Vec<TextConfig>,
    aspect_ratio_configs: Vec<f32>,
    image_element_configs: Vec<ImageSource>,
    floating_element_configs: Vec<FloatingConfig>,
    clip_element_configs: Vec<ClipConfig>,
    custom_element_configs: Vec<CustomElementData>,
    border_element_configs: Vec<BorderConfig>,
    shared_element_configs: Vec<SharedElementConfig>,

    // Per-element shader effects (indexed by layout element index)
    element_effects: Vec<Vec<ShaderConfig>>,
    // Per-element group shaders (indexed by layout element index)
    element_shaders: Vec<Vec<ShaderConfig>>,

    // Per-element visual rotation (indexed by layout element index)
    element_visual_rotations: Vec<Option<VisualRotationConfig>>,

    // Per-element shape rotation (indexed by layout element index)
    element_shape_rotations: Vec<Option<ShapeRotationConfig>>,
    // Original dimensions before AABB expansion (only set when shape_rotation is active)
    element_pre_rotation_dimensions: Vec<Option<Dimensions>>,

    // String IDs for debug
    layout_element_id_strings: Vec<StringId>,

    // Text wrapping
    wrapped_text_lines: Vec<WrappedTextLine>,

    // Tree traversal
    tree_node_array: Vec<LayoutElementTreeNode>,
    layout_element_tree_roots: Vec<LayoutElementTreeRoot>,

    // Layout element map: element id -> element data (bounding box, hover callback, etc.)
    layout_element_map: FxHashMap<u32, LayoutElementHashMapItem>,

    // Text measurement cache: content hash -> measured dimensions and words
    measure_text_cache: FxHashMap<u32, MeasureTextCacheItem>,
    measured_words: Vec<MeasuredWord>,
    measured_words_free_list: Vec<i32>,

    // Clip/scroll
    open_clip_element_stack: Vec<i32>,
    pointer_over_ids: Vec<Id>,
    pressed_element_ids: Vec<Id>,
    scroll_container_datas: Vec<ScrollContainerDataInternal>,

    // Accessibility / focus
    pub focused_element_id: u32, // 0 = no focus
    /// True when focus was set via keyboard (Tab/arrow keys), false when via mouse click.
    pub(crate) focus_from_keyboard: bool,
    focusable_elements: Vec<FocusableEntry>,
    pub(crate) accessibility_configs: FxHashMap<u32, crate::accessibility::AccessibilityConfig>,
    pub(crate) accessibility_element_order: Vec<u32>,

    // Text input
    pub(crate) text_edit_states: FxHashMap<u32, crate::text_input::TextEditState>,
    text_input_configs: Vec<crate::text_input::TextInputConfig>,
    /// Set of element IDs that are text inputs this frame.
    pub(crate) text_input_element_ids: Vec<u32>,
    /// Pending click on a text input: (element_id, click_x_relative, click_y_relative, shift_held)
    pub(crate) pending_text_click: Option<(u32, f32, f32, bool)>,
    /// Text input drag-scroll state (mobile-first: drag scrolls, doesn't select).
    pub(crate) text_input_drag_active: bool,
    pub(crate) text_input_drag_origin: crate::math::Vector2,
    pub(crate) text_input_drag_scroll_origin: crate::math::Vector2,
    pub(crate) text_input_drag_element_id: u32,
    /// Current absolute time in seconds (set by lib.rs each frame).
    pub(crate) current_time: f64,
    /// Delta time for the current frame in seconds (set by lib.rs each frame).
    pub(crate) frame_delta_time: f32,

    // Visited flags for DFS
    tree_node_visited: Vec<bool>,

    // Dynamic string data (for int-to-string etc.)
    dynamic_string_data: Vec<u8>,

    // Font height cache: (font_key, font_size) -> height in pixels.
    // Avoids repeated calls to measure_fn("Mg", ...) which are expensive.
    font_height_cache: FxHashMap<(&'static str, u16), f32>,

    // The key of the default font (set by Ply::new, used in debug view)
    pub(crate) default_font_key: &'static str,

    // Debug view: heap-allocated strings that survive the frame
}

fn hash_data_scalar(data: &[u8]) -> u64 {
    let mut hash: u64 = 0;
    for &b in data {
        hash = hash.wrapping_add(b as u64);
        hash = hash.wrapping_add(hash << 10);
        hash ^= hash >> 6;
    }
    hash
}

pub fn hash_string(key: &str, seed: u32) -> Id {
    let mut hash: u32 = seed;
    for b in key.bytes() {
        hash = hash.wrapping_add(b as u32);
        hash = hash.wrapping_add(hash << 10);
        hash ^= hash >> 6;
    }
    hash = hash.wrapping_add(hash << 3);
    hash ^= hash >> 11;
    hash = hash.wrapping_add(hash << 15);
    Id {
        id: hash.wrapping_add(1),
        offset: 0,
        base_id: hash.wrapping_add(1),
        string_id: StringId::from_str(key),
    }
}

pub fn hash_string_with_offset(key: &str, offset: u32, seed: u32) -> Id {
    let mut base: u32 = seed;
    for b in key.bytes() {
        base = base.wrapping_add(b as u32);
        base = base.wrapping_add(base << 10);
        base ^= base >> 6;
    }
    let mut hash = base;
    hash = hash.wrapping_add(offset);
    hash = hash.wrapping_add(hash << 10);
    hash ^= hash >> 6;

    hash = hash.wrapping_add(hash << 3);
    base = base.wrapping_add(base << 3);
    hash ^= hash >> 11;
    base ^= base >> 11;
    hash = hash.wrapping_add(hash << 15);
    base = base.wrapping_add(base << 15);
    Id {
        id: hash.wrapping_add(1),
        offset,
        base_id: base.wrapping_add(1),
        string_id: StringId::from_str(key),
    }
}

fn hash_number(offset: u32, seed: u32) -> Id {
    let mut hash = seed;
    hash = hash.wrapping_add(offset.wrapping_add(48));
    hash = hash.wrapping_add(hash << 10);
    hash ^= hash >> 6;
    hash = hash.wrapping_add(hash << 3);
    hash ^= hash >> 11;
    hash = hash.wrapping_add(hash << 15);
    Id {
        id: hash.wrapping_add(1),
        offset,
        base_id: seed,
        string_id: StringId::empty(),
    }
}

fn hash_string_contents_with_config(
    text: &str,
    config: &TextConfig,
) -> u32 {
    let mut hash: u32 = (hash_data_scalar(text.as_bytes()) % u32::MAX as u64) as u32;
    // Fold in font key bytes
    for &b in config.font_asset.map(|a| a.key()).unwrap_or("").as_bytes() {
        hash = hash.wrapping_add(b as u32);
        hash = hash.wrapping_add(hash << 10);
        hash ^= hash >> 6;
    }
    hash = hash.wrapping_add(config.font_size as u32);
    hash = hash.wrapping_add(hash << 10);
    hash ^= hash >> 6;
    hash = hash.wrapping_add(config.letter_spacing as u32);
    hash = hash.wrapping_add(hash << 10);
    hash ^= hash >> 6;
    hash = hash.wrapping_add(hash << 3);
    hash ^= hash >> 11;
    hash = hash.wrapping_add(hash << 15);
    hash.wrapping_add(1)
}

fn float_equal(left: f32, right: f32) -> bool {
    let diff = left - right;
    diff < EPSILON && diff > -EPSILON
}

fn point_is_inside_rect(point: Vector2, rect: BoundingBox) -> bool {
    point.x >= rect.x
        && point.x <= rect.x + rect.width
        && point.y >= rect.y
        && point.y <= rect.y + rect.height
}

impl<CustomElementData: Clone + Default + std::fmt::Debug> PlyContext<CustomElementData> {
    pub fn new(dimensions: Dimensions) -> Self {
        let max_element_count = DEFAULT_MAX_ELEMENT_COUNT;
        let max_measure_text_cache_word_count = DEFAULT_MAX_MEASURE_TEXT_WORD_CACHE_COUNT;

        let ctx = Self {
            max_element_count,
            max_measure_text_cache_word_count,
            debug_mode_enabled: false,
            culling_disabled: false,
            external_scroll_handling_enabled: false,
            debug_selected_element_id: 0,
            generation: 0,
            boolean_warnings: BooleanWarnings::default(),
            pointer_info: PointerData::default(),
            layout_dimensions: dimensions,
            dynamic_element_index: 0,
            measure_text_fn: None,
            layout_elements: Vec::new(),
            render_commands: Vec::new(),
            open_layout_element_stack: Vec::new(),
            layout_element_children: Vec::new(),
            layout_element_children_buffer: Vec::new(),
            text_element_data: Vec::new(),
            aspect_ratio_element_indexes: Vec::new(),
            reusable_element_index_buffer: Vec::new(),
            layout_element_clip_element_ids: Vec::new(),
            layout_configs: Vec::new(),
            element_configs: Vec::new(),
            text_element_configs: Vec::new(),
            aspect_ratio_configs: Vec::new(),
            image_element_configs: Vec::new(),
            floating_element_configs: Vec::new(),
            clip_element_configs: Vec::new(),
            custom_element_configs: Vec::new(),
            border_element_configs: Vec::new(),
            shared_element_configs: Vec::new(),
            element_effects: Vec::new(),
            element_shaders: Vec::new(),
            element_visual_rotations: Vec::new(),
            element_shape_rotations: Vec::new(),
            element_pre_rotation_dimensions: Vec::new(),
            layout_element_id_strings: Vec::new(),
            wrapped_text_lines: Vec::new(),
            tree_node_array: Vec::new(),
            layout_element_tree_roots: Vec::new(),
            layout_element_map: FxHashMap::default(),
            measure_text_cache: FxHashMap::default(),
            measured_words: Vec::new(),
            measured_words_free_list: Vec::new(),
            open_clip_element_stack: Vec::new(),
            pointer_over_ids: Vec::new(),
            pressed_element_ids: Vec::new(),
            scroll_container_datas: Vec::new(),
            focused_element_id: 0,
            focus_from_keyboard: false,
            focusable_elements: Vec::new(),
            accessibility_configs: FxHashMap::default(),
            accessibility_element_order: Vec::new(),
            text_edit_states: FxHashMap::default(),
            text_input_configs: Vec::new(),
            text_input_element_ids: Vec::new(),
            pending_text_click: None,
            text_input_drag_active: false,
            text_input_drag_origin: Vector2::default(),
            text_input_drag_scroll_origin: Vector2::default(),
            text_input_drag_element_id: 0,
            current_time: 0.0,
            frame_delta_time: 0.0,
            tree_node_visited: Vec::new(),
            dynamic_string_data: Vec::new(),
            font_height_cache: FxHashMap::default(),
            default_font_key: "",
        };
        ctx
    }

    fn get_open_layout_element(&self) -> usize {
        let idx = *self.open_layout_element_stack.last().unwrap();
        idx as usize
    }

    /// Returns the internal u32 id of the currently open element.
    pub fn get_open_element_id(&self) -> u32 {
        let open_idx = self.get_open_layout_element();
        self.layout_elements[open_idx].id
    }

    pub fn get_parent_element_id(&self) -> u32 {
        let stack_len = self.open_layout_element_stack.len();
        let parent_idx = self.open_layout_element_stack[stack_len - 2] as usize;
        self.layout_elements[parent_idx].id
    }

    fn add_hash_map_item(
        &mut self,
        element_id: &Id,
        layout_element_index: i32,
    ) {
        let gen = self.generation;
        match self.layout_element_map.entry(element_id.id) {
            std::collections::hash_map::Entry::Occupied(mut entry) => {
                let item = entry.get_mut();
                if item.generation <= gen {
                    item.element_id = element_id.clone();
                    item.generation = gen + 1;
                    item.layout_element_index = layout_element_index;
                    item.collision = false;
                    item.on_hover_fn = None;
                    item.on_press_fn = None;
                    item.on_release_fn = None;
                    item.on_focus_fn = None;
                    item.on_unfocus_fn = None;
                    item.on_text_changed_fn = None;
                    item.on_text_submit_fn = None;
                    item.is_text_input = false;
                    item.preserve_focus = false;
                } else {
                    // Duplicate ID
                    item.collision = true;
                }
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(LayoutElementHashMapItem {
                    element_id: element_id.clone(),
                    layout_element_index,
                    generation: gen + 1,
                    bounding_box: BoundingBox::default(),
                    on_hover_fn: None,
                    on_press_fn: None,
                    on_release_fn: None,
                    on_focus_fn: None,
                    on_unfocus_fn: None,
                    on_text_changed_fn: None,
                    on_text_submit_fn: None,
                    is_text_input: false,
                    preserve_focus: false,
                    collision: false,
                    collapsed: false,
                });
            }
        }
    }

    fn generate_id_for_anonymous_element(&mut self, open_element_index: usize) -> Id {
        let stack_len = self.open_layout_element_stack.len();
        let parent_idx = self.open_layout_element_stack[stack_len - 2] as usize;
        let parent = &self.layout_elements[parent_idx];
        let offset =
            parent.children_length as u32 + parent.floating_children_count as u32;
        let parent_id = parent.id;
        let element_id = hash_number(offset, parent_id);
        self.layout_elements[open_element_index].id = element_id.id;
        self.add_hash_map_item(&element_id, open_element_index as i32);
        if self.debug_mode_enabled {
            self.layout_element_id_strings.push(element_id.string_id.clone());
        }
        element_id
    }

    fn element_has_config(
        &self,
        element_index: usize,
        config_type: ElementConfigType,
    ) -> bool {
        let element = &self.layout_elements[element_index];
        let start = element.element_configs.start;
        let length = element.element_configs.length;
        for i in 0..length {
            let config = &self.element_configs[start + i as usize];
            if config.config_type == config_type {
                return true;
            }
        }
        false
    }

    fn find_element_config_index(
        &self,
        element_index: usize,
        config_type: ElementConfigType,
    ) -> Option<usize> {
        let element = &self.layout_elements[element_index];
        let start = element.element_configs.start;
        let length = element.element_configs.length;
        for i in 0..length {
            let config = &self.element_configs[start + i as usize];
            if config.config_type == config_type {
                return Some(config.config_index);
            }
        }
        None
    }

    fn update_aspect_ratio_box(&mut self, element_index: usize) {
        if let Some(config_idx) =
            self.find_element_config_index(element_index, ElementConfigType::Aspect)
        {
            let aspect_ratio = self.aspect_ratio_configs[config_idx];
            if aspect_ratio == 0.0 {
                return;
            }
            let elem = &mut self.layout_elements[element_index];
            if elem.dimensions.width == 0.0 && elem.dimensions.height != 0.0 {
                elem.dimensions.width = elem.dimensions.height * aspect_ratio;
            } else if elem.dimensions.width != 0.0 && elem.dimensions.height == 0.0 {
                elem.dimensions.height = elem.dimensions.width * (1.0 / aspect_ratio);
            }
        }
    }

    pub fn store_text_element_config(
        &mut self,
        config: TextConfig,
    ) -> usize {
        self.text_element_configs.push(config);
        self.text_element_configs.len() - 1
    }

    fn store_layout_config(&mut self, config: LayoutConfig) -> usize {
        self.layout_configs.push(config);
        self.layout_configs.len() - 1
    }

    fn store_shared_config(&mut self, config: SharedElementConfig) -> usize {
        self.shared_element_configs.push(config);
        self.shared_element_configs.len() - 1
    }

    fn attach_element_config(&mut self, config_type: ElementConfigType, config_index: usize) {
        if self.boolean_warnings.max_elements_exceeded {
            return;
        }
        let open_idx = self.get_open_layout_element();
        self.layout_elements[open_idx].element_configs.length += 1;
        self.element_configs.push(ElementConfig {
            config_type,
            config_index,
        });
    }

    pub fn open_element(&mut self) {
        if self.boolean_warnings.max_elements_exceeded {
            return;
        }
        let elem = LayoutElement {
            text_data_index: -1,
            ..Default::default()
        };
        self.layout_elements.push(elem);
        let idx = (self.layout_elements.len() - 1) as i32;
        self.open_layout_element_stack.push(idx);

        // Ensure clip IDs array is large enough
        while self.layout_element_clip_element_ids.len() < self.layout_elements.len() {
            self.layout_element_clip_element_ids.push(0);
        }

        self.generate_id_for_anonymous_element(idx as usize);

        if !self.open_clip_element_stack.is_empty() {
            let clip_id = *self.open_clip_element_stack.last().unwrap();
            self.layout_element_clip_element_ids[idx as usize] = clip_id;
        } else {
            self.layout_element_clip_element_ids[idx as usize] = 0;
        }
    }

    pub fn open_element_with_id(&mut self, element_id: &Id) {
        if self.boolean_warnings.max_elements_exceeded {
            return;
        }
        let mut elem = LayoutElement {
            text_data_index: -1,
            ..Default::default()
        };
        elem.id = element_id.id;
        self.layout_elements.push(elem);
        let idx = (self.layout_elements.len() - 1) as i32;
        self.open_layout_element_stack.push(idx);

        while self.layout_element_clip_element_ids.len() < self.layout_elements.len() {
            self.layout_element_clip_element_ids.push(0);
        }

        self.add_hash_map_item(element_id, idx);
        if self.debug_mode_enabled {
            self.layout_element_id_strings.push(element_id.string_id.clone());
        }

        if !self.open_clip_element_stack.is_empty() {
            let clip_id = *self.open_clip_element_stack.last().unwrap();
            self.layout_element_clip_element_ids[idx as usize] = clip_id;
        } else {
            self.layout_element_clip_element_ids[idx as usize] = 0;
        }
    }

    pub fn configure_open_element(&mut self, declaration: &ElementDeclaration<CustomElementData>) {
        if self.boolean_warnings.max_elements_exceeded {
            return;
        }
        let open_idx = self.get_open_layout_element();
        let layout_config_index = self.store_layout_config(declaration.layout);
        self.layout_elements[open_idx].layout_config_index = layout_config_index;

        // Record the start of element configs for this element
        self.layout_elements[open_idx].element_configs.start = self.element_configs.len();

        // Shared config (background color, corner radius, user data)
        let mut shared_config_index: Option<usize> = None;
        if declaration.background_color.a > 0.0 {
            let idx = self.store_shared_config(SharedElementConfig {
                background_color: declaration.background_color,
                corner_radius: CornerRadius::default(),
                user_data: 0,
            });
            shared_config_index = Some(idx);
            self.attach_element_config(ElementConfigType::Shared, idx);
        }
        if !declaration.corner_radius.is_zero() {
            if let Some(idx) = shared_config_index {
                self.shared_element_configs[idx].corner_radius = declaration.corner_radius;
            } else {
                let idx = self.store_shared_config(SharedElementConfig {
                    background_color: Color::rgba(0.0, 0.0, 0.0, 0.0),
                    corner_radius: declaration.corner_radius,
                    user_data: 0,
                });
                shared_config_index = Some(idx);
                self.attach_element_config(ElementConfigType::Shared, idx);
            }
        }
        if declaration.user_data != 0 {
            if let Some(idx) = shared_config_index {
                self.shared_element_configs[idx].user_data = declaration.user_data;
            } else {
                let idx = self.store_shared_config(SharedElementConfig {
                    background_color: Color::rgba(0.0, 0.0, 0.0, 0.0),
                    corner_radius: CornerRadius::default(),
                    user_data: declaration.user_data,
                });
                self.attach_element_config(ElementConfigType::Shared, idx);
            }
        }

        // Image config
        if let Some(image_data) = declaration.image_data.clone() {
            self.image_element_configs.push(image_data);
            let idx = self.image_element_configs.len() - 1;
            self.attach_element_config(ElementConfigType::Image, idx);
        }

        // Aspect ratio config
        if declaration.aspect_ratio > 0.0 {
            self.aspect_ratio_configs.push(declaration.aspect_ratio);
            let idx = self.aspect_ratio_configs.len() - 1;
            self.attach_element_config(ElementConfigType::Aspect, idx);
            self.aspect_ratio_element_indexes
                .push((self.layout_elements.len() - 1) as i32);
        }

        // Floating config
        if declaration.floating.attach_to != FloatingAttachToElement::None {
            let mut floating_config = declaration.floating;
            let stack_len = self.open_layout_element_stack.len();

            if stack_len >= 2 {
                let hierarchical_parent_idx =
                    self.open_layout_element_stack[stack_len - 2] as usize;
                let hierarchical_parent_id = self.layout_elements[hierarchical_parent_idx].id;

                let mut clip_element_id: u32 = 0;

                if declaration.floating.attach_to == FloatingAttachToElement::Parent {
                    floating_config.parent_id = hierarchical_parent_id;
                    if !self.open_clip_element_stack.is_empty() {
                        clip_element_id =
                            *self.open_clip_element_stack.last().unwrap() as u32;
                    }
                } else if declaration.floating.attach_to
                    == FloatingAttachToElement::ElementWithId
                {
                    if let Some(parent_item) =
                        self.layout_element_map.get(&floating_config.parent_id)
                    {
                        let parent_elem_idx = parent_item.layout_element_index as usize;
                        clip_element_id =
                            self.layout_element_clip_element_ids[parent_elem_idx] as u32;
                    }
                } else if declaration.floating.attach_to
                    == FloatingAttachToElement::Root
                {
                    floating_config.parent_id =
                        hash_string("Ply__RootContainer", 0).id;
                }

                if declaration.floating.clip_to == FloatingClipToElement::None {
                    clip_element_id = 0;
                }

                let current_element_index =
                    *self.open_layout_element_stack.last().unwrap();
                self.layout_element_clip_element_ids[current_element_index as usize] =
                    clip_element_id as i32;
                self.open_clip_element_stack.push(clip_element_id as i32);

                self.layout_element_tree_roots
                    .push(LayoutElementTreeRoot {
                        layout_element_index: current_element_index,
                        parent_id: floating_config.parent_id,
                        clip_element_id,
                        z_index: floating_config.z_index,
                        pointer_offset: Vector2::default(),
                    });

                self.floating_element_configs.push(floating_config);
                let idx = self.floating_element_configs.len() - 1;
                self.attach_element_config(ElementConfigType::Floating, idx);
            }
        }

        // Custom config
        if let Some(ref custom_data) = declaration.custom_data {
            self.custom_element_configs.push(custom_data.clone());
            let idx = self.custom_element_configs.len() - 1;
            self.attach_element_config(ElementConfigType::Custom, idx);
        }

        // Clip config
        if declaration.clip.horizontal || declaration.clip.vertical {
            let mut clip = declaration.clip;

            let elem_id = self.layout_elements[open_idx].id;

            // Auto-apply stored scroll position as child_offset
            if clip.scroll_x || clip.scroll_y {
                for scd in &self.scroll_container_datas {
                    if scd.element_id == elem_id {
                        clip.child_offset = scd.scroll_position;
                        break;
                    }
                }
            }

            self.clip_element_configs.push(clip);
            let idx = self.clip_element_configs.len() - 1;
            self.attach_element_config(ElementConfigType::Clip, idx);

            self.open_clip_element_stack.push(elem_id as i32);

            // Track scroll container
            if clip.scroll_x || clip.scroll_y {
                let mut found_existing = false;
                for scd in &mut self.scroll_container_datas {
                    if elem_id == scd.element_id {
                        scd.layout_element_index = open_idx as i32;
                        scd.open_this_frame = true;
                        found_existing = true;
                        break;
                    }
                }
                if !found_existing {
                    self.scroll_container_datas.push(ScrollContainerDataInternal {
                        layout_element_index: open_idx as i32,
                        scroll_origin: Vector2::new(-1.0, -1.0),
                        element_id: elem_id,
                        open_this_frame: true,
                        ..Default::default()
                    });
                }
            }
        }

        // Border config
        if !declaration.border.width.is_zero() {
            self.border_element_configs.push(declaration.border);
            let idx = self.border_element_configs.len() - 1;
            self.attach_element_config(ElementConfigType::Border, idx);
        }

        // Store per-element shader effects
        // Ensure element_effects is large enough for open_idx
        while self.element_effects.len() <= open_idx {
            self.element_effects.push(Vec::new());
        }
        self.element_effects[open_idx] = declaration.effects.clone();

        // Store per-element group shaders
        while self.element_shaders.len() <= open_idx {
            self.element_shaders.push(Vec::new());
        }
        self.element_shaders[open_idx] = declaration.shaders.clone();

        // Store per-element visual rotation
        while self.element_visual_rotations.len() <= open_idx {
            self.element_visual_rotations.push(None);
        }
        self.element_visual_rotations[open_idx] = declaration.visual_rotation;

        // Store per-element shape rotation
        while self.element_shape_rotations.len() <= open_idx {
            self.element_shape_rotations.push(None);
        }
        self.element_shape_rotations[open_idx] = declaration.shape_rotation;

        // Accessibility config
        if let Some(ref a11y) = declaration.accessibility {
            let elem_id = self.layout_elements[open_idx].id;
            if a11y.focusable {
                self.focusable_elements.push(FocusableEntry {
                    element_id: elem_id,
                    tab_index: a11y.tab_index,
                    insertion_order: self.focusable_elements.len() as u32,
                });
            }
            self.accessibility_configs.insert(elem_id, a11y.clone());
            self.accessibility_element_order.push(elem_id);
        }

        // Text input config
        if let Some(ref ti_config) = declaration.text_input {
            let elem_id = self.layout_elements[open_idx].id;
            self.text_input_configs.push(ti_config.clone());
            let idx = self.text_input_configs.len() - 1;
            self.attach_element_config(ElementConfigType::TextInput, idx);
            self.text_input_element_ids.push(elem_id);

            // Mark the element as a text input in the layout map
            if let Some(item) = self.layout_element_map.get_mut(&elem_id) {
                item.is_text_input = true;
            }

            // Ensure a TextEditState exists for this element
            self.text_edit_states.entry(elem_id)
                .or_insert_with(crate::text_input::TextEditState::default);

            // Sync config flags to persistent state
            if let Some(state) = self.text_edit_states.get_mut(&elem_id) {
                state.no_styles_movement = ti_config.no_styles_movement;
            }

            // Process any pending click on this text input
            if let Some((click_elem, click_x, click_y, click_shift)) = self.pending_text_click.take() {
                if click_elem == elem_id {
                    if let Some(ref measure_fn) = self.measure_text_fn {
                        let state = self.text_edit_states.get(&elem_id).cloned()
                            .unwrap_or_default();
                        let disp_text = crate::text_input::display_text(
                            &state.text,
                            &ti_config.placeholder,
                            ti_config.is_password,
                        );
                        // Only position cursor in actual text, not placeholder
                        if !state.text.is_empty() {
                            // Double-click detection
                            let is_double_click = state.last_click_element == elem_id
                                && (self.current_time - state.last_click_time) < 0.4;

                            if ti_config.is_multiline {
                                // Multiline: determine which visual line was clicked
                                let elem_width = self.layout_element_map.get(&elem_id)
                                    .map(|item| item.bounding_box.width)
                                    .unwrap_or(200.0);
                                let visual_lines = crate::text_input::wrap_lines(
                                    &disp_text,
                                    elem_width,
                                    ti_config.font_asset,
                                    ti_config.font_size,
                                    measure_fn.as_ref(),
                                );
                                let font_height = {
                                    let config = crate::text::TextConfig {
                                        font_asset: ti_config.font_asset,
                                        font_size: ti_config.font_size,
                                        ..Default::default()
                                    };
                                    measure_fn(&"Mg", &config).height
                                };
                                let adjusted_y = click_y + state.scroll_offset_y;
                                let clicked_line = (adjusted_y / font_height).floor().max(0.0) as usize;
                                let clicked_line = clicked_line.min(visual_lines.len().saturating_sub(1));

                                let vl = &visual_lines[clicked_line];
                                let line_char_x_positions = crate::text_input::compute_char_x_positions(
                                    &vl.text,
                                    ti_config.font_asset,
                                    ti_config.font_size,
                                    measure_fn.as_ref(),
                                );
                                let col = crate::text_input::find_nearest_char_boundary(
                                    click_x, &line_char_x_positions,
                                );
                                let global_pos = vl.global_char_start + col;

                                if let Some(state) = self.text_edit_states.get_mut(&elem_id) {
                                    #[cfg(feature = "text-styling")]
                                    {
                                        let visual_pos = crate::text_input::styling_cursor::raw_to_cursor(&state.text, global_pos);
                                        if is_double_click {
                                            state.select_word_at_styled(visual_pos);
                                        } else {
                                            state.click_to_cursor_styled(visual_pos, click_shift);
                                        }
                                    }
                                    #[cfg(not(feature = "text-styling"))]
                                    {
                                        if is_double_click {
                                            state.select_word_at(global_pos);
                                        } else {
                                            if click_shift {
                                                if state.selection_anchor.is_none() {
                                                    state.selection_anchor = Some(state.cursor_pos);
                                                }
                                            } else {
                                                state.selection_anchor = None;
                                            }
                                            state.cursor_pos = global_pos;
                                            state.reset_blink();
                                        }
                                    }
                                    state.last_click_time = self.current_time;
                                    state.last_click_element = elem_id;
                                }
                            } else {
                                // Single-line: existing behavior
                                let char_x_positions = crate::text_input::compute_char_x_positions(
                                    &disp_text,
                                    ti_config.font_asset,
                                    ti_config.font_size,
                                    measure_fn.as_ref(),
                                );
                                let adjusted_x = click_x + state.scroll_offset;

                                if let Some(state) = self.text_edit_states.get_mut(&elem_id) {
                                    let raw_click_pos = crate::text_input::find_nearest_char_boundary(
                                        adjusted_x, &char_x_positions,
                                    );
                                    #[cfg(feature = "text-styling")]
                                    {
                                        let visual_pos = crate::text_input::styling_cursor::raw_to_cursor(&state.text, raw_click_pos);
                                        if is_double_click {
                                            state.select_word_at_styled(visual_pos);
                                        } else {
                                            state.click_to_cursor_styled(visual_pos, click_shift);
                                        }
                                    }
                                    #[cfg(not(feature = "text-styling"))]
                                    {
                                        if is_double_click {
                                            state.select_word_at(raw_click_pos);
                                        } else {
                                            state.click_to_cursor(adjusted_x, &char_x_positions, click_shift);
                                        }
                                    }
                                    state.last_click_time = self.current_time;
                                    state.last_click_element = elem_id;
                                }
                            }
                        } else if let Some(state) = self.text_edit_states.get_mut(&elem_id) {
                            state.cursor_pos = 0;
                            state.selection_anchor = None;
                            state.last_click_time = self.current_time;
                            state.last_click_element = elem_id;
                            state.reset_blink();
                        }
                    }
                } else {
                    // Wasn't for this element, put it back
                    self.pending_text_click = Some((click_elem, click_x, click_y, click_shift));
                }
            }

            // Auto-register as focusable if not already done via accessibility
            if declaration.accessibility.is_none() || !declaration.accessibility.as_ref().unwrap().focusable {
                // Check it's not already registered
                let already = self.focusable_elements.iter().any(|e| e.element_id == elem_id);
                if !already {
                    self.focusable_elements.push(FocusableEntry {
                        element_id: elem_id,
                        tab_index: None,
                        insertion_order: self.focusable_elements.len() as u32,
                    });
                }
            }
        }

        // Preserve-focus flag
        if declaration.preserve_focus {
            let elem_id = self.layout_elements[open_idx].id;
            if let Some(item) = self.layout_element_map.get_mut(&elem_id) {
                item.preserve_focus = true;
            }
        }
    }

    pub fn close_element(&mut self) {
        if self.boolean_warnings.max_elements_exceeded {
            return;
        }

        let open_idx = self.get_open_layout_element();
        let layout_config_index = self.layout_elements[open_idx].layout_config_index;
        let layout_config = self.layout_configs[layout_config_index];

        // Check for clip and floating configs
        let mut element_has_clip_horizontal = false;
        let mut element_has_clip_vertical = false;
        let element_configs_start = self.layout_elements[open_idx].element_configs.start;
        let element_configs_length = self.layout_elements[open_idx].element_configs.length;

        for i in 0..element_configs_length {
            let config = &self.element_configs[element_configs_start + i as usize];
            if config.config_type == ElementConfigType::Clip {
                let clip = &self.clip_element_configs[config.config_index];
                element_has_clip_horizontal = clip.horizontal;
                element_has_clip_vertical = clip.vertical;
                self.open_clip_element_stack.pop();
                break;
            } else if config.config_type == ElementConfigType::Floating {
                self.open_clip_element_stack.pop();
            }
        }

        let left_right_padding =
            (layout_config.padding.left + layout_config.padding.right) as f32;
        let top_bottom_padding =
            (layout_config.padding.top + layout_config.padding.bottom) as f32;

        let children_length = self.layout_elements[open_idx].children_length;

        // Attach children to the current open element
        let children_start = self.layout_element_children.len();
        self.layout_elements[open_idx].children_start = children_start;

        if layout_config.layout_direction == LayoutDirection::LeftToRight {
            self.layout_elements[open_idx].dimensions.width = left_right_padding;
            self.layout_elements[open_idx].min_dimensions.width = left_right_padding;

            for i in 0..children_length {
                let buf_idx = self.layout_element_children_buffer.len()
                    - children_length as usize
                    + i as usize;
                let child_index = self.layout_element_children_buffer[buf_idx];
                let child = &self.layout_elements[child_index as usize];
                let child_width = child.dimensions.width;
                let child_height = child.dimensions.height;
                let child_min_width = child.min_dimensions.width;
                let child_min_height = child.min_dimensions.height;

                self.layout_elements[open_idx].dimensions.width += child_width;
                let current_height = self.layout_elements[open_idx].dimensions.height;
                self.layout_elements[open_idx].dimensions.height =
                    f32::max(current_height, child_height + top_bottom_padding);

                if !element_has_clip_horizontal {
                    self.layout_elements[open_idx].min_dimensions.width += child_min_width;
                }
                if !element_has_clip_vertical {
                    let current_min_h = self.layout_elements[open_idx].min_dimensions.height;
                    self.layout_elements[open_idx].min_dimensions.height =
                        f32::max(current_min_h, child_min_height + top_bottom_padding);
                }
                self.layout_element_children.push(child_index);
            }
            let child_gap =
                (children_length.saturating_sub(1) as u32 * layout_config.child_gap as u32) as f32;
            self.layout_elements[open_idx].dimensions.width += child_gap;
            if !element_has_clip_horizontal {
                self.layout_elements[open_idx].min_dimensions.width += child_gap;
            }
        } else {
            // TopToBottom
            self.layout_elements[open_idx].dimensions.height = top_bottom_padding;
            self.layout_elements[open_idx].min_dimensions.height = top_bottom_padding;

            for i in 0..children_length {
                let buf_idx = self.layout_element_children_buffer.len()
                    - children_length as usize
                    + i as usize;
                let child_index = self.layout_element_children_buffer[buf_idx];
                let child = &self.layout_elements[child_index as usize];
                let child_width = child.dimensions.width;
                let child_height = child.dimensions.height;
                let child_min_width = child.min_dimensions.width;
                let child_min_height = child.min_dimensions.height;

                self.layout_elements[open_idx].dimensions.height += child_height;
                let current_width = self.layout_elements[open_idx].dimensions.width;
                self.layout_elements[open_idx].dimensions.width =
                    f32::max(current_width, child_width + left_right_padding);

                if !element_has_clip_vertical {
                    self.layout_elements[open_idx].min_dimensions.height += child_min_height;
                }
                if !element_has_clip_horizontal {
                    let current_min_w = self.layout_elements[open_idx].min_dimensions.width;
                    self.layout_elements[open_idx].min_dimensions.width =
                        f32::max(current_min_w, child_min_width + left_right_padding);
                }
                self.layout_element_children.push(child_index);
            }
            let child_gap =
                (children_length.saturating_sub(1) as u32 * layout_config.child_gap as u32) as f32;
            self.layout_elements[open_idx].dimensions.height += child_gap;
            if !element_has_clip_vertical {
                self.layout_elements[open_idx].min_dimensions.height += child_gap;
            }
        }

        // Remove children from buffer
        let remove_count = children_length as usize;
        let new_len = self.layout_element_children_buffer.len().saturating_sub(remove_count);
        self.layout_element_children_buffer.truncate(new_len);

        // Clamp width
        {
            let sizing_type = self.layout_configs[layout_config_index].sizing.width.type_;
            if sizing_type != SizingType::Percent {
                let mut max_w = self.layout_configs[layout_config_index].sizing.width.min_max.max;
                if max_w <= 0.0 {
                    max_w = MAXFLOAT;
                    self.layout_configs[layout_config_index].sizing.width.min_max.max = max_w;
                }
                let min_w = self.layout_configs[layout_config_index].sizing.width.min_max.min;
                self.layout_elements[open_idx].dimensions.width = f32::min(
                    f32::max(self.layout_elements[open_idx].dimensions.width, min_w),
                    max_w,
                );
                self.layout_elements[open_idx].min_dimensions.width = f32::min(
                    f32::max(self.layout_elements[open_idx].min_dimensions.width, min_w),
                    max_w,
                );
            } else {
                self.layout_elements[open_idx].dimensions.width = 0.0;
            }
        }

        // Clamp height
        {
            let sizing_type = self.layout_configs[layout_config_index].sizing.height.type_;
            if sizing_type != SizingType::Percent {
                let mut max_h = self.layout_configs[layout_config_index].sizing.height.min_max.max;
                if max_h <= 0.0 {
                    max_h = MAXFLOAT;
                    self.layout_configs[layout_config_index].sizing.height.min_max.max = max_h;
                }
                let min_h = self.layout_configs[layout_config_index].sizing.height.min_max.min;
                self.layout_elements[open_idx].dimensions.height = f32::min(
                    f32::max(self.layout_elements[open_idx].dimensions.height, min_h),
                    max_h,
                );
                self.layout_elements[open_idx].min_dimensions.height = f32::min(
                    f32::max(self.layout_elements[open_idx].min_dimensions.height, min_h),
                    max_h,
                );
            } else {
                self.layout_elements[open_idx].dimensions.height = 0.0;
            }
        }

        self.update_aspect_ratio_box(open_idx);

        // Apply shape rotation AABB expansion
        if let Some(shape_rot) = self.element_shape_rotations.get(open_idx).copied().flatten() {
            if !shape_rot.is_noop() {
                let orig_w = self.layout_elements[open_idx].dimensions.width;
                let orig_h = self.layout_elements[open_idx].dimensions.height;

                // Find corner radius for this element
                let cr = self
                    .find_element_config_index(open_idx, ElementConfigType::Shared)
                    .map(|idx| self.shared_element_configs[idx].corner_radius)
                    .unwrap_or_default();

                let (eff_w, eff_h) = crate::math::compute_rotated_aabb(
                    orig_w,
                    orig_h,
                    &cr,
                    shape_rot.rotation_radians,
                );

                // Store original dimensions for renderer
                while self.element_pre_rotation_dimensions.len() <= open_idx {
                    self.element_pre_rotation_dimensions.push(None);
                }
                self.element_pre_rotation_dimensions[open_idx] =
                    Some(Dimensions::new(orig_w, orig_h));

                // Replace layout dimensions with AABB
                self.layout_elements[open_idx].dimensions.width = eff_w;
                self.layout_elements[open_idx].dimensions.height = eff_h;
                self.layout_elements[open_idx].min_dimensions.width = eff_w;
                self.layout_elements[open_idx].min_dimensions.height = eff_h;
            }
        }

        let element_is_floating =
            self.element_has_config(open_idx, ElementConfigType::Floating);

        // Pop from open stack
        self.open_layout_element_stack.pop();

        // Add to parent's children
        if self.open_layout_element_stack.len() > 1 {
            if element_is_floating {
                let parent_idx = self.get_open_layout_element();
                self.layout_elements[parent_idx].floating_children_count += 1;
                return;
            }
            let parent_idx = self.get_open_layout_element();
            self.layout_elements[parent_idx].children_length += 1;
            self.layout_element_children_buffer.push(open_idx as i32);
        }
    }

    pub fn open_text_element(
        &mut self,
        text: &str,
        text_config_index: usize,
    ) {
        if self.boolean_warnings.max_elements_exceeded {
            return;
        }

        let parent_idx = self.get_open_layout_element();
        let parent_id = self.layout_elements[parent_idx].id;
        let parent_children_count = self.layout_elements[parent_idx].children_length;

        // Create text layout element
        let text_element = LayoutElement {
            text_data_index: -1,
            ..Default::default()
        };
        self.layout_elements.push(text_element);
        let text_elem_idx = (self.layout_elements.len() - 1) as i32;

        while self.layout_element_clip_element_ids.len() < self.layout_elements.len() {
            self.layout_element_clip_element_ids.push(0);
        }
        if !self.open_clip_element_stack.is_empty() {
            let clip_id = *self.open_clip_element_stack.last().unwrap();
            self.layout_element_clip_element_ids[text_elem_idx as usize] = clip_id;
        } else {
            self.layout_element_clip_element_ids[text_elem_idx as usize] = 0;
        }

        self.layout_element_children_buffer.push(text_elem_idx);

        // Measure text
        let text_config = self.text_element_configs[text_config_index].clone();
        let text_measured =
            self.measure_text_cached(text, &text_config);

        let element_id = hash_number(parent_children_count as u32, parent_id);
        self.layout_elements[text_elem_idx as usize].id = element_id.id;
        self.add_hash_map_item(&element_id, text_elem_idx);
        if self.debug_mode_enabled {
            self.layout_element_id_strings.push(element_id.string_id);
        }

        // If the text element is marked accessible, register it in the
        // accessibility tree with a StaticText role and the text content
        // as the label.
        if text_config.accessible {
            let a11y = crate::accessibility::AccessibilityConfig {
                role: crate::accessibility::AccessibilityRole::StaticText,
                label: text.to_string(),
                ..Default::default()
            };
            self.accessibility_configs.insert(element_id.id, a11y);
            self.accessibility_element_order.push(element_id.id);
        }

        let text_width = text_measured.unwrapped_dimensions.width;
        let text_height = if text_config.line_height > 0 {
            text_config.line_height as f32
        } else {
            text_measured.unwrapped_dimensions.height
        };
        let min_width = text_measured.min_width;

        self.layout_elements[text_elem_idx as usize].dimensions =
            Dimensions::new(text_width, text_height);
        self.layout_elements[text_elem_idx as usize].min_dimensions =
            Dimensions::new(min_width, text_height);

        // Store text element data
        let text_data = TextElementData {
            text: text.to_string(),
            preferred_dimensions: text_measured.unwrapped_dimensions,
            element_index: text_elem_idx,
            wrapped_lines_start: 0,
            wrapped_lines_length: 0,
        };
        self.text_element_data.push(text_data);
        let text_data_idx = (self.text_element_data.len() - 1) as i32;
        self.layout_elements[text_elem_idx as usize].text_data_index = text_data_idx;

        // Attach text config
        self.layout_elements[text_elem_idx as usize].element_configs.start =
            self.element_configs.len();
        self.element_configs.push(ElementConfig {
            config_type: ElementConfigType::Text,
            config_index: text_config_index,
        });
        self.layout_elements[text_elem_idx as usize].element_configs.length = 1;

        // Set default layout config
        let default_layout_idx = self.store_layout_config(LayoutConfig::default());
        self.layout_elements[text_elem_idx as usize].layout_config_index = default_layout_idx;

        // Add to parent's children count
        self.layout_elements[parent_idx].children_length += 1;
    }

    /// Returns the cached font height for the given (font_asset, font_size) pair.
    /// Measures `"Mg"` on the first call for each pair and caches the result.
    fn font_height(&mut self, font_asset: Option<&'static crate::renderer::FontAsset>, font_size: u16) -> f32 {
        let font_key = font_asset.map(|a| a.key()).unwrap_or("");
        let key = (font_key, font_size);
        if let Some(&h) = self.font_height_cache.get(&key) {
            return h;
        }
        let h = if let Some(ref measure_fn) = self.measure_text_fn {
            let config = TextConfig {
                font_asset,
                font_size,
                ..Default::default()
            };
            measure_fn("Mg", &config).height
        } else {
            font_size as f32
        };
        self.font_height_cache.insert(key, h);
        h
    }

    fn measure_text_cached(
        &mut self,
        text: &str,
        config: &TextConfig,
    ) -> MeasureTextCacheItem {
        match &self.measure_text_fn {
            Some(_) => {},
            None => {
                if !self.boolean_warnings.text_measurement_fn_not_set {
                    self.boolean_warnings.text_measurement_fn_not_set = true;
                }
                return MeasureTextCacheItem::default();
            }
        };

        let id = hash_string_contents_with_config(text, config);

        // Check cache
        if let Some(item) = self.measure_text_cache.get_mut(&id) {
            item.generation = self.generation;
            return *item;
        }

        // Not cached - measure now
        let text_data = text.as_bytes();
        let text_length = text_data.len() as i32;

        let space_str = " ";
        let space_width = (self.measure_text_fn.as_ref().unwrap())(space_str, config).width;

        let mut start: i32 = 0;
        let mut end: i32 = 0;
        let mut line_width: f32 = 0.0;
        let mut measured_width: f32 = 0.0;
        let mut measured_height: f32 = 0.0;
        let mut min_width: f32 = 0.0;
        let mut contains_newlines = false;

        let mut temp_word_next: i32 = -1;
        let mut previous_word_index: i32 = -1;

        while end < text_length {
            let current = text_data[end as usize];
            if current == b' ' || current == b'\n' {
                let length = end - start;
                let mut dimensions = Dimensions::default();
                if length > 0 {
                    let substr =
                        core::str::from_utf8(&text_data[start as usize..end as usize]).unwrap();
                    dimensions = (self.measure_text_fn.as_ref().unwrap())(substr, config);
                }
                min_width = f32::max(dimensions.width, min_width);
                measured_height = f32::max(measured_height, dimensions.height);

                if current == b' ' {
                    dimensions.width += space_width;
                    let word = MeasuredWord {
                        start_offset: start,
                        length: length + 1,
                        width: dimensions.width,
                        next: -1,
                    };
                    let word_idx = self.add_measured_word(word, previous_word_index);
                    if previous_word_index == -1 {
                        temp_word_next = word_idx;
                    }
                    previous_word_index = word_idx;
                    line_width += dimensions.width;
                }
                if current == b'\n' {
                    if length > 0 {
                        let word = MeasuredWord {
                            start_offset: start,
                            length,
                            width: dimensions.width,
                            next: -1,
                        };
                        let word_idx = self.add_measured_word(word, previous_word_index);
                        if previous_word_index == -1 {
                            temp_word_next = word_idx;
                        }
                        previous_word_index = word_idx;
                    }
                    let newline_word = MeasuredWord {
                        start_offset: end + 1,
                        length: 0,
                        width: 0.0,
                        next: -1,
                    };
                    let word_idx = self.add_measured_word(newline_word, previous_word_index);
                    if previous_word_index == -1 {
                        temp_word_next = word_idx;
                    }
                    previous_word_index = word_idx;
                    line_width += dimensions.width;
                    measured_width = f32::max(line_width, measured_width);
                    contains_newlines = true;
                    line_width = 0.0;
                }
                start = end + 1;
            }
            end += 1;
        }

        if end - start > 0 {
            let substr =
                core::str::from_utf8(&text_data[start as usize..end as usize]).unwrap();
            let dimensions = (self.measure_text_fn.as_ref().unwrap())(substr, config);
            let word = MeasuredWord {
                start_offset: start,
                length: end - start,
                width: dimensions.width,
                next: -1,
            };
            let word_idx = self.add_measured_word(word, previous_word_index);
            if previous_word_index == -1 {
                temp_word_next = word_idx;
            }
            line_width += dimensions.width;
            measured_height = f32::max(measured_height, dimensions.height);
            min_width = f32::max(dimensions.width, min_width);
        }

        measured_width =
            f32::max(line_width, measured_width) - config.letter_spacing as f32;

        let result = MeasureTextCacheItem {
            id,
            generation: self.generation,
            measured_words_start_index: temp_word_next,
            unwrapped_dimensions: Dimensions::new(measured_width, measured_height),
            min_width,
            contains_newlines,
        };
        self.measure_text_cache.insert(id, result);
        result
    }

    fn add_measured_word(&mut self, word: MeasuredWord, previous_word_index: i32) -> i32 {
        let new_index: i32;
        if let Some(&free_idx) = self.measured_words_free_list.last() {
            self.measured_words_free_list.pop();
            new_index = free_idx;
            self.measured_words[free_idx as usize] = word;
        } else {
            self.measured_words.push(word);
            new_index = (self.measured_words.len() - 1) as i32;
        }
        if previous_word_index >= 0 {
            self.measured_words[previous_word_index as usize].next = new_index;
        }
        new_index
    }

    pub fn begin_layout(&mut self) {
        self.initialize_ephemeral_memory();
        self.generation += 1;
        self.dynamic_element_index = 0;

        // Evict stale text measurement cache entries
        self.evict_stale_text_cache();

        let root_width = self.layout_dimensions.width;
        let root_height = self.layout_dimensions.height;

        self.boolean_warnings = BooleanWarnings::default();

        let root_id = hash_string("Ply__RootContainer", 0);
        self.open_element_with_id(&root_id);

        let root_decl = ElementDeclaration {
            layout: LayoutConfig {
                sizing: SizingConfig {
                    width: SizingAxis {
                        type_: SizingType::Fixed,
                        min_max: SizingMinMax {
                            min: root_width,
                            max: root_width,
                        },
                        percent: 0.0,
                    },
                    height: SizingAxis {
                        type_: SizingType::Fixed,
                        min_max: SizingMinMax {
                            min: root_height,
                            max: root_height,
                        },
                        percent: 0.0,
                    },
                },
                ..Default::default()
            },
            ..Default::default()
        };
        self.configure_open_element(&root_decl);
        self.open_layout_element_stack.push(0);
        self.layout_element_tree_roots.push(LayoutElementTreeRoot {
            layout_element_index: 0,
            ..Default::default()
        });
    }

    pub fn end_layout(&mut self) -> &[InternalRenderCommand<CustomElementData>] {
        self.close_element();

        if self.open_layout_element_stack.len() > 1 {
            // Unbalanced open/close warning
        }

        if self.debug_mode_enabled {
            self.render_debug_view();
        }

        self.calculate_final_layout();
        &self.render_commands
    }

    /// Evicts stale entries from the text measurement cache.
    /// Entries that haven't been used for more than 2 generations are removed.
    fn evict_stale_text_cache(&mut self) {
        let gen = self.generation;
        let measured_words = &mut self.measured_words;
        let free_list = &mut self.measured_words_free_list;
        self.measure_text_cache.retain(|_, item| {
            if gen.wrapping_sub(item.generation) <= 2 {
                true
            } else {
                // Clean up measured words for this evicted entry
                let mut idx = item.measured_words_start_index;
                while idx != -1 {
                    let word = measured_words[idx as usize];
                    free_list.push(idx);
                    idx = word.next;
                }
                false
            }
        });
    }

    fn initialize_ephemeral_memory(&mut self) {
        self.layout_element_children_buffer.clear();
        self.layout_elements.clear();
        self.layout_configs.clear();
        self.element_configs.clear();
        self.text_element_configs.clear();
        self.aspect_ratio_configs.clear();
        self.image_element_configs.clear();
        self.floating_element_configs.clear();
        self.clip_element_configs.clear();
        self.custom_element_configs.clear();
        self.border_element_configs.clear();
        self.shared_element_configs.clear();
        self.element_effects.clear();
        self.element_shaders.clear();
        self.element_visual_rotations.clear();
        self.element_shape_rotations.clear();
        self.element_pre_rotation_dimensions.clear();
        self.layout_element_id_strings.clear();
        self.wrapped_text_lines.clear();
        self.tree_node_array.clear();
        self.layout_element_tree_roots.clear();
        self.layout_element_children.clear();
        self.open_layout_element_stack.clear();
        self.text_element_data.clear();
        self.aspect_ratio_element_indexes.clear();
        self.render_commands.clear();
        self.tree_node_visited.clear();
        self.open_clip_element_stack.clear();
        self.reusable_element_index_buffer.clear();
        self.layout_element_clip_element_ids.clear();
        self.dynamic_string_data.clear();
        self.focusable_elements.clear();
        self.accessibility_configs.clear();
        self.accessibility_element_order.clear();
        self.text_input_configs.clear();
        self.text_input_element_ids.clear();
    }

    fn size_containers_along_axis(&mut self, x_axis: bool) {
        let mut bfs_buffer: Vec<i32> = Vec::new();
        let mut resizable_container_buffer: Vec<i32> = Vec::new();

        for root_index in 0..self.layout_element_tree_roots.len() {
            bfs_buffer.clear();
            let root = self.layout_element_tree_roots[root_index];
            let root_elem_idx = root.layout_element_index as usize;
            bfs_buffer.push(root.layout_element_index);

            // Size floating containers to their parents
            if self.element_has_config(root_elem_idx, ElementConfigType::Floating) {
                if let Some(float_cfg_idx) =
                    self.find_element_config_index(root_elem_idx, ElementConfigType::Floating)
                {
                    let parent_id = self.floating_element_configs[float_cfg_idx].parent_id;
                    if let Some(parent_item) = self.layout_element_map.get(&parent_id) {
                        let parent_elem_idx = parent_item.layout_element_index as usize;
                        let parent_dims = self.layout_elements[parent_elem_idx].dimensions;
                        let root_layout_idx =
                            self.layout_elements[root_elem_idx].layout_config_index;

                        let w_type = self.layout_configs[root_layout_idx].sizing.width.type_;
                        match w_type {
                            SizingType::Grow => {
                                self.layout_elements[root_elem_idx].dimensions.width =
                                    parent_dims.width;
                            }
                            SizingType::Percent => {
                                self.layout_elements[root_elem_idx].dimensions.width =
                                    parent_dims.width
                                        * self.layout_configs[root_layout_idx]
                                            .sizing
                                            .width
                                            .percent;
                            }
                            _ => {}
                        }
                        let h_type = self.layout_configs[root_layout_idx].sizing.height.type_;
                        match h_type {
                            SizingType::Grow => {
                                self.layout_elements[root_elem_idx].dimensions.height =
                                    parent_dims.height;
                            }
                            SizingType::Percent => {
                                self.layout_elements[root_elem_idx].dimensions.height =
                                    parent_dims.height
                                        * self.layout_configs[root_layout_idx]
                                            .sizing
                                            .height
                                            .percent;
                            }
                            _ => {}
                        }
                    }
                }
            }

            // Clamp root element
            let root_layout_idx = self.layout_elements[root_elem_idx].layout_config_index;
            if self.layout_configs[root_layout_idx].sizing.width.type_ != SizingType::Percent {
                let min = self.layout_configs[root_layout_idx].sizing.width.min_max.min;
                let max = self.layout_configs[root_layout_idx].sizing.width.min_max.max;
                self.layout_elements[root_elem_idx].dimensions.width = f32::min(
                    f32::max(self.layout_elements[root_elem_idx].dimensions.width, min),
                    max,
                );
            }
            if self.layout_configs[root_layout_idx].sizing.height.type_ != SizingType::Percent {
                let min = self.layout_configs[root_layout_idx].sizing.height.min_max.min;
                let max = self.layout_configs[root_layout_idx].sizing.height.min_max.max;
                self.layout_elements[root_elem_idx].dimensions.height = f32::min(
                    f32::max(self.layout_elements[root_elem_idx].dimensions.height, min),
                    max,
                );
            }

            let mut i = 0;
            while i < bfs_buffer.len() {
                let parent_index = bfs_buffer[i] as usize;
                i += 1;

                let parent_layout_idx = self.layout_elements[parent_index].layout_config_index;
                let parent_config = self.layout_configs[parent_layout_idx];
                let parent_size = if x_axis {
                    self.layout_elements[parent_index].dimensions.width
                } else {
                    self.layout_elements[parent_index].dimensions.height
                };
                let parent_padding = if x_axis {
                    (parent_config.padding.left + parent_config.padding.right) as f32
                } else {
                    (parent_config.padding.top + parent_config.padding.bottom) as f32
                };
                let sizing_along_axis = (x_axis
                    && parent_config.layout_direction == LayoutDirection::LeftToRight)
                    || (!x_axis
                        && parent_config.layout_direction == LayoutDirection::TopToBottom);

                let mut inner_content_size: f32 = 0.0;
                let mut total_padding_and_child_gaps = parent_padding;
                let mut grow_container_count: i32 = 0;
                let parent_child_gap = parent_config.child_gap as f32;

                resizable_container_buffer.clear();

                let children_start = self.layout_elements[parent_index].children_start;
                let children_length = self.layout_elements[parent_index].children_length as usize;

                for child_offset in 0..children_length {
                    let child_element_index =
                        self.layout_element_children[children_start + child_offset] as usize;
                    let child_layout_idx =
                        self.layout_elements[child_element_index].layout_config_index;
                    let child_sizing = if x_axis {
                        self.layout_configs[child_layout_idx].sizing.width
                    } else {
                        self.layout_configs[child_layout_idx].sizing.height
                    };
                    let child_size = if x_axis {
                        self.layout_elements[child_element_index].dimensions.width
                    } else {
                        self.layout_elements[child_element_index].dimensions.height
                    };

                    let is_text_element =
                        self.element_has_config(child_element_index, ElementConfigType::Text);
                    let has_children = self.layout_elements[child_element_index].children_length > 0;

                    if !is_text_element && has_children {
                        bfs_buffer.push(child_element_index as i32);
                    }

                    let is_wrapping_text = if is_text_element {
                        if let Some(text_cfg_idx) = self.find_element_config_index(
                            child_element_index,
                            ElementConfigType::Text,
                        ) {
                            self.text_element_configs[text_cfg_idx].wrap_mode
                                == WrapMode::Words
                        } else {
                            false
                        }
                    } else {
                        false
                    };

                    if child_sizing.type_ != SizingType::Percent
                        && child_sizing.type_ != SizingType::Fixed
                        && (!is_text_element || is_wrapping_text)
                    {
                        resizable_container_buffer.push(child_element_index as i32);
                    }

                    if sizing_along_axis {
                        inner_content_size += if child_sizing.type_ == SizingType::Percent {
                            0.0
                        } else {
                            child_size
                        };
                        if child_sizing.type_ == SizingType::Grow {
                            grow_container_count += 1;
                        }
                        if child_offset > 0 {
                            inner_content_size += parent_child_gap;
                            total_padding_and_child_gaps += parent_child_gap;
                        }
                    } else {
                        inner_content_size = f32::max(child_size, inner_content_size);
                    }
                }

                // Expand percentage containers
                for child_offset in 0..children_length {
                    let child_element_index =
                        self.layout_element_children[children_start + child_offset] as usize;
                    let child_layout_idx =
                        self.layout_elements[child_element_index].layout_config_index;
                    let child_sizing = if x_axis {
                        self.layout_configs[child_layout_idx].sizing.width
                    } else {
                        self.layout_configs[child_layout_idx].sizing.height
                    };
                    if child_sizing.type_ == SizingType::Percent {
                        let new_size =
                            (parent_size - total_padding_and_child_gaps) * child_sizing.percent;
                        if x_axis {
                            self.layout_elements[child_element_index].dimensions.width = new_size;
                        } else {
                            self.layout_elements[child_element_index].dimensions.height = new_size;
                        }
                        if sizing_along_axis {
                            inner_content_size += new_size;
                        }
                        self.update_aspect_ratio_box(child_element_index);
                    }
                }

                if sizing_along_axis {
                    let size_to_distribute = parent_size - parent_padding - inner_content_size;

                    if size_to_distribute < 0.0 {
                        // Check if parent clips
                        let parent_clips = if let Some(clip_idx) = self
                            .find_element_config_index(parent_index, ElementConfigType::Clip)
                        {
                            let clip = &self.clip_element_configs[clip_idx];
                            (x_axis && clip.horizontal) || (!x_axis && clip.vertical)
                        } else {
                            false
                        };
                        if parent_clips {
                            continue;
                        }

                        // Compress children
                        let mut distribute = size_to_distribute;
                        while distribute < -EPSILON && !resizable_container_buffer.is_empty() {
                            let mut largest: f32 = 0.0;
                            let mut second_largest: f32 = 0.0;
                            let mut width_to_add = distribute;

                            for &child_idx in &resizable_container_buffer {
                                let cs = if x_axis {
                                    self.layout_elements[child_idx as usize].dimensions.width
                                } else {
                                    self.layout_elements[child_idx as usize].dimensions.height
                                };
                                if float_equal(cs, largest) {
                                    continue;
                                }
                                if cs > largest {
                                    second_largest = largest;
                                    largest = cs;
                                }
                                if cs < largest {
                                    second_largest = f32::max(second_largest, cs);
                                    width_to_add = second_largest - largest;
                                }
                            }
                            width_to_add = f32::max(
                                width_to_add,
                                distribute / resizable_container_buffer.len() as f32,
                            );

                            let mut j = 0;
                            while j < resizable_container_buffer.len() {
                                let child_idx = resizable_container_buffer[j] as usize;
                                let current_size = if x_axis {
                                    self.layout_elements[child_idx].dimensions.width
                                } else {
                                    self.layout_elements[child_idx].dimensions.height
                                };
                                let min_size = if x_axis {
                                    self.layout_elements[child_idx].min_dimensions.width
                                } else {
                                    self.layout_elements[child_idx].min_dimensions.height
                                };
                                if float_equal(current_size, largest) {
                                    let new_size = current_size + width_to_add;
                                    if new_size <= min_size {
                                        if x_axis {
                                            self.layout_elements[child_idx].dimensions.width = min_size;
                                        } else {
                                            self.layout_elements[child_idx].dimensions.height = min_size;
                                        }
                                        distribute -= min_size - current_size;
                                        resizable_container_buffer.swap_remove(j);
                                        continue;
                                    }
                                    if x_axis {
                                        self.layout_elements[child_idx].dimensions.width = new_size;
                                    } else {
                                        self.layout_elements[child_idx].dimensions.height = new_size;
                                    }
                                    distribute -= new_size - current_size;
                                }
                                j += 1;
                            }
                        }
                    } else if size_to_distribute > 0.0 && grow_container_count > 0 {
                        // Remove non-grow from resizable buffer
                        let mut j = 0;
                        while j < resizable_container_buffer.len() {
                            let child_idx = resizable_container_buffer[j] as usize;
                            let child_layout_idx =
                                self.layout_elements[child_idx].layout_config_index;
                            let child_sizing_type = if x_axis {
                                self.layout_configs[child_layout_idx].sizing.width.type_
                            } else {
                                self.layout_configs[child_layout_idx].sizing.height.type_
                            };
                            if child_sizing_type != SizingType::Grow {
                                resizable_container_buffer.swap_remove(j);
                            } else {
                                j += 1;
                            }
                        }

                        let mut distribute = size_to_distribute;
                        while distribute > EPSILON && !resizable_container_buffer.is_empty() {
                            let mut smallest: f32 = MAXFLOAT;
                            let mut second_smallest: f32 = MAXFLOAT;
                            let mut width_to_add = distribute;

                            for &child_idx in &resizable_container_buffer {
                                let cs = if x_axis {
                                    self.layout_elements[child_idx as usize].dimensions.width
                                } else {
                                    self.layout_elements[child_idx as usize].dimensions.height
                                };
                                if float_equal(cs, smallest) {
                                    continue;
                                }
                                if cs < smallest {
                                    second_smallest = smallest;
                                    smallest = cs;
                                }
                                if cs > smallest {
                                    second_smallest = f32::min(second_smallest, cs);
                                    width_to_add = second_smallest - smallest;
                                }
                            }
                            width_to_add = f32::min(
                                width_to_add,
                                distribute / resizable_container_buffer.len() as f32,
                            );

                            let mut j = 0;
                            while j < resizable_container_buffer.len() {
                                let child_idx = resizable_container_buffer[j] as usize;
                                let child_layout_idx =
                                    self.layout_elements[child_idx].layout_config_index;
                                let max_size = if x_axis {
                                    self.layout_configs[child_layout_idx]
                                        .sizing
                                        .width
                                        .min_max
                                        .max
                                } else {
                                    self.layout_configs[child_layout_idx]
                                        .sizing
                                        .height
                                        .min_max
                                        .max
                                };
                                let child_size_ref = if x_axis {
                                    &mut self.layout_elements[child_idx].dimensions.width
                                } else {
                                    &mut self.layout_elements[child_idx].dimensions.height
                                };
                                if float_equal(*child_size_ref, smallest) {
                                    let previous = *child_size_ref;
                                    *child_size_ref += width_to_add;
                                    if *child_size_ref >= max_size {
                                        *child_size_ref = max_size;
                                        resizable_container_buffer.swap_remove(j);
                                        continue;
                                    }
                                    distribute -= *child_size_ref - previous;
                                }
                                j += 1;
                            }
                        }
                    }
                } else {
                    // Off-axis sizing
                    for &child_idx in &resizable_container_buffer {
                        let child_idx = child_idx as usize;
                        let child_layout_idx =
                            self.layout_elements[child_idx].layout_config_index;
                        let child_sizing = if x_axis {
                            self.layout_configs[child_layout_idx].sizing.width
                        } else {
                            self.layout_configs[child_layout_idx].sizing.height
                        };
                        let min_size = if x_axis {
                            self.layout_elements[child_idx].min_dimensions.width
                        } else {
                            self.layout_elements[child_idx].min_dimensions.height
                        };

                        let mut max_size = parent_size - parent_padding;
                        if let Some(clip_idx) =
                            self.find_element_config_index(parent_index, ElementConfigType::Clip)
                        {
                            let clip = &self.clip_element_configs[clip_idx];
                            if (x_axis && clip.horizontal) || (!x_axis && clip.vertical) {
                                max_size = f32::max(max_size, inner_content_size);
                            }
                        }

                        let child_size_ref = if x_axis {
                            &mut self.layout_elements[child_idx].dimensions.width
                        } else {
                            &mut self.layout_elements[child_idx].dimensions.height
                        };

                        if child_sizing.type_ == SizingType::Grow {
                            *child_size_ref =
                                f32::min(max_size, child_sizing.min_max.max);
                        }
                        *child_size_ref = f32::max(min_size, f32::min(*child_size_ref, max_size));
                    }
                }
            }
        }
    }

    fn calculate_final_layout(&mut self) {
        // Size along X axis
        self.size_containers_along_axis(true);

        // Wrap text
        self.wrap_text();

        // Scale vertical heights by aspect ratio
        for i in 0..self.aspect_ratio_element_indexes.len() {
            let elem_idx = self.aspect_ratio_element_indexes[i] as usize;
            if let Some(cfg_idx) =
                self.find_element_config_index(elem_idx, ElementConfigType::Aspect)
            {
                let aspect_ratio = self.aspect_ratio_configs[cfg_idx];
                self.layout_elements[elem_idx].dimensions.height =
                    (1.0 / aspect_ratio) * self.layout_elements[elem_idx].dimensions.width;
                let layout_idx = self.layout_elements[elem_idx].layout_config_index;
                self.layout_configs[layout_idx].sizing.height.min_max.max =
                    self.layout_elements[elem_idx].dimensions.height;
            }
        }

        // Propagate height changes up tree (DFS)
        self.propagate_sizes_up_tree();

        // Size along Y axis
        self.size_containers_along_axis(false);

        // Scale horizontal widths by aspect ratio
        for i in 0..self.aspect_ratio_element_indexes.len() {
            let elem_idx = self.aspect_ratio_element_indexes[i] as usize;
            if let Some(cfg_idx) =
                self.find_element_config_index(elem_idx, ElementConfigType::Aspect)
            {
                let aspect_ratio = self.aspect_ratio_configs[cfg_idx];
                self.layout_elements[elem_idx].dimensions.width =
                    aspect_ratio * self.layout_elements[elem_idx].dimensions.height;
            }
        }

        // Sort tree roots by z-index (bubble sort)
        let mut sort_max = self.layout_element_tree_roots.len().saturating_sub(1);
        while sort_max > 0 {
            for i in 0..sort_max {
                if self.layout_element_tree_roots[i + 1].z_index
                    < self.layout_element_tree_roots[i].z_index
                {
                    self.layout_element_tree_roots.swap(i, i + 1);
                }
            }
            sort_max -= 1;
        }

        // Generate render commands
        self.generate_render_commands();
    }

    fn wrap_text(&mut self) {
        for text_idx in 0..self.text_element_data.len() {
            let elem_index = self.text_element_data[text_idx].element_index as usize;
            let text = self.text_element_data[text_idx].text.clone();
            let preferred_dims = self.text_element_data[text_idx].preferred_dimensions;

            self.text_element_data[text_idx].wrapped_lines_start = self.wrapped_text_lines.len();
            self.text_element_data[text_idx].wrapped_lines_length = 0;

            let container_width = self.layout_elements[elem_index].dimensions.width;

            // Find text config
            let text_config_idx = self
                .find_element_config_index(elem_index, ElementConfigType::Text)
                .unwrap_or(0);
            let text_config = self.text_element_configs[text_config_idx].clone();

            let measured = self.measure_text_cached(&text, &text_config);

            let line_height = if text_config.line_height > 0 {
                text_config.line_height as f32
            } else {
                preferred_dims.height
            };

            if !measured.contains_newlines && preferred_dims.width <= container_width {
                // Single line
                self.wrapped_text_lines.push(WrappedTextLine {
                    dimensions: self.layout_elements[elem_index].dimensions,
                    start: 0,
                    length: text.len(),
                });
                self.text_element_data[text_idx].wrapped_lines_length = 1;
                continue;
            }

            // Multi-line wrapping
            let measure_fn = self.measure_text_fn.as_ref().unwrap();
            let space_width = {
                let space_config = text_config.clone();
                measure_fn(" ", &space_config).width
            };

            let mut word_index = measured.measured_words_start_index;
            let mut line_width: f32 = 0.0;
            let mut line_length_chars: i32 = 0;
            let mut line_start_offset: i32 = 0;

            while word_index != -1 {
                let measured_word = self.measured_words[word_index as usize];

                // Word doesn't fit but it's the only word on the line
                if line_length_chars == 0 && line_width + measured_word.width > container_width {
                    self.wrapped_text_lines.push(WrappedTextLine {
                        dimensions: Dimensions::new(measured_word.width, line_height),
                        start: measured_word.start_offset as usize,
                        length: measured_word.length as usize,
                    });
                    self.text_element_data[text_idx].wrapped_lines_length += 1;
                    word_index = measured_word.next;
                    line_start_offset = measured_word.start_offset + measured_word.length;
                }
                // Newline or overflow
                else if measured_word.length == 0
                    || line_width + measured_word.width > container_width
                {
                    let text_bytes = text.as_bytes();
                    let final_char_idx = (line_start_offset + line_length_chars - 1).max(0) as usize;
                    let final_char_is_space =
                        final_char_idx < text_bytes.len() && text_bytes[final_char_idx] == b' ';
                    let adj_width = line_width
                        + if final_char_is_space {
                            -space_width
                        } else {
                            0.0
                        };
                    let adj_length = line_length_chars
                        + if final_char_is_space { -1 } else { 0 };

                    self.wrapped_text_lines.push(WrappedTextLine {
                        dimensions: Dimensions::new(adj_width, line_height),
                        start: line_start_offset as usize,
                        length: adj_length as usize,
                    });
                    self.text_element_data[text_idx].wrapped_lines_length += 1;

                    if line_length_chars == 0 || measured_word.length == 0 {
                        word_index = measured_word.next;
                    }
                    line_width = 0.0;
                    line_length_chars = 0;
                    line_start_offset = measured_word.start_offset;
                } else {
                    line_width += measured_word.width + text_config.letter_spacing as f32;
                    line_length_chars += measured_word.length;
                    word_index = measured_word.next;
                }
            }

            if line_length_chars > 0 {
                self.wrapped_text_lines.push(WrappedTextLine {
                    dimensions: Dimensions::new(
                        line_width - text_config.letter_spacing as f32,
                        line_height,
                    ),
                    start: line_start_offset as usize,
                    length: line_length_chars as usize,
                });
                self.text_element_data[text_idx].wrapped_lines_length += 1;
            }

            let num_lines = self.text_element_data[text_idx].wrapped_lines_length;
            self.layout_elements[elem_index].dimensions.height =
                line_height * num_lines as f32;
        }
    }

    fn propagate_sizes_up_tree(&mut self) {
        let mut dfs_buffer: Vec<i32> = Vec::new();
        let mut visited: Vec<bool> = Vec::new();

        for i in 0..self.layout_element_tree_roots.len() {
            let root = self.layout_element_tree_roots[i];
            dfs_buffer.push(root.layout_element_index);
            visited.push(false);
        }

        while !dfs_buffer.is_empty() {
            let buf_idx = dfs_buffer.len() - 1;
            let current_elem_idx = dfs_buffer[buf_idx] as usize;

            if !visited[buf_idx] {
                visited[buf_idx] = true;
                let is_text =
                    self.element_has_config(current_elem_idx, ElementConfigType::Text);
                let children_length = self.layout_elements[current_elem_idx].children_length;
                if is_text || children_length == 0 {
                    dfs_buffer.pop();
                    visited.pop();
                    continue;
                }
                let children_start = self.layout_elements[current_elem_idx].children_start;
                for j in 0..children_length as usize {
                    let child_idx = self.layout_element_children[children_start + j];
                    dfs_buffer.push(child_idx);
                    visited.push(false);
                }
                continue;
            }

            dfs_buffer.pop();
            visited.pop();

            let layout_idx = self.layout_elements[current_elem_idx].layout_config_index;
            let layout_config = self.layout_configs[layout_idx];
            let children_start = self.layout_elements[current_elem_idx].children_start;
            let children_length = self.layout_elements[current_elem_idx].children_length;

            if layout_config.layout_direction == LayoutDirection::LeftToRight {
                for j in 0..children_length as usize {
                    let child_idx =
                        self.layout_element_children[children_start + j] as usize;
                    let child_height_with_padding = f32::max(
                        self.layout_elements[child_idx].dimensions.height
                            + layout_config.padding.top as f32
                            + layout_config.padding.bottom as f32,
                        self.layout_elements[current_elem_idx].dimensions.height,
                    );
                    self.layout_elements[current_elem_idx].dimensions.height = f32::min(
                        f32::max(
                            child_height_with_padding,
                            layout_config.sizing.height.min_max.min,
                        ),
                        layout_config.sizing.height.min_max.max,
                    );
                }
            } else {
                let mut content_height = layout_config.padding.top as f32
                    + layout_config.padding.bottom as f32;
                for j in 0..children_length as usize {
                    let child_idx =
                        self.layout_element_children[children_start + j] as usize;
                    content_height += self.layout_elements[child_idx].dimensions.height;
                }
                content_height += children_length.saturating_sub(1) as f32
                    * layout_config.child_gap as f32;
                self.layout_elements[current_elem_idx].dimensions.height = f32::min(
                    f32::max(content_height, layout_config.sizing.height.min_max.min),
                    layout_config.sizing.height.min_max.max,
                );
            }
        }
    }

    fn element_is_offscreen(&self, bbox: &BoundingBox) -> bool {
        if self.culling_disabled {
            return false;
        }
        bbox.x > self.layout_dimensions.width
            || bbox.y > self.layout_dimensions.height
            || bbox.x + bbox.width < 0.0
            || bbox.y + bbox.height < 0.0
    }

    fn add_render_command(&mut self, cmd: InternalRenderCommand<CustomElementData>) {
        self.render_commands.push(cmd);
    }

    fn generate_render_commands(&mut self) {
        self.render_commands.clear();
        let mut dfs_buffer: Vec<LayoutElementTreeNode> = Vec::new();
        let mut visited: Vec<bool> = Vec::new();

        for root_index in 0..self.layout_element_tree_roots.len() {
            dfs_buffer.clear();
            visited.clear();
            let root = self.layout_element_tree_roots[root_index];
            let root_elem_idx = root.layout_element_index as usize;
            let root_element = &self.layout_elements[root_elem_idx];
            let mut root_position = Vector2::default();

            // Position floating containers
            if self.element_has_config(root_elem_idx, ElementConfigType::Floating) {
                if let Some(parent_item) = self.layout_element_map.get(&root.parent_id) {
                    let parent_bbox = parent_item.bounding_box;
                    if let Some(float_cfg_idx) = self
                        .find_element_config_index(root_elem_idx, ElementConfigType::Floating)
                    {
                        let config = &self.floating_element_configs[float_cfg_idx];
                        let root_dims = root_element.dimensions;
                        let mut target = Vector2::default();

                        // X position - parent attach point
                        match config.attach_points.parent_x {
                            AlignX::Left => {
                                target.x = parent_bbox.x;
                            }
                            AlignX::CenterX => {
                                target.x = parent_bbox.x + parent_bbox.width / 2.0;
                            }
                            AlignX::Right => {
                                target.x = parent_bbox.x + parent_bbox.width;
                            }
                        }
                        // X position - element attach point
                        match config.attach_points.element_x {
                            AlignX::Left => {}
                            AlignX::CenterX => {
                                target.x -= root_dims.width / 2.0;
                            }
                            AlignX::Right => {
                                target.x -= root_dims.width;
                            }
                        }
                        // Y position - parent attach point
                        match config.attach_points.parent_y {
                            AlignY::Top => {
                                target.y = parent_bbox.y;
                            }
                            AlignY::CenterY => {
                                target.y = parent_bbox.y + parent_bbox.height / 2.0;
                            }
                            AlignY::Bottom => {
                                target.y = parent_bbox.y + parent_bbox.height;
                            }
                        }
                        // Y position - element attach point
                        match config.attach_points.element_y {
                            AlignY::Top => {}
                            AlignY::CenterY => {
                                target.y -= root_dims.height / 2.0;
                            }
                            AlignY::Bottom => {
                                target.y -= root_dims.height;
                            }
                        }
                        target.x += config.offset.x;
                        target.y += config.offset.y;
                        root_position = target;
                    }
                }
            }

            // Clip scissor start
            if root.clip_element_id != 0 {
                if let Some(clip_item) = self.layout_element_map.get(&root.clip_element_id) {
                    let clip_bbox = clip_item.bounding_box;
                    self.add_render_command(InternalRenderCommand {
                        bounding_box: clip_bbox,
                        command_type: RenderCommandType::ScissorStart,
                        id: hash_number(
                            root_element.id,
                            root_element.children_length as u32 + 10,
                        )
                        .id,
                        z_index: root.z_index,
                        ..Default::default()
                    });
                }
            }

            let root_layout_idx = self.layout_elements[root_elem_idx].layout_config_index;
            let root_padding_left = self.layout_configs[root_layout_idx].padding.left as f32;
            let root_padding_top = self.layout_configs[root_layout_idx].padding.top as f32;

            dfs_buffer.push(LayoutElementTreeNode {
                layout_element_index: root.layout_element_index,
                position: root_position,
                next_child_offset: Vector2::new(root_padding_left, root_padding_top),
            });
            visited.push(false);

            while !dfs_buffer.is_empty() {
                let buf_idx = dfs_buffer.len() - 1;
                let current_node = dfs_buffer[buf_idx];
                let current_elem_idx = current_node.layout_element_index as usize;
                let layout_idx = self.layout_elements[current_elem_idx].layout_config_index;
                let layout_config = self.layout_configs[layout_idx];
                let mut scroll_offset = Vector2::default();

                if !visited[buf_idx] {
                    visited[buf_idx] = true;

                    let current_bbox = BoundingBox::new(
                        current_node.position.x,
                        current_node.position.y,
                        self.layout_elements[current_elem_idx].dimensions.width,
                        self.layout_elements[current_elem_idx].dimensions.height,
                    );

                    // Apply scroll offset
                    let mut _scroll_container_data_idx: Option<usize> = None;
                    if self.element_has_config(current_elem_idx, ElementConfigType::Clip) {
                        if let Some(clip_cfg_idx) = self
                            .find_element_config_index(current_elem_idx, ElementConfigType::Clip)
                        {
                            let clip_config = self.clip_element_configs[clip_cfg_idx];
                            for si in 0..self.scroll_container_datas.len() {
                                if self.scroll_container_datas[si].layout_element_index
                                    == current_elem_idx as i32
                                {
                                    _scroll_container_data_idx = Some(si);
                                    self.scroll_container_datas[si].bounding_box = current_bbox;
                                    scroll_offset = clip_config.child_offset;
                                    break;
                                }
                            }
                        }
                    }

                    // Update hash map bounding box
                    let elem_id = self.layout_elements[current_elem_idx].id;
                    if let Some(item) = self.layout_element_map.get_mut(&elem_id) {
                        item.bounding_box = current_bbox;
                    }

                    // Generate render commands for this element
                    let shared_config = self
                        .find_element_config_index(current_elem_idx, ElementConfigType::Shared)
                        .map(|idx| self.shared_element_configs[idx]);
                    let shared = shared_config.unwrap_or_default();
                    let mut emit_rectangle = shared.background_color.a > 0.0;
                    let offscreen = self.element_is_offscreen(&current_bbox);
                    let should_render_base = !offscreen;

                    // Get per-element shader effects
                    let elem_effects = self.element_effects.get(current_elem_idx).cloned().unwrap_or_default();

                    // Get per-element visual rotation
                    let elem_visual_rotation = self.element_visual_rotations.get(current_elem_idx).cloned().flatten();
                    // Filter out no-op rotations
                    let elem_visual_rotation = elem_visual_rotation.filter(|vr| !vr.is_noop());

                    // Get per-element shape rotation and compute original bbox
                    let elem_shape_rotation = self.element_shape_rotations.get(current_elem_idx).cloned().flatten()
                        .filter(|sr| !sr.is_noop());
                    // If shape rotation is active, current_bbox has AABB dims.
                    // Compute the original-dimension bbox centered within the AABB.
                    let shape_draw_bbox = if let Some(ref _sr) = elem_shape_rotation {
                        if let Some(orig_dims) = self.element_pre_rotation_dimensions.get(current_elem_idx).copied().flatten() {
                            let offset_x = (current_bbox.width - orig_dims.width) / 2.0;
                            let offset_y = (current_bbox.height - orig_dims.height) / 2.0;
                            BoundingBox::new(
                                current_bbox.x + offset_x,
                                current_bbox.y + offset_y,
                                orig_dims.width,
                                orig_dims.height,
                            )
                        } else {
                            current_bbox
                        }
                    } else {
                        current_bbox
                    };

                    // Emit GroupBegin commands for group shaders BEFORE element drawing
                    // so that the element's background, children, and border are all captured.
                    // If visual_rotation is present, it is attached to the outermost group.
                    let elem_shaders = self.element_shaders.get(current_elem_idx).cloned().unwrap_or_default();

                    if !elem_shaders.is_empty() {
                        // Emit GroupBegin for each shader (outermost first = reversed order)
                        for (i, shader) in elem_shaders.iter().rev().enumerate() {
                            // Attach visual_rotation to the outermost GroupBegin (i == 0)
                            let vr = if i == 0 { elem_visual_rotation } else { None };
                            self.add_render_command(InternalRenderCommand {
                                bounding_box: current_bbox,
                                command_type: RenderCommandType::GroupBegin,
                                effects: vec![shader.clone()],
                                id: elem_id,
                                z_index: root.z_index,
                                visual_rotation: vr,
                                ..Default::default()
                            });
                        }
                    } else if let Some(vr) = elem_visual_rotation {
                        // No shaders but visual rotation: emit standalone GroupBegin/End
                        self.add_render_command(InternalRenderCommand {
                            bounding_box: current_bbox,
                            command_type: RenderCommandType::GroupBegin,
                            effects: vec![],
                            id: elem_id,
                            z_index: root.z_index,
                            visual_rotation: Some(vr),
                            ..Default::default()
                        });
                    }

                    // Process each config
                    let configs_start = self.layout_elements[current_elem_idx].element_configs.start;
                    let configs_length =
                        self.layout_elements[current_elem_idx].element_configs.length;

                    for cfg_i in 0..configs_length {
                        let config = self.element_configs[configs_start + cfg_i as usize];
                        let should_render = should_render_base;

                        match config.config_type {
                            ElementConfigType::Shared
                            | ElementConfigType::Aspect
                            | ElementConfigType::Floating
                            | ElementConfigType::Border => {}
                            ElementConfigType::Clip => {
                                if should_render {
                                    let clip = &self.clip_element_configs[config.config_index];
                                    self.add_render_command(InternalRenderCommand {
                                        bounding_box: current_bbox,
                                        command_type: RenderCommandType::ScissorStart,
                                        render_data: InternalRenderData::Clip {
                                            horizontal: clip.horizontal,
                                            vertical: clip.vertical,
                                        },
                                        user_data: 0,
                                        id: elem_id,
                                        z_index: root.z_index,
                                        visual_rotation: None,
                                        shape_rotation: None,
                                        effects: Vec::new(),
                                    });
                                }
                            }
                            ElementConfigType::Image => {
                                if should_render {
                                    let image_data =
                                        self.image_element_configs[config.config_index].clone();
                                    self.add_render_command(InternalRenderCommand {
                                        bounding_box: shape_draw_bbox,
                                        command_type: RenderCommandType::Image,
                                        render_data: InternalRenderData::Image {
                                            background_color: shared.background_color,
                                            corner_radius: shared.corner_radius,
                                            image_data,
                                        },
                                        user_data: shared.user_data,
                                        id: elem_id,
                                        z_index: root.z_index,
                                        visual_rotation: None,
                                        shape_rotation: elem_shape_rotation,
                                        effects: elem_effects.clone(),
                                    });
                                }
                                emit_rectangle = false;
                            }
                            ElementConfigType::Text => {
                                if !should_render {
                                    continue;
                                }
                                let text_config =
                                    self.text_element_configs[config.config_index].clone();
                                let text_data_idx =
                                    self.layout_elements[current_elem_idx].text_data_index;
                                if text_data_idx < 0 {
                                    continue;
                                }
                                let text_data = &self.text_element_data[text_data_idx as usize];
                                let natural_line_height = text_data.preferred_dimensions.height;
                                let final_line_height = if text_config.line_height > 0 {
                                    text_config.line_height as f32
                                } else {
                                    natural_line_height
                                };
                                let line_height_offset =
                                    (final_line_height - natural_line_height) / 2.0;
                                let mut y_position = line_height_offset;

                                let lines_start = text_data.wrapped_lines_start;
                                let lines_length = text_data.wrapped_lines_length;
                                let parent_text = text_data.text.clone();

                                // Collect line data first to avoid borrow issues
                                let lines_data: Vec<_> = (0..lines_length)
                                    .map(|li| {
                                        let line = &self.wrapped_text_lines[lines_start + li as usize];
                                        (line.start, line.length, line.dimensions)
                                    })
                                    .collect();

                                for (line_index, &(start, length, line_dims)) in lines_data.iter().enumerate() {
                                    if length == 0 {
                                        y_position += final_line_height;
                                        continue;
                                    }

                                    let line_text = parent_text[start..start + length].to_string();

                                    let mut offset =
                                        current_bbox.width - line_dims.width;
                                    if text_config.alignment == AlignX::Left {
                                        offset = 0.0;
                                    }
                                    if text_config.alignment == AlignX::CenterX {
                                        offset /= 2.0;
                                    }

                                    self.add_render_command(InternalRenderCommand {
                                        bounding_box: BoundingBox::new(
                                            current_bbox.x + offset,
                                            current_bbox.y + y_position,
                                            line_dims.width,
                                            line_dims.height,
                                        ),
                                        command_type: RenderCommandType::Text,
                                        render_data: InternalRenderData::Text {
                                            text: line_text,
                                            text_color: text_config.color,
                                            font_size: text_config.font_size,
                                            letter_spacing: text_config.letter_spacing,
                                            line_height: text_config.line_height,
                                            font_asset: text_config.font_asset,
                                        },
                                        user_data: text_config.user_data,
                                        id: hash_number(line_index as u32, elem_id).id,
                                        z_index: root.z_index,
                                        visual_rotation: None,
                                        shape_rotation: None,
                                        effects: text_config.effects.clone(),
                                    });
                                    y_position += final_line_height;
                                }
                            }
                            ElementConfigType::Custom => {
                                if should_render {
                                    let custom_data =
                                        self.custom_element_configs[config.config_index].clone();
                                    self.add_render_command(InternalRenderCommand {
                                        bounding_box: shape_draw_bbox,
                                        command_type: RenderCommandType::Custom,
                                        render_data: InternalRenderData::Custom {
                                            background_color: shared.background_color,
                                            corner_radius: shared.corner_radius,
                                            custom_data,
                                        },
                                        user_data: shared.user_data,
                                        id: elem_id,
                                        z_index: root.z_index,
                                        visual_rotation: None,
                                        shape_rotation: elem_shape_rotation,
                                        effects: elem_effects.clone(),
                                    });
                                }
                                emit_rectangle = false;
                            }
                            ElementConfigType::TextInput => {
                                if should_render {
                                    let ti_config = self.text_input_configs[config.config_index].clone();
                                    let is_focused = self.focused_element_id == elem_id;

                                    // Emit background rectangle FIRST so text renders on top
                                    if shared.background_color.a > 0.0 || shared.corner_radius.bottom_left > 0.0 {
                                        self.add_render_command(InternalRenderCommand {
                                            bounding_box: shape_draw_bbox,
                                            command_type: RenderCommandType::Rectangle,
                                            render_data: InternalRenderData::Rectangle {
                                                background_color: shared.background_color,
                                                corner_radius: shared.corner_radius,
                                            },
                                            user_data: shared.user_data,
                                            id: elem_id,
                                            z_index: root.z_index,
                                            visual_rotation: None,
                                            shape_rotation: elem_shape_rotation,
                                            effects: elem_effects.clone(),
                                        });
                                    }

                                    // Get or create edit state
                                    let state = self.text_edit_states
                                        .entry(elem_id)
                                        .or_insert_with(crate::text_input::TextEditState::default)
                                        .clone();

                                    let disp_text = crate::text_input::display_text(
                                        &state.text,
                                        &ti_config.placeholder,
                                        ti_config.is_password && !ti_config.is_multiline,
                                    );

                                    let is_placeholder = state.text.is_empty();
                                    let text_color = if is_placeholder {
                                        ti_config.placeholder_color
                                    } else {
                                        ti_config.text_color
                                    };

                                    // Measure font height for cursor
                                    let font_height = self.font_height(ti_config.font_asset, ti_config.font_size);

                                    // Clip text content to the element's bounding box
                                    self.add_render_command(InternalRenderCommand {
                                        bounding_box: current_bbox,
                                        command_type: RenderCommandType::ScissorStart,
                                        render_data: InternalRenderData::Clip {
                                            horizontal: true,
                                            vertical: true,
                                        },
                                        user_data: 0,
                                        id: hash_number(1000, elem_id).id,
                                        z_index: root.z_index,
                                        visual_rotation: None,
                                        shape_rotation: None,
                                        effects: Vec::new(),
                                    });

                                    if ti_config.is_multiline {
                                        // ── Multiline rendering (with word wrapping) ──
                                        let scroll_offset_x = state.scroll_offset;
                                        let scroll_offset_y = state.scroll_offset_y;

                                        let visual_lines = if let Some(ref measure_fn) = self.measure_text_fn {
                                            crate::text_input::wrap_lines(
                                                &disp_text,
                                                current_bbox.width,
                                                ti_config.font_asset,
                                                ti_config.font_size,
                                                measure_fn.as_ref(),
                                            )
                                        } else {
                                            vec![crate::text_input::VisualLine {
                                                text: disp_text.clone(),
                                                global_char_start: 0,
                                                char_count: disp_text.chars().count(),
                                            }]
                                        };

                                        let (cursor_line, cursor_col) = if is_placeholder {
                                            (0, 0)
                                        } else {
                                            #[cfg(feature = "text-styling")]
                                            let raw_cursor = state.cursor_pos_raw();
                                            #[cfg(not(feature = "text-styling"))]
                                            let raw_cursor = state.cursor_pos;
                                            crate::text_input::cursor_to_visual_pos(&visual_lines, raw_cursor)
                                        };

                                        // Compute per-line char positions
                                        let line_positions: Vec<Vec<f32>> = if let Some(ref measure_fn) = self.measure_text_fn {
                                            visual_lines.iter().map(|vl| {
                                                crate::text_input::compute_char_x_positions(
                                                    &vl.text,
                                                    ti_config.font_asset,
                                                    ti_config.font_size,
                                                    measure_fn.as_ref(),
                                                )
                                            }).collect()
                                        } else {
                                            visual_lines.iter().map(|_| vec![0.0]).collect()
                                        };

                                        // Selection rendering (multiline)
                                        if is_focused {
                                            #[cfg(feature = "text-styling")]
                                            let sel_range = state.selection_range_raw();
                                            #[cfg(not(feature = "text-styling"))]
                                            let sel_range = state.selection_range();
                                            if let Some((sel_start, sel_end)) = sel_range {
                                                let (sel_start_line, sel_start_col) = crate::text_input::cursor_to_visual_pos(&visual_lines, sel_start);
                                                let (sel_end_line, sel_end_col) = crate::text_input::cursor_to_visual_pos(&visual_lines, sel_end);
                                                for (line_idx, vl) in visual_lines.iter().enumerate() {
                                                    if line_idx < sel_start_line || line_idx > sel_end_line {
                                                        continue;
                                                    }
                                                    let positions = &line_positions[line_idx];
                                                    let col_start = if line_idx == sel_start_line { sel_start_col } else { 0 };
                                                    let col_end = if line_idx == sel_end_line { sel_end_col } else { vl.char_count };
                                                    let x_start = positions.get(col_start).copied().unwrap_or(0.0);
                                                    let x_end = positions.get(col_end).copied().unwrap_or(
                                                        positions.last().copied().unwrap_or(0.0)
                                                    );
                                                    let sel_width = x_end - x_start;
                                                    if sel_width > 0.0 {
                                                        let sel_y = current_bbox.y + line_idx as f32 * font_height - scroll_offset_y;
                                                        self.add_render_command(InternalRenderCommand {
                                                            bounding_box: BoundingBox::new(
                                                                current_bbox.x - scroll_offset_x + x_start,
                                                                sel_y,
                                                                sel_width,
                                                                font_height,
                                                            ),
                                                            command_type: RenderCommandType::Rectangle,
                                                            render_data: InternalRenderData::Rectangle {
                                                                background_color: ti_config.selection_color,
                                                                corner_radius: CornerRadius::default(),
                                                            },
                                                            user_data: 0,
                                                            id: hash_number(1001 + line_idx as u32, elem_id).id,
                                                            z_index: root.z_index,
                                                            visual_rotation: None,
                                                            shape_rotation: None,
                                                            effects: Vec::new(),
                                                        });
                                                    }
                                                }
                                            }
                                        }

                                        // Render each visual line of text
                                        for (line_idx, vl) in visual_lines.iter().enumerate() {
                                            if !vl.text.is_empty() {
                                                let positions = &line_positions[line_idx];
                                                let text_width = positions.last().copied().unwrap_or(0.0);
                                                let line_y = current_bbox.y + line_idx as f32 * font_height - scroll_offset_y;
                                                self.add_render_command(InternalRenderCommand {
                                                    bounding_box: BoundingBox::new(
                                                        current_bbox.x - scroll_offset_x,
                                                        line_y,
                                                        text_width,
                                                        font_height,
                                                    ),
                                                    command_type: RenderCommandType::Text,
                                                    render_data: InternalRenderData::Text {
                                                        text: vl.text.clone(),
                                                        text_color,
                                                        font_size: ti_config.font_size,
                                                        letter_spacing: 0,
                                                        line_height: 0,
                                                        font_asset: ti_config.font_asset,
                                                    },
                                                    user_data: 0,
                                                    id: hash_number(2000 + line_idx as u32, elem_id).id,
                                                    z_index: root.z_index,
                                                    visual_rotation: None,
                                                    shape_rotation: None,
                                                    effects: Vec::new(),
                                                });
                                            }
                                        }

                                        // Cursor (multiline)
                                        if is_focused && state.cursor_visible() {
                                            let cursor_positions = &line_positions[cursor_line.min(line_positions.len() - 1)];
                                            let cursor_x_pos = cursor_positions.get(cursor_col).copied().unwrap_or(0.0);
                                            let cursor_y = current_bbox.y + cursor_line as f32 * font_height - scroll_offset_y;
                                            self.add_render_command(InternalRenderCommand {
                                                bounding_box: BoundingBox::new(
                                                    current_bbox.x - scroll_offset_x + cursor_x_pos,
                                                    cursor_y,
                                                    2.0,
                                                    font_height,
                                                ),
                                                command_type: RenderCommandType::Rectangle,
                                                render_data: InternalRenderData::Rectangle {
                                                    background_color: ti_config.cursor_color,
                                                    corner_radius: CornerRadius::default(),
                                                },
                                                user_data: 0,
                                                id: hash_number(1003, elem_id).id,
                                                z_index: root.z_index,
                                                visual_rotation: None,
                                                shape_rotation: None,
                                                effects: Vec::new(),
                                            });
                                        }
                                    } else {
                                        // ── Single-line rendering ──
                                        let char_x_positions = if let Some(ref measure_fn) = self.measure_text_fn {
                                            crate::text_input::compute_char_x_positions(
                                                &disp_text,
                                                ti_config.font_asset,
                                                ti_config.font_size,
                                                measure_fn.as_ref(),
                                            )
                                        } else {
                                            vec![0.0]
                                        };

                                        let scroll_offset = state.scroll_offset;
                                        let text_x = current_bbox.x - scroll_offset;

                                        // Convert cursor/selection to raw positions for char_x_positions indexing
                                        #[cfg(feature = "text-styling")]
                                        let render_cursor_pos = if is_placeholder { 0 } else { state.cursor_pos_raw() };
                                        #[cfg(not(feature = "text-styling"))]
                                        let render_cursor_pos = if is_placeholder { 0 } else { state.cursor_pos };

                                        #[cfg(feature = "text-styling")]
                                        let render_selection = if !is_placeholder { state.selection_range_raw() } else { None };
                                        #[cfg(not(feature = "text-styling"))]
                                        let render_selection = if !is_placeholder { state.selection_range() } else { None };

                                        // Selection highlight
                                        if is_focused {
                                            if let Some((sel_start, sel_end)) = render_selection {
                                                let sel_start_x = char_x_positions.get(sel_start).copied().unwrap_or(0.0);
                                                let sel_end_x = char_x_positions.get(sel_end).copied().unwrap_or(0.0);
                                                let sel_width = sel_end_x - sel_start_x;
                                                if sel_width > 0.0 {
                                                    let sel_y = current_bbox.y + (current_bbox.height - font_height) / 2.0;
                                                    self.add_render_command(InternalRenderCommand {
                                                        bounding_box: BoundingBox::new(
                                                            text_x + sel_start_x,
                                                            sel_y,
                                                            sel_width,
                                                            font_height,
                                                        ),
                                                        command_type: RenderCommandType::Rectangle,
                                                        render_data: InternalRenderData::Rectangle {
                                                            background_color: ti_config.selection_color,
                                                            corner_radius: CornerRadius::default(),
                                                        },
                                                        user_data: 0,
                                                        id: hash_number(1001, elem_id).id,
                                                        z_index: root.z_index,
                                                        visual_rotation: None,
                                                        shape_rotation: None,
                                                        effects: Vec::new(),
                                                    });
                                                }
                                            }
                                        }

                                        // Text
                                        if !disp_text.is_empty() {
                                            let text_width = char_x_positions.last().copied().unwrap_or(0.0);
                                            let text_y = current_bbox.y + (current_bbox.height - font_height) / 2.0;
                                            self.add_render_command(InternalRenderCommand {
                                                bounding_box: BoundingBox::new(
                                                    text_x,
                                                    text_y,
                                                    text_width,
                                                    font_height,
                                                ),
                                                command_type: RenderCommandType::Text,
                                                render_data: InternalRenderData::Text {
                                                    text: disp_text,
                                                    text_color,
                                                    font_size: ti_config.font_size,
                                                    letter_spacing: 0,
                                                    line_height: 0,
                                                    font_asset: ti_config.font_asset,
                                                },
                                                user_data: 0,
                                                id: hash_number(1002, elem_id).id,
                                                z_index: root.z_index,
                                                visual_rotation: None,
                                                shape_rotation: None,
                                                effects: Vec::new(),
                                            });
                                        }

                                        // Cursor
                                        if is_focused && state.cursor_visible() {
                                            let cursor_x_pos = char_x_positions
                                                .get(render_cursor_pos)
                                                .copied()
                                                .unwrap_or(0.0);
                                            let cursor_y = current_bbox.y + (current_bbox.height - font_height) / 2.0;
                                            self.add_render_command(InternalRenderCommand {
                                                bounding_box: BoundingBox::new(
                                                    text_x + cursor_x_pos,
                                                    cursor_y,
                                                    2.0,
                                                    font_height,
                                                ),
                                                command_type: RenderCommandType::Rectangle,
                                                render_data: InternalRenderData::Rectangle {
                                                    background_color: ti_config.cursor_color,
                                                    corner_radius: CornerRadius::default(),
                                                },
                                                user_data: 0,
                                                id: hash_number(1003, elem_id).id,
                                                z_index: root.z_index,
                                                visual_rotation: None,
                                                shape_rotation: None,
                                                effects: Vec::new(),
                                            });
                                        }
                                    }

                                    // End clipping
                                    self.add_render_command(InternalRenderCommand {
                                        bounding_box: current_bbox,
                                        command_type: RenderCommandType::ScissorEnd,
                                        render_data: InternalRenderData::None,
                                        user_data: 0,
                                        id: hash_number(1004, elem_id).id,
                                        z_index: root.z_index,
                                        visual_rotation: None,
                                        shape_rotation: None,
                                        effects: Vec::new(),
                                    });
                                }
                                // Background already emitted above; skip the default rectangle
                                emit_rectangle = false;
                            }
                        }
                    }

                    if emit_rectangle {
                        self.add_render_command(InternalRenderCommand {
                            bounding_box: shape_draw_bbox,
                            command_type: RenderCommandType::Rectangle,
                            render_data: InternalRenderData::Rectangle {
                                background_color: shared.background_color,
                                corner_radius: shared.corner_radius,
                            },
                            user_data: shared.user_data,
                            id: elem_id,
                            z_index: root.z_index,
                            visual_rotation: None,
                            shape_rotation: elem_shape_rotation,
                            effects: elem_effects.clone(),
                        });
                    }

                    // Setup child alignment
                    let is_text =
                        self.element_has_config(current_elem_idx, ElementConfigType::Text);
                    if !is_text {
                        let children_start =
                            self.layout_elements[current_elem_idx].children_start;
                        let children_length =
                            self.layout_elements[current_elem_idx].children_length as usize;

                        if layout_config.layout_direction == LayoutDirection::LeftToRight {
                            let mut content_width: f32 = 0.0;
                            for ci in 0..children_length {
                                let child_idx =
                                    self.layout_element_children[children_start + ci] as usize;
                                content_width +=
                                    self.layout_elements[child_idx].dimensions.width;
                            }
                            content_width += children_length.saturating_sub(1) as f32
                                * layout_config.child_gap as f32;
                            let mut extra_space = self.layout_elements[current_elem_idx]
                                .dimensions
                                .width
                                - (layout_config.padding.left + layout_config.padding.right) as f32
                                - content_width;
                            match layout_config.child_alignment.x {
                                AlignX::Left => extra_space = 0.0,
                                AlignX::CenterX => extra_space /= 2.0,
                                _ => {} // Right - keep full extra_space
                            }
                            dfs_buffer[buf_idx].next_child_offset.x += extra_space;
                        } else {
                            let mut content_height: f32 = 0.0;
                            for ci in 0..children_length {
                                let child_idx =
                                    self.layout_element_children[children_start + ci] as usize;
                                content_height +=
                                    self.layout_elements[child_idx].dimensions.height;
                            }
                            content_height += children_length.saturating_sub(1) as f32
                                * layout_config.child_gap as f32;
                            let mut extra_space = self.layout_elements[current_elem_idx]
                                .dimensions
                                .height
                                - (layout_config.padding.top + layout_config.padding.bottom) as f32
                                - content_height;
                            match layout_config.child_alignment.y {
                                AlignY::Top => extra_space = 0.0,
                                AlignY::CenterY => extra_space /= 2.0,
                                _ => {}
                            }
                            dfs_buffer[buf_idx].next_child_offset.y += extra_space;
                        }

                        // Update scroll container content size
                        if let Some(si) = _scroll_container_data_idx {
                            let child_gap_total = children_length.saturating_sub(1) as f32
                                * layout_config.child_gap as f32;
                            let content_w: f32 = (0..children_length)
                                .map(|ci| {
                                    let idx = self.layout_element_children[children_start + ci]
                                        as usize;
                                    self.layout_elements[idx].dimensions.width
                                })
                                .sum::<f32>()
                                + (layout_config.padding.left + layout_config.padding.right) as f32
                                + if layout_config.layout_direction == LayoutDirection::LeftToRight {
                                    child_gap_total
                                } else {
                                    0.0
                                };
                            let content_h: f32 = (0..children_length)
                                .map(|ci| {
                                    let idx = self.layout_element_children[children_start + ci]
                                        as usize;
                                    self.layout_elements[idx].dimensions.height
                                })
                                .sum::<f32>()
                                + (layout_config.padding.top + layout_config.padding.bottom) as f32
                                + if layout_config.layout_direction == LayoutDirection::TopToBottom {
                                    child_gap_total
                                } else {
                                    0.0
                                };
                            self.scroll_container_datas[si].content_size =
                                Dimensions::new(content_w, content_h);
                        }
                    }
                } else {
                    // Returning upward in DFS

                    let mut close_clip = false;

                    if self.element_has_config(current_elem_idx, ElementConfigType::Clip) {
                        close_clip = true;
                        if let Some(clip_cfg_idx) = self
                            .find_element_config_index(current_elem_idx, ElementConfigType::Clip)
                        {
                            let clip_config = self.clip_element_configs[clip_cfg_idx];
                            for si in 0..self.scroll_container_datas.len() {
                                if self.scroll_container_datas[si].layout_element_index
                                    == current_elem_idx as i32
                                {
                                    scroll_offset = clip_config.child_offset;
                                    break;
                                }
                            }
                        }
                    }

                    // Generate border render commands
                    if self.element_has_config(current_elem_idx, ElementConfigType::Border) {
                        let border_elem_id = self.layout_elements[current_elem_idx].id;
                        if let Some(border_bbox) = self.layout_element_map.get(&border_elem_id).map(|item| item.bounding_box) {
                            let bbox = border_bbox;
                            if !self.element_is_offscreen(&bbox) {
                                let shared = self
                                    .find_element_config_index(
                                        current_elem_idx,
                                        ElementConfigType::Shared,
                                    )
                                    .map(|idx| self.shared_element_configs[idx])
                                    .unwrap_or_default();
                                let border_cfg_idx = self
                                    .find_element_config_index(
                                        current_elem_idx,
                                        ElementConfigType::Border,
                                    )
                                    .unwrap();
                                let border_config = self.border_element_configs[border_cfg_idx];

                                let children_count =
                                    self.layout_elements[current_elem_idx].children_length;
                                self.add_render_command(InternalRenderCommand {
                                    bounding_box: bbox,
                                    command_type: RenderCommandType::Border,
                                    render_data: InternalRenderData::Border {
                                        color: border_config.color,
                                        corner_radius: shared.corner_radius,
                                        width: border_config.width,
                                    },
                                    user_data: shared.user_data,
                                    id: hash_number(
                                        self.layout_elements[current_elem_idx].id,
                                        children_count as u32,
                                    )
                                    .id,
                                    z_index: root.z_index,
                                    visual_rotation: None,
                                    shape_rotation: None,
                                    effects: Vec::new(),
                                });

                                // between-children borders
                                if border_config.width.between_children > 0
                                    && border_config.color.a > 0.0
                                {
                                    let half_gap = layout_config.child_gap as f32 / 2.0;
                                    let children_start =
                                        self.layout_elements[current_elem_idx].children_start;
                                    let children_length = self.layout_elements[current_elem_idx]
                                        .children_length
                                        as usize;

                                    if layout_config.layout_direction
                                        == LayoutDirection::LeftToRight
                                    {
                                        let mut border_offset_x =
                                            layout_config.padding.left as f32 - half_gap;
                                        for ci in 0..children_length {
                                            let child_idx = self.layout_element_children
                                                [children_start + ci]
                                                as usize;
                                            if ci > 0 {
                                                self.add_render_command(InternalRenderCommand {
                                                    bounding_box: BoundingBox::new(
                                                        bbox.x + border_offset_x + scroll_offset.x,
                                                        bbox.y + scroll_offset.y,
                                                        border_config.width.between_children as f32,
                                                        self.layout_elements[current_elem_idx]
                                                            .dimensions
                                                            .height,
                                                    ),
                                                    command_type: RenderCommandType::Rectangle,
                                                    render_data: InternalRenderData::Rectangle {
                                                        background_color: border_config.color,
                                                        corner_radius: CornerRadius::default(),
                                                    },
                                                    user_data: shared.user_data,
                                                    id: hash_number(
                                                        self.layout_elements[current_elem_idx].id,
                                                        children_count as u32 + 1 + ci as u32,
                                                    )
                                                    .id,
                                                    z_index: root.z_index,
                                                    visual_rotation: None,
                                                    shape_rotation: None,
                                                    effects: Vec::new(),
                                                });
                                            }
                                            border_offset_x +=
                                                self.layout_elements[child_idx].dimensions.width
                                                    + layout_config.child_gap as f32;
                                        }
                                    } else {
                                        let mut border_offset_y =
                                            layout_config.padding.top as f32 - half_gap;
                                        for ci in 0..children_length {
                                            let child_idx = self.layout_element_children
                                                [children_start + ci]
                                                as usize;
                                            if ci > 0 {
                                                self.add_render_command(InternalRenderCommand {
                                                    bounding_box: BoundingBox::new(
                                                        bbox.x + scroll_offset.x,
                                                        bbox.y + border_offset_y + scroll_offset.y,
                                                        self.layout_elements[current_elem_idx]
                                                            .dimensions
                                                            .width,
                                                        border_config.width.between_children as f32,
                                                    ),
                                                    command_type: RenderCommandType::Rectangle,
                                                    render_data: InternalRenderData::Rectangle {
                                                        background_color: border_config.color,
                                                        corner_radius: CornerRadius::default(),
                                                    },
                                                    user_data: shared.user_data,
                                                    id: hash_number(
                                                        self.layout_elements[current_elem_idx].id,
                                                        children_count as u32 + 1 + ci as u32,
                                                    )
                                                    .id,
                                                    z_index: root.z_index,
                                                    visual_rotation: None,
                                                    shape_rotation: None,
                                                    effects: Vec::new(),
                                                });
                                            }
                                            border_offset_y +=
                                                self.layout_elements[child_idx].dimensions.height
                                                    + layout_config.child_gap as f32;
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if close_clip {
                        let root_elem = &self.layout_elements[root_elem_idx];
                        self.add_render_command(InternalRenderCommand {
                            command_type: RenderCommandType::ScissorEnd,
                            id: hash_number(
                                self.layout_elements[current_elem_idx].id,
                                root_elem.children_length as u32 + 11,
                            )
                            .id,
                            ..Default::default()
                        });
                    }

                    // Emit GroupEnd commands AFTER border and scissor (innermost first, outermost last)
                    let elem_shaders = self.element_shaders.get(current_elem_idx).cloned().unwrap_or_default();
                    let elem_visual_rotation = self.element_visual_rotations.get(current_elem_idx).cloned().flatten()
                        .filter(|vr| !vr.is_noop());

                    // GroupEnd for each shader
                    for _shader in elem_shaders.iter() {
                        self.add_render_command(InternalRenderCommand {
                            command_type: RenderCommandType::GroupEnd,
                            id: self.layout_elements[current_elem_idx].id,
                            z_index: root.z_index,
                            ..Default::default()
                        });
                    }
                    // If no shaders but visual rotation was present, emit its GroupEnd
                    if elem_shaders.is_empty() && elem_visual_rotation.is_some() {
                        self.add_render_command(InternalRenderCommand {
                            command_type: RenderCommandType::GroupEnd,
                            id: self.layout_elements[current_elem_idx].id,
                            z_index: root.z_index,
                            ..Default::default()
                        });
                    }

                    dfs_buffer.pop();
                    visited.pop();
                    continue;
                }

                // Add children to DFS buffer (in reverse for correct traversal order)
                let is_text =
                    self.element_has_config(current_elem_idx, ElementConfigType::Text);
                if !is_text {
                    let children_start = self.layout_elements[current_elem_idx].children_start;
                    let children_length =
                        self.layout_elements[current_elem_idx].children_length as usize;

                    // Pre-grow dfs_buffer and visited
                    let new_len = dfs_buffer.len() + children_length;
                    dfs_buffer.resize(new_len, LayoutElementTreeNode::default());
                    visited.resize(new_len, false);

                    for ci in 0..children_length {
                        let child_idx =
                            self.layout_element_children[children_start + ci] as usize;
                        let child_layout_idx =
                            self.layout_elements[child_idx].layout_config_index;

                        // Alignment along non-layout axis
                        let mut child_offset = dfs_buffer[buf_idx].next_child_offset;
                        if layout_config.layout_direction == LayoutDirection::LeftToRight {
                            child_offset.y = layout_config.padding.top as f32;
                            let whitespace = self.layout_elements[current_elem_idx].dimensions.height
                                - (layout_config.padding.top + layout_config.padding.bottom) as f32
                                - self.layout_elements[child_idx].dimensions.height;
                            match layout_config.child_alignment.y {
                                AlignY::Top => {}
                                AlignY::CenterY => {
                                    child_offset.y += whitespace / 2.0;
                                }
                                AlignY::Bottom => {
                                    child_offset.y += whitespace;
                                }
                            }
                        } else {
                            child_offset.x = layout_config.padding.left as f32;
                            let whitespace = self.layout_elements[current_elem_idx].dimensions.width
                                - (layout_config.padding.left + layout_config.padding.right) as f32
                                - self.layout_elements[child_idx].dimensions.width;
                            match layout_config.child_alignment.x {
                                AlignX::Left => {}
                                AlignX::CenterX => {
                                    child_offset.x += whitespace / 2.0;
                                }
                                AlignX::Right => {
                                    child_offset.x += whitespace;
                                }
                            }
                        }

                        let child_position = Vector2::new(
                            dfs_buffer[buf_idx].position.x + child_offset.x + scroll_offset.x,
                            dfs_buffer[buf_idx].position.y + child_offset.y + scroll_offset.y,
                        );

                        let new_node_index = new_len - 1 - ci;
                        let child_padding_left =
                            self.layout_configs[child_layout_idx].padding.left as f32;
                        let child_padding_top =
                            self.layout_configs[child_layout_idx].padding.top as f32;
                        dfs_buffer[new_node_index] = LayoutElementTreeNode {
                            layout_element_index: child_idx as i32,
                            position: child_position,
                            next_child_offset: Vector2::new(child_padding_left, child_padding_top),
                        };
                        visited[new_node_index] = false;

                        // Update parent offset
                        if layout_config.layout_direction == LayoutDirection::LeftToRight {
                            dfs_buffer[buf_idx].next_child_offset.x +=
                                self.layout_elements[child_idx].dimensions.width
                                    + layout_config.child_gap as f32;
                        } else {
                            dfs_buffer[buf_idx].next_child_offset.y +=
                                self.layout_elements[child_idx].dimensions.height
                                    + layout_config.child_gap as f32;
                        }
                    }
                }
            }

            // End clip
            if root.clip_element_id != 0 {
                let root_elem = &self.layout_elements[root_elem_idx];
                self.add_render_command(InternalRenderCommand {
                    command_type: RenderCommandType::ScissorEnd,
                    id: hash_number(root_elem.id, root_elem.children_length as u32 + 11).id,
                    ..Default::default()
                });
            }
        }

        // Focus ring: render a red border around the focused element (keyboard focus only)
        if self.focused_element_id != 0 && self.focus_from_keyboard {
            if let Some(item) = self.layout_element_map.get(&self.focused_element_id) {
                let bbox = item.bounding_box;
                if !self.element_is_offscreen(&bbox) {
                    let elem_idx = item.layout_element_index as usize;
                    let corner_radius = self
                        .find_element_config_index(elem_idx, ElementConfigType::Shared)
                        .map(|idx| self.shared_element_configs[idx].corner_radius)
                        .unwrap_or_default();
                    let ring_width = 2u16;
                    let ring_color = Color::rgba(255.0, 60.0, 40.0, 255.0);
                    // Expand bounding box outward by ring width so the ring doesn't overlap content
                    let expanded_bbox = BoundingBox::new(
                        bbox.x - ring_width as f32,
                        bbox.y - ring_width as f32,
                        bbox.width + ring_width as f32 * 2.0,
                        bbox.height + ring_width as f32 * 2.0,
                    );
                    self.add_render_command(InternalRenderCommand {
                        bounding_box: expanded_bbox,
                        command_type: RenderCommandType::Border,
                        render_data: InternalRenderData::Border {
                            color: ring_color,
                            corner_radius: CornerRadius {
                                top_left: corner_radius.top_left + ring_width as f32,
                                top_right: corner_radius.top_right + ring_width as f32,
                                bottom_left: corner_radius.bottom_left + ring_width as f32,
                                bottom_right: corner_radius.bottom_right + ring_width as f32,
                            },
                            width: BorderWidth {
                                left: ring_width,
                                right: ring_width,
                                top: ring_width,
                                bottom: ring_width,
                                between_children: 0,
                            },
                        },
                        id: hash_number(self.focused_element_id, 0xF0C5).id,
                        z_index: 32764, // just below debug panel
                        ..Default::default()
                    });
                }
            }
        }
    }

    pub fn set_layout_dimensions(&mut self, dimensions: Dimensions) {
        self.layout_dimensions = dimensions;
    }

    pub fn set_pointer_state(&mut self, position: Vector2, is_down: bool) {
        if self.boolean_warnings.max_elements_exceeded {
            return;
        }
        self.pointer_info.position = position;
        self.pointer_over_ids.clear();

        // Check which elements are under the pointer
        for root_index in (0..self.layout_element_tree_roots.len()).rev() {
            let root = self.layout_element_tree_roots[root_index];
            let mut dfs: Vec<i32> = vec![root.layout_element_index];
            let mut vis: Vec<bool> = vec![false];
            let mut found = false;

            while !dfs.is_empty() {
                let idx = dfs.len() - 1;
                if vis[idx] {
                    dfs.pop();
                    vis.pop();
                    continue;
                }
                vis[idx] = true;
                let current_idx = dfs[idx] as usize;
                let elem_id = self.layout_elements[current_idx].id;

                // Copy data from map to avoid borrow issues with mutable access later
                let map_data = self.layout_element_map.get(&elem_id).map(|item| {
                    (item.bounding_box, item.element_id.clone(), item.on_hover_fn.is_some())
                });
                if let Some((raw_box, elem_id_copy, has_hover)) = map_data {
                    let mut elem_box = raw_box;
                    elem_box.x -= root.pointer_offset.x;
                    elem_box.y -= root.pointer_offset.y;

                    let clip_id =
                        self.layout_element_clip_element_ids[current_idx] as u32;
                    let clip_ok = clip_id == 0
                        || self
                            .layout_element_map
                            .get(&clip_id)
                            .map(|ci| {
                                point_is_inside_rect(
                                    position,
                                    ci.bounding_box,
                                )
                            })
                            .unwrap_or(false);

                    if point_is_inside_rect(position, elem_box) && clip_ok {
                        // Call hover callbacks
                        if has_hover {
                            let pointer_data = self.pointer_info;
                            if let Some(item) = self.layout_element_map.get_mut(&elem_id) {
                                if let Some(ref mut callback) = item.on_hover_fn {
                                    callback(elem_id_copy.clone(), pointer_data);
                                }
                            }
                        }
                        self.pointer_over_ids.push(elem_id_copy);
                        found = true;
                    }

                    if self.element_has_config(current_idx, ElementConfigType::Text) {
                        dfs.pop();
                        vis.pop();
                        continue;
                    }
                    let children_start = self.layout_elements[current_idx].children_start;
                    let children_length =
                        self.layout_elements[current_idx].children_length as usize;
                    for ci in (0..children_length).rev() {
                        let child = self.layout_element_children[children_start + ci];
                        dfs.push(child);
                        vis.push(false);
                    }
                } else {
                    dfs.pop();
                    vis.pop();
                }
            }

            if found {
                let root_elem_idx = root.layout_element_index as usize;
                if self.element_has_config(root_elem_idx, ElementConfigType::Floating) {
                    if let Some(cfg_idx) = self
                        .find_element_config_index(root_elem_idx, ElementConfigType::Floating)
                    {
                        if self.floating_element_configs[cfg_idx].pointer_capture_mode
                            == PointerCaptureMode::Capture
                        {
                            break;
                        }
                    }
                }
            }
        }

        // Update pointer state
        if is_down {
            match self.pointer_info.state {
                PointerDataInteractionState::PressedThisFrame => {
                    self.pointer_info.state = PointerDataInteractionState::Pressed;
                }
                s if s != PointerDataInteractionState::Pressed => {
                    self.pointer_info.state = PointerDataInteractionState::PressedThisFrame;
                }
                _ => {}
            }
        } else {
            match self.pointer_info.state {
                PointerDataInteractionState::ReleasedThisFrame => {
                    self.pointer_info.state = PointerDataInteractionState::Released;
                }
                s if s != PointerDataInteractionState::Released => {
                    self.pointer_info.state = PointerDataInteractionState::ReleasedThisFrame;
                }
                _ => {}
            }
        }

        // Fire on_press / on_release callbacks and track pressed element
        match self.pointer_info.state {
            PointerDataInteractionState::PressedThisFrame => {
                // Check if clicked element is a text input
                let clicked_text_input = self.pointer_over_ids.last()
                    .and_then(|top| self.layout_element_map.get(&top.id))
                    .map(|item| item.is_text_input)
                    .unwrap_or(false);

                if clicked_text_input {
                    // Focus the text input (or keep focus if already focused)
                    self.focus_from_keyboard = false;
                    if let Some(top) = self.pointer_over_ids.last().cloned() {
                        if self.focused_element_id != top.id {
                            self.change_focus(top.id);
                        }
                        // Compute click x,y relative to the element's bounding box
                        if let Some(item) = self.layout_element_map.get(&top.id) {
                            let click_x = self.pointer_info.position.x - item.bounding_box.x;
                            let click_y = self.pointer_info.position.y - item.bounding_box.y;
                            // We can't check shift from here (no keyboard state);
                            // lib.rs will set shift via a dedicated method if needed.
                            self.pending_text_click = Some((top.id, click_x, click_y, false));
                        }
                        self.pressed_element_ids = self.pointer_over_ids.clone();
                    }
                } else {
                    // Check if any element in the pointer stack preserves focus
                    // (e.g. a toolbar button's child text element inherits the parent's preserve_focus)
                    let preserves = self.pointer_over_ids.iter().any(|eid| {
                        self.layout_element_map.get(&eid.id)
                            .map(|item| item.preserve_focus)
                            .unwrap_or(false)
                    });

                    // Clear keyboard focus when the user clicks, unless the element preserves focus
                    if !preserves && self.focused_element_id != 0 {
                        self.change_focus(0);
                    }

                    // Mark all hovered elements as pressed and fire on_press callbacks
                    self.pressed_element_ids = self.pointer_over_ids.clone();
                    for eid in self.pointer_over_ids.clone().iter() {
                        if let Some(item) = self.layout_element_map.get_mut(&eid.id) {
                            if let Some(ref mut callback) = item.on_press_fn {
                                callback(eid.clone(), self.pointer_info);
                            }
                        }
                    }
                }
            }
            PointerDataInteractionState::ReleasedThisFrame => {
                // Fire on_release for all elements that were in the pressed chain
                let pressed = std::mem::take(&mut self.pressed_element_ids);
                for eid in pressed.iter() {
                    if let Some(item) = self.layout_element_map.get_mut(&eid.id) {
                        if let Some(ref mut callback) = item.on_release_fn {
                            callback(eid.clone(), self.pointer_info);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// Physics constants for scroll momentum
    const SCROLL_DECEL: f32 = 5.0; // Exponential decay rate (reaches ~0.7% after 1s)
    const SCROLL_MIN_VELOCITY: f32 = 5.0; // px/s below which momentum stops
    const SCROLL_VELOCITY_SMOOTHING: f32 = 0.4; // EMA factor for velocity tracking

    pub fn update_scroll_containers(
        &mut self,
        enable_drag_scrolling: bool,
        scroll_delta: Vector2,
        delta_time: f32,
    ) {
        let pointer = self.pointer_info.position;
        let dt = delta_time.max(0.0001); // Guard against zero/negative dt

        // Remove containers that weren't open this frame, reset flag for next frame
        let mut i = 0;
        while i < self.scroll_container_datas.len() {
            if !self.scroll_container_datas[i].open_this_frame {
                self.scroll_container_datas.swap_remove(i);
                continue;
            }
            self.scroll_container_datas[i].open_this_frame = false;
            i += 1;
        }

        // --- Drag scrolling ---
        if enable_drag_scrolling {
            let pointer_state = self.pointer_info.state;

            match pointer_state {
                PointerDataInteractionState::PressedThisFrame => {
                    // Find the deepest scroll container under the pointer and start drag
                    let mut best: Option<usize> = None;
                    for si in 0..self.scroll_container_datas.len() {
                        let bb = self.scroll_container_datas[si].bounding_box;
                        if pointer.x >= bb.x
                            && pointer.x <= bb.x + bb.width
                            && pointer.y >= bb.y
                            && pointer.y <= bb.y + bb.height
                        {
                            best = Some(si);
                        }
                    }
                    if let Some(si) = best {
                        let scd = &mut self.scroll_container_datas[si];
                        scd.pointer_scroll_active = true;
                        scd.pointer_origin = pointer;
                        scd.scroll_origin = scd.scroll_position;
                        scd.scroll_momentum = Vector2::default();
                        scd.previous_delta = Vector2::default();
                    }
                }
                PointerDataInteractionState::Pressed => {
                    // Update drag: move scroll position to follow pointer
                    for si in 0..self.scroll_container_datas.len() {
                        let scd = &mut self.scroll_container_datas[si];
                        if !scd.pointer_scroll_active {
                            continue;
                        }

                        let drag_delta = Vector2::new(
                            pointer.x - scd.pointer_origin.x,
                            pointer.y - scd.pointer_origin.y,
                        );
                        scd.scroll_position = Vector2::new(
                            scd.scroll_origin.x + drag_delta.x,
                            scd.scroll_origin.y + drag_delta.y,
                        );

                        // Check if pointer actually moved this frame
                        let frame_delta = Vector2::new(
                            drag_delta.x - scd.previous_delta.x,
                            drag_delta.y - scd.previous_delta.y,
                        );
                        let moved = frame_delta.x.abs() > 0.5 || frame_delta.y.abs() > 0.5;

                        if moved {
                            // Pointer moved — update velocity EMA and reset freshness timer
                            let instant_velocity = Vector2::new(
                                frame_delta.x / dt,
                                frame_delta.y / dt,
                            );
                            let s = Self::SCROLL_VELOCITY_SMOOTHING;
                            scd.scroll_momentum = Vector2::new(
                                scd.scroll_momentum.x * (1.0 - s) + instant_velocity.x * s,
                                scd.scroll_momentum.y * (1.0 - s) + instant_velocity.y * s,
                            );
                        }
                        scd.previous_delta = drag_delta;
                    }
                }
                PointerDataInteractionState::ReleasedThisFrame
                | PointerDataInteractionState::Released => {
                    for si in 0..self.scroll_container_datas.len() {
                        let scd = &mut self.scroll_container_datas[si];
                        if !scd.pointer_scroll_active {
                            continue;
                        }
                        scd.pointer_scroll_active = false;
                    }
                }
            }
        }

        // --- Momentum scrolling (apply when not actively dragging) ---
        for si in 0..self.scroll_container_datas.len() {
            let scd = &mut self.scroll_container_datas[si];
            if scd.pointer_scroll_active {
                // Still dragging — skip momentum
            } else if scd.scroll_momentum.x.abs() > Self::SCROLL_MIN_VELOCITY
                || scd.scroll_momentum.y.abs() > Self::SCROLL_MIN_VELOCITY
            {
                // Apply momentum
                scd.scroll_position.x += scd.scroll_momentum.x * dt;
                scd.scroll_position.y += scd.scroll_momentum.y * dt;

                // Exponential decay (frame-rate independent)
                let decay = (-Self::SCROLL_DECEL * dt).exp();
                scd.scroll_momentum.x *= decay;
                scd.scroll_momentum.y *= decay;

                // Stop if below threshold
                if scd.scroll_momentum.x.abs() < Self::SCROLL_MIN_VELOCITY {
                    scd.scroll_momentum.x = 0.0;
                }
                if scd.scroll_momentum.y.abs() < Self::SCROLL_MIN_VELOCITY {
                    scd.scroll_momentum.y = 0.0;
                }
            }
        }

        // --- Mouse wheel / external scroll delta ---
        if scroll_delta.x != 0.0 || scroll_delta.y != 0.0 {
            // Find the deepest (last in list) scroll container the pointer is inside
            let mut best: Option<usize> = None;
            for si in 0..self.scroll_container_datas.len() {
                let bb = self.scroll_container_datas[si].bounding_box;
                if pointer.x >= bb.x
                    && pointer.x <= bb.x + bb.width
                    && pointer.y >= bb.y
                    && pointer.y <= bb.y + bb.height
                {
                    best = Some(si);
                }
            }
            if let Some(si) = best {
                let scd = &mut self.scroll_container_datas[si];
                scd.scroll_position.y += scroll_delta.y;
                scd.scroll_position.x += scroll_delta.x;
                // Kill any active momentum when mouse wheel is used
                scd.scroll_momentum = Vector2::default();
            }
        }

        // --- Clamp all scroll positions ---
        for si in 0..self.scroll_container_datas.len() {
            let scd = &mut self.scroll_container_datas[si];
            let max_scroll_y =
                -(scd.content_size.height - scd.bounding_box.height).max(0.0);
            let max_scroll_x =
                -(scd.content_size.width - scd.bounding_box.width).max(0.0);
            scd.scroll_position.y = scd.scroll_position.y.clamp(max_scroll_y, 0.0);
            scd.scroll_position.x = scd.scroll_position.x.clamp(max_scroll_x, 0.0);

            // Also kill momentum at bounds
            if scd.scroll_position.y >= 0.0 || scd.scroll_position.y <= max_scroll_y {
                scd.scroll_momentum.y = 0.0;
            }
            if scd.scroll_position.x >= 0.0 || scd.scroll_position.x <= max_scroll_x {
                scd.scroll_momentum.x = 0.0;
            }
        }
    }

    pub fn hovered(&self) -> bool {
        let open_idx = self.get_open_layout_element();
        let elem_id = self.layout_elements[open_idx].id;
        self.pointer_over_ids.iter().any(|eid| eid.id == elem_id)
    }

    pub fn on_hover(&mut self, callback: Box<dyn FnMut(Id, PointerData)>) {
        let open_idx = self.get_open_layout_element();
        let elem_id = self.layout_elements[open_idx].id;
        if let Some(item) = self.layout_element_map.get_mut(&elem_id) {
            item.on_hover_fn = Some(callback);
        }
    }

    pub fn pressed(&self) -> bool {
        let open_idx = self.get_open_layout_element();
        let elem_id = self.layout_elements[open_idx].id;
        self.pressed_element_ids.iter().any(|eid| eid.id == elem_id)
    }

    pub fn set_press_callbacks(
        &mut self,
        on_press: Option<Box<dyn FnMut(Id, PointerData)>>,
        on_release: Option<Box<dyn FnMut(Id, PointerData)>>,
    ) {
        let open_idx = self.get_open_layout_element();
        let elem_id = self.layout_elements[open_idx].id;
        if let Some(item) = self.layout_element_map.get_mut(&elem_id) {
            item.on_press_fn = on_press;
            item.on_release_fn = on_release;
        }
    }

    /// Returns true if the currently open element has focus.
    pub fn focused(&self) -> bool {
        let open_idx = self.get_open_layout_element();
        let elem_id = self.layout_elements[open_idx].id;
        self.focused_element_id == elem_id && elem_id != 0
    }

    /// Returns the currently focused element's ID, or None.
    pub fn focused_element(&self) -> Option<Id> {
        if self.focused_element_id != 0 {
            self.layout_element_map
                .get(&self.focused_element_id)
                .map(|item| item.element_id.clone())
        } else {
            None
        }
    }

    /// Sets focus to the element with the given ID, firing on_unfocus/on_focus callbacks.
    pub fn set_focus(&mut self, element_id: u32) {
        self.change_focus(element_id);
    }

    /// Clears focus (no element is focused).
    pub fn clear_focus(&mut self) {
        self.change_focus(0);
    }

    /// Internal: changes focus, firing on_unfocus on old and on_focus on new.
    pub(crate) fn change_focus(&mut self, new_id: u32) {
        let old_id = self.focused_element_id;
        if old_id == new_id {
            return;
        }
        self.focused_element_id = new_id;
        if new_id == 0 {
            self.focus_from_keyboard = false;
        }

        // Fire on_unfocus on old element
        if old_id != 0 {
            if let Some(item) = self.layout_element_map.get_mut(&old_id) {
                let id_copy = item.element_id.clone();
                if let Some(ref mut callback) = item.on_unfocus_fn {
                    callback(id_copy);
                }
            }
        }

        // Fire on_focus on new element
        if new_id != 0 {
            if let Some(item) = self.layout_element_map.get_mut(&new_id) {
                let id_copy = item.element_id.clone();
                if let Some(ref mut callback) = item.on_focus_fn {
                    callback(id_copy);
                }
            }
        }
    }

    /// Fire the on_press callback for the element with the given u32 ID.
    /// Used by screen reader action handling.
    #[allow(dead_code)]
    pub(crate) fn fire_press(&mut self, element_id: u32) {
        if let Some(item) = self.layout_element_map.get_mut(&element_id) {
            let id_copy = item.element_id.clone();
            if let Some(ref mut callback) = item.on_press_fn {
                callback(id_copy, PointerData::default());
            }
        }
    }

    pub fn set_focus_callbacks(
        &mut self,
        on_focus: Option<Box<dyn FnMut(Id)>>,
        on_unfocus: Option<Box<dyn FnMut(Id)>>,
    ) {
        let open_idx = self.get_open_layout_element();
        let elem_id = self.layout_elements[open_idx].id;
        if let Some(item) = self.layout_element_map.get_mut(&elem_id) {
            item.on_focus_fn = on_focus;
            item.on_unfocus_fn = on_unfocus;
        }
    }

    /// Sets text input callbacks for the currently open element.
    pub fn set_text_input_callbacks(
        &mut self,
        on_changed: Option<Box<dyn FnMut(&str)>>,
        on_submit: Option<Box<dyn FnMut(&str)>>,
    ) {
        let open_idx = self.get_open_layout_element();
        let elem_id = self.layout_elements[open_idx].id;
        if let Some(item) = self.layout_element_map.get_mut(&elem_id) {
            item.on_text_changed_fn = on_changed;
            item.on_text_submit_fn = on_submit;
        }
    }

    /// Returns true if the currently focused element is a text input.
    pub fn is_text_input_focused(&self) -> bool {
        if self.focused_element_id == 0 {
            return false;
        }
        self.text_edit_states.contains_key(&self.focused_element_id)
    }

    /// Returns true if the currently focused text input is multiline.
    pub fn is_focused_text_input_multiline(&self) -> bool {
        if self.focused_element_id == 0 {
            return false;
        }
        self.text_input_element_ids.iter()
            .position(|&id| id == self.focused_element_id)
            .and_then(|idx| self.text_input_configs.get(idx))
            .map_or(false, |cfg| cfg.is_multiline)
    }

    /// Returns the text value for a text input element, or empty string if not found.
    pub fn get_text_value(&self, element_id: u32) -> &str {
        self.text_edit_states
            .get(&element_id)
            .map(|state| state.text.as_str())
            .unwrap_or("")
    }

    /// Sets the text value for a text input element.
    pub fn set_text_value(&mut self, element_id: u32, value: &str) {
        if let Some(state) = self.text_edit_states.get_mut(&element_id) {
            state.text = value.to_string();
            #[cfg(feature = "text-styling")]
            let max_pos = crate::text_input::styling_cursor::cursor_len(&state.text);
            #[cfg(not(feature = "text-styling"))]
            let max_pos = state.text.chars().count();
            if state.cursor_pos > max_pos {
                state.cursor_pos = max_pos;
            }
            state.selection_anchor = None;
            state.reset_blink();
        }
    }

    /// Returns the cursor position for a text input element, or 0 if not found.
    /// When text-styling is enabled, this returns the visual position.
    pub fn get_cursor_pos(&self, element_id: u32) -> usize {
        self.text_edit_states
            .get(&element_id)
            .map(|state| state.cursor_pos)
            .unwrap_or(0)
    }

    /// Sets the cursor position for a text input element.
    /// When text-styling is enabled, `pos` is in visual space.
    /// Clamps to the text length and clears any selection.
    pub fn set_cursor_pos(&mut self, element_id: u32, pos: usize) {
        if let Some(state) = self.text_edit_states.get_mut(&element_id) {
            #[cfg(feature = "text-styling")]
            let max_pos = crate::text_input::styling_cursor::cursor_len(&state.text);
            #[cfg(not(feature = "text-styling"))]
            let max_pos = state.text.chars().count();
            state.cursor_pos = pos.min(max_pos);
            state.selection_anchor = None;
            state.reset_blink();
        }
    }

    /// Returns the selection range (start, end) for a text input element, or None.
    /// When text-styling is enabled, these are visual positions.
    pub fn get_selection_range(&self, element_id: u32) -> Option<(usize, usize)> {
        self.text_edit_states
            .get(&element_id)
            .and_then(|state| state.selection_range())
    }

    /// Sets the selection range for a text input element.
    /// `anchor` is where selection started, `cursor` is where it ends.
    /// When text-styling is enabled, these are visual positions.
    pub fn set_selection(&mut self, element_id: u32, anchor: usize, cursor: usize) {
        if let Some(state) = self.text_edit_states.get_mut(&element_id) {
            #[cfg(feature = "text-styling")]
            let max_pos = crate::text_input::styling_cursor::cursor_len(&state.text);
            #[cfg(not(feature = "text-styling"))]
            let max_pos = state.text.chars().count();
            state.selection_anchor = Some(anchor.min(max_pos));
            state.cursor_pos = cursor.min(max_pos);
            state.reset_blink();
        }
    }

    /// Returns true if the given element ID is currently pressed.
    pub fn is_element_pressed(&self, element_id: u32) -> bool {
        self.pressed_element_ids.iter().any(|eid| eid.id == element_id)
    }

    /// Process a character input event for the focused text input.
    /// Returns true if the character was consumed by a text input.
    pub fn process_text_input_char(&mut self, ch: char) -> bool {
        if !self.is_text_input_focused() {
            return false;
        }
        let elem_id = self.focused_element_id;

        // Get max_length from current config (if available this frame)
        let max_length = self.text_input_element_ids.iter()
            .position(|&id| id == elem_id)
            .and_then(|idx| self.text_input_configs.get(idx))
            .and_then(|cfg| cfg.max_length);

        if let Some(state) = self.text_edit_states.get_mut(&elem_id) {
            let old_text = state.text.clone();
            state.push_undo(crate::text_input::UndoActionKind::InsertChar);
            #[cfg(feature = "text-styling")]
            {
                state.insert_char_styled(ch, max_length);
            }
            #[cfg(not(feature = "text-styling"))]
            {
                state.insert_text(&ch.to_string(), max_length);
            }
            if state.text != old_text {
                let new_text = state.text.clone();
                // Fire on_changed callback
                if let Some(item) = self.layout_element_map.get_mut(&elem_id) {
                    if let Some(ref mut callback) = item.on_text_changed_fn {
                        callback(&new_text);
                    }
                }
            }
            true
        } else {
            false
        }
    }

    /// Process a key event for the focused text input.
    /// `action` specifies which editing action to perform.
    /// Returns true if the key was consumed.
    pub fn process_text_input_action(&mut self, action: TextInputAction) -> bool {
        if !self.is_text_input_focused() {
            return false;
        }
        let elem_id = self.focused_element_id;

        // Get config for the focused element
        let config_idx = self.text_input_element_ids.iter()
            .position(|&id| id == elem_id);
        let (max_length, is_multiline, font_asset, font_size) = config_idx
            .and_then(|idx| self.text_input_configs.get(idx))
            .map(|cfg| (cfg.max_length, cfg.is_multiline, cfg.font_asset, cfg.font_size))
            .unwrap_or((None, false, None, 16));

        // For multiline visual navigation, compute visual lines
        let visual_lines_opt = if is_multiline {
            let visible_width = self.layout_element_map
                .get(&elem_id)
                .map(|item| item.bounding_box.width)
                .unwrap_or(0.0);
            if visible_width > 0.0 {
                if let Some(state) = self.text_edit_states.get(&elem_id) {
                    if let Some(ref measure_fn) = self.measure_text_fn {
                        Some(crate::text_input::wrap_lines(
                            &state.text,
                            visible_width,
                            font_asset,
                            font_size,
                            measure_fn.as_ref(),
                        ))
                    } else { None }
                } else { None }
            } else { None }
        } else { None };

        if let Some(state) = self.text_edit_states.get_mut(&elem_id) {
            let old_text = state.text.clone();

            // Push undo before text-modifying actions
            match &action {
                TextInputAction::Backspace => state.push_undo(crate::text_input::UndoActionKind::Backspace),
                TextInputAction::Delete => state.push_undo(crate::text_input::UndoActionKind::Delete),
                TextInputAction::BackspaceWord => state.push_undo(crate::text_input::UndoActionKind::DeleteWord),
                TextInputAction::DeleteWord => state.push_undo(crate::text_input::UndoActionKind::DeleteWord),
                TextInputAction::Cut => state.push_undo(crate::text_input::UndoActionKind::Cut),
                TextInputAction::Paste { .. } => state.push_undo(crate::text_input::UndoActionKind::Paste),
                TextInputAction::Submit if is_multiline => state.push_undo(crate::text_input::UndoActionKind::InsertChar),
                _ => {}
            }

            match action {
                TextInputAction::MoveLeft { shift } => {
                    #[cfg(feature = "text-styling")]
                    { state.move_left_styled(shift); }
                    #[cfg(not(feature = "text-styling"))]
                    { state.move_left(shift); }
                }
                TextInputAction::MoveRight { shift } => {
                    #[cfg(feature = "text-styling")]
                    { state.move_right_styled(shift); }
                    #[cfg(not(feature = "text-styling"))]
                    { state.move_right(shift); }
                }
                TextInputAction::MoveWordLeft { shift } => {
                    #[cfg(feature = "text-styling")]
                    { state.move_word_left_styled(shift); }
                    #[cfg(not(feature = "text-styling"))]
                    { state.move_word_left(shift); }
                }
                TextInputAction::MoveWordRight { shift } => {
                    #[cfg(feature = "text-styling")]
                    { state.move_word_right_styled(shift); }
                    #[cfg(not(feature = "text-styling"))]
                    { state.move_word_right(shift); }
                }
                TextInputAction::MoveHome { shift } => {
                    // Multiline uses visual line navigation (raw positions)
                    #[cfg(not(feature = "text-styling"))]
                    {
                        if let Some(ref vl) = visual_lines_opt {
                            let new_pos = crate::text_input::visual_line_home(vl, state.cursor_pos);
                            if shift && state.selection_anchor.is_none() {
                                state.selection_anchor = Some(state.cursor_pos);
                            }
                            state.cursor_pos = new_pos;
                            if !shift { state.selection_anchor = None; }
                            else if state.selection_anchor == Some(state.cursor_pos) { state.selection_anchor = None; }
                            state.reset_blink();
                        } else {
                            state.move_home(shift);
                        }
                    }
                    #[cfg(feature = "text-styling")]
                    {
                        state.move_home_styled(shift);
                    }
                }
                TextInputAction::MoveEnd { shift } => {
                    #[cfg(not(feature = "text-styling"))]
                    {
                        if let Some(ref vl) = visual_lines_opt {
                            let new_pos = crate::text_input::visual_line_end(vl, state.cursor_pos);
                            if shift && state.selection_anchor.is_none() {
                                state.selection_anchor = Some(state.cursor_pos);
                            }
                            state.cursor_pos = new_pos;
                            if !shift { state.selection_anchor = None; }
                            else if state.selection_anchor == Some(state.cursor_pos) { state.selection_anchor = None; }
                            state.reset_blink();
                        } else {
                            state.move_end(shift);
                        }
                    }
                    #[cfg(feature = "text-styling")]
                    {
                        state.move_end_styled(shift);
                    }
                }
                TextInputAction::MoveUp { shift } => {
                    #[cfg(not(feature = "text-styling"))]
                    {
                        if let Some(ref vl) = visual_lines_opt {
                            let new_pos = crate::text_input::visual_move_up(vl, state.cursor_pos);
                            if shift && state.selection_anchor.is_none() {
                                state.selection_anchor = Some(state.cursor_pos);
                            }
                            state.cursor_pos = new_pos;
                            if !shift { state.selection_anchor = None; }
                            else if state.selection_anchor == Some(state.cursor_pos) { state.selection_anchor = None; }
                            state.reset_blink();
                        } else {
                            state.move_up(shift);
                        }
                    }
                    #[cfg(feature = "text-styling")]
                    {
                        state.move_up_styled(shift, visual_lines_opt.as_deref());
                    }
                }
                TextInputAction::MoveDown { shift } => {
                    #[cfg(not(feature = "text-styling"))]
                    {
                        if let Some(ref vl) = visual_lines_opt {
                            let text_len = state.text.chars().count();
                            let new_pos = crate::text_input::visual_move_down(vl, state.cursor_pos, text_len);
                            if shift && state.selection_anchor.is_none() {
                                state.selection_anchor = Some(state.cursor_pos);
                            }
                            state.cursor_pos = new_pos;
                            if !shift { state.selection_anchor = None; }
                            else if state.selection_anchor == Some(state.cursor_pos) { state.selection_anchor = None; }
                            state.reset_blink();
                        } else {
                            state.move_down(shift);
                        }
                    }
                    #[cfg(feature = "text-styling")]
                    {
                        state.move_down_styled(shift, visual_lines_opt.as_deref());
                    }
                }
                TextInputAction::Backspace => {
                    #[cfg(feature = "text-styling")]
                    { state.backspace_styled(); }
                    #[cfg(not(feature = "text-styling"))]
                    { state.backspace(); }
                }
                TextInputAction::Delete => {
                    #[cfg(feature = "text-styling")]
                    { state.delete_forward_styled(); }
                    #[cfg(not(feature = "text-styling"))]
                    { state.delete_forward(); }
                }
                TextInputAction::BackspaceWord => {
                    #[cfg(feature = "text-styling")]
                    { state.backspace_word_styled(); }
                    #[cfg(not(feature = "text-styling"))]
                    { state.backspace_word(); }
                }
                TextInputAction::DeleteWord => {
                    #[cfg(feature = "text-styling")]
                    { state.delete_word_forward_styled(); }
                    #[cfg(not(feature = "text-styling"))]
                    { state.delete_word_forward(); }
                }
                TextInputAction::SelectAll => {
                    #[cfg(feature = "text-styling")]
                    { state.select_all_styled(); }
                    #[cfg(not(feature = "text-styling"))]
                    { state.select_all(); }
                }
                TextInputAction::Copy => {
                    // Copying doesn't modify state; handled by lib.rs
                }
                TextInputAction::Cut => {
                    #[cfg(feature = "text-styling")]
                    { state.delete_selection_styled(); }
                    #[cfg(not(feature = "text-styling"))]
                    { state.delete_selection(); }
                }
                TextInputAction::Paste { text } => {
                    #[cfg(feature = "text-styling")]
                    {
                        let escaped = crate::text_input::styling_cursor::escape_str(&text);
                        state.insert_text_styled(&escaped, max_length);
                    }
                    #[cfg(not(feature = "text-styling"))]
                    {
                        state.insert_text(&text, max_length);
                    }
                }
                TextInputAction::Submit => {
                    if is_multiline {
                        #[cfg(feature = "text-styling")]
                        { state.insert_text_styled("\n", max_length); }
                        #[cfg(not(feature = "text-styling"))]
                        { state.insert_text("\n", max_length); }
                    } else {
                        let text = state.text.clone();
                        // Fire on_submit callback
                        if let Some(item) = self.layout_element_map.get_mut(&elem_id) {
                            if let Some(ref mut callback) = item.on_text_submit_fn {
                                callback(&text);
                            }
                        }
                        return true;
                    }
                }
                TextInputAction::Undo => {
                    state.undo();
                }
                TextInputAction::Redo => {
                    state.redo();
                }
            }
            if state.text != old_text {
                let new_text = state.text.clone();
                if let Some(item) = self.layout_element_map.get_mut(&elem_id) {
                    if let Some(ref mut callback) = item.on_text_changed_fn {
                        callback(&new_text);
                    }
                }
            }
            true
        } else {
            false
        }
    }

    /// Update blink timers for all text input states.
    pub fn update_text_input_blink_timers(&mut self) {
        let dt = self.frame_delta_time as f64;
        for state in self.text_edit_states.values_mut() {
            state.cursor_blink_timer += dt;
        }
    }

    /// Update scroll offsets for text inputs to ensure cursor visibility.
    pub fn update_text_input_scroll(&mut self) {
        let focused = self.focused_element_id;
        if focused == 0 {
            return;
        }
        // Get bounding box for the focused text input
        let (visible_width, visible_height) = self.layout_element_map
            .get(&focused)
            .map(|item| (item.bounding_box.width, item.bounding_box.height))
            .unwrap_or((0.0, 0.0));
        if visible_width <= 0.0 {
            return;
        }

        // Get cursor x-position
        if let Some(state) = self.text_edit_states.get(&focused) {
            let config_idx = self.text_input_element_ids.iter()
                .position(|&id| id == focused);
            if let Some(idx) = config_idx {
                if let Some(cfg) = self.text_input_configs.get(idx) {
                    if let Some(ref measure_fn) = self.measure_text_fn {
                        let disp_text = crate::text_input::display_text(
                            &state.text,
                            &cfg.placeholder,
                            cfg.is_password && !cfg.is_multiline,
                        );
                        if !state.text.is_empty() {
                            if cfg.is_multiline {
                                // Multiline: use visual lines with word wrapping
                                let visual_lines = crate::text_input::wrap_lines(
                                    &disp_text,
                                    visible_width,
                                    cfg.font_asset,
                                    cfg.font_size,
                                    measure_fn.as_ref(),
                                );
                                #[cfg(feature = "text-styling")]
                                let raw_cursor = state.cursor_pos_raw();
                                #[cfg(not(feature = "text-styling"))]
                                let raw_cursor = state.cursor_pos;
                                let (cursor_line, cursor_col) = crate::text_input::cursor_to_visual_pos(&visual_lines, raw_cursor);
                                let vl_text = visual_lines.get(cursor_line).map(|vl| vl.text.as_str()).unwrap_or("");
                                let line_positions = crate::text_input::compute_char_x_positions(
                                    vl_text,
                                    cfg.font_asset,
                                    cfg.font_size,
                                    measure_fn.as_ref(),
                                );
                                let cursor_x = line_positions.get(cursor_col).copied().unwrap_or(0.0);
                                let line_height = self.font_height(cfg.font_asset, cfg.font_size);
                                if let Some(state_mut) = self.text_edit_states.get_mut(&focused) {
                                    state_mut.ensure_cursor_visible(cursor_x, visible_width);
                                    state_mut.ensure_cursor_visible_vertical(cursor_line, line_height, visible_height);
                                }
                            } else {
                                let char_x_positions = crate::text_input::compute_char_x_positions(
                                    &disp_text,
                                    cfg.font_asset,
                                    cfg.font_size,
                                    measure_fn.as_ref(),
                                );
                                #[cfg(feature = "text-styling")]
                                let raw_cursor = state.cursor_pos_raw();
                                #[cfg(not(feature = "text-styling"))]
                                let raw_cursor = state.cursor_pos;
                                let cursor_x = char_x_positions
                                    .get(raw_cursor)
                                    .copied()
                                    .unwrap_or(0.0);
                                if let Some(state_mut) = self.text_edit_states.get_mut(&focused) {
                                    state_mut.ensure_cursor_visible(cursor_x, visible_width);
                                }
                            }
                        } else if let Some(state_mut) = self.text_edit_states.get_mut(&focused) {
                            state_mut.scroll_offset = 0.0;
                            state_mut.scroll_offset_y = 0.0;
                        }
                    }
                }
            }
        }
    }

    /// Handle pointer-based scrolling for text inputs: scroll wheel and drag-to-scroll.
    /// Mobile-first: dragging scrolls the content rather than selecting text.
    /// `scroll_delta` contains (x, y) scroll wheel deltas. For single-line, both axes
    /// map to horizontal scroll. For multiline, y scrolls vertically.
    pub fn update_text_input_pointer_scroll(&mut self, scroll_delta: Vector2) -> bool {
        let mut consumed_scroll = false;

        let focused = self.focused_element_id;

        // --- Scroll wheel: scroll any hovered text input (even if unfocused) ---
        let has_scroll = scroll_delta.x.abs() > 0.01 || scroll_delta.y.abs() > 0.01;
        if has_scroll {
            let p = self.pointer_info.position;
            // Find the text input under the pointer
            let hovered_ti = self.text_input_element_ids.iter().enumerate().find(|&(_, &id)| {
                self.layout_element_map.get(&id)
                    .map(|item| {
                        let bb = item.bounding_box;
                        p.x >= bb.x && p.x <= bb.x + bb.width
                            && p.y >= bb.y && p.y <= bb.y + bb.height
                    })
                    .unwrap_or(false)
            });
            if let Some((idx, &elem_id)) = hovered_ti {
                let is_multiline = self.text_input_configs.get(idx)
                    .map(|cfg| cfg.is_multiline)
                    .unwrap_or(false);
                if let Some(state) = self.text_edit_states.get_mut(&elem_id) {
                    if is_multiline {
                        if scroll_delta.y.abs() > 0.01 {
                            state.scroll_offset_y -= scroll_delta.y;
                            if state.scroll_offset_y < 0.0 {
                                state.scroll_offset_y = 0.0;
                            }
                        }
                    } else {
                        let h_delta = if scroll_delta.x.abs() > scroll_delta.y.abs() {
                            scroll_delta.x
                        } else {
                            scroll_delta.y
                        };
                        if h_delta.abs() > 0.01 {
                            state.scroll_offset -= h_delta;
                            if state.scroll_offset < 0.0 {
                                state.scroll_offset = 0.0;
                            }
                        }
                    }
                    consumed_scroll = true;
                }
            }
        }

        // --- Drag scrolling (focused text input only) ---
        if focused == 0 {
            if self.text_input_drag_active {
                let pointer_state = self.pointer_info.state;
                if matches!(pointer_state, PointerDataInteractionState::ReleasedThisFrame | PointerDataInteractionState::Released) {
                    self.text_input_drag_active = false;
                }
            }
            return consumed_scroll;
        }

        let ti_info = self.text_input_element_ids.iter()
            .position(|&id| id == focused)
            .and_then(|idx| self.text_input_configs.get(idx).map(|cfg| cfg.is_multiline));
        let is_text_input = ti_info.is_some();
        let is_multiline = ti_info.unwrap_or(false);

        if !is_text_input {
            if self.text_input_drag_active {
                let pointer_state = self.pointer_info.state;
                if matches!(pointer_state, PointerDataInteractionState::ReleasedThisFrame | PointerDataInteractionState::Released) {
                    self.text_input_drag_active = false;
                }
            }
            return consumed_scroll;
        }

        let pointer_over_focused = self.layout_element_map.get(&focused)
            .map(|item| {
                let bb = item.bounding_box;
                let p = self.pointer_info.position;
                p.x >= bb.x && p.x <= bb.x + bb.width
                    && p.y >= bb.y && p.y <= bb.y + bb.height
            })
            .unwrap_or(false);

        let pointer = self.pointer_info.position;
        let pointer_state = self.pointer_info.state;

        match pointer_state {
            PointerDataInteractionState::PressedThisFrame => {
                if pointer_over_focused {
                    let (scroll_x, scroll_y) = self.text_edit_states.get(&focused)
                        .map(|s| (s.scroll_offset, s.scroll_offset_y))
                        .unwrap_or((0.0, 0.0));
                    self.text_input_drag_active = true;
                    self.text_input_drag_origin = pointer;
                    self.text_input_drag_scroll_origin = Vector2::new(scroll_x, scroll_y);
                    self.text_input_drag_element_id = focused;
                }
            }
            PointerDataInteractionState::Pressed => {
                if self.text_input_drag_active {
                    if let Some(state) = self.text_edit_states.get_mut(&self.text_input_drag_element_id) {
                        if is_multiline {
                            let drag_delta_y = self.text_input_drag_origin.y - pointer.y;
                            state.scroll_offset_y = (self.text_input_drag_scroll_origin.y + drag_delta_y).max(0.0);
                        } else {
                            let drag_delta_x = self.text_input_drag_origin.x - pointer.x;
                            state.scroll_offset = (self.text_input_drag_scroll_origin.x + drag_delta_x).max(0.0);
                        }
                    }
                }
            }
            PointerDataInteractionState::ReleasedThisFrame
            | PointerDataInteractionState::Released => {
                self.text_input_drag_active = false;
            }
        }
        consumed_scroll
    }

    /// Clamp text input scroll offsets to valid ranges.
    /// For multiline: clamp scroll_offset_y to [0, total_height - visible_height].
    /// For single-line: clamp scroll_offset to [0, total_width - visible_width].
    pub fn clamp_text_input_scroll(&mut self) {
        for i in 0..self.text_input_element_ids.len() {
            let elem_id = self.text_input_element_ids[i];
            let cfg = match self.text_input_configs.get(i) {
                Some(c) => c,
                None => continue,
            };

            let font_asset = cfg.font_asset;
            let font_size = cfg.font_size;
            let is_multiline = cfg.is_multiline;
            let is_password = cfg.is_password;

            let (visible_width, visible_height) = self.layout_element_map.get(&elem_id)
                .map(|item| (item.bounding_box.width, item.bounding_box.height))
                .unwrap_or((200.0, 0.0));

            let text_empty = self.text_edit_states.get(&elem_id)
                .map(|s| s.text.is_empty())
                .unwrap_or(true);

            if text_empty {
                if let Some(state_mut) = self.text_edit_states.get_mut(&elem_id) {
                    state_mut.scroll_offset = 0.0;
                    state_mut.scroll_offset_y = 0.0;
                }
                continue;
            }

            if let Some(ref measure_fn) = self.measure_text_fn {
                let disp_text = self.text_edit_states.get(&elem_id)
                    .map(|s| crate::text_input::display_text(&s.text, "", is_password && !is_multiline))
                    .unwrap_or_default();

                if is_multiline {
                    let visual_lines = crate::text_input::wrap_lines(
                        &disp_text,
                        visible_width,
                        font_asset,
                        font_size,
                        measure_fn.as_ref(),
                    );
                    let font_height = self.font_height(font_asset, font_size);
                    let total_height = visual_lines.len() as f32 * font_height;
                    let max_scroll = (total_height - visible_height).max(0.0);
                    if let Some(state_mut) = self.text_edit_states.get_mut(&elem_id) {
                        if state_mut.scroll_offset_y > max_scroll {
                            state_mut.scroll_offset_y = max_scroll;
                        }
                    }
                } else {
                    // Single-line: clamp horizontal scroll
                    let char_x_positions = crate::text_input::compute_char_x_positions(
                        &disp_text,
                        font_asset,
                        font_size,
                        measure_fn.as_ref(),
                    );
                    let total_width = char_x_positions.last().copied().unwrap_or(0.0);
                    let max_scroll = (total_width - visible_width).max(0.0);
                    if let Some(state_mut) = self.text_edit_states.get_mut(&elem_id) {
                        if state_mut.scroll_offset > max_scroll {
                            state_mut.scroll_offset = max_scroll;
                        }
                    }
                }
            }
        }
    }

    /// Cycle focus to the next (or previous, if `reverse` is true) focusable element.
    /// This is called when Tab (or Shift+Tab) is pressed.
    pub fn cycle_focus(&mut self, reverse: bool) {
        if self.focusable_elements.is_empty() {
            return;
        }
        self.focus_from_keyboard = true;

        // Sort: explicit tab_index first (ascending), then insertion order
        let mut sorted: Vec<FocusableEntry> = self.focusable_elements.clone();
        sorted.sort_by(|a, b| {
            match (a.tab_index, b.tab_index) {
                (Some(ai), Some(bi)) => ai.cmp(&bi).then(a.insertion_order.cmp(&b.insertion_order)),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => a.insertion_order.cmp(&b.insertion_order),
            }
        });

        // Find current focus position
        let current_pos = sorted
            .iter()
            .position(|e| e.element_id == self.focused_element_id);

        let next_pos = match current_pos {
            Some(pos) => {
                if reverse {
                    if pos == 0 { sorted.len() - 1 } else { pos - 1 }
                } else {
                    if pos + 1 >= sorted.len() { 0 } else { pos + 1 }
                }
            }
            None => {
                // No current focus — go to first (or last if reverse)
                if reverse { sorted.len() - 1 } else { 0 }
            }
        };

        self.change_focus(sorted[next_pos].element_id);
    }

    /// Move focus based on arrow key direction, using `focus_left/right/up/down` overrides.
    pub fn arrow_focus(&mut self, direction: ArrowDirection) {
        if self.focused_element_id == 0 {
            return;
        }
        self.focus_from_keyboard = true;
        if let Some(config) = self.accessibility_configs.get(&self.focused_element_id) {
            let target = match direction {
                ArrowDirection::Left => config.focus_left,
                ArrowDirection::Right => config.focus_right,
                ArrowDirection::Up => config.focus_up,
                ArrowDirection::Down => config.focus_down,
            };
            if let Some(target_id) = target {
                self.change_focus(target_id);
            }
        }
    }

    /// Handle keyboard activation (Enter/Space) on the focused element.
    pub fn handle_keyboard_activation(&mut self, pressed: bool, released: bool) {
        if self.focused_element_id == 0 {
            return;
        }
        if pressed {
            let id_copy = self
                .layout_element_map
                .get(&self.focused_element_id)
                .map(|item| item.element_id.clone());
            if let Some(id) = id_copy {
                self.pressed_element_ids = vec![id.clone()];
                if let Some(item) = self.layout_element_map.get_mut(&self.focused_element_id) {
                    if let Some(ref mut callback) = item.on_press_fn {
                        callback(id, PointerData::default());
                    }
                }
            }
        }
        if released {
            let pressed = std::mem::take(&mut self.pressed_element_ids);
            for eid in pressed.iter() {
                if let Some(item) = self.layout_element_map.get_mut(&eid.id) {
                    if let Some(ref mut callback) = item.on_release_fn {
                        callback(eid.clone(), PointerData::default());
                    }
                }
            }
        }
    }

    pub fn pointer_over(&self, element_id: Id) -> bool {
        self.pointer_over_ids.iter().any(|eid| eid.id == element_id.id)
    }

    pub fn get_pointer_over_ids(&self) -> &[Id] {
        &self.pointer_over_ids
    }

    pub fn get_element_data(&self, id: Id) -> Option<BoundingBox> {
        self.layout_element_map
            .get(&id.id)
            .map(|item| item.bounding_box)
    }

    pub fn get_scroll_container_data(&self, id: Id) -> ScrollContainerData {
        for scd in &self.scroll_container_datas {
            if scd.element_id == id.id {
                return ScrollContainerData {
                    scroll_position: scd.scroll_position,
                    scroll_container_dimensions: Dimensions::new(
                        scd.bounding_box.width,
                        scd.bounding_box.height,
                    ),
                    content_dimensions: scd.content_size,
                    horizontal: false,
                    vertical: false,
                    found: true,
                };
            }
        }
        ScrollContainerData::default()
    }

    pub fn get_scroll_offset(&self) -> Vector2 {
        let open_idx = self.get_open_layout_element();
        let elem_id = self.layout_elements[open_idx].id;
        for scd in &self.scroll_container_datas {
            if scd.element_id == elem_id {
                return scd.scroll_position;
            }
        }
        Vector2::default()
    }

    const DEBUG_VIEW_WIDTH: f32 = 400.0;
    const DEBUG_VIEW_ROW_HEIGHT: f32 = 30.0;
    const DEBUG_VIEW_OUTER_PADDING: u16 = 10;
    const DEBUG_VIEW_INDENT_WIDTH: u16 = 16;

    const DEBUG_COLOR_1: Color = Color::rgba(58.0, 56.0, 52.0, 255.0);
    const DEBUG_COLOR_2: Color = Color::rgba(62.0, 60.0, 58.0, 255.0);
    const DEBUG_COLOR_3: Color = Color::rgba(141.0, 133.0, 135.0, 255.0);
    const DEBUG_COLOR_4: Color = Color::rgba(238.0, 226.0, 231.0, 255.0);
    #[allow(dead_code)]
    const DEBUG_COLOR_SELECTED_ROW: Color = Color::rgba(102.0, 80.0, 78.0, 255.0);
    const DEBUG_HIGHLIGHT_COLOR: Color = Color::rgba(168.0, 66.0, 28.0, 100.0);

    /// Escape text-styling special characters (`{`, `}`, `|`, `\`) so that
    /// debug view strings are never interpreted as styling markup.
    #[cfg(feature = "text-styling")]
    fn debug_escape_str(s: &str) -> String {
        let mut result = String::with_capacity(s.len());
        for c in s.chars() {
            match c {
                '{' | '}' | '|' | '\\' => {
                    result.push('\\');
                    result.push(c);
                }
                _ => result.push(c),
            }
        }
        result
    }

    /// Helper: emit a text element with a static string.
    /// When `text-styling` is enabled the string is escaped first so that
    /// braces and pipes are rendered literally.
    fn debug_text(&mut self, text: &'static str, config_index: usize) {
        #[cfg(feature = "text-styling")]
        {
            let escaped = Self::debug_escape_str(text);
            self.open_text_element(&escaped, config_index);
        }
        #[cfg(not(feature = "text-styling"))]
        {
            self.open_text_element(text, config_index);
        }
    }

    /// Helper: emit a text element from a string (e.g. element IDs
    /// or text previews). Escapes text-styling characters when that feature is
    /// active.
    fn debug_raw_text(&mut self, text: &str, config_index: usize) {
        #[cfg(feature = "text-styling")]
        {
            let escaped = Self::debug_escape_str(text);
            self.open_text_element(&escaped, config_index);
        }
        #[cfg(not(feature = "text-styling"))]
        {
            self.open_text_element(text, config_index);
        }
    }

    /// Helper: format a number as a string and emit a text element.
    fn debug_int_text(&mut self, value: f32, config_index: usize) {
        let s = format!("{}", value as i32);
        self.open_text_element(&s, config_index);
    }

    /// Helper: format a float with 2 decimal places and emit a text element.
    fn debug_float_text(&mut self, value: f32, config_index: usize) {
        let s = format!("{:.2}", value);
        self.open_text_element(&s, config_index);
    }

    /// Helper: open an element, configure, return nothing. Caller must close_element().
    fn debug_open(&mut self, decl: &ElementDeclaration<CustomElementData>) {
        self.open_element();
        self.configure_open_element(decl);
    }

    /// Helper: open a named element, configure. Caller must close_element().
    fn debug_open_id(&mut self, name: &str, decl: &ElementDeclaration<CustomElementData>) {
        self.open_element_with_id(&hash_string(name, 0));
        self.configure_open_element(decl);
    }

    /// Helper: open a named+indexed element, configure. Caller must close_element().
    fn debug_open_idi(&mut self, name: &str, offset: u32, decl: &ElementDeclaration<CustomElementData>) {
        self.open_element_with_id(&hash_string_with_offset(name, offset, 0));
        self.configure_open_element(decl);
    }

    fn debug_get_config_type_label(config_type: ElementConfigType) -> (&'static str, Color) {
        match config_type {
            ElementConfigType::Shared => ("Shared", Color::rgba(243.0, 134.0, 48.0, 255.0)),
            ElementConfigType::Text => ("Text", Color::rgba(105.0, 210.0, 231.0, 255.0)),
            ElementConfigType::Aspect => ("Aspect", Color::rgba(101.0, 149.0, 194.0, 255.0)),
            ElementConfigType::Image => ("Image", Color::rgba(121.0, 189.0, 154.0, 255.0)),
            ElementConfigType::Floating => ("Floating", Color::rgba(250.0, 105.0, 0.0, 255.0)),
            ElementConfigType::Clip => ("Overflow", Color::rgba(242.0, 196.0, 90.0, 255.0)),
            ElementConfigType::Border => ("Border", Color::rgba(108.0, 91.0, 123.0, 255.0)),
            ElementConfigType::Custom => ("Custom", Color::rgba(11.0, 72.0, 107.0, 255.0)),
            ElementConfigType::TextInput => ("TextInput", Color::rgba(52.0, 152.0, 219.0, 255.0)),
        }
    }

    /// Render the debug view sizing info for one axis.
    fn render_debug_layout_sizing(&mut self, sizing: SizingAxis, config_index: usize) {
        let label = match sizing.type_ {
            SizingType::Fit => "FIT",
            SizingType::Grow => "GROW",
            SizingType::Percent => "PERCENT",
            SizingType::Fixed => "FIXED",
            // Default handled by Grow arm above
        };
        self.debug_text(label, config_index);
        if matches!(sizing.type_, SizingType::Grow | SizingType::Fit | SizingType::Fixed) {
            self.debug_text("(", config_index);
            if sizing.min_max.min != 0.0 {
                self.debug_text("min: ", config_index);
                self.debug_int_text(sizing.min_max.min, config_index);
                if sizing.min_max.max != MAXFLOAT {
                    self.debug_text(", ", config_index);
                }
            }
            if sizing.min_max.max != MAXFLOAT {
                self.debug_text("max: ", config_index);
                self.debug_int_text(sizing.min_max.max, config_index);
            }
            self.debug_text(")", config_index);
        } else if sizing.type_ == SizingType::Percent {
            self.debug_text("(", config_index);
            self.debug_int_text(sizing.percent * 100.0, config_index);
            self.debug_text("%)", config_index);
        }
    }

    /// Render a config type header in the selected element detail panel.
    fn render_debug_view_element_config_header(
        &mut self,
        element_id_string: StringId,
        config_type: ElementConfigType,
        _info_title_config: usize,
    ) {
        let (label, label_color) = Self::debug_get_config_type_label(config_type);
        self.render_debug_view_category_header(label, label_color, element_id_string);
    }

    /// Render a category header badge with arbitrary label and color.
    fn render_debug_view_category_header(
        &mut self,
        label: &str,
        label_color: Color,
        element_id_string: StringId,
    ) {
        let bg = Color::rgba(label_color.r, label_color.g, label_color.b, 90.0);
        self.debug_open(&ElementDeclaration {
            layout: LayoutConfig {
                sizing: SizingConfig {
                    width: SizingAxis { type_: SizingType::Grow, ..Default::default() },
                    ..Default::default()
                },
                padding: PaddingConfig {
                    left: Self::DEBUG_VIEW_OUTER_PADDING,
                    right: Self::DEBUG_VIEW_OUTER_PADDING,
                    top: Self::DEBUG_VIEW_OUTER_PADDING,
                    bottom: Self::DEBUG_VIEW_OUTER_PADDING,
                },
                child_alignment: ChildAlignmentConfig { x: AlignX::Left, y: AlignY::CenterY },
                ..Default::default()
            },
            ..Default::default()
        });
        {
            // Badge
            self.debug_open(&ElementDeclaration {
                layout: LayoutConfig {
                    padding: PaddingConfig { left: 8, right: 8, top: 2, bottom: 2 },
                    ..Default::default()
                },
                background_color: bg,
                corner_radius: CornerRadius { top_left: 4.0, top_right: 4.0, bottom_left: 4.0, bottom_right: 4.0 },
                border: BorderConfig {
                    color: label_color,
                    width: BorderWidth { left: 1, right: 1, top: 1, bottom: 1, between_children: 0 },
                },
                ..Default::default()
            });
            {
                let tc = self.store_text_element_config(TextConfig {
                    color: Self::DEBUG_COLOR_4,
                    font_size: 16,
                    ..Default::default()
                });
                self.debug_raw_text(label, tc);
            }
            self.close_element();
            // Spacer
            self.debug_open(&ElementDeclaration {
                layout: LayoutConfig {
                    sizing: SizingConfig {
                        width: SizingAxis { type_: SizingType::Grow, ..Default::default() },
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            });
            self.close_element();
            // Element ID string
            let tc = self.store_text_element_config(TextConfig {
                color: Self::DEBUG_COLOR_3,
                font_size: 16,
                wrap_mode: WrapMode::None,
                ..Default::default()
            });
            if !element_id_string.is_empty() {
                self.debug_raw_text(element_id_string.as_str(), tc);
            }
        }
        self.close_element();
    }

    /// Render a color value in the debug view.
    fn render_debug_view_color(&mut self, color: Color, config_index: usize) {
        self.debug_open(&ElementDeclaration {
            layout: LayoutConfig {
                child_alignment: ChildAlignmentConfig { x: AlignX::Left, y: AlignY::CenterY },
                ..Default::default()
            },
            ..Default::default()
        });
        {
            self.debug_text("{ r: ", config_index);
            self.debug_int_text(color.r, config_index);
            self.debug_text(", g: ", config_index);
            self.debug_int_text(color.g, config_index);
            self.debug_text(", b: ", config_index);
            self.debug_int_text(color.b, config_index);
            self.debug_text(", a: ", config_index);
            self.debug_int_text(color.a, config_index);
            self.debug_text(" }", config_index);
            // Spacer
            self.debug_open(&ElementDeclaration {
                layout: LayoutConfig {
                    sizing: SizingConfig {
                        width: SizingAxis {
                            type_: SizingType::Fixed,
                            min_max: SizingMinMax { min: 10.0, max: 10.0 },
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            });
            self.close_element();
            // Color swatch
            let swatch_size = Self::DEBUG_VIEW_ROW_HEIGHT - 8.0;
            self.debug_open(&ElementDeclaration {
                layout: LayoutConfig {
                    sizing: SizingConfig {
                        width: SizingAxis {
                            type_: SizingType::Fixed,
                            min_max: SizingMinMax { min: swatch_size, max: swatch_size },
                            ..Default::default()
                        },
                        height: SizingAxis {
                            type_: SizingType::Fixed,
                            min_max: SizingMinMax { min: swatch_size, max: swatch_size },
                            ..Default::default()
                        },
                    },
                    ..Default::default()
                },
                background_color: color,
                corner_radius: CornerRadius { top_left: 4.0, top_right: 4.0, bottom_left: 4.0, bottom_right: 4.0 },
                border: BorderConfig {
                    color: Self::DEBUG_COLOR_4,
                    width: BorderWidth { left: 1, right: 1, top: 1, bottom: 1, between_children: 0 },
                },
                ..Default::default()
            });
            self.close_element();
        }
        self.close_element();
    }

    /// Render a corner radius value in the debug view.
    fn render_debug_view_corner_radius(&mut self, cr: CornerRadius, config_index: usize) {
        self.debug_open(&ElementDeclaration {
            layout: LayoutConfig {
                child_alignment: ChildAlignmentConfig { x: AlignX::Left, y: AlignY::CenterY },
                ..Default::default()
            },
            ..Default::default()
        });
        {
            self.debug_text("{ topLeft: ", config_index);
            self.debug_int_text(cr.top_left, config_index);
            self.debug_text(", topRight: ", config_index);
            self.debug_int_text(cr.top_right, config_index);
            self.debug_text(", bottomLeft: ", config_index);
            self.debug_int_text(cr.bottom_left, config_index);
            self.debug_text(", bottomRight: ", config_index);
            self.debug_int_text(cr.bottom_right, config_index);
            self.debug_text(" }", config_index);
        }
        self.close_element();
    }

    /// Render a shader uniform value in the debug view.
    fn render_debug_shader_uniform_value(&mut self, value: &crate::shaders::ShaderUniformValue, config_index: usize) {
        use crate::shaders::ShaderUniformValue;
        match value {
            ShaderUniformValue::Float(v) => {
                self.debug_float_text(*v, config_index);
            }
            ShaderUniformValue::Vec2(v) => {
                self.debug_text("(", config_index);
                self.debug_float_text(v[0], config_index);
                self.debug_text(", ", config_index);
                self.debug_float_text(v[1], config_index);
                self.debug_text(")", config_index);
            }
            ShaderUniformValue::Vec3(v) => {
                self.debug_text("(", config_index);
                self.debug_float_text(v[0], config_index);
                self.debug_text(", ", config_index);
                self.debug_float_text(v[1], config_index);
                self.debug_text(", ", config_index);
                self.debug_float_text(v[2], config_index);
                self.debug_text(")", config_index);
            }
            ShaderUniformValue::Vec4(v) => {
                self.debug_text("(", config_index);
                self.debug_float_text(v[0], config_index);
                self.debug_text(", ", config_index);
                self.debug_float_text(v[1], config_index);
                self.debug_text(", ", config_index);
                self.debug_float_text(v[2], config_index);
                self.debug_text(", ", config_index);
                self.debug_float_text(v[3], config_index);
                self.debug_text(")", config_index);
            }
            ShaderUniformValue::Int(v) => {
                self.debug_int_text(*v as f32, config_index);
            }
            ShaderUniformValue::Mat4(_) => {
                self.debug_text("[mat4]", config_index);
            }
        }
    }

    /// Render the debug layout elements tree list. Returns (row_count, selected_element_row_index).
    fn render_debug_layout_elements_list(
        &mut self,
        initial_roots_length: usize,
        highlighted_row: i32,
    ) -> (i32, i32) {
        let row_height = Self::DEBUG_VIEW_ROW_HEIGHT;
        let indent_width = Self::DEBUG_VIEW_INDENT_WIDTH;
        let mut row_count: i32 = 0;
        let mut selected_element_row_index: i32 = 0;
        let mut highlighted_element_id: u32 = 0;

        let scroll_item_layout = LayoutConfig {
            sizing: SizingConfig {
                height: SizingAxis {
                    type_: SizingType::Fixed,
                    min_max: SizingMinMax { min: row_height, max: row_height },
                    ..Default::default()
                },
                ..Default::default()
            },
            child_gap: 6,
            child_alignment: ChildAlignmentConfig { x: AlignX::Left, y: AlignY::CenterY },
            ..Default::default()
        };

        let name_text_config = TextConfig {
            color: Self::DEBUG_COLOR_4,
            font_size: 16,
            wrap_mode: WrapMode::None,
            ..Default::default()
        };

        for root_index in 0..initial_roots_length {
            let mut dfs_buffer: Vec<i32> = Vec::new();
            let root_layout_index = self.layout_element_tree_roots[root_index].layout_element_index;
            dfs_buffer.push(root_layout_index);
            let mut visited: Vec<bool> = vec![false; self.layout_elements.len()];

            // Separator between roots
            if root_index > 0 {
                self.debug_open_idi("Ply__DebugView_EmptyRowOuter", root_index as u32, &ElementDeclaration {
                    layout: LayoutConfig {
                        sizing: SizingConfig {
                            width: SizingAxis { type_: SizingType::Grow, ..Default::default() },
                            ..Default::default()
                        },
                        padding: PaddingConfig { left: indent_width / 2, right: 0, top: 0, bottom: 0 },
                        ..Default::default()
                    },
                    ..Default::default()
                });
                {
                    self.debug_open_idi("Ply__DebugView_EmptyRow", root_index as u32, &ElementDeclaration {
                        layout: LayoutConfig {
                            sizing: SizingConfig {
                                width: SizingAxis { type_: SizingType::Grow, ..Default::default() },
                                height: SizingAxis {
                                    type_: SizingType::Fixed,
                                    min_max: SizingMinMax { min: row_height, max: row_height },
                                    ..Default::default()
                                },
                            },
                            ..Default::default()
                        },
                        border: BorderConfig {
                            color: Self::DEBUG_COLOR_3,
                            width: BorderWidth { top: 1, ..Default::default() },
                        },
                        ..Default::default()
                    });
                    self.close_element();
                }
                self.close_element();
                row_count += 1;
            }

            while !dfs_buffer.is_empty() {
                let current_element_index = *dfs_buffer.last().unwrap() as usize;
                let depth = dfs_buffer.len() - 1;

                if visited[depth] {
                    // Closing: pop from stack and close containers if non-text with children
                    let is_text = self.element_has_config(current_element_index, ElementConfigType::Text);
                    let children_len = self.layout_elements[current_element_index].children_length;
                    if !is_text && children_len > 0 {
                        self.close_element();
                        self.close_element();
                        self.close_element();
                    }
                    dfs_buffer.pop();
                    continue;
                }

                // Check if this row is highlighted
                if highlighted_row == row_count {
                    if self.pointer_info.state == PointerDataInteractionState::PressedThisFrame {
                        let elem_id = self.layout_elements[current_element_index].id;
                        if self.debug_selected_element_id == elem_id {
                            self.debug_selected_element_id = 0; // Deselect on re-click
                        } else {
                            self.debug_selected_element_id = elem_id;
                        }
                    }
                    highlighted_element_id = self.layout_elements[current_element_index].id;
                }

                visited[depth] = true;
                let current_elem_id = self.layout_elements[current_element_index].id;

                // Get bounding box and collision info from hash map
                let bounding_box = self.layout_element_map
                    .get(&current_elem_id)
                    .map(|item| item.bounding_box)
                    .unwrap_or_default();
                let collision = self.layout_element_map
                    .get(&current_elem_id)
                    .map(|item| item.collision)
                    .unwrap_or(false);
                let collapsed = self.layout_element_map
                    .get(&current_elem_id)
                    .map(|item| item.collapsed)
                    .unwrap_or(false);

                let offscreen = self.element_is_offscreen(&bounding_box);

                if self.debug_selected_element_id == current_elem_id {
                    selected_element_row_index = row_count;
                }

                // Row for this element
                let row_bg = if self.debug_selected_element_id == current_elem_id {
                    Color::rgba(217.0, 91.0, 67.0, 40.0) // Slight red for selected
                } else {
                    Color::rgba(0.0, 0.0, 0.0, 0.0)
                };
                self.debug_open_idi("Ply__DebugView_ElementOuter", current_elem_id, &ElementDeclaration {
                    layout: scroll_item_layout,
                    background_color: row_bg,
                    ..Default::default()
                });
                {
                    let is_text = self.element_has_config(current_element_index, ElementConfigType::Text);
                    let children_len = self.layout_elements[current_element_index].children_length;

                    // Collapse icon / button or dot
                    if !is_text && children_len > 0 {
                        // Collapse button
                        self.debug_open_idi("Ply__DebugView_CollapseElement", current_elem_id, &ElementDeclaration {
                            layout: LayoutConfig {
                                sizing: SizingConfig {
                                    width: SizingAxis { type_: SizingType::Fixed, min_max: SizingMinMax { min: 16.0, max: 16.0 }, ..Default::default() },
                                    height: SizingAxis { type_: SizingType::Fixed, min_max: SizingMinMax { min: 16.0, max: 16.0 }, ..Default::default() },
                                },
                                child_alignment: ChildAlignmentConfig { x: AlignX::CenterX, y: AlignY::CenterY },
                                ..Default::default()
                            },
                            corner_radius: CornerRadius { top_left: 4.0, top_right: 4.0, bottom_left: 4.0, bottom_right: 4.0 },
                            border: BorderConfig {
                                color: Self::DEBUG_COLOR_3,
                                width: BorderWidth { left: 1, right: 1, top: 1, bottom: 1, between_children: 0 },
                            },
                            ..Default::default()
                        });
                        {
                            let tc = self.store_text_element_config(TextConfig {
                                color: Self::DEBUG_COLOR_4,
                                font_size: 16,
                                ..Default::default()
                            });
                            if collapsed {
                                self.debug_text("+", tc);
                            } else {
                                self.debug_text("-", tc);
                            }
                        }
                        self.close_element();
                    } else {
                        // Empty dot for leaf elements
                        self.debug_open(&ElementDeclaration {
                            layout: LayoutConfig {
                                sizing: SizingConfig {
                                    width: SizingAxis { type_: SizingType::Fixed, min_max: SizingMinMax { min: 16.0, max: 16.0 }, ..Default::default() },
                                    height: SizingAxis { type_: SizingType::Fixed, min_max: SizingMinMax { min: 16.0, max: 16.0 }, ..Default::default() },
                                },
                                child_alignment: ChildAlignmentConfig { x: AlignX::CenterX, y: AlignY::CenterY },
                                ..Default::default()
                            },
                            ..Default::default()
                        });
                        {
                            self.debug_open(&ElementDeclaration {
                                layout: LayoutConfig {
                                    sizing: SizingConfig {
                                        width: SizingAxis { type_: SizingType::Fixed, min_max: SizingMinMax { min: 8.0, max: 8.0 }, ..Default::default() },
                                        height: SizingAxis { type_: SizingType::Fixed, min_max: SizingMinMax { min: 8.0, max: 8.0 }, ..Default::default() },
                                    },
                                    ..Default::default()
                                },
                                background_color: Self::DEBUG_COLOR_3,
                                corner_radius: CornerRadius { top_left: 2.0, top_right: 2.0, bottom_left: 2.0, bottom_right: 2.0 },
                                ..Default::default()
                            });
                            self.close_element();
                        }
                        self.close_element();
                    }

                    // Collision warning badge
                    if collision {
                        self.debug_open(&ElementDeclaration {
                            layout: LayoutConfig {
                                padding: PaddingConfig { left: 8, right: 8, top: 2, bottom: 2 },
                                ..Default::default()
                            },
                            border: BorderConfig {
                                color: Color::rgba(177.0, 147.0, 8.0, 255.0),
                                width: BorderWidth { left: 1, right: 1, top: 1, bottom: 1, between_children: 0 },
                            },
                            ..Default::default()
                        });
                        {
                            let tc = self.store_text_element_config(TextConfig {
                                color: Self::DEBUG_COLOR_3,
                                font_size: 16,
                                ..Default::default()
                            });
                            self.debug_text("Duplicate ID", tc);
                        }
                        self.close_element();
                    }

                    // Offscreen badge
                    if offscreen {
                        self.debug_open(&ElementDeclaration {
                            layout: LayoutConfig {
                                padding: PaddingConfig { left: 8, right: 8, top: 2, bottom: 2 },
                                ..Default::default()
                            },
                            border: BorderConfig {
                                color: Self::DEBUG_COLOR_3,
                                width: BorderWidth { left: 1, right: 1, top: 1, bottom: 1, between_children: 0 },
                            },
                            ..Default::default()
                        });
                        {
                            let tc = self.store_text_element_config(TextConfig {
                                color: Self::DEBUG_COLOR_3,
                                font_size: 16,
                                ..Default::default()
                            });
                            self.debug_text("Offscreen", tc);
                        }
                        self.close_element();
                    }

                    // Element name
                    let id_string = if current_element_index < self.layout_element_id_strings.len() {
                        self.layout_element_id_strings[current_element_index].clone()
                    } else {
                        StringId::empty()
                    };
                    if !id_string.is_empty() {
                        let tc = if offscreen {
                            self.store_text_element_config(TextConfig {
                                color: Self::DEBUG_COLOR_3,
                                font_size: 16,
                                ..Default::default()
                            })
                        } else {
                            self.store_text_element_config(name_text_config.clone())
                        };
                        self.debug_raw_text(id_string.as_str(), tc);
                    }

                    // Config type badges
                    let configs_start = self.layout_elements[current_element_index].element_configs.start;
                    let configs_len = self.layout_elements[current_element_index].element_configs.length;
                    for ci in 0..configs_len {
                        let ec = self.element_configs[configs_start + ci as usize];
                        if ec.config_type == ElementConfigType::Shared {
                            let shared = self.shared_element_configs[ec.config_index];
                            let label_color = Color::rgba(243.0, 134.0, 48.0, 90.0);
                            if shared.background_color.a > 0.0 {
                                self.debug_open(&ElementDeclaration {
                                    layout: LayoutConfig {
                                        padding: PaddingConfig { left: 8, right: 8, top: 2, bottom: 2 },
                                        ..Default::default()
                                    },
                                    background_color: label_color,
                                    corner_radius: CornerRadius { top_left: 4.0, top_right: 4.0, bottom_left: 4.0, bottom_right: 4.0 },
                                    border: BorderConfig {
                                        color: label_color,
                                        width: BorderWidth { left: 1, right: 1, top: 1, bottom: 1, between_children: 0 },
                                    },
                                    ..Default::default()
                                });
                                {
                                    let tc = self.store_text_element_config(TextConfig {
                                        color: if offscreen { Self::DEBUG_COLOR_3 } else { Self::DEBUG_COLOR_4 },
                                        font_size: 16,
                                        ..Default::default()
                                    });
                                    self.debug_text("Color", tc);
                                }
                                self.close_element();
                            }
                            if shared.corner_radius.bottom_left > 0.0 {
                                let radius_color = Color::rgba(26.0, 188.0, 156.0, 90.0);
                                self.debug_open(&ElementDeclaration {
                                    layout: LayoutConfig {
                                        padding: PaddingConfig { left: 8, right: 8, top: 2, bottom: 2 },
                                        ..Default::default()
                                    },
                                    background_color: radius_color,
                                    corner_radius: CornerRadius { top_left: 4.0, top_right: 4.0, bottom_left: 4.0, bottom_right: 4.0 },
                                    border: BorderConfig {
                                        color: radius_color,
                                        width: BorderWidth { left: 1, right: 1, top: 1, bottom: 1, between_children: 0 },
                                    },
                                    ..Default::default()
                                });
                                {
                                    let tc = self.store_text_element_config(TextConfig {
                                        color: if offscreen { Self::DEBUG_COLOR_3 } else { Self::DEBUG_COLOR_4 },
                                        font_size: 16,
                                        ..Default::default()
                                    });
                                    self.debug_text("Radius", tc);
                                }
                                self.close_element();
                            }
                            continue;
                        }
                        let (label, label_color) = Self::debug_get_config_type_label(ec.config_type);
                        let bg = Color::rgba(label_color.r, label_color.g, label_color.b, 90.0);
                        self.debug_open(&ElementDeclaration {
                            layout: LayoutConfig {
                                padding: PaddingConfig { left: 8, right: 8, top: 2, bottom: 2 },
                                ..Default::default()
                            },
                            background_color: bg,
                            corner_radius: CornerRadius { top_left: 4.0, top_right: 4.0, bottom_left: 4.0, bottom_right: 4.0 },
                            border: BorderConfig {
                                color: label_color,
                                width: BorderWidth { left: 1, right: 1, top: 1, bottom: 1, between_children: 0 },
                            },
                            ..Default::default()
                        });
                        {
                            let tc = self.store_text_element_config(TextConfig {
                                color: if offscreen { Self::DEBUG_COLOR_3 } else { Self::DEBUG_COLOR_4 },
                                font_size: 16,
                                ..Default::default()
                            });
                            self.debug_text(label, tc);
                        }
                        self.close_element();
                    }

                    // Shader badge
                    let has_shaders = self.element_shaders.get(current_element_index)
                        .map_or(false, |s| !s.is_empty());
                    if has_shaders {
                        let badge_color = Color::rgba(155.0, 89.0, 182.0, 90.0);
                        self.debug_open(&ElementDeclaration {
                            layout: LayoutConfig {
                                padding: PaddingConfig { left: 8, right: 8, top: 2, bottom: 2 },
                                ..Default::default()
                            },
                            background_color: badge_color,
                            corner_radius: CornerRadius { top_left: 4.0, top_right: 4.0, bottom_left: 4.0, bottom_right: 4.0 },
                            border: BorderConfig {
                                color: badge_color,
                                width: BorderWidth { left: 1, right: 1, top: 1, bottom: 1, between_children: 0 },
                            },
                            ..Default::default()
                        });
                        {
                            let tc = self.store_text_element_config(TextConfig {
                                color: if offscreen { Self::DEBUG_COLOR_3 } else { Self::DEBUG_COLOR_4 },
                                font_size: 16,
                                ..Default::default()
                            });
                            self.debug_text("Shader", tc);
                        }
                        self.close_element();
                    }

                    // Visual Rotation badge
                    let has_visual_rot = self.element_visual_rotations.get(current_element_index)
                        .map_or(false, |r| r.is_some());
                    if has_visual_rot {
                        let badge_color = Color::rgba(155.0, 89.0, 182.0, 90.0);
                        self.debug_open(&ElementDeclaration {
                            layout: LayoutConfig {
                                padding: PaddingConfig { left: 8, right: 8, top: 2, bottom: 2 },
                                ..Default::default()
                            },
                            background_color: badge_color,
                            corner_radius: CornerRadius { top_left: 4.0, top_right: 4.0, bottom_left: 4.0, bottom_right: 4.0 },
                            border: BorderConfig {
                                color: badge_color,
                                width: BorderWidth { left: 1, right: 1, top: 1, bottom: 1, between_children: 0 },
                            },
                            ..Default::default()
                        });
                        {
                            let tc = self.store_text_element_config(TextConfig {
                                color: if offscreen { Self::DEBUG_COLOR_3 } else { Self::DEBUG_COLOR_4 },
                                font_size: 16,
                                ..Default::default()
                            });
                            self.debug_text("VisualRot", tc);
                        }
                        self.close_element();
                    }

                    // Shape Rotation badge
                    let has_shape_rot = self.element_shape_rotations.get(current_element_index)
                        .map_or(false, |r| r.is_some());
                    if has_shape_rot {
                        let badge_color = Color::rgba(26.0, 188.0, 156.0, 90.0);
                        self.debug_open(&ElementDeclaration {
                            layout: LayoutConfig {
                                padding: PaddingConfig { left: 8, right: 8, top: 2, bottom: 2 },
                                ..Default::default()
                            },
                            background_color: badge_color,
                            corner_radius: CornerRadius { top_left: 4.0, top_right: 4.0, bottom_left: 4.0, bottom_right: 4.0 },
                            border: BorderConfig {
                                color: badge_color,
                                width: BorderWidth { left: 1, right: 1, top: 1, bottom: 1, between_children: 0 },
                            },
                            ..Default::default()
                        });
                        {
                            let tc = self.store_text_element_config(TextConfig {
                                color: if offscreen { Self::DEBUG_COLOR_3 } else { Self::DEBUG_COLOR_4 },
                                font_size: 16,
                                ..Default::default()
                            });
                            self.debug_text("ShapeRot", tc);
                        }
                        self.close_element();
                    }
                }
                self.close_element(); // ElementOuter row

                // Text element content row
                let is_text = self.element_has_config(current_element_index, ElementConfigType::Text);
                let children_len = self.layout_elements[current_element_index].children_length;
                if is_text {
                    row_count += 1;
                    let text_data_idx = self.layout_elements[current_element_index].text_data_index;
                    let text_content = if text_data_idx >= 0 {
                        self.text_element_data[text_data_idx as usize].text.clone()
                    } else {
                        String::new()
                    };
                    let raw_tc_idx = if offscreen {
                        self.store_text_element_config(TextConfig {
                            color: Self::DEBUG_COLOR_3,
                            font_size: 16,
                            ..Default::default()
                        })
                    } else {
                        self.store_text_element_config(name_text_config.clone())
                    };
                    self.debug_open(&ElementDeclaration {
                        layout: LayoutConfig {
                            sizing: SizingConfig {
                                height: SizingAxis {
                                    type_: SizingType::Fixed,
                                    min_max: SizingMinMax { min: row_height, max: row_height },
                                    ..Default::default()
                                },
                                ..Default::default()
                            },
                            child_alignment: ChildAlignmentConfig { x: AlignX::Left, y: AlignY::CenterY },
                            ..Default::default()
                        },
                        ..Default::default()
                    });
                    {
                        // Indent spacer
                        self.debug_open(&ElementDeclaration {
                            layout: LayoutConfig {
                                sizing: SizingConfig {
                                    width: SizingAxis {
                                        type_: SizingType::Fixed,
                                        min_max: SizingMinMax {
                                            min: (indent_width + 16) as f32,
                                            max: (indent_width + 16) as f32,
                                        },
                                        ..Default::default()
                                    },
                                    ..Default::default()
                                },
                                ..Default::default()
                            },
                            ..Default::default()
                        });
                        self.close_element();
                        self.debug_text("\"", raw_tc_idx);
                        if text_content.len() > 40 {
                            let mut end = 40;
                            while !text_content.is_char_boundary(end) { end -= 1; }
                            self.debug_raw_text(&text_content[..end], raw_tc_idx);
                            self.debug_text("...", raw_tc_idx);
                        } else if !text_content.is_empty() {
                            self.debug_raw_text(&text_content, raw_tc_idx);
                        }
                        self.debug_text("\"", raw_tc_idx);
                    }
                    self.close_element();
                } else if children_len > 0 {
                    // Open containers for child indentation
                    self.open_element();
                    self.configure_open_element(&ElementDeclaration {
                        layout: LayoutConfig {
                            padding: PaddingConfig { left: 8, ..Default::default() },
                            ..Default::default()
                        },
                        ..Default::default()
                    });
                    self.open_element();
                    self.configure_open_element(&ElementDeclaration {
                        layout: LayoutConfig {
                            padding: PaddingConfig { left: indent_width, ..Default::default() },
                            ..Default::default()
                        },
                        border: BorderConfig {
                            color: Self::DEBUG_COLOR_3,
                            width: BorderWidth { left: 1, ..Default::default() },
                        },
                        ..Default::default()
                    });
                    self.open_element();
                    self.configure_open_element(&ElementDeclaration {
                        layout: LayoutConfig {
                            layout_direction: LayoutDirection::TopToBottom,
                            ..Default::default()
                        },
                        ..Default::default()
                    });
                }

                row_count += 1;

                // Push children in reverse order for DFS (if not text and not collapsed)
                if !is_text && !collapsed {
                    let children_start = self.layout_elements[current_element_index].children_start;
                    let children_length = self.layout_elements[current_element_index].children_length as usize;
                    for i in (0..children_length).rev() {
                        let child_idx = self.layout_element_children[children_start + i];
                        dfs_buffer.push(child_idx);
                        // Ensure visited vec is large enough
                        while visited.len() <= dfs_buffer.len() {
                            visited.push(false);
                        }
                        visited[dfs_buffer.len() - 1] = false;
                    }
                }
            }
        }

        // Handle collapse button clicks
        if self.pointer_info.state == PointerDataInteractionState::PressedThisFrame {
            let collapse_base_id = hash_string("Ply__DebugView_CollapseElement", 0).base_id;
            for i in (0..self.pointer_over_ids.len()).rev() {
                let element_id = self.pointer_over_ids[i].clone();
                if element_id.base_id == collapse_base_id {
                    if let Some(item) = self.layout_element_map.get_mut(&element_id.offset) {
                        item.collapsed = !item.collapsed;
                    }
                    break;
                }
            }
        }

        // Render highlight on hovered or selected element
        // When an element is selected, show its bounding box; otherwise show hovered
        let highlight_target = if self.debug_selected_element_id != 0 {
            self.debug_selected_element_id
        } else {
            highlighted_element_id
        };
        if highlight_target != 0 {
            self.debug_open_id("Ply__DebugView_ElementHighlight", &ElementDeclaration {
                layout: LayoutConfig {
                    sizing: SizingConfig {
                        width: SizingAxis { type_: SizingType::Grow, ..Default::default() },
                        height: SizingAxis { type_: SizingType::Grow, ..Default::default() },
                    },
                    ..Default::default()
                },
                floating: FloatingConfig {
                    parent_id: highlight_target,
                    z_index: 32767,
                    pointer_capture_mode: PointerCaptureMode::Passthrough,
                    attach_to: FloatingAttachToElement::ElementWithId,
                    ..Default::default()
                },
                ..Default::default()
            });
            {
                self.debug_open_id("Ply__DebugView_ElementHighlightRectangle", &ElementDeclaration {
                    layout: LayoutConfig {
                        sizing: SizingConfig {
                            width: SizingAxis { type_: SizingType::Grow, ..Default::default() },
                            height: SizingAxis { type_: SizingType::Grow, ..Default::default() },
                        },
                        ..Default::default()
                    },
                    background_color: Self::DEBUG_HIGHLIGHT_COLOR,
                    ..Default::default()
                });
                self.close_element();
            }
            self.close_element();
        }

        (row_count, selected_element_row_index)
    }

    /// Main debug view rendering. Called from end_layout() when debug mode is enabled.
    fn render_debug_view(&mut self) {
        let initial_roots_length = self.layout_element_tree_roots.len();
        let initial_elements_length = self.layout_elements.len();
        let row_height = Self::DEBUG_VIEW_ROW_HEIGHT;
        let outer_padding = Self::DEBUG_VIEW_OUTER_PADDING;
        let debug_width = Self::DEBUG_VIEW_WIDTH;

        let info_text_config = self.store_text_element_config(TextConfig {
            color: Self::DEBUG_COLOR_4,
            font_size: 16,
            wrap_mode: WrapMode::None,
            ..Default::default()
        });
        let info_title_config = self.store_text_element_config(TextConfig {
            color: Self::DEBUG_COLOR_3,
            font_size: 16,
            wrap_mode: WrapMode::None,
            ..Default::default()
        });

        // Determine scroll offset for the debug scroll pane
        let scroll_id = hash_string("Ply__DebugViewOuterScrollPane", 0);
        let mut scroll_y_offset: f32 = 0.0;
        // Only exclude the bottom 300px from tree interaction when the detail panel is shown
        let detail_panel_height = if self.debug_selected_element_id != 0 { 300.0 } else { 0.0 };
        let mut pointer_in_debug_view = self.pointer_info.position.y < self.layout_dimensions.height - detail_panel_height;
        for scd in &self.scroll_container_datas {
            if scd.element_id == scroll_id.id {
                if !self.external_scroll_handling_enabled {
                    scroll_y_offset = scd.scroll_position.y;
                } else {
                    pointer_in_debug_view = self.pointer_info.position.y + scd.scroll_position.y
                        < self.layout_dimensions.height - detail_panel_height;
                }
                break;
            }
        }

        let highlighted_row = if pointer_in_debug_view {
            ((self.pointer_info.position.y - scroll_y_offset) / row_height) as i32 - 1
        } else {
            -1
        };
        let highlighted_row = if self.pointer_info.position.x < self.layout_dimensions.width - debug_width {
            -1
        } else {
            highlighted_row
        };

        // Main debug view panel (floating)
        self.debug_open_id("Ply__DebugView", &ElementDeclaration {
            layout: LayoutConfig {
                sizing: SizingConfig {
                    width: SizingAxis {
                        type_: SizingType::Fixed,
                        min_max: SizingMinMax { min: debug_width, max: debug_width },
                        ..Default::default()
                    },
                    height: SizingAxis {
                        type_: SizingType::Fixed,
                        min_max: SizingMinMax { min: self.layout_dimensions.height, max: self.layout_dimensions.height },
                        ..Default::default()
                    },
                },
                layout_direction: LayoutDirection::TopToBottom,
                ..Default::default()
            },
            floating: FloatingConfig {
                z_index: 32765,
                attach_points: FloatingAttachPoints {
                    element_x: AlignX::Right,
                    element_y: AlignY::CenterY,
                    parent_x: AlignX::Right,
                    parent_y: AlignY::CenterY,
                },
                attach_to: FloatingAttachToElement::Root,
                clip_to: FloatingClipToElement::AttachedParent,
                ..Default::default()
            },
            border: BorderConfig {
                color: Self::DEBUG_COLOR_3,
                width: BorderWidth { bottom: 1, ..Default::default() },
            },
            ..Default::default()
        });
        {
            // Header bar
            self.debug_open(&ElementDeclaration {
                layout: LayoutConfig {
                    sizing: SizingConfig {
                        width: SizingAxis { type_: SizingType::Grow, ..Default::default() },
                        height: SizingAxis {
                            type_: SizingType::Fixed,
                            min_max: SizingMinMax { min: row_height, max: row_height },
                            ..Default::default()
                        },
                    },
                    padding: PaddingConfig { left: outer_padding, right: outer_padding, top: 0, bottom: 0 },
                    child_alignment: ChildAlignmentConfig { x: AlignX::Left, y: AlignY::CenterY },
                    ..Default::default()
                },
                background_color: Self::DEBUG_COLOR_2,
                ..Default::default()
            });
            {
                self.debug_text("Ply Debug Tools", info_text_config);
                // Spacer
                self.debug_open(&ElementDeclaration {
                    layout: LayoutConfig {
                        sizing: SizingConfig {
                            width: SizingAxis { type_: SizingType::Grow, ..Default::default() },
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    ..Default::default()
                });
                self.close_element();
                // Close button
                let close_size = row_height - 10.0;
                self.debug_open_id("Ply__DebugView_CloseButton", &ElementDeclaration {
                    layout: LayoutConfig {
                        sizing: SizingConfig {
                            width: SizingAxis { type_: SizingType::Fixed, min_max: SizingMinMax { min: close_size, max: close_size }, ..Default::default() },
                            height: SizingAxis { type_: SizingType::Fixed, min_max: SizingMinMax { min: close_size, max: close_size }, ..Default::default() },
                        },
                        child_alignment: ChildAlignmentConfig { x: AlignX::CenterX, y: AlignY::CenterY },
                        ..Default::default()
                    },
                    background_color: Color::rgba(217.0, 91.0, 67.0, 80.0),
                    corner_radius: CornerRadius { top_left: 4.0, top_right: 4.0, bottom_left: 4.0, bottom_right: 4.0 },
                    border: BorderConfig {
                        color: Color::rgba(217.0, 91.0, 67.0, 255.0),
                        width: BorderWidth { left: 1, right: 1, top: 1, bottom: 1, between_children: 0 },
                    },
                    ..Default::default()
                });
                {
                    let tc = self.store_text_element_config(TextConfig {
                        color: Self::DEBUG_COLOR_4,
                        font_size: 16,
                        ..Default::default()
                    });
                    self.debug_text("x", tc);
                }
                self.close_element();
            }
            self.close_element();

            // Separator line
            self.debug_open(&ElementDeclaration {
                layout: LayoutConfig {
                    sizing: SizingConfig {
                        width: SizingAxis { type_: SizingType::Grow, ..Default::default() },
                        height: SizingAxis { type_: SizingType::Fixed, min_max: SizingMinMax { min: 1.0, max: 1.0 }, ..Default::default() },
                    },
                    ..Default::default()
                },
                background_color: Self::DEBUG_COLOR_3,
                ..Default::default()
            });
            self.close_element();

            // Scroll pane
            self.open_element_with_id(&scroll_id);
            self.configure_open_element(&ElementDeclaration {
                layout: LayoutConfig {
                    sizing: SizingConfig {
                        width: SizingAxis { type_: SizingType::Grow, ..Default::default() },
                        height: SizingAxis { type_: SizingType::Grow, ..Default::default() },
                    },
                    ..Default::default()
                },
                clip: ClipConfig {
                    horizontal: true,
                    vertical: true,
                    scroll_x: true,
                    scroll_y: true,
                    child_offset: self.get_scroll_offset(),
                },
                ..Default::default()
            });
            {
                let alt_bg = if (initial_elements_length + initial_roots_length) & 1 == 0 {
                    Self::DEBUG_COLOR_2
                } else {
                    Self::DEBUG_COLOR_1
                };
                // Content container — Fit height so it extends beyond the scroll pane
                self.debug_open(&ElementDeclaration {
                    layout: LayoutConfig {
                        sizing: SizingConfig {
                            width: SizingAxis { type_: SizingType::Grow, ..Default::default() },
                            ..Default::default() // height defaults to Fit
                        },
                        padding: PaddingConfig {
                            left: outer_padding,
                            right: outer_padding,
                            top: 0,
                            bottom: 0,
                        },
                        layout_direction: LayoutDirection::TopToBottom,
                        ..Default::default()
                    },
                    background_color: alt_bg,
                    ..Default::default()
                });
                {
                    let _layout_data = self.render_debug_layout_elements_list(
                        initial_roots_length,
                        highlighted_row,
                    );
                }
                self.close_element(); // content container
            }
            self.close_element(); // scroll pane

            // Separator
            self.debug_open(&ElementDeclaration {
                layout: LayoutConfig {
                    sizing: SizingConfig {
                        width: SizingAxis { type_: SizingType::Grow, ..Default::default() },
                        height: SizingAxis { type_: SizingType::Fixed, min_max: SizingMinMax { min: 1.0, max: 1.0 }, ..Default::default() },
                    },
                    ..Default::default()
                },
                background_color: Self::DEBUG_COLOR_3,
                ..Default::default()
            });
            self.close_element();

            // Selected element detail panel
            if self.debug_selected_element_id != 0 {
                self.render_debug_selected_element_panel(info_text_config, info_title_config);
            }
        }
        self.close_element(); // Ply__DebugView

        // Handle close button click
        if self.pointer_info.state == PointerDataInteractionState::PressedThisFrame {
            let close_base_id = hash_string("Ply__DebugView_CloseButton", 0).id;
            let header_base_id = hash_string("Ply__DebugView_LayoutConfigHeader", 0).id;
            for i in (0..self.pointer_over_ids.len()).rev() {
                let id = self.pointer_over_ids[i].id;
                if id == close_base_id {
                    self.debug_mode_enabled = false;
                    break;
                }
                if id == header_base_id {
                    self.debug_selected_element_id = 0;
                    break;
                }
            }
        }
    }

    /// Render the selected element detail panel in the debug view.
    fn render_debug_selected_element_panel(
        &mut self,
        info_text_config: usize,
        info_title_config: usize,
    ) {
        let row_height = Self::DEBUG_VIEW_ROW_HEIGHT;
        let outer_padding = Self::DEBUG_VIEW_OUTER_PADDING;
        let attr_padding = PaddingConfig {
            left: outer_padding,
            right: outer_padding,
            top: 8,
            bottom: 8,
        };

        let selected_id = self.debug_selected_element_id;
        let selected_item = match self.layout_element_map.get(&selected_id) {
            Some(item) => item.clone(),
            None => return,
        };
        let layout_elem_idx = selected_item.layout_element_index as usize;
        if layout_elem_idx >= self.layout_elements.len() {
            return;
        }

        let layout_config_index = self.layout_elements[layout_elem_idx].layout_config_index;
        let layout_config = self.layout_configs[layout_config_index];

        self.debug_open(&ElementDeclaration {
            layout: LayoutConfig {
                sizing: SizingConfig {
                    width: SizingAxis { type_: SizingType::Grow, ..Default::default() },
                    height: SizingAxis {
                        type_: SizingType::Fixed,
                        min_max: SizingMinMax { min: 316.0, max: 316.0 },
                        ..Default::default()
                    },
                },
                layout_direction: LayoutDirection::TopToBottom,
                ..Default::default()
            },
            background_color: Self::DEBUG_COLOR_2,
            clip: ClipConfig {
                vertical: true,
                scroll_y: true,
                child_offset: self.get_scroll_offset(),
                ..Default::default()
            },
            border: BorderConfig {
                color: Self::DEBUG_COLOR_3,
                width: BorderWidth { between_children: 1, ..Default::default() },
            },
            ..Default::default()
        });
        {
            // Header: "Layout Config" + element ID
            self.debug_open_id("Ply__DebugView_LayoutConfigHeader", &ElementDeclaration {
                layout: LayoutConfig {
                    sizing: SizingConfig {
                        width: SizingAxis { type_: SizingType::Grow, ..Default::default() },
                        height: SizingAxis {
                            type_: SizingType::Fixed,
                            min_max: SizingMinMax { min: row_height + 8.0, max: row_height + 8.0 },
                            ..Default::default()
                        },
                    },
                    padding: PaddingConfig { left: outer_padding, right: outer_padding, top: 0, bottom: 0 },
                    child_alignment: ChildAlignmentConfig { x: AlignX::Left, y: AlignY::CenterY },
                    ..Default::default()
                },
                ..Default::default()
            });
            {
                self.debug_text("Layout Config", info_text_config);
                // Spacer
                self.debug_open(&ElementDeclaration {
                    layout: LayoutConfig {
                        sizing: SizingConfig {
                            width: SizingAxis { type_: SizingType::Grow, ..Default::default() },
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    ..Default::default()
                });
                self.close_element();
                // Element ID string
                let sid = selected_item.element_id.string_id.clone();
                if !sid.is_empty() {
                    self.debug_raw_text(sid.as_str(), info_title_config);
                    if selected_item.element_id.offset != 0 {
                        self.debug_text(" (", info_title_config);
                        self.debug_int_text(selected_item.element_id.offset as f32, info_title_config);
                        self.debug_text(")", info_title_config);
                    }
                }
            }
            self.close_element();

            // Layout config details
            self.debug_open(&ElementDeclaration {
                layout: LayoutConfig {
                    padding: attr_padding,
                    child_gap: 8,
                    layout_direction: LayoutDirection::TopToBottom,
                    ..Default::default()
                },
                ..Default::default()
            });
            {
                // Bounding Box
                self.debug_text("Bounding Box", info_title_config);
                self.debug_open(&ElementDeclaration::default());
                {
                    self.debug_text("{ x: ", info_text_config);
                    self.debug_int_text(selected_item.bounding_box.x, info_text_config);
                    self.debug_text(", y: ", info_text_config);
                    self.debug_int_text(selected_item.bounding_box.y, info_text_config);
                    self.debug_text(", width: ", info_text_config);
                    self.debug_int_text(selected_item.bounding_box.width, info_text_config);
                    self.debug_text(", height: ", info_text_config);
                    self.debug_int_text(selected_item.bounding_box.height, info_text_config);
                    self.debug_text(" }", info_text_config);
                }
                self.close_element();

                // Layout Direction
                self.debug_text("Layout Direction", info_title_config);
                if layout_config.layout_direction == LayoutDirection::TopToBottom {
                    self.debug_text("TOP_TO_BOTTOM", info_text_config);
                } else {
                    self.debug_text("LEFT_TO_RIGHT", info_text_config);
                }

                // Sizing
                self.debug_text("Sizing", info_title_config);
                self.debug_open(&ElementDeclaration::default());
                {
                    self.debug_text("width: ", info_text_config);
                    self.render_debug_layout_sizing(layout_config.sizing.width, info_text_config);
                }
                self.close_element();
                self.debug_open(&ElementDeclaration::default());
                {
                    self.debug_text("height: ", info_text_config);
                    self.render_debug_layout_sizing(layout_config.sizing.height, info_text_config);
                }
                self.close_element();

                // Padding
                self.debug_text("Padding", info_title_config);
                self.debug_open_id("Ply__DebugViewElementInfoPadding", &ElementDeclaration::default());
                {
                    self.debug_text("{ left: ", info_text_config);
                    self.debug_int_text(layout_config.padding.left as f32, info_text_config);
                    self.debug_text(", right: ", info_text_config);
                    self.debug_int_text(layout_config.padding.right as f32, info_text_config);
                    self.debug_text(", top: ", info_text_config);
                    self.debug_int_text(layout_config.padding.top as f32, info_text_config);
                    self.debug_text(", bottom: ", info_text_config);
                    self.debug_int_text(layout_config.padding.bottom as f32, info_text_config);
                    self.debug_text(" }", info_text_config);
                }
                self.close_element();

                // Child Gap
                self.debug_text("Child Gap", info_title_config);
                self.debug_int_text(layout_config.child_gap as f32, info_text_config);

                // Child Alignment
                self.debug_text("Child Alignment", info_title_config);
                self.debug_open(&ElementDeclaration::default());
                {
                    self.debug_text("{ x: ", info_text_config);
                    let align_x = Self::align_x_name(layout_config.child_alignment.x);
                    self.debug_text(align_x, info_text_config);
                    self.debug_text(", y: ", info_text_config);
                    let align_y = Self::align_y_name(layout_config.child_alignment.y);
                    self.debug_text(align_y, info_text_config);
                    self.debug_text(" }", info_text_config);
                }
                self.close_element();
            }
            self.close_element(); // layout config details

            // ── Collect data for grouped categories ──
            let configs_start = self.layout_elements[layout_elem_idx].element_configs.start;
            let configs_len = self.layout_elements[layout_elem_idx].element_configs.length;
            let elem_id_string = selected_item.element_id.string_id.clone();

            // Shared data (split into Color + Shape)
            let mut shared_bg_color: Option<Color> = None;
            let mut shared_corner_radius: Option<CornerRadius> = None;
            for ci in 0..configs_len {
                let ec = self.element_configs[configs_start + ci as usize];
                if ec.config_type == ElementConfigType::Shared {
                    let shared = self.shared_element_configs[ec.config_index];
                    shared_bg_color = Some(shared.background_color);
                    shared_corner_radius = Some(shared.corner_radius);
                }
            }

            // Per-element data (not in element_configs system)
            let shape_rot = self.element_shape_rotations.get(layout_elem_idx).copied().flatten();
            let visual_rot = self.element_visual_rotations.get(layout_elem_idx).cloned().flatten();
            let effects = self.element_effects.get(layout_elem_idx).cloned().unwrap_or_default();
            let shaders = self.element_shaders.get(layout_elem_idx).cloned().unwrap_or_default();

            // ── [Color] section ──
            let has_color = shared_bg_color.map_or(false, |c| c.a > 0.0);
            if has_color {
                let color_label_color = Color::rgba(243.0, 134.0, 48.0, 255.0);
                self.render_debug_view_category_header("Color", color_label_color, elem_id_string.clone());
                self.debug_open(&ElementDeclaration {
                    layout: LayoutConfig {
                        padding: attr_padding,
                        child_gap: 8,
                        layout_direction: LayoutDirection::TopToBottom,
                        ..Default::default()
                    },
                    ..Default::default()
                });
                {
                    self.debug_text("Background Color", info_title_config);
                    self.render_debug_view_color(shared_bg_color.unwrap(), info_text_config);
                }
                self.close_element();
            }

            // ── [Shape] section (Corner Radius + Shape Rotation) ──
            let has_corner_radius = shared_corner_radius.map_or(false, |cr| !cr.is_zero());
            let has_shape_rot = shape_rot.is_some();
            if has_corner_radius || has_shape_rot {
                let shape_label_color = Color::rgba(26.0, 188.0, 156.0, 255.0);
                self.render_debug_view_category_header("Shape", shape_label_color, elem_id_string.clone());
                self.debug_open(&ElementDeclaration {
                    layout: LayoutConfig {
                        padding: attr_padding,
                        child_gap: 8,
                        layout_direction: LayoutDirection::TopToBottom,
                        ..Default::default()
                    },
                    ..Default::default()
                });
                {
                    if let Some(cr) = shared_corner_radius {
                        if !cr.is_zero() {
                            self.debug_text("Corner Radius", info_title_config);
                            self.render_debug_view_corner_radius(cr, info_text_config);
                        }
                    }
                    if let Some(sr) = shape_rot {
                        self.debug_text("Shape Rotation", info_title_config);
                        self.debug_open(&ElementDeclaration::default());
                        {
                            self.debug_text("angle: ", info_text_config);
                            self.debug_float_text(sr.rotation_radians, info_text_config);
                            self.debug_text(" rad", info_text_config);
                        }
                        self.close_element();
                        self.debug_open(&ElementDeclaration::default());
                        {
                            self.debug_text("flip_x: ", info_text_config);
                            self.debug_text(if sr.flip_x { "true" } else { "false" }, info_text_config);
                            self.debug_text(", flip_y: ", info_text_config);
                            self.debug_text(if sr.flip_y { "true" } else { "false" }, info_text_config);
                        }
                        self.close_element();
                    }
                }
                self.close_element();
            }

            // ── Config-type sections (Text, Image, Floating, Clip, Border, etc.) ──
            for ci in 0..configs_len {
                let ec = self.element_configs[configs_start + ci as usize];
                match ec.config_type {
                    ElementConfigType::Shared => {} // handled above as [Color] + [Shape]
                    ElementConfigType::Text => {
                        self.render_debug_view_element_config_header(elem_id_string.clone(), ec.config_type, info_title_config);
                        let text_config = self.text_element_configs[ec.config_index].clone();
                        self.debug_open(&ElementDeclaration {
                            layout: LayoutConfig {
                                padding: attr_padding,
                                child_gap: 8,
                                layout_direction: LayoutDirection::TopToBottom,
                                ..Default::default()
                            },
                            ..Default::default()
                        });
                        {
                            self.debug_text("Font Size", info_title_config);
                            self.debug_int_text(text_config.font_size as f32, info_text_config);
                            self.debug_text("Font", info_title_config);
                            {
                                let label = if let Some(asset) = text_config.font_asset {
                                    asset.key().to_string()
                                } else {
                                    format!("default ({})", self.default_font_key)
                                };
                                self.open_text_element(&label, info_text_config);
                            }
                            self.debug_text("Line Height", info_title_config);
                            if text_config.line_height == 0 {
                                self.debug_text("auto", info_text_config);
                            } else {
                                self.debug_int_text(text_config.line_height as f32, info_text_config);
                            }
                            self.debug_text("Letter Spacing", info_title_config);
                            self.debug_int_text(text_config.letter_spacing as f32, info_text_config);
                            self.debug_text("Wrap Mode", info_title_config);
                            let wrap = match text_config.wrap_mode {
                                WrapMode::None => "NONE",
                                WrapMode::Newline => "NEWLINES",
                                _ => "WORDS",
                            };
                            self.debug_text(wrap, info_text_config);
                            self.debug_text("Text Alignment", info_title_config);
                            let align = match text_config.alignment {
                                AlignX::CenterX => "CENTER",
                                AlignX::Right => "RIGHT",
                                _ => "LEFT",
                            };
                            self.debug_text(align, info_text_config);
                            self.debug_text("Text Color", info_title_config);
                            self.render_debug_view_color(text_config.color, info_text_config);
                        }
                        self.close_element();
                    }
                    ElementConfigType::Image => {
                        let image_label_color = Color::rgba(121.0, 189.0, 154.0, 255.0);
                        self.render_debug_view_category_header("Image", image_label_color, elem_id_string.clone());
                        let image_data = self.image_element_configs[ec.config_index].clone();
                        self.debug_open(&ElementDeclaration {
                            layout: LayoutConfig {
                                padding: attr_padding,
                                child_gap: 8,
                                layout_direction: LayoutDirection::TopToBottom,
                                ..Default::default()
                            },
                            ..Default::default()
                        });
                        {
                            self.debug_text("Source", info_title_config);
                            let name = image_data.get_name();
                            self.debug_raw_text(name, info_text_config);
                        }
                        self.close_element();
                    }
                    ElementConfigType::Clip => {
                        self.render_debug_view_element_config_header(elem_id_string.clone(), ec.config_type, info_title_config);
                        let clip_config = self.clip_element_configs[ec.config_index];
                        self.debug_open(&ElementDeclaration {
                            layout: LayoutConfig {
                                padding: attr_padding,
                                child_gap: 8,
                                layout_direction: LayoutDirection::TopToBottom,
                                ..Default::default()
                            },
                            ..Default::default()
                        });
                        {
                            self.debug_text("Overflow", info_title_config);
                            self.debug_open(&ElementDeclaration::default());
                            {
                                let x_label = if clip_config.scroll_x {
                                    "SCROLL"
                                } else if clip_config.horizontal {
                                    "CLIP"
                                } else {
                                    "OVERFLOW"
                                };
                                let y_label = if clip_config.scroll_y {
                                    "SCROLL"
                                } else if clip_config.vertical {
                                    "CLIP"
                                } else {
                                    "OVERFLOW"
                                };
                                self.debug_text("{ x: ", info_text_config);
                                self.debug_text(x_label, info_text_config);
                                self.debug_text(", y: ", info_text_config);
                                self.debug_text(y_label, info_text_config);
                                self.debug_text(" }", info_text_config);
                            }
                            self.close_element();
                        }
                        self.close_element();
                    }
                    ElementConfigType::Floating => {
                        self.render_debug_view_element_config_header(elem_id_string.clone(), ec.config_type, info_title_config);
                        let float_config = self.floating_element_configs[ec.config_index];
                        self.debug_open(&ElementDeclaration {
                            layout: LayoutConfig {
                                padding: attr_padding,
                                child_gap: 8,
                                layout_direction: LayoutDirection::TopToBottom,
                                ..Default::default()
                            },
                            ..Default::default()
                        });
                        {
                            self.debug_text("Offset", info_title_config);
                            self.debug_open(&ElementDeclaration::default());
                            {
                                self.debug_text("{ x: ", info_text_config);
                                self.debug_int_text(float_config.offset.x, info_text_config);
                                self.debug_text(", y: ", info_text_config);
                                self.debug_int_text(float_config.offset.y, info_text_config);
                                self.debug_text(" }", info_text_config);
                            }
                            self.close_element();

                            self.debug_text("z-index", info_title_config);
                            self.debug_int_text(float_config.z_index as f32, info_text_config);

                            self.debug_text("Parent", info_title_config);
                            let parent_name = self.layout_element_map
                                .get(&float_config.parent_id)
                                .map(|item| item.element_id.string_id.clone())
                                .unwrap_or(StringId::empty());
                            if !parent_name.is_empty() {
                                self.debug_raw_text(parent_name.as_str(), info_text_config);
                            }

                            self.debug_text("Attach Points", info_title_config);
                            self.debug_open(&ElementDeclaration::default());
                            {
                                self.debug_text("{ element: (", info_text_config);
                                self.debug_text(Self::align_x_name(float_config.attach_points.element_x), info_text_config);
                                self.debug_text(", ", info_text_config);
                                self.debug_text(Self::align_y_name(float_config.attach_points.element_y), info_text_config);
                                self.debug_text("), parent: (", info_text_config);
                                self.debug_text(Self::align_x_name(float_config.attach_points.parent_x), info_text_config);
                                self.debug_text(", ", info_text_config);
                                self.debug_text(Self::align_y_name(float_config.attach_points.parent_y), info_text_config);
                                self.debug_text(") }", info_text_config);
                            }
                            self.close_element();

                            self.debug_text("Pointer Capture Mode", info_title_config);
                            let pcm = if float_config.pointer_capture_mode == PointerCaptureMode::Passthrough {
                                "PASSTHROUGH"
                            } else {
                                "NONE"
                            };
                            self.debug_text(pcm, info_text_config);

                            self.debug_text("Attach To", info_title_config);
                            let at = match float_config.attach_to {
                                FloatingAttachToElement::Parent => "PARENT",
                                FloatingAttachToElement::ElementWithId => "ELEMENT_WITH_ID",
                                FloatingAttachToElement::Root => "ROOT",
                                _ => "NONE",
                            };
                            self.debug_text(at, info_text_config);

                            self.debug_text("Clip To", info_title_config);
                            let ct = if float_config.clip_to == FloatingClipToElement::None {
                                "NONE"
                            } else {
                                "ATTACHED_PARENT"
                            };
                            self.debug_text(ct, info_text_config);
                        }
                        self.close_element();
                    }
                    ElementConfigType::Border => {
                        self.render_debug_view_element_config_header(elem_id_string.clone(), ec.config_type, info_title_config);
                        let border_config = self.border_element_configs[ec.config_index];
                        self.debug_open_id("Ply__DebugViewElementInfoBorderBody", &ElementDeclaration {
                            layout: LayoutConfig {
                                padding: attr_padding,
                                child_gap: 8,
                                layout_direction: LayoutDirection::TopToBottom,
                                ..Default::default()
                            },
                            ..Default::default()
                        });
                        {
                            self.debug_text("Border Widths", info_title_config);
                            self.debug_open(&ElementDeclaration::default());
                            {
                                self.debug_text("{ left: ", info_text_config);
                                self.debug_int_text(border_config.width.left as f32, info_text_config);
                                self.debug_text(", right: ", info_text_config);
                                self.debug_int_text(border_config.width.right as f32, info_text_config);
                                self.debug_text(", top: ", info_text_config);
                                self.debug_int_text(border_config.width.top as f32, info_text_config);
                                self.debug_text(", bottom: ", info_text_config);
                                self.debug_int_text(border_config.width.bottom as f32, info_text_config);
                                self.debug_text(" }", info_text_config);
                            }
                            self.close_element();
                            self.debug_text("Border Color", info_title_config);
                            self.render_debug_view_color(border_config.color, info_text_config);
                        }
                        self.close_element();
                    }
                    ElementConfigType::TextInput => {
                        // ── [Input] section for text input config ──
                        let input_label_color = Color::rgba(52.0, 152.0, 219.0, 255.0);
                        self.render_debug_view_category_header("Input", input_label_color, elem_id_string.clone());
                        let ti_cfg = self.text_input_configs[ec.config_index].clone();
                        self.debug_open(&ElementDeclaration {
                            layout: LayoutConfig {
                                padding: attr_padding,
                                child_gap: 8,
                                layout_direction: LayoutDirection::TopToBottom,
                                ..Default::default()
                            },
                            ..Default::default()
                        });
                        {
                            if !ti_cfg.placeholder.is_empty() {
                                self.debug_text("Placeholder", info_title_config);
                                self.debug_raw_text(&ti_cfg.placeholder, info_text_config);
                            }
                            self.debug_text("Max Length", info_title_config);
                            if let Some(max_len) = ti_cfg.max_length {
                                self.debug_int_text(max_len as f32, info_text_config);
                            } else {
                                self.debug_text("unlimited", info_text_config);
                            }
                            self.debug_text("Password", info_title_config);
                            self.debug_text(if ti_cfg.is_password { "true" } else { "false" }, info_text_config);
                            self.debug_text("Multiline", info_title_config);
                            self.debug_text(if ti_cfg.is_multiline { "true" } else { "false" }, info_text_config);
                            self.debug_text("Font", info_title_config);
                            self.debug_open(&ElementDeclaration::default());
                            {
                                let label = if let Some(asset) = ti_cfg.font_asset {
                                    asset.key().to_string()
                                } else {
                                    format!("default ({})", self.default_font_key)
                                };
                                self.open_text_element(&label, info_text_config);
                                self.debug_text(", size: ", info_text_config);
                                self.debug_int_text(ti_cfg.font_size as f32, info_text_config);
                            }
                            self.close_element();
                            self.debug_text("Text Color", info_title_config);
                            self.render_debug_view_color(ti_cfg.text_color, info_text_config);
                            self.debug_text("Cursor Color", info_title_config);
                            self.render_debug_view_color(ti_cfg.cursor_color, info_text_config);
                            self.debug_text("Selection Color", info_title_config);
                            self.render_debug_view_color(ti_cfg.selection_color, info_text_config);
                            // Show current text value
                            let state_data = self.text_edit_states.get(&selected_id)
                                .map(|s| (s.text.clone(), s.cursor_pos));
                            if let Some((text_val, cursor_pos)) = state_data {
                                self.debug_text("Value", info_title_config);
                                let preview = if text_val.len() > 40 {
                                    let mut end = 40;
                                    while !text_val.is_char_boundary(end) { end -= 1; }
                                    format!("\"{}...\"", &text_val[..end])
                                } else {
                                    format!("\"{}\"", &text_val)
                                };
                                self.debug_raw_text(&preview, info_text_config);
                                self.debug_text("Cursor Position", info_title_config);
                                self.debug_int_text(cursor_pos as f32, info_text_config);
                            }
                        }
                        self.close_element();
                    }
                    _ => {}
                }
            }

            // ── [Effects] section (Visual Rotation + Shaders + Effects) ──
            let has_visual_rot = visual_rot.is_some();
            let has_effects = !effects.is_empty();
            let has_shaders = !shaders.is_empty();
            if has_visual_rot || has_effects || has_shaders {
                let effects_label_color = Color::rgba(155.0, 89.0, 182.0, 255.0);
                self.render_debug_view_category_header("Effects", effects_label_color, elem_id_string.clone());
                self.debug_open(&ElementDeclaration {
                    layout: LayoutConfig {
                        padding: attr_padding,
                        child_gap: 8,
                        layout_direction: LayoutDirection::TopToBottom,
                        ..Default::default()
                    },
                    ..Default::default()
                });
                {
                    if let Some(vr) = visual_rot {
                        self.debug_text("Visual Rotation", info_title_config);
                        self.debug_open(&ElementDeclaration::default());
                        {
                            self.debug_text("angle: ", info_text_config);
                            self.debug_float_text(vr.rotation_radians, info_text_config);
                            self.debug_text(" rad", info_text_config);
                        }
                        self.close_element();
                        self.debug_open(&ElementDeclaration::default());
                        {
                            self.debug_text("pivot: (", info_text_config);
                            self.debug_float_text(vr.pivot_x, info_text_config);
                            self.debug_text(", ", info_text_config);
                            self.debug_float_text(vr.pivot_y, info_text_config);
                            self.debug_text(")", info_text_config);
                        }
                        self.close_element();
                        self.debug_open(&ElementDeclaration::default());
                        {
                            self.debug_text("flip_x: ", info_text_config);
                            self.debug_text(if vr.flip_x { "true" } else { "false" }, info_text_config);
                            self.debug_text(", flip_y: ", info_text_config);
                            self.debug_text(if vr.flip_y { "true" } else { "false" }, info_text_config);
                        }
                        self.close_element();
                    }
                    for (i, effect) in effects.iter().enumerate() {
                        let label = format!("Effect {}", i + 1);
                        self.debug_text("Effect", info_title_config);
                        self.debug_open(&ElementDeclaration::default());
                        {
                            self.debug_raw_text(&label, info_text_config);
                            self.debug_text(": ", info_text_config);
                            self.debug_raw_text(&effect.name, info_text_config);
                        }
                        self.close_element();
                        for uniform in &effect.uniforms {
                            self.debug_open(&ElementDeclaration::default());
                            {
                                self.debug_text("  ", info_text_config);
                                self.debug_raw_text(&uniform.name, info_text_config);
                                self.debug_text(": ", info_text_config);
                                self.render_debug_shader_uniform_value(&uniform.value, info_text_config);
                            }
                            self.close_element();
                        }
                    }
                    for (i, shader) in shaders.iter().enumerate() {
                        let label = format!("Shader {}", i + 1);
                        self.debug_text("Shader", info_title_config);
                        self.debug_open(&ElementDeclaration::default());
                        {
                            self.debug_raw_text(&label, info_text_config);
                            self.debug_text(": ", info_text_config);
                            self.debug_raw_text(&shader.name, info_text_config);
                        }
                        self.close_element();
                        for uniform in &shader.uniforms {
                            self.debug_open(&ElementDeclaration::default());
                            {
                                self.debug_text("  ", info_text_config);
                                self.debug_raw_text(&uniform.name, info_text_config);
                                self.debug_text(": ", info_text_config);
                                self.render_debug_shader_uniform_value(&uniform.value, info_text_config);
                            }
                            self.close_element();
                        }
                    }
                }
                self.close_element();
            }
        }
        self.close_element(); // detail panel
    }

    fn align_x_name(value: AlignX) -> &'static str {
        match value {
            AlignX::Left => "LEFT",
            AlignX::CenterX => "CENTER",
            AlignX::Right => "RIGHT",
        }
    }

    fn align_y_name(value: AlignY) -> &'static str {
        match value {
            AlignY::Top => "TOP",
            AlignY::CenterY => "CENTER",
            AlignY::Bottom => "BOTTOM",
        }
    }

    pub fn set_max_element_count(&mut self, count: i32) {
        self.max_element_count = count;
    }

    pub fn set_max_measure_text_cache_word_count(&mut self, count: i32) {
        self.max_measure_text_cache_word_count = count;
    }

    pub fn set_debug_mode_enabled(&mut self, enabled: bool) {
        self.debug_mode_enabled = enabled;
    }

    pub fn is_debug_mode_enabled(&self) -> bool {
        self.debug_mode_enabled
    }

    pub fn set_culling_enabled(&mut self, enabled: bool) {
        self.culling_disabled = !enabled;
    }

    pub fn set_measure_text_function(
        &mut self,
        f: Box<dyn Fn(&str, &TextConfig) -> Dimensions>,
    ) {
        self.measure_text_fn = Some(f);
        // Invalidate the font height cache since the measurement function changed.
        self.font_height_cache.clear();
    }
}
