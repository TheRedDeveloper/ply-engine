//! Pure Rust implementation of the Ply layout engine.
//! A UI layout engine inspired by Clay.

use std::collections::HashMap;

use crate::color::Color;
use crate::renderer::Asset;
use crate::elements::{
    FloatingAttachPointType, FloatingAttachToElement, FloatingClipToElement, PointerCaptureMode,
};
use crate::layout::{LayoutAlignmentX, LayoutAlignmentY, LayoutDirection};
use crate::math::{BoundingBox, Dimensions, Vector2};
use crate::text::{TextAlignment, TextConfig, TextElementConfigWrapMode};

// ============================================================================
// Constants
// ============================================================================

const DEFAULT_MAX_ELEMENT_COUNT: i32 = 8192;
const DEFAULT_MAX_MEASURE_TEXT_WORD_CACHE_COUNT: i32 = 16384;
const MAXFLOAT: f32 = 3.40282346638528859812e+38;
const EPSILON: f32 = 0.01;

// ============================================================================
// Enums
// ============================================================================

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
}

// ============================================================================
// Config structs (public, used by Declaration in lib.rs)
// ============================================================================

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
    pub x: LayoutAlignmentX,
    pub y: LayoutAlignmentY,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct LayoutConfig {
    pub sizing: SizingConfig,
    pub padding: PaddingConfig,
    pub child_gap: u16,
    pub child_alignment: ChildAlignmentConfig,
    pub layout_direction: LayoutDirection,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CornerRadius {
    pub top_left: f32,
    pub top_right: f32,
    pub bottom_left: f32,
    pub bottom_right: f32,
}

impl CornerRadius {
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

#[derive(Debug, Clone, Copy, Default)]
pub struct FloatingAttachPoints {
    pub element: FloatingAttachPointType,
    pub parent: FloatingAttachPointType,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct FloatingConfig {
    pub offset: Vector2,
    pub expand: Dimensions,
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
    pub image_data: Option<&'static Asset>,
    pub floating: FloatingConfig,
    pub custom_data: Option<CustomElementData>,
    pub clip: ClipConfig,
    pub border: BorderConfig,
    pub user_data: usize,
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
        }
    }
}

// ============================================================================
// ElementId
// ============================================================================

#[derive(Debug, Clone, Default)]
pub struct ElementId {
    pub id: u32,
    pub offset: u32,
    pub base_id: u32,
    pub string_id: StringId,
}

/// Owned string for debug/display purposes.
#[derive(Debug, Clone, Default)]
pub struct StringId {
    text: String,
}

impl StringId {
    pub fn from_str(s: &str) -> Self {
        Self {
            text: s.to_string(),
        }
    }

    pub fn empty() -> Self {
        Self::default()
    }

    /// Get the string content.
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// Returns true if the string is empty.
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }
}

// ============================================================================
// Internal engine types
// ============================================================================

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
    element_id: ElementId,
    layout_element_index: i32,
    on_hover_fn: Option<Box<dyn FnMut(ElementId, PointerData)>>,
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
    momentum_time: f32,
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

// ============================================================================
// Render command types
// ============================================================================

#[derive(Debug, Clone)]
pub struct InternalRenderCommand<CustomElementData: Clone + Default + std::fmt::Debug = ()> {
    pub bounding_box: BoundingBox,
    pub command_type: RenderCommandType,
    pub render_data: InternalRenderData<CustomElementData>,
    pub user_data: usize,
    pub id: u32,
    pub z_index: i16,
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
        font_id: u16,
        font_size: u16,
        letter_spacing: u16,
        line_height: u16,
    },
    Image {
        background_color: Color,
        corner_radius: CornerRadius,
        image_data: &'static Asset,
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
        }
    }
}

// ============================================================================
// Scroll container data (public)
// ============================================================================

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

// ============================================================================
// PlyContext - the main layout engine context
// ============================================================================

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
    image_element_configs: Vec<&'static Asset>,
    floating_element_configs: Vec<FloatingConfig>,
    clip_element_configs: Vec<ClipConfig>,
    custom_element_configs: Vec<CustomElementData>,
    border_element_configs: Vec<BorderConfig>,
    shared_element_configs: Vec<SharedElementConfig>,

    // String IDs for debug
    layout_element_id_strings: Vec<StringId>,

    // Text wrapping
    wrapped_text_lines: Vec<WrappedTextLine>,

    // Tree traversal
    tree_node_array: Vec<LayoutElementTreeNode>,
    layout_element_tree_roots: Vec<LayoutElementTreeRoot>,

    // Layout element map: element id -> element data (bounding box, hover callback, etc.)
    layout_element_map: HashMap<u32, LayoutElementHashMapItem>,

    // Text measurement cache: content hash -> measured dimensions and words
    measure_text_cache: HashMap<u32, MeasureTextCacheItem>,
    measured_words: Vec<MeasuredWord>,
    measured_words_free_list: Vec<i32>,

    // Clip/scroll
    open_clip_element_stack: Vec<i32>,
    pointer_over_ids: Vec<ElementId>,
    scroll_container_datas: Vec<ScrollContainerDataInternal>,

    // Visited flags for DFS
    tree_node_visited: Vec<bool>,

    // Dynamic string data (for int-to-string etc.)
    dynamic_string_data: Vec<u8>,

    // Debug view: heap-allocated strings that survive the frame
}

// ============================================================================
// Hash functions
// ============================================================================

fn hash_data_scalar(data: &[u8]) -> u64 {
    let mut hash: u64 = 0;
    for &b in data {
        hash = hash.wrapping_add(b as u64);
        hash = hash.wrapping_add(hash << 10);
        hash ^= hash >> 6;
    }
    hash
}

pub fn hash_string(key: &str, seed: u32) -> ElementId {
    let mut hash: u32 = seed;
    for b in key.bytes() {
        hash = hash.wrapping_add(b as u32);
        hash = hash.wrapping_add(hash << 10);
        hash ^= hash >> 6;
    }
    hash = hash.wrapping_add(hash << 3);
    hash ^= hash >> 11;
    hash = hash.wrapping_add(hash << 15);
    ElementId {
        id: hash.wrapping_add(1),
        offset: 0,
        base_id: hash.wrapping_add(1),
        string_id: StringId::from_str(key),
    }
}

pub fn hash_string_with_offset(key: &str, offset: u32, seed: u32) -> ElementId {
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
    ElementId {
        id: hash.wrapping_add(1),
        offset,
        base_id: base.wrapping_add(1),
        string_id: StringId::from_str(key),
    }
}

fn hash_number(offset: u32, seed: u32) -> ElementId {
    let mut hash = seed;
    hash = hash.wrapping_add(offset.wrapping_add(48));
    hash = hash.wrapping_add(hash << 10);
    hash ^= hash >> 6;
    hash = hash.wrapping_add(hash << 3);
    hash ^= hash >> 11;
    hash = hash.wrapping_add(hash << 15);
    ElementId {
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
    hash = hash.wrapping_add(config.font_id as u32);
    hash = hash.wrapping_add(hash << 10);
    hash ^= hash >> 6;
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

// ============================================================================
// Helper functions
// ============================================================================

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

// ============================================================================
// PlyContext implementation
// ============================================================================

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
            layout_element_id_strings: Vec::new(),
            wrapped_text_lines: Vec::new(),
            tree_node_array: Vec::new(),
            layout_element_tree_roots: Vec::new(),
            layout_element_map: HashMap::new(),
            measure_text_cache: HashMap::new(),
            measured_words: Vec::new(),
            measured_words_free_list: Vec::new(),
            open_clip_element_stack: Vec::new(),
            pointer_over_ids: Vec::new(),
            scroll_container_datas: Vec::new(),
            tree_node_visited: Vec::new(),
            dynamic_string_data: Vec::new(),
        };
        ctx
    }

    // ========================================================================
    // Internal helpers
    // ========================================================================

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
        element_id: &ElementId,
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
                    collision: false,
                    collapsed: false,
                });
            }
        }
    }

    fn generate_id_for_anonymous_element(&mut self, open_element_index: usize) -> ElementId {
        let stack_len = self.open_layout_element_stack.len();
        let parent_idx = self.open_layout_element_stack[stack_len - 2] as usize;
        let parent = &self.layout_elements[parent_idx];
        let offset =
            parent.children_length as u32 + parent.floating_children_count as u32;
        let parent_id = parent.id;
        let element_id = hash_number(offset, parent_id);
        self.layout_elements[open_element_index].id = element_id.id;
        self.add_hash_map_item(&element_id, open_element_index as i32);
        self.layout_element_id_strings.push(element_id.string_id.clone());
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

    // ========================================================================
    // Store config functions
    // ========================================================================

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

    // ========================================================================
    // Element open / close / configure
    // ========================================================================

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

    pub fn open_element_with_id(&mut self, element_id: &ElementId) {
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
        self.layout_element_id_strings.push(element_id.string_id.clone());

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
        if let Some(image_data) = declaration.image_data {
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
            for scd in &self.scroll_container_datas {
                if scd.element_id == elem_id {
                    clip.child_offset = scd.scroll_position;
                    break;
                }
            }

            self.clip_element_configs.push(clip);
            let idx = self.clip_element_configs.len() - 1;
            self.attach_element_config(ElementConfigType::Clip, idx);

            self.open_clip_element_stack.push(elem_id as i32);

            // Track scroll container
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

        // Border config
        if !declaration.border.width.is_zero() {
            self.border_element_configs.push(declaration.border);
            let idx = self.border_element_configs.len() - 1;
            self.attach_element_config(ElementConfigType::Border, idx);
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

    // ========================================================================
    // Text elements
    // ========================================================================

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
        let text_config = self.text_element_configs[text_config_index];
        let text_measured =
            self.measure_text_cached(text, &text_config);

        let element_id = hash_number(parent_children_count as u32, parent_id);
        self.layout_elements[text_elem_idx as usize].id = element_id.id;
        self.add_hash_map_item(&element_id, text_elem_idx);
        self.layout_element_id_strings.push(element_id.string_id);

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

    // ========================================================================
    // Text measurement cache
    // ========================================================================

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

    // ========================================================================
    // Begin / End layout
    // ========================================================================

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
    }

    // ========================================================================
    // Layout algorithm
    // ========================================================================

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
                                == TextElementConfigWrapMode::Words
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
            let text_config = self.text_element_configs[text_config_idx];

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
                let space_config = text_config;
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
                        match config.attach_points.parent {
                            FloatingAttachPointType::LeftTop
                            | FloatingAttachPointType::LeftCenter
                            | FloatingAttachPointType::LeftBottom => {
                                target.x = parent_bbox.x;
                            }
                            FloatingAttachPointType::CenterTop
                            | FloatingAttachPointType::CenterCenter
                            | FloatingAttachPointType::CenterBottom => {
                                target.x = parent_bbox.x + parent_bbox.width / 2.0;
                            }
                            FloatingAttachPointType::RightTop
                            | FloatingAttachPointType::RightCenter
                            | FloatingAttachPointType::RightBottom => {
                                target.x = parent_bbox.x + parent_bbox.width;
                            }
                        }
                        // X position - element attach point
                        match config.attach_points.element {
                            FloatingAttachPointType::CenterTop
                            | FloatingAttachPointType::CenterCenter
                            | FloatingAttachPointType::CenterBottom => {
                                target.x -= root_dims.width / 2.0;
                            }
                            FloatingAttachPointType::RightTop
                            | FloatingAttachPointType::RightCenter
                            | FloatingAttachPointType::RightBottom => {
                                target.x -= root_dims.width;
                            }
                            _ => {}
                        }
                        // Y position - parent attach point
                        match config.attach_points.parent {
                            FloatingAttachPointType::LeftTop
                            | FloatingAttachPointType::RightTop
                            | FloatingAttachPointType::CenterTop => {
                                target.y = parent_bbox.y;
                            }
                            FloatingAttachPointType::LeftCenter
                            | FloatingAttachPointType::CenterCenter
                            | FloatingAttachPointType::RightCenter => {
                                target.y = parent_bbox.y + parent_bbox.height / 2.0;
                            }
                            FloatingAttachPointType::LeftBottom
                            | FloatingAttachPointType::CenterBottom
                            | FloatingAttachPointType::RightBottom => {
                                target.y = parent_bbox.y + parent_bbox.height;
                            }
                        }
                        // Y position - element attach point
                        match config.attach_points.element {
                            FloatingAttachPointType::LeftCenter
                            | FloatingAttachPointType::CenterCenter
                            | FloatingAttachPointType::RightCenter => {
                                target.y -= root_dims.height / 2.0;
                            }
                            FloatingAttachPointType::LeftBottom
                            | FloatingAttachPointType::CenterBottom
                            | FloatingAttachPointType::RightBottom => {
                                target.y -= root_dims.height;
                            }
                            _ => {}
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

                    let mut current_bbox = BoundingBox::new(
                        current_node.position.x,
                        current_node.position.y,
                        self.layout_elements[current_elem_idx].dimensions.width,
                        self.layout_elements[current_elem_idx].dimensions.height,
                    );

                    // Expand for floating elements
                    if self.element_has_config(current_elem_idx, ElementConfigType::Floating) {
                        if let Some(float_cfg_idx) = self.find_element_config_index(
                            current_elem_idx,
                            ElementConfigType::Floating,
                        ) {
                            let expand = self.floating_element_configs[float_cfg_idx].expand;
                            current_bbox.x -= expand.width;
                            current_bbox.width += expand.width * 2.0;
                            current_bbox.y -= expand.height;
                            current_bbox.height += expand.height * 2.0;
                        }
                    }

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
                                    });
                                }
                            }
                            ElementConfigType::Image => {
                                if should_render {
                                    let image_data =
                                        self.image_element_configs[config.config_index];
                                    self.add_render_command(InternalRenderCommand {
                                        bounding_box: current_bbox,
                                        command_type: RenderCommandType::Image,
                                        render_data: InternalRenderData::Image {
                                            background_color: shared.background_color,
                                            corner_radius: shared.corner_radius,
                                            image_data,
                                        },
                                        user_data: shared.user_data,
                                        id: elem_id,
                                        z_index: root.z_index,
                                    });
                                }
                                emit_rectangle = false;
                            }
                            ElementConfigType::Text => {
                                if !should_render {
                                    continue;
                                }
                                let text_config =
                                    self.text_element_configs[config.config_index];
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
                                    if text_config.alignment == TextAlignment::Left {
                                        offset = 0.0;
                                    }
                                    if text_config.alignment == TextAlignment::Center {
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
                                            font_id: text_config.font_id,
                                            font_size: text_config.font_size,
                                            letter_spacing: text_config.letter_spacing,
                                            line_height: text_config.line_height,
                                        },
                                        user_data: text_config.user_data,
                                        id: hash_number(line_index as u32, elem_id).id,
                                        z_index: root.z_index,
                                    });
                                    y_position += final_line_height;
                                }
                            }
                            ElementConfigType::Custom => {
                                if should_render {
                                    let custom_data =
                                        self.custom_element_configs[config.config_index].clone();
                                    self.add_render_command(InternalRenderCommand {
                                        bounding_box: current_bbox,
                                        command_type: RenderCommandType::Custom,
                                        render_data: InternalRenderData::Custom {
                                            background_color: shared.background_color,
                                            corner_radius: shared.corner_radius,
                                            custom_data,
                                        },
                                        user_data: shared.user_data,
                                        id: elem_id,
                                        z_index: root.z_index,
                                    });
                                }
                                emit_rectangle = false;
                            }
                        }
                    }

                    if emit_rectangle {
                        self.add_render_command(InternalRenderCommand {
                            bounding_box: current_bbox,
                            command_type: RenderCommandType::Rectangle,
                            render_data: InternalRenderData::Rectangle {
                                background_color: shared.background_color,
                                corner_radius: shared.corner_radius,
                            },
                            user_data: shared.user_data,
                            id: elem_id,
                            z_index: root.z_index,
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
                                LayoutAlignmentX::Left => extra_space = 0.0,
                                LayoutAlignmentX::Center => extra_space /= 2.0,
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
                                LayoutAlignmentY::Top => extra_space = 0.0,
                                LayoutAlignmentY::Center => extra_space /= 2.0,
                                _ => {}
                            }
                            dfs_buffer[buf_idx].next_child_offset.y += extra_space;
                        }

                        // Update scroll container content size
                        if let Some(si) = _scroll_container_data_idx {
                            let content_w: f32 = (0..children_length)
                                .map(|ci| {
                                    let idx = self.layout_element_children[children_start + ci]
                                        as usize;
                                    self.layout_elements[idx].dimensions.width
                                })
                                .sum::<f32>()
                                + (layout_config.padding.left + layout_config.padding.right) as f32;
                            let content_h: f32 = (0..children_length)
                                .map(|ci| {
                                    let idx = self.layout_element_children[children_start + ci]
                                        as usize;
                                    self.layout_elements[idx].dimensions.height
                                })
                                .sum::<f32>()
                                + (layout_config.padding.top + layout_config.padding.bottom) as f32;
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
                                LayoutAlignmentY::Top => {}
                                LayoutAlignmentY::Center => {
                                    child_offset.y += whitespace / 2.0;
                                }
                                LayoutAlignmentY::Bottom => {
                                    child_offset.y += whitespace;
                                }
                            }
                        } else {
                            child_offset.x = layout_config.padding.left as f32;
                            let whitespace = self.layout_elements[current_elem_idx].dimensions.width
                                - (layout_config.padding.left + layout_config.padding.right) as f32
                                - self.layout_elements[child_idx].dimensions.width;
                            match layout_config.child_alignment.x {
                                LayoutAlignmentX::Left => {}
                                LayoutAlignmentX::Center => {
                                    child_offset.x += whitespace / 2.0;
                                }
                                LayoutAlignmentX::Right => {
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
    }

    // ========================================================================
    // Public API
    // ========================================================================

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
    }

    pub fn update_scroll_containers(
        &mut self,
        _enable_drag_scrolling: bool,
        scroll_delta: Vector2,
        _delta_time: f32,
    ) {
        let pointer = self.pointer_info.position;

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

        // Apply scroll delta to the deepest scroll container the pointer is over
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
                // macroquad mouse_wheel: positive y = scroll up → content moves down → scroll_position.y should decrease
                scd.scroll_position.y += scroll_delta.y;
                scd.scroll_position.x += scroll_delta.x;

                // Clamp: scroll_position should be <= 0 (scrolling "up" means offset becomes more negative)
                let max_scroll_y =
                    -(scd.content_size.height - scd.bounding_box.height).max(0.0);
                let max_scroll_x =
                    -(scd.content_size.width - scd.bounding_box.width).max(0.0);
                scd.scroll_position.y = scd.scroll_position.y.clamp(max_scroll_y, 0.0);
                scd.scroll_position.x = scd.scroll_position.x.clamp(max_scroll_x, 0.0);
            }
        }
    }

    pub fn hovered(&self) -> bool {
        let open_idx = self.get_open_layout_element();
        let elem_id = self.layout_elements[open_idx].id;
        self.pointer_over_ids.iter().any(|eid| eid.id == elem_id)
    }

    pub fn on_hover(&mut self, callback: Box<dyn FnMut(ElementId, PointerData)>) {
        let open_idx = self.get_open_layout_element();
        let elem_id = self.layout_elements[open_idx].id;
        if let Some(item) = self.layout_element_map.get_mut(&elem_id) {
            item.on_hover_fn = Some(callback);
        }
    }

    pub fn pointer_over(&self, element_id: ElementId) -> bool {
        self.pointer_over_ids.iter().any(|eid| eid.id == element_id.id)
    }

    pub fn get_pointer_over_ids(&self) -> &[ElementId] {
        &self.pointer_over_ids
    }

    pub fn get_element_data(&self, id: ElementId) -> Option<BoundingBox> {
        self.layout_element_map
            .get(&id.id)
            .map(|item| item.bounding_box)
    }

    pub fn get_scroll_container_data(&self, id: ElementId) -> ScrollContainerData {
        for scd in &self.scroll_container_datas {
            if scd.element_id == id.id {
                return ScrollContainerData {
                    scroll_position: scd.scroll_position,
                    scroll_container_dimensions: Dimensions::new(
                        scd.bounding_box.width,
                        scd.bounding_box.height,
                    ),
                    content_dimensions: scd.content_size,
                    horizontal: false, // TODO
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

    // ========================================================================
    // Debug View
    // ========================================================================

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
            ElementConfigType::Clip => ("Scroll", Color::rgba(242.0, 196.0, 90.0, 255.0)),
            ElementConfigType::Border => ("Border", Color::rgba(108.0, 91.0, 123.0, 255.0)),
            ElementConfigType::Custom => ("Custom", Color::rgba(11.0, 72.0, 107.0, 255.0)),
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
        _element_id_string: StringId,
        config_type: ElementConfigType,
        _info_title_config: usize,
    ) {
        let (label, label_color) = Self::debug_get_config_type_label(config_type);
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
                child_alignment: ChildAlignmentConfig { x: LayoutAlignmentX::Left, y: LayoutAlignmentY::Center },
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
                self.debug_text(label, tc);
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
                wrap_mode: TextElementConfigWrapMode::None,
                ..Default::default()
            });
            if !_element_id_string.is_empty() {
                self.debug_raw_text(_element_id_string.as_str(), tc);
            }
        }
        self.close_element();
    }

    /// Render a color value in the debug view.
    fn render_debug_view_color(&mut self, color: Color, config_index: usize) {
        self.debug_open(&ElementDeclaration {
            layout: LayoutConfig {
                child_alignment: ChildAlignmentConfig { x: LayoutAlignmentX::Left, y: LayoutAlignmentY::Center },
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
                child_alignment: ChildAlignmentConfig { x: LayoutAlignmentX::Left, y: LayoutAlignmentY::Center },
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
            child_alignment: ChildAlignmentConfig { x: LayoutAlignmentX::Left, y: LayoutAlignmentY::Center },
            ..Default::default()
        };

        let name_text_config = TextConfig {
            color: Self::DEBUG_COLOR_4,
            font_size: 16,
            wrap_mode: TextElementConfigWrapMode::None,
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
                        self.debug_selected_element_id = elem_id;
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
                self.debug_open_idi("Ply__DebugView_ElementOuter", current_elem_id, &ElementDeclaration {
                    layout: scroll_item_layout,
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
                                child_alignment: ChildAlignmentConfig { x: LayoutAlignmentX::Center, y: LayoutAlignmentY::Center },
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
                                child_alignment: ChildAlignmentConfig { x: LayoutAlignmentX::Center, y: LayoutAlignmentY::Center },
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
                            self.store_text_element_config(name_text_config)
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
                        self.store_text_element_config(name_text_config)
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
                            child_alignment: ChildAlignmentConfig { x: LayoutAlignmentX::Left, y: LayoutAlignmentY::Center },
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

        // Render highlight on hovered element
        if highlighted_element_id != 0 {
            self.debug_open_id("Ply__DebugView_ElementHighlight", &ElementDeclaration {
                layout: LayoutConfig {
                    sizing: SizingConfig {
                        width: SizingAxis { type_: SizingType::Grow, ..Default::default() },
                        height: SizingAxis { type_: SizingType::Grow, ..Default::default() },
                    },
                    ..Default::default()
                },
                floating: FloatingConfig {
                    parent_id: highlighted_element_id,
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
            wrap_mode: TextElementConfigWrapMode::None,
            ..Default::default()
        });
        let info_title_config = self.store_text_element_config(TextConfig {
            color: Self::DEBUG_COLOR_3,
            font_size: 16,
            wrap_mode: TextElementConfigWrapMode::None,
            ..Default::default()
        });

        // Determine scroll offset for the debug scroll pane
        let scroll_id = hash_string("Ply__DebugViewOuterScrollPane", 0);
        let mut scroll_y_offset: f32 = 0.0;
        let mut pointer_in_debug_view = self.pointer_info.position.y < self.layout_dimensions.height - 300.0;
        for scd in &self.scroll_container_datas {
            if scd.element_id == scroll_id.id {
                if !self.external_scroll_handling_enabled {
                    scroll_y_offset = scd.scroll_position.y;
                } else {
                    pointer_in_debug_view = self.pointer_info.position.y + scd.scroll_position.y
                        < self.layout_dimensions.height - 300.0;
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
                    element: FloatingAttachPointType::RightCenter,
                    parent: FloatingAttachPointType::RightCenter,
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
                    child_alignment: ChildAlignmentConfig { x: LayoutAlignmentX::Left, y: LayoutAlignmentY::Center },
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
                self.debug_open(&ElementDeclaration {
                    layout: LayoutConfig {
                        sizing: SizingConfig {
                            width: SizingAxis { type_: SizingType::Fixed, min_max: SizingMinMax { min: close_size, max: close_size }, ..Default::default() },
                            height: SizingAxis { type_: SizingType::Fixed, min_max: SizingMinMax { min: close_size, max: close_size }, ..Default::default() },
                        },
                        child_alignment: ChildAlignmentConfig { x: LayoutAlignmentX::Center, y: LayoutAlignmentY::Center },
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
                self.debug_open(&ElementDeclaration {
                    layout: LayoutConfig {
                        sizing: SizingConfig {
                            width: SizingAxis { type_: SizingType::Grow, ..Default::default() },
                            height: SizingAxis { type_: SizingType::Grow, ..Default::default() },
                        },
                        layout_direction: LayoutDirection::TopToBottom,
                        ..Default::default()
                    },
                    background_color: alt_bg,
                    ..Default::default()
                });
                {
                    // Floating element list overlay
                    let panel_contents_id = hash_string("Ply__DebugViewPaneOuter", 0);
                    let panel_contents_id_num = panel_contents_id.id;
                    self.open_element_with_id(&panel_contents_id);
                    self.configure_open_element(&ElementDeclaration {
                        layout: LayoutConfig {
                            sizing: SizingConfig {
                                width: SizingAxis { type_: SizingType::Grow, ..Default::default() },
                                height: SizingAxis { type_: SizingType::Grow, ..Default::default() },
                            },
                            ..Default::default()
                        },
                        floating: FloatingConfig {
                            z_index: 32766,
                            pointer_capture_mode: PointerCaptureMode::Passthrough,
                            attach_to: FloatingAttachToElement::Parent,
                            clip_to: FloatingClipToElement::AttachedParent,
                            ..Default::default()
                        },
                        ..Default::default()
                    });
                    {
                        self.debug_open(&ElementDeclaration {
                            layout: LayoutConfig {
                                sizing: SizingConfig {
                                    width: SizingAxis { type_: SizingType::Grow, ..Default::default() },
                                    height: SizingAxis { type_: SizingType::Grow, ..Default::default() },
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
                            ..Default::default()
                        });
                        {
                            let _layout_data = self.render_debug_layout_elements_list(
                                initial_roots_length,
                                highlighted_row,
                            );

                            // Row backgrounds (behind the floating element list)
                            // We need to close the float containers first
                            // Actually the C code does this after closing the float
                            // Let me replicate the structure: close the inner padding container
                            self.close_element(); // inner padding
                        }
                        self.close_element(); // panel_contents_id (floating)

                        // Now render row backgrounds
                        // Get content width from the panel
                        let content_width = self.layout_element_map
                            .get(&panel_contents_id_num)
                            .and_then(|item| {
                                let idx = item.layout_element_index as usize;
                                if idx < self.layout_elements.len() {
                                    Some(self.layout_elements[idx].dimensions.width)
                                } else {
                                    None
                                }
                            })
                            .unwrap_or(debug_width);

                        // Column spacer with content width
                        self.debug_open(&ElementDeclaration {
                            layout: LayoutConfig {
                                sizing: SizingConfig {
                                    width: SizingAxis {
                                        type_: SizingType::Fixed,
                                        min_max: SizingMinMax { min: content_width, max: content_width },
                                        ..Default::default()
                                    },
                                    ..Default::default()
                                },
                                layout_direction: LayoutDirection::TopToBottom,
                                ..Default::default()
                            },
                            ..Default::default()
                        });
                        self.close_element();

                        // Render row color backgrounds
                        // We need layout_data but it was in a nested scope. Let me restructure.
                        // For simplicity, re-derive from the stored state.
                    }
                    self.close_element(); // alt_bg container
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
            self.debug_open(&ElementDeclaration {
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
                    child_alignment: ChildAlignmentConfig { x: LayoutAlignmentX::Left, y: LayoutAlignmentY::Center },
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
                    let align_x = match layout_config.child_alignment.x {
                        LayoutAlignmentX::Center => "CENTER",
                        LayoutAlignmentX::Right => "RIGHT",
                        _ => "LEFT",
                    };
                    self.debug_text(align_x, info_text_config);
                    self.debug_text(", y: ", info_text_config);
                    let align_y = match layout_config.child_alignment.y {
                        LayoutAlignmentY::Center => "CENTER",
                        LayoutAlignmentY::Bottom => "BOTTOM",
                        _ => "TOP",
                    };
                    self.debug_text(align_y, info_text_config);
                    self.debug_text(" }", info_text_config);
                }
                self.close_element();
            }
            self.close_element(); // layout config details

            // Per-config type detail sections
            let configs_start = self.layout_elements[layout_elem_idx].element_configs.start;
            let configs_len = self.layout_elements[layout_elem_idx].element_configs.length;
            for ci in 0..configs_len {
                let ec = self.element_configs[configs_start + ci as usize];
                let elem_id_string = selected_item.element_id.string_id.clone();
                self.render_debug_view_element_config_header(elem_id_string, ec.config_type, info_title_config);

                match ec.config_type {
                    ElementConfigType::Shared => {
                        let shared = self.shared_element_configs[ec.config_index];
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
                            self.render_debug_view_color(shared.background_color, info_text_config);
                            self.debug_text("Corner Radius", info_title_config);
                            self.render_debug_view_corner_radius(shared.corner_radius, info_text_config);
                        }
                        self.close_element();
                    }
                    ElementConfigType::Text => {
                        let text_config = self.text_element_configs[ec.config_index];
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
                            self.debug_text("Font ID", info_title_config);
                            self.debug_int_text(text_config.font_id as f32, info_text_config);
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
                                TextElementConfigWrapMode::None => "NONE",
                                TextElementConfigWrapMode::Newline => "NEWLINES",
                                _ => "WORDS",
                            };
                            self.debug_text(wrap, info_text_config);
                            self.debug_text("Text Alignment", info_title_config);
                            let align = match text_config.alignment {
                                TextAlignment::Center => "CENTER",
                                TextAlignment::Right => "RIGHT",
                                _ => "LEFT",
                            };
                            self.debug_text(align, info_text_config);
                            self.debug_text("Text Color", info_title_config);
                            self.render_debug_view_color(text_config.color, info_text_config);
                        }
                        self.close_element();
                    }
                    ElementConfigType::Clip => {
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
                            self.debug_text("Vertical", info_title_config);
                            self.debug_text(if clip_config.vertical { "true" } else { "false" }, info_text_config);
                            self.debug_text("Horizontal", info_title_config);
                            self.debug_text(if clip_config.horizontal { "true" } else { "false" }, info_text_config);
                        }
                        self.close_element();
                    }
                    ElementConfigType::Floating => {
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

                            self.debug_text("Expand", info_title_config);
                            self.debug_open(&ElementDeclaration::default());
                            {
                                self.debug_text("{ width: ", info_text_config);
                                self.debug_int_text(float_config.expand.width, info_text_config);
                                self.debug_text(", height: ", info_text_config);
                                self.debug_int_text(float_config.expand.height, info_text_config);
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
                                self.debug_text("{ element: ", info_text_config);
                                let elem_ap = Self::attach_point_name(float_config.attach_points.element);
                                self.debug_text(elem_ap, info_text_config);
                                self.debug_text(", parent: ", info_text_config);
                                let parent_ap = Self::attach_point_name(float_config.attach_points.parent);
                                self.debug_text(parent_ap, info_text_config);
                                self.debug_text(" }", info_text_config);
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
                    _ => {}
                }
            }
        }
        self.close_element(); // detail panel
    }

    fn attach_point_name(value: FloatingAttachPointType) -> &'static str {
        match value {
            FloatingAttachPointType::LeftTop => "LEFT_TOP",
            FloatingAttachPointType::LeftCenter => "LEFT_CENTER",
            FloatingAttachPointType::LeftBottom => "LEFT_BOTTOM",
            FloatingAttachPointType::CenterTop => "CENTER_TOP",
            FloatingAttachPointType::CenterCenter => "CENTER_CENTER",
            FloatingAttachPointType::CenterBottom => "CENTER_BOTTOM",
            FloatingAttachPointType::RightTop => "RIGHT_TOP",
            FloatingAttachPointType::RightCenter => "RIGHT_CENTER",
            FloatingAttachPointType::RightBottom => "RIGHT_BOTTOM",
        }
    }

    // ========================================================================
    // Public settings
    // ========================================================================

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
    }
}
