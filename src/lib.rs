pub mod accessibility;
#[cfg(target_arch = "wasm32")]
pub mod accessibility_web;
#[cfg(all(feature = "native-a11y", not(target_arch = "wasm32")))]
pub mod accessibility_native;
pub mod align;
pub mod color;
pub mod elements;
pub mod engine;
pub mod errors;
pub mod id;
pub mod layout;
pub mod math;
pub mod render_commands;
pub mod shader_build;
pub mod shaders;
pub mod text;
pub mod text_input;
pub mod renderer;
#[cfg(feature = "text-styling")]
pub mod text_styling;
#[cfg(feature = "built-in-shaders")]
pub mod built_in_shaders;
pub mod prelude;

use id::Id;
use math::{Dimensions, Vector2};
use render_commands::RenderCommand;
use text::TextConfig;

pub use color::Color;

#[allow(dead_code)]
pub struct Ply<CustomElementData: Clone + Default + std::fmt::Debug = ()> {
    context: engine::PlyContext<CustomElementData>,
    headless: bool,
    fonts: Vec<macroquad::prelude::Font>,
    /// Key repeat tracking for text input control keys
    text_input_repeat_key: u32,
    text_input_repeat_first: f64,
    text_input_repeat_last: f64,
    /// Which element was focused when the current repeat started.
    /// Used to clear stale repeat state on focus change.
    text_input_repeat_focus_id: u32,
    /// Track virtual keyboard state to avoid redundant show/hide calls
    was_text_input_focused: bool,
    #[cfg(target_arch = "wasm32")]
    web_a11y_state: accessibility_web::WebAccessibilityState,
    #[cfg(all(feature = "native-a11y", not(target_arch = "wasm32")))]
    native_a11y_state: accessibility_native::NativeAccessibilityState,
}

pub struct Ui<'ply, CustomElementData: Clone + Default + std::fmt::Debug = ()> {
    ply: &'ply mut Ply<CustomElementData>,
}

/// Builder for creating elements with closure-based syntax.
/// Methods return `self` by value for chaining. Finalize with `.children()` or `.empty()`.
pub struct ElementBuilder<'ply, CustomElementData: Clone + Default + std::fmt::Debug = ()> {
    ply: &'ply mut Ply<CustomElementData>,
    inner: engine::ElementDeclaration<CustomElementData>,
    id: Option<Id>,
    on_press_fn: Option<Box<dyn FnMut(Id) + 'static>>,
    on_release_fn: Option<Box<dyn FnMut(Id) + 'static>>,
    on_focus_fn: Option<Box<dyn FnMut(Id) + 'static>>,
    on_unfocus_fn: Option<Box<dyn FnMut(Id) + 'static>>,
    text_input_on_changed_fn: Option<Box<dyn FnMut(&str) + 'static>>,
    text_input_on_submit_fn: Option<Box<dyn FnMut(&str) + 'static>>,
}

impl<'ply, CustomElementData: Clone + Default + std::fmt::Debug>
    ElementBuilder<'ply, CustomElementData>
{
    /// Sets the width of the element.
    #[inline]
    pub fn width(mut self, width: layout::Sizing) -> Self {
        self.inner.layout.sizing.width = width.into();
        self
    }

    /// Sets the height of the element.
    #[inline]
    pub fn height(mut self, height: layout::Sizing) -> Self {
        self.inner.layout.sizing.height = height.into();
        self
    }

    /// Sets the background color of the element.
    #[inline]
    pub fn background_color(mut self, color: impl Into<Color>) -> Self {
        self.inner.background_color = color.into();
        self
    }

    /// Sets the corner radius.
    /// Accepts `f32` (all corners) or `(f32, f32, f32, f32)` in CSS order (top-left, top-right, bottom-right, bottom-left).
    #[inline]
    pub fn corner_radius(mut self, radius: impl Into<engine::CornerRadius>) -> Self {
        self.inner.corner_radius = radius.into();
        self
    }

    /// Sets the element's ID.
    ///
    /// Accepts an `Id` or a `&'static str` label.
    #[inline]
    pub fn id(mut self, id: impl Into<Id>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Sets the aspect ratio of the element.
    #[inline]
    pub fn aspect_ratio(mut self, aspect_ratio: f32) -> Self {
        self.inner.aspect_ratio = aspect_ratio;
        self
    }

    /// Sets clipping on the element.
    #[inline]
    pub fn clip(mut self, horizontal: bool, vertical: bool) -> Self {
        self.inner.clip.horizontal = horizontal;
        self.inner.clip.vertical = vertical;
        self
    }

    /// Sets custom element data.
    #[inline]
    pub fn custom_element(mut self, data: CustomElementData) -> Self {
        self.inner.custom_data = Some(data);
        self
    }

    /// Configures layout properties using a closure.
    #[inline]
    pub fn layout(mut self, f: impl for<'a> FnOnce(&'a mut layout::LayoutBuilder) -> &'a mut layout::LayoutBuilder) -> Self {
        let mut builder = layout::LayoutBuilder { config: self.inner.layout };
        f(&mut builder);
        self.inner.layout = builder.config;
        self
    }

    /// Configures floating properties using a closure.
    #[inline]
    pub fn floating(mut self, f: impl for<'a> FnOnce(&'a mut elements::FloatingBuilder) -> &'a mut elements::FloatingBuilder) -> Self {
        let mut builder = elements::FloatingBuilder { config: self.inner.floating };
        f(&mut builder);
        self.inner.floating = builder.config;
        self
    }

    /// Configures border properties using a closure.
    #[inline]
    pub fn border(mut self, f: impl for<'a> FnOnce(&'a mut elements::BorderBuilder) -> &'a mut elements::BorderBuilder) -> Self {
        let mut builder = elements::BorderBuilder { config: self.inner.border };
        f(&mut builder);
        self.inner.border = builder.config;
        self
    }

    /// Sets the image data for this element.
    ///
    /// Accepts anything that implements `Into<ImageSource>`:
    /// - `&'static GraphicAsset` — static file path or embedded bytes
    /// - `Texture2D` — pre-existing GPU texture handle
    /// - `tinyvg::format::Image` — procedural TinyVG scene graph (requires `tinyvg` feature)
    #[inline]
    pub fn image(mut self, data: impl Into<renderer::ImageSource>) -> Self {
        self.inner.image_data = Some(data.into());
        self
    }

    /// Adds a per-element shader effect using a closure-based uniform API.
    ///
    /// The shader modifies the fragment output of the element's draw call directly.
    /// Multiple `.effect()` calls are supported — each adds to the effects list.
    ///
    /// # Example
    /// ```rust,ignore
    /// ui.element()
    ///     .effect(&MY_SHADER, |s| s
    ///         .uniform("time", time)
    ///         .uniform("intensity", 0.5f32)
    ///     )
    ///     .empty();
    /// ```
    #[inline]
    pub fn effect(mut self, asset: &shaders::ShaderAsset, f: impl FnOnce(&mut shaders::ShaderBuilder<'_>)) -> Self {
        let mut builder = shaders::ShaderBuilder::new(asset);
        f(&mut builder);
        self.inner.effects.push(builder.into_config());
        self
    }

    /// Adds a group shader that captures all children to an offscreen buffer,
    /// then applies a fragment shader as a post-process.
    ///
    /// Multiple `.shader()` calls are supported — each adds a nesting level.
    /// The first shader is applied innermost (directly to children), subsequent
    /// shaders wrap earlier ones.
    ///
    /// # Example
    /// ```rust,ignore
    /// ui.element()
    ///     .shader(&FOIL_EFFECT, |s| s
    ///         .uniform("time", time)
    ///         .uniform("seed", card_seed)
    ///     )
    ///     .children(|ui| {
    ///         // All children captured to offscreen buffer
    ///     });
    /// ```
    #[inline]
    pub fn shader(mut self, asset: &shaders::ShaderAsset, f: impl FnOnce(&mut shaders::ShaderBuilder<'_>)) -> Self {
        let mut builder = shaders::ShaderBuilder::new(asset);
        f(&mut builder);
        self.inner.shaders.push(builder.into_config());
        self
    }

    /// Applies a visual rotation to the element and all its children.
    ///
    /// This renders the element to an offscreen buffer and draws it back with
    /// rotation, flip, and pivot applied. It does **not** affect layout.
    ///
    /// When combined with `.shader()`, the rotation shares the same render
    /// target (no extra GPU cost).
    ///
    /// # Example
    /// ```rust,ignore
    /// ui.element()
    ///     .rotate_visual(|r| r
    ///         .degrees(15.0)
    ///         .pivot(0.5, 0.5)
    ///         .flip_x()
    ///     )
    ///     .children(|ui| { /* ... */ });
    /// ```
    #[inline]
    pub fn rotate_visual(mut self, f: impl for<'a> FnOnce(&'a mut elements::VisualRotationBuilder) -> &'a mut elements::VisualRotationBuilder) -> Self {
        let mut builder = elements::VisualRotationBuilder {
            config: engine::VisualRotationConfig::default(),
        };
        f(&mut builder);
        self.inner.visual_rotation = Some(builder.config);
        self
    }

    /// Applies vertex-level shape rotation to this element's geometry.
    ///
    /// Rotates the element's own rectangle / image / border at the vertex level
    /// and adjusts its layout bounding box to the AABB of the rotated shape.
    /// Children, text, and shaders are **not** affected.
    ///
    /// There is no pivot — shape rotation always rotates around the center.
    ///
    /// # Example
    /// ```rust,ignore
    /// ui.element()
    ///     .rotate_shape(|r| r.degrees(45.0).flip_x())
    ///     .empty();
    /// ```
    #[inline]
    pub fn rotate_shape(mut self, f: impl for<'a> FnOnce(&'a mut elements::ShapeRotationBuilder) -> &'a mut elements::ShapeRotationBuilder) -> Self {
        let mut builder = elements::ShapeRotationBuilder {
            config: engine::ShapeRotationConfig::default(),
        };
        f(&mut builder);
        self.inner.shape_rotation = Some(builder.config);
        self
    }

    /// Configures accessibility properties using a closure.
    ///
    /// # Example
    /// ```rust,ignore
    /// ui.element()
    ///     .id("submit_btn")
    ///     .accessibility(|a| a
    ///         .button("Submit")
    ///         .tab_index(1)
    ///     )
    ///     .empty();
    /// ```
    #[inline]
    pub fn accessibility(
        mut self,
        f: impl for<'a> FnOnce(&'a mut accessibility::AccessibilityBuilder) -> &'a mut accessibility::AccessibilityBuilder,
    ) -> Self {
        let mut builder = accessibility::AccessibilityBuilder::new();
        f(&mut builder);
        self.inner.accessibility = Some(builder.config);
        self
    }

    /// Registers a callback that fires once when the element is pressed
    /// (pointer click or Enter/Space on focused element).
    #[inline]
    pub fn on_press<F>(mut self, callback: F) -> Self
    where
        F: FnMut(Id) + 'static,
    {
        self.on_press_fn = Some(Box::new(callback));
        self
    }

    /// Registers a callback that fires once when the element is released
    /// (pointer release or key release on focused element).
    #[inline]
    pub fn on_release<F>(mut self, callback: F) -> Self
    where
        F: FnMut(Id) + 'static,
    {
        self.on_release_fn = Some(Box::new(callback));
        self
    }

    /// Registers a callback that fires when this element receives focus
    /// (via Tab navigation, arrow keys, or programmatic `set_focus`).
    #[inline]
    pub fn on_focus<F>(mut self, callback: F) -> Self
    where
        F: FnMut(Id) + 'static,
    {
        self.on_focus_fn = Some(Box::new(callback));
        self
    }

    /// Registers a callback that fires when this element loses focus.
    #[inline]
    pub fn on_unfocus<F>(mut self, callback: F) -> Self
    where
        F: FnMut(Id) + 'static,
    {
        self.on_unfocus_fn = Some(Box::new(callback));
        self
    }

    /// Configures this element as a text input using a closure-based builder.
    ///
    /// The element will capture keyboard input when focused and render
    /// text, cursor, and selection internally.
    ///
    /// # Example
    /// ```rust,ignore
    /// ui.element()
    ///     .id("username")
    ///     .text_input(|t| t
    ///         .placeholder("Enter username")
    ///         .max_length(32)
    ///         .font_size(18)
    ///         .on_changed(|text| println!("Text changed: {}", text))
    ///         .on_submit(|text| println!("Submitted: {}", text))
    ///     )
    ///     .empty();
    /// ```
    #[inline]
    pub fn text_input(
        mut self,
        f: impl for<'a> FnOnce(&'a mut text_input::TextInputBuilder) -> &'a mut text_input::TextInputBuilder,
    ) -> Self {
        let mut builder = text_input::TextInputBuilder::new();
        f(&mut builder);
        self.inner.text_input = Some(builder.config);
        self.text_input_on_changed_fn = builder.on_changed_fn;
        self.text_input_on_submit_fn = builder.on_submit_fn;
        self
    }

    /// Finalizes the element with children defined in a closure.
    pub fn children(self, f: impl FnOnce(&mut Ui<'_, CustomElementData>)) -> Id {
        let ElementBuilder {
            ply, inner, id,
            on_press_fn, on_release_fn, on_focus_fn, on_unfocus_fn,
            text_input_on_changed_fn, text_input_on_submit_fn,
        } = self;
        if let Some(ref id) = id {
            ply.context.open_element_with_id(id);
        } else {
            ply.context.open_element();
        }
        ply.context.configure_open_element(&inner);
        let element_id = ply.context.get_open_element_id();

        if on_press_fn.is_some() || on_release_fn.is_some() {
            ply.context.set_press_callbacks(on_press_fn, on_release_fn);
        }
        if on_focus_fn.is_some() || on_unfocus_fn.is_some() {
            ply.context.set_focus_callbacks(on_focus_fn, on_unfocus_fn);
        }
        if text_input_on_changed_fn.is_some() || text_input_on_submit_fn.is_some() {
            ply.context.set_text_input_callbacks(text_input_on_changed_fn, text_input_on_submit_fn);
        }

        let mut ui = Ui { ply };
        f(&mut ui);
        ui.ply.context.close_element();

        Id { id: element_id, ..Default::default() }
    }

    /// Finalizes the element with no children.
    pub fn empty(self) -> Id {
        self.children(|_| {})
    }
}

impl<'ply, CustomElementData: Clone + Default + std::fmt::Debug> core::ops::Deref
    for Ui<'ply, CustomElementData>
{
    type Target = Ply<CustomElementData>;

    fn deref(&self) -> &Self::Target {
        self.ply
    }
}

impl<'ply, CustomElementData: Clone + Default + std::fmt::Debug> core::ops::DerefMut
    for Ui<'ply, CustomElementData>
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.ply
    }
}

impl<'ply, CustomElementData: Clone + Default + std::fmt::Debug> Ui<'ply, CustomElementData> {
    /// Creates a new element builder for configuring and adding an element.
    /// Finalize with `.children(|ui| {...})` or `.empty()`.
    pub fn element(&mut self) -> ElementBuilder<'_, CustomElementData> {
        ElementBuilder {
            ply: &mut *self.ply,
            inner: engine::ElementDeclaration::default(),
            id: None,
            on_press_fn: None,
            on_release_fn: None,
            on_focus_fn: None,
            on_unfocus_fn: None,
            text_input_on_changed_fn: None,
            text_input_on_submit_fn: None,
        }
    }

    /// Adds a text element to the current open element or to the root layout.
    pub fn text(&mut self, text: &str, config_fn: impl FnOnce(&mut TextConfig) -> &mut TextConfig) {
        let mut config = TextConfig::new();
        config_fn(&mut config);
        let text_config_index = self.ply.context.store_text_element_config(config);
        self.ply.context.open_text_element(text, text_config_index);
    }

    pub fn on_hover<F>(&mut self, callback: F)
    where
        F: FnMut(Id, engine::PointerData) + 'static,
    {
        self.ply.context.on_hover(Box::new(callback));
    }

    pub fn scroll_offset(&self) -> Vector2 {
        self.ply.context.get_scroll_offset()
    }

    /// Generates a locally unique ID based on the given `label`.
    ///
    /// The ID is unique within a specific local scope but not globally.
    #[inline]
    pub fn id_local(&self, label: &'static str) -> id::Id {
        let parent_id = self.ply.context.get_parent_element_id();
        id::Id::new_index_local_with_parent(label, 0, parent_id)
    }

    /// Generates a locally unique indexed ID based on the given `label` and `index`.
    ///
    /// This is useful for differentiating elements within a local scope while keeping their labels consistent.
    #[inline]
    pub fn id_index_local(&self, label: &'static str, index: u32) -> id::Id {
        let parent_id = self.ply.context.get_parent_element_id();
        id::Id::new_index_local_with_parent(label, index, parent_id)
    }

    /// Returns if the current element you are creating is hovered
    pub fn hovered(&self) -> bool {
        self.ply.context.hovered()
    }

    /// Returns if the current element you are creating is pressed
    /// (pointer held down on it, or Enter/Space held on focused element)
    pub fn pressed(&self) -> bool {
        self.ply.context.pressed()
    }

    /// Returns if the current element you are creating has focus.
    pub fn focused(&self) -> bool {
        self.ply.context.focused()
    }
}

impl<CustomElementData: Clone + Default + std::fmt::Debug> Ply<CustomElementData> {
    pub fn begin(
        &mut self,
    ) -> Ui<'_, CustomElementData> {
        if !self.headless {
            self.context.set_layout_dimensions(Dimensions::new(
                macroquad::prelude::screen_width(),
                macroquad::prelude::screen_height(),
            ));

            // Update timing
            self.context.current_time = macroquad::prelude::get_time();
            self.context.frame_delta_time = macroquad::prelude::get_frame_time();
        }

        // Update blink timers for text inputs
        self.context.update_text_input_blink_timers();

        // Auto-update pointer state from macroquad
        if !self.headless {
            let (mx, my) = macroquad::prelude::mouse_position();
            let is_down = macroquad::prelude::is_mouse_button_down(
                macroquad::prelude::MouseButton::Left,
            );

            // Check shift state for text input click-to-cursor
            // Must happen AFTER set_pointer_state, since that's what creates pending_text_click.
            self.context.set_pointer_state(Vector2::new(mx, my), is_down);

            {
                use macroquad::prelude::{is_key_down, KeyCode};
                let shift = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);
                if shift {
                    // If shift is held and there's a pending text click, update it
                    if let Some(ref mut pending) = self.context.pending_text_click {
                        pending.3 = true;
                    }
                }
            }

            let (scroll_x, scroll_y) = macroquad::prelude::mouse_wheel();
            const SCROLL_SPEED: f32 = 20.0;
            // Shift+scroll wheel swaps vertical to horizontal scrolling
            let scroll_shift = {
                use macroquad::prelude::{is_key_down, KeyCode};
                is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift)
            };
            let scroll_delta = if scroll_shift {
                // Shift held: vertical scroll becomes horizontal
                Vector2::new(
                    (scroll_x + scroll_y) * SCROLL_SPEED,
                    0.0,
                )
            } else {
                Vector2::new(scroll_x * SCROLL_SPEED, scroll_y * SCROLL_SPEED)
            };

            // Text input pointer scrolling (scroll wheel + drag) — consumes scroll if applicable
            let text_consumed_scroll = self.context.update_text_input_pointer_scroll(scroll_delta);
            self.context.clamp_text_input_scroll();

            // Only pass scroll to scroll containers if text input didn't consume it
            let container_scroll = if text_consumed_scroll {
                Vector2::new(0.0, 0.0)
            } else {
                scroll_delta
            };
            self.context.update_scroll_containers(
                true,
                container_scroll,
                macroquad::prelude::get_frame_time(),
            );

            // Keyboard input handling
            use macroquad::prelude::{is_key_pressed, is_key_down, is_key_released, KeyCode};

            let text_input_focused = self.context.is_text_input_focused();
            let current_focused_id = self.context.focused_element_id;

            // Clear key-repeat state when focus changes (prevents stale
            // repeat from one text input bleeding into another).
            if current_focused_id != self.text_input_repeat_focus_id {
                self.text_input_repeat_key = 0;
                self.text_input_repeat_focus_id = current_focused_id;
            }

            // Tab always cycles focus (even when text input is focused)
            if is_key_pressed(KeyCode::Tab) {
                let shift = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);
                self.context.cycle_focus(shift);
            } else if text_input_focused {
                // Route keyboard input to text editing
                let shift = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);
                let ctrl = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl);
                let time = self.context.current_time;

                // Key repeat constants
                const INITIAL_DELAY: f64 = 0.5;
                const REPEAT_INTERVAL: f64 = 0.033;

                // Helper: check if a key should fire (pressed or repeating)
                macro_rules! key_fires {
                    ($key:expr, $id:expr) => {{
                        if is_key_pressed($key) {
                            self.text_input_repeat_key = $id;
                            self.text_input_repeat_first = time;
                            self.text_input_repeat_last = time;
                            true
                        } else if is_key_down($key) && self.text_input_repeat_key == $id {
                            let since_first = time - self.text_input_repeat_first;
                            let since_last = time - self.text_input_repeat_last;
                            if since_first > INITIAL_DELAY && since_last > REPEAT_INTERVAL {
                                self.text_input_repeat_last = time;
                                true
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    }};
                }

                // Handle special keys with repeat support
                let mut cursor_moved = false;
                if key_fires!(KeyCode::Left, 1) {
                    if ctrl {
                        self.context.process_text_input_action(engine::TextInputAction::MoveWordLeft { shift });
                    } else {
                        self.context.process_text_input_action(engine::TextInputAction::MoveLeft { shift });
                    }
                    cursor_moved = true;
                }
                if key_fires!(KeyCode::Right, 2) {
                    if ctrl {
                        self.context.process_text_input_action(engine::TextInputAction::MoveWordRight { shift });
                    } else {
                        self.context.process_text_input_action(engine::TextInputAction::MoveRight { shift });
                    }
                    cursor_moved = true;
                }
                if key_fires!(KeyCode::Backspace, 3) {
                    if ctrl {
                        self.context.process_text_input_action(engine::TextInputAction::BackspaceWord);
                    } else {
                        self.context.process_text_input_action(engine::TextInputAction::Backspace);
                    }
                    cursor_moved = true;
                }
                if key_fires!(KeyCode::Delete, 4) {
                    if ctrl {
                        self.context.process_text_input_action(engine::TextInputAction::DeleteWord);
                    } else {
                        self.context.process_text_input_action(engine::TextInputAction::Delete);
                    }
                    cursor_moved = true;
                }
                if key_fires!(KeyCode::Home, 5) {
                    self.context.process_text_input_action(engine::TextInputAction::MoveHome { shift });
                    cursor_moved = true;
                }
                if key_fires!(KeyCode::End, 6) {
                    self.context.process_text_input_action(engine::TextInputAction::MoveEnd { shift });
                    cursor_moved = true;
                }

                // Up/Down arrows for multiline
                if self.context.is_focused_text_input_multiline() {
                    if key_fires!(KeyCode::Up, 7) {
                        self.context.process_text_input_action(engine::TextInputAction::MoveUp { shift });
                        cursor_moved = true;
                    }
                    if key_fires!(KeyCode::Down, 8) {
                        self.context.process_text_input_action(engine::TextInputAction::MoveDown { shift });
                        cursor_moved = true;
                    }
                }

                // Non-repeating keys
                if is_key_pressed(KeyCode::Enter) {
                    self.context.process_text_input_action(engine::TextInputAction::Submit);
                    cursor_moved = true;
                }
                if ctrl && is_key_pressed(KeyCode::A) {
                    self.context.process_text_input_action(engine::TextInputAction::SelectAll);
                }
                if ctrl && is_key_pressed(KeyCode::Z) {
                    if shift {
                        self.context.process_text_input_action(engine::TextInputAction::Redo);
                    } else {
                        self.context.process_text_input_action(engine::TextInputAction::Undo);
                    }
                    cursor_moved = true;
                }
                if ctrl && is_key_pressed(KeyCode::Y) {
                    self.context.process_text_input_action(engine::TextInputAction::Redo);
                    cursor_moved = true;
                }
                if ctrl && is_key_pressed(KeyCode::C) {
                    // Copy selected text to clipboard
                    let elem_id = self.context.focused_element_id;
                    if let Some(state) = self.context.text_edit_states.get(&elem_id) {
                        let selected = state.selected_text().to_string();
                        if !selected.is_empty() {
                            macroquad::miniquad::window::clipboard_set(&selected);
                        }
                    }
                }
                if ctrl && is_key_pressed(KeyCode::X) {
                    // Cut: copy then delete selection
                    let elem_id = self.context.focused_element_id;
                    if let Some(state) = self.context.text_edit_states.get(&elem_id) {
                        let selected = state.selected_text().to_string();
                        if !selected.is_empty() {
                            macroquad::miniquad::window::clipboard_set(&selected);
                        }
                    }
                    self.context.process_text_input_action(engine::TextInputAction::Cut);
                    cursor_moved = true;
                }
                if ctrl && is_key_pressed(KeyCode::V) {
                    // Paste from clipboard
                    if let Some(text) = macroquad::miniquad::window::clipboard_get() {
                        self.context.process_text_input_action(engine::TextInputAction::Paste { text });
                        cursor_moved = true;
                    }
                }

                // Escape unfocuses the text input
                if is_key_pressed(KeyCode::Escape) {
                    self.context.clear_focus();
                }

                // Clear repeat state if the tracked key was released
                if self.text_input_repeat_key != 0 {
                    let still_down = match self.text_input_repeat_key {
                        1 => is_key_down(KeyCode::Left),
                        2 => is_key_down(KeyCode::Right),
                        3 => is_key_down(KeyCode::Backspace),
                        4 => is_key_down(KeyCode::Delete),
                        5 => is_key_down(KeyCode::Home),
                        6 => is_key_down(KeyCode::End),
                        7 => is_key_down(KeyCode::Up),
                        8 => is_key_down(KeyCode::Down),
                        _ => false,
                    };
                    if !still_down {
                        self.text_input_repeat_key = 0;
                    }
                }

                // Drain character input queue
                while let Some(ch) = macroquad::prelude::get_char_pressed() {
                    // Filter out control characters and Ctrl-key combos
                    if !ch.is_control() && !ctrl {
                        self.context.process_text_input_char(ch);
                        cursor_moved = true;
                    }
                }

                // Update scroll to keep cursor visible (only when cursor moved, not every frame,
                // so that manual scrolling via scroll wheel / drag isn't immediately undone).
                if cursor_moved {
                    self.context.update_text_input_scroll();
                }
                self.context.clamp_text_input_scroll();
            } else {
                // Normal keyboard navigation (non-text-input)
                if is_key_pressed(KeyCode::Right) { self.context.arrow_focus(engine::ArrowDirection::Right); }
                if is_key_pressed(KeyCode::Left)  { self.context.arrow_focus(engine::ArrowDirection::Left); }
                if is_key_pressed(KeyCode::Up)    { self.context.arrow_focus(engine::ArrowDirection::Up); }
                if is_key_pressed(KeyCode::Down)  { self.context.arrow_focus(engine::ArrowDirection::Down); }

                let activate_pressed = is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::Space);
                let activate_released = is_key_released(KeyCode::Enter) || is_key_released(KeyCode::Space);
                self.context.handle_keyboard_activation(activate_pressed, activate_released);
            }
        }

        // Show/hide virtual keyboard when text input focus changes (mobile)
        {
            let text_input_focused = self.context.is_text_input_focused();
            if text_input_focused != self.was_text_input_focused {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    macroquad::miniquad::window::show_keyboard(text_input_focused);
                }
                #[cfg(target_arch = "wasm32")]
                {
                    unsafe { ply_show_virtual_keyboard(text_input_focused); }
                }
                self.was_text_input_focused = text_input_focused;
            }
        }

        self.context.begin_layout();
        Ui {
            ply: self,
        }
    }

    /// Create a new Ply engine with the given fonts.
    ///
    /// Screen dimensions are obtained automatically from macroquad.
    /// Text measurement is set up automatically from the provided fonts.
    /// For custom text measurement, use [`Ply::new_headless`] and
    /// [`Ply::set_measure_text_function`].
    pub fn new(fonts: Vec<macroquad::prelude::Font>) -> Self {
        let dimensions = Dimensions::new(
            macroquad::prelude::screen_width(),
            macroquad::prelude::screen_height(),
        );
        let mut ply = Self {
            context: engine::PlyContext::new(dimensions),
            headless: false,
            fonts: fonts.clone(),
            text_input_repeat_key: 0,
            text_input_repeat_first: 0.0,
            text_input_repeat_last: 0.0,
            text_input_repeat_focus_id: 0,
            was_text_input_focused: false,
            #[cfg(target_arch = "wasm32")]
            web_a11y_state: accessibility_web::WebAccessibilityState::default(),
            #[cfg(all(feature = "native-a11y", not(target_arch = "wasm32")))]
            native_a11y_state: accessibility_native::NativeAccessibilityState::default(),
        };
        ply.set_measure_text_function(renderer::create_measure_text_function(fonts));
        ply
    }

    /// Create a new Ply engine without text measurement.
    ///
    /// Use [`Ply::set_measure_text_function`] to configure text measurement
    /// before rendering any text elements.
    pub fn new_headless(dimensions: Dimensions) -> Self {
        Self {
            context: engine::PlyContext::new(dimensions),
            headless: true,
            fonts: vec![],
            text_input_repeat_key: 0,
            text_input_repeat_first: 0.0,
            text_input_repeat_last: 0.0,
            text_input_repeat_focus_id: 0,
            was_text_input_focused: false,
            #[cfg(target_arch = "wasm32")]
            web_a11y_state: accessibility_web::WebAccessibilityState::default(),
            #[cfg(all(feature = "native-a11y", not(target_arch = "wasm32")))]
            native_a11y_state: accessibility_native::NativeAccessibilityState::default(),
        }
    }

    /// Generates a locally unique ID based on the given `label`.
    ///
    pub fn pointer_over(&self, cfg: Id) -> bool {
        self.context.pointer_over(cfg)
    }

    /// Z-sorted list of element IDs that the cursor is currently over
    pub fn pointer_over_ids(&self) -> Vec<Id> {
        self.context.get_pointer_over_ids().to_vec()
    }

    /// Set the callback for text measurement with user data
    pub fn set_measure_text_function_user_data<F, T>(
        &mut self,
        userdata: T,
        callback: F,
    ) where
        F: Fn(&str, &TextConfig, &mut T) -> Dimensions + 'static,
        T: 'static,
    {
        let data = std::cell::RefCell::new(userdata);
        self.context.set_measure_text_function(Box::new(
            move |text: &str, config: &TextConfig| -> Dimensions {
                callback(text, config, &mut data.borrow_mut())
            },
        ));
    }

    /// Set the callback for text measurement
    pub fn set_measure_text_function<F>(&mut self, callback: F)
    where
        F: Fn(&str, &TextConfig) -> Dimensions + 'static,
    {
        self.context.set_measure_text_function(Box::new(
            move |text: &str, config: &TextConfig| -> Dimensions {
                callback(text, config)
            },
        ));
    }

    /// Sets the maximum number of elements that ply supports
    /// **Use only if you know what you are doing or you're getting errors from ply**
    pub fn max_element_count(&mut self, max_element_count: u32) {
        self.context.set_max_element_count(max_element_count as i32);
    }

    /// Sets the capacity of the cache used for text in the measure text function
    /// **Use only if you know what you are doing or you're getting errors from ply**
    pub fn max_measure_text_cache_word_count(&mut self, count: u32) {
        self.context.set_max_measure_text_cache_word_count(count as i32);
    }

    /// Enables or disables the debug mode of ply
    pub fn set_debug_mode(&mut self, enable: bool) {
        self.context.set_debug_mode_enabled(enable);
    }

    /// Returns if debug mode is enabled
    pub fn is_debug_mode(&self) -> bool {
        self.context.is_debug_mode_enabled()
    }

    /// Enables or disables culling
    pub fn set_culling(&mut self, enable: bool) {
        self.context.set_culling_enabled(enable);
    }

    /// Sets the dimensions of the global layout, use if, for example the window size you render to
    /// changed
    pub fn set_layout_dimensions(&mut self, dimensions: Dimensions) {
        self.context.set_layout_dimensions(dimensions);
    }

    /// Updates the state of the pointer for ply. Used to update scroll containers and for
    /// interactions functions
    pub fn pointer_state(&mut self, position: Vector2, is_down: bool) {
        self.context.set_pointer_state(position, is_down);
    }

    pub fn update_scroll_containers(
        &mut self,
        drag_scrolling_enabled: bool,
        scroll_delta: Vector2,
        delta_time: f32,
    ) {
        self.context
            .update_scroll_containers(drag_scrolling_enabled, scroll_delta, delta_time);
    }

    /// Returns the ID of the currently focused element, or None.
    pub fn focused_element(&self) -> Option<u32> {
        self.context.focused_element()
    }

    /// Sets focus to the element with the given ID.
    pub fn set_focus(&mut self, id: Id) {
        self.context.set_focus(id.id);
    }

    /// Clears focus (no element is focused).
    pub fn clear_focus(&mut self) {
        self.context.clear_focus();
    }

    /// Returns the text value of a text input element.
    /// Returns an empty string if the element is not a text input or doesn't exist.
    pub fn get_text_value(&self, id: impl Into<Id>) -> &str {
        self.context.get_text_value(id.into().id)
    }

    /// Sets the text value of a text input element.
    pub fn set_text_value(&mut self, id: impl Into<Id>, value: &str) {
        self.context.set_text_value(id.into().id, value);
    }

    /// Returns true if the given element is currently pressed
    /// (pointer held down on it).
    pub fn is_pressed(&self, id: impl Into<Id>) -> bool {
        self.context.is_element_pressed(id.into().id)
    }

    pub fn bounding_box(&self, id: Id) -> Option<math::BoundingBox> {
        self.context.get_element_data(id)
    }

    pub fn scroll_container_data(&self, id: Id) -> Option<engine::ScrollContainerData> {
        let data = self.context.get_scroll_container_data(id);
        if data.found {
            Some(data)
        } else {
            None
        }
    }

    /// Evaluate the layout and return all render commands.
    pub fn eval(&mut self) -> Vec<RenderCommand<CustomElementData>> {
        let commands = self.context.end_layout();
        let mut result = Vec::new();
        for cmd in commands {
            result.push(RenderCommand::from_engine_render_command(cmd));
        }

        // Sync the hidden DOM accessibility tree (web/WASM only)
        #[cfg(target_arch = "wasm32")]
        {
            accessibility_web::sync_accessibility_tree(
                &mut self.web_a11y_state,
                &self.context.accessibility_configs,
                &self.context.accessibility_element_order,
                self.context.focused_element_id,
            );
        }

        // Sync accessibility tree via AccessKit (native platforms)
        #[cfg(all(feature = "native-a11y", not(target_arch = "wasm32")))]
        {
            let a11y_actions = accessibility_native::sync_accessibility_tree(
                &mut self.native_a11y_state,
                &self.context.accessibility_configs,
                &self.context.accessibility_element_order,
                self.context.focused_element_id,
            );
            for action in a11y_actions {
                match action {
                    accessibility_native::PendingA11yAction::Focus(target_id) => {
                        self.context.change_focus(target_id);
                    }
                    accessibility_native::PendingA11yAction::Click(target_id) => {
                        self.context.fire_press(target_id);
                    }
                }
            }
        }

        result
    }

    /// Evaluate the layout and render all commands.
    pub async fn show(
        &mut self,
        handle_custom_command: impl Fn(&RenderCommand<CustomElementData>),
    ) {
        let commands = self.eval();
        renderer::render(commands, &self.fonts, handle_custom_command).await;
    }
}

#[cfg(target_arch = "wasm32")]
extern "C" {
    fn ply_show_virtual_keyboard(show: bool);
}

#[cfg(test)]
mod tests {
    use super::*;
    use color::Color;
    use layout::{Padding, Sizing};

    #[rustfmt::skip]
    #[test]
    fn test_begin() {
        let mut ply = Ply::<()>::new_headless(Dimensions::new(800.0, 600.0));

        ply.set_measure_text_function(|_, _| {
            Dimensions::new(100.0, 24.0)
        });

        let mut ui = ply.begin();

        ui.element().width(fixed!(100.0)).height(fixed!(100.0))
            .background_color(0xFFFFFF)
            .children(|ui| {
                ui.element().width(fixed!(100.0)).height(fixed!(100.0))
                    .background_color(0xFFFFFF)
                    .children(|ui| {
                        ui.element().width(fixed!(100.0)).height(fixed!(100.0))
                            .background_color(0xFFFFFF)
                            .children(|ui| {
                                ui.text("test", |t| t
                                    .color(0xFFFFFF)
                                    .font_size(24)
                                );
                            });
                    });
            });

        ui.element()
            .border(|b| b
                .color(0xFFFF00)
                .all(2)
            )
            .corner_radius(10.0)
            .children(|ui| {
                ui.element().width(fixed!(50.0)).height(fixed!(50.0))
                    .background_color(0x00FFFF)
                    .empty();
            });

        let items = ui.eval();

        for item in &items {
            println!(
                "id: {}\nbbox: {:?}\nconfig: {:?}",
                item.id, item.bounding_box, item.config,
            );
        }

        assert_eq!(items.len(), 6);
        
        assert_eq!(items[0].bounding_box.x, 0.0);
        assert_eq!(items[0].bounding_box.y, 0.0);
        assert_eq!(items[0].bounding_box.width, 100.0);
        assert_eq!(items[0].bounding_box.height, 100.0);
        match &items[0].config {
            render_commands::RenderCommandConfig::Rectangle(rect) => {
                assert_eq!(rect.color.r, 255.0);
                assert_eq!(rect.color.g, 255.0);
                assert_eq!(rect.color.b, 255.0);
                assert_eq!(rect.color.a, 255.0);
            }
            _ => panic!("Expected Rectangle config for item 0"),
        }
        
        assert_eq!(items[1].bounding_box.x, 0.0);
        assert_eq!(items[1].bounding_box.y, 0.0);
        assert_eq!(items[1].bounding_box.width, 100.0);
        assert_eq!(items[1].bounding_box.height, 100.0);
        match &items[1].config {
            render_commands::RenderCommandConfig::Rectangle(rect) => {
                assert_eq!(rect.color.r, 255.0);
                assert_eq!(rect.color.g, 255.0);
                assert_eq!(rect.color.b, 255.0);
                assert_eq!(rect.color.a, 255.0);
            }
            _ => panic!("Expected Rectangle config for item 1"),
        }
        
        assert_eq!(items[2].bounding_box.x, 0.0);
        assert_eq!(items[2].bounding_box.y, 0.0);
        assert_eq!(items[2].bounding_box.width, 100.0);
        assert_eq!(items[2].bounding_box.height, 100.0);
        match &items[2].config {
            render_commands::RenderCommandConfig::Rectangle(rect) => {
                assert_eq!(rect.color.r, 255.0);
                assert_eq!(rect.color.g, 255.0);
                assert_eq!(rect.color.b, 255.0);
                assert_eq!(rect.color.a, 255.0);
            }
            _ => panic!("Expected Rectangle config for item 2"),
        }
        
        assert_eq!(items[3].bounding_box.x, 0.0);
        assert_eq!(items[3].bounding_box.y, 0.0);
        assert_eq!(items[3].bounding_box.width, 100.0);
        assert_eq!(items[3].bounding_box.height, 24.0);
        match &items[3].config {
            render_commands::RenderCommandConfig::Text(text) => {
                assert_eq!(text.text, "test");
                assert_eq!(text.color.r, 255.0);
                assert_eq!(text.color.g, 255.0);
                assert_eq!(text.color.b, 255.0);
                assert_eq!(text.color.a, 255.0);
                assert_eq!(text.font_size, 24);
            }
            _ => panic!("Expected Text config for item 3"),
        }
        
        assert_eq!(items[4].bounding_box.x, 100.0);
        assert_eq!(items[4].bounding_box.y, 0.0);
        assert_eq!(items[4].bounding_box.width, 50.0);
        assert_eq!(items[4].bounding_box.height, 50.0);
        match &items[4].config {
            render_commands::RenderCommandConfig::Rectangle(rect) => {
                assert_eq!(rect.color.r, 0.0);
                assert_eq!(rect.color.g, 255.0);
                assert_eq!(rect.color.b, 255.0);
                assert_eq!(rect.color.a, 255.0);
            }
            _ => panic!("Expected Rectangle config for item 4"),
        }
        
        assert_eq!(items[5].bounding_box.x, 100.0);
        assert_eq!(items[5].bounding_box.y, 0.0);
        assert_eq!(items[5].bounding_box.width, 50.0);
        assert_eq!(items[5].bounding_box.height, 50.0);
        match &items[5].config {
            render_commands::RenderCommandConfig::Border(border) => {
                assert_eq!(border.color.r, 255.0);
                assert_eq!(border.color.g, 255.0);
                assert_eq!(border.color.b, 0.0);
                assert_eq!(border.color.a, 255.0);
                assert_eq!(border.corner_radii.top_left, 10.0);
                assert_eq!(border.corner_radii.top_right, 10.0);
                assert_eq!(border.corner_radii.bottom_left, 10.0);
                assert_eq!(border.corner_radii.bottom_right, 10.0);
                assert_eq!(border.width.left, 2);
                assert_eq!(border.width.right, 2);
                assert_eq!(border.width.top, 2);
                assert_eq!(border.width.bottom, 2);
            }
            _ => panic!("Expected Border config for item 5"),
        }
    }

    #[rustfmt::skip]
    #[test]
    fn test_example() {
        let mut ply = Ply::<()>::new_headless(Dimensions::new(1000.0, 1000.0));

        let mut ui = ply.begin();

        ui.set_measure_text_function(|_, _| {
            Dimensions::new(100.0, 24.0)
        });

        for &(label, level) in &[("Road", 1), ("Wall", 2), ("Tower", 3)] {
            ui.element().width(grow!()).height(fixed!(36.0))
                .layout(|l| l
                    .direction(crate::layout::LayoutDirection::LeftToRight)
                    .gap(12)
                    .align(crate::align::AlignX::Left, crate::align::AlignY::CenterY)
                )
                .children(|ui| {
                    ui.text(label, |t| t
                        .font_size(18)
                        .color(0xFFFFFF)
                    );
                    ui.element().width(grow!()).height(fixed!(18.0))
                        .corner_radius(9.0)
                        .background_color(0x555555)
                        .children(|ui| {
                            ui.element()
                                .width(fixed!(300.0 * level as f32 / 3.0))
                                .height(grow!())
                                .corner_radius(9.0)
                                .background_color(0x45A85A)
                                .empty();
                        });
                });
        }

        let items = ui.eval();

        for item in &items {
            println!(
                "id: {}\nbbox: {:?}\nconfig: {:?}",
                item.id, item.bounding_box, item.config,
            );
        }

        assert_eq!(items.len(), 9);

        // Road label
        assert_eq!(items[0].bounding_box.x, 0.0);
        assert_eq!(items[0].bounding_box.y, 6.0);
        assert_eq!(items[0].bounding_box.width, 100.0);
        assert_eq!(items[0].bounding_box.height, 24.0);
        match &items[0].config {
            render_commands::RenderCommandConfig::Text(text) => {
                assert_eq!(text.text, "Road");
                assert_eq!(text.color.r, 255.0);
                assert_eq!(text.color.g, 255.0);
                assert_eq!(text.color.b, 255.0);
                assert_eq!(text.color.a, 255.0);
                assert_eq!(text.font_size, 18);
            }
            _ => panic!("Expected Text config for item 0"),
        }

        // Road background box
        assert_eq!(items[1].bounding_box.x, 112.0);
        assert_eq!(items[1].bounding_box.y, 9.0);
        assert_eq!(items[1].bounding_box.width, 163.99142);
        assert_eq!(items[1].bounding_box.height, 18.0);
        match &items[1].config {
            render_commands::RenderCommandConfig::Rectangle(rect) => {
                assert_eq!(rect.color.r, 85.0);
                assert_eq!(rect.color.g, 85.0);
                assert_eq!(rect.color.b, 85.0);
                assert_eq!(rect.color.a, 255.0);
                assert_eq!(rect.corner_radii.top_left, 9.0);
                assert_eq!(rect.corner_radii.top_right, 9.0);
                assert_eq!(rect.corner_radii.bottom_left, 9.0);
                assert_eq!(rect.corner_radii.bottom_right, 9.0);
            }
            _ => panic!("Expected Rectangle config for item 1"),
        }

        // Road progress bar
        assert_eq!(items[2].bounding_box.x, 112.0);
        assert_eq!(items[2].bounding_box.y, 9.0);
        assert_eq!(items[2].bounding_box.width, 100.0);
        assert_eq!(items[2].bounding_box.height, 18.0);
        match &items[2].config {
            render_commands::RenderCommandConfig::Rectangle(rect) => {
                assert_eq!(rect.color.r, 69.0);
                assert_eq!(rect.color.g, 168.0);
                assert_eq!(rect.color.b, 90.0);
                assert_eq!(rect.color.a, 255.0);
                assert_eq!(rect.corner_radii.top_left, 9.0);
                assert_eq!(rect.corner_radii.top_right, 9.0);
                assert_eq!(rect.corner_radii.bottom_left, 9.0);
                assert_eq!(rect.corner_radii.bottom_right, 9.0);
            }
            _ => panic!("Expected Rectangle config for item 2"),
        }

        // Wall label
        assert_eq!(items[3].bounding_box.x, 275.99142);
        assert_eq!(items[3].bounding_box.y, 6.0);
        assert_eq!(items[3].bounding_box.width, 100.0);
        assert_eq!(items[3].bounding_box.height, 24.0);
        match &items[3].config {
            render_commands::RenderCommandConfig::Text(text) => {
                assert_eq!(text.text, "Wall");
                assert_eq!(text.color.r, 255.0);
                assert_eq!(text.color.g, 255.0);
                assert_eq!(text.color.b, 255.0);
                assert_eq!(text.color.a, 255.0);
                assert_eq!(text.font_size, 18);
            }
            _ => panic!("Expected Text config for item 3"),
        }

        // Wall background box
        assert_eq!(items[4].bounding_box.x, 387.99142);
        assert_eq!(items[4].bounding_box.y, 9.0);
        assert_eq!(items[4].bounding_box.width, 200.0);
        assert_eq!(items[4].bounding_box.height, 18.0);
        match &items[4].config {
            render_commands::RenderCommandConfig::Rectangle(rect) => {
                assert_eq!(rect.color.r, 85.0);
                assert_eq!(rect.color.g, 85.0);
                assert_eq!(rect.color.b, 85.0);
                assert_eq!(rect.color.a, 255.0);
                assert_eq!(rect.corner_radii.top_left, 9.0);
                assert_eq!(rect.corner_radii.top_right, 9.0);
                assert_eq!(rect.corner_radii.bottom_left, 9.0);
                assert_eq!(rect.corner_radii.bottom_right, 9.0);
            }
            _ => panic!("Expected Rectangle config for item 4"),
        }

        // Wall progress bar
        assert_eq!(items[5].bounding_box.x, 387.99142);
        assert_eq!(items[5].bounding_box.y, 9.0);
        assert_eq!(items[5].bounding_box.width, 200.0);
        assert_eq!(items[5].bounding_box.height, 18.0);
        match &items[5].config {
            render_commands::RenderCommandConfig::Rectangle(rect) => {
                assert_eq!(rect.color.r, 69.0);
                assert_eq!(rect.color.g, 168.0);
                assert_eq!(rect.color.b, 90.0);
                assert_eq!(rect.color.a, 255.0);
                assert_eq!(rect.corner_radii.top_left, 9.0);
                assert_eq!(rect.corner_radii.top_right, 9.0);
                assert_eq!(rect.corner_radii.bottom_left, 9.0);
                assert_eq!(rect.corner_radii.bottom_right, 9.0);
            }
            _ => panic!("Expected Rectangle config for item 5"),
        }

        // Tower label
        assert_eq!(items[6].bounding_box.x, 587.99146);
        assert_eq!(items[6].bounding_box.y, 6.0);
        assert_eq!(items[6].bounding_box.width, 100.0);
        assert_eq!(items[6].bounding_box.height, 24.0);
        match &items[6].config {
            render_commands::RenderCommandConfig::Text(text) => {
                assert_eq!(text.text, "Tower");
                assert_eq!(text.color.r, 255.0);
                assert_eq!(text.color.g, 255.0);
                assert_eq!(text.color.b, 255.0);
                assert_eq!(text.color.a, 255.0);
                assert_eq!(text.font_size, 18);
            }
            _ => panic!("Expected Text config for item 6"),
        }

        // Tower background box
        assert_eq!(items[7].bounding_box.x, 699.99146);
        assert_eq!(items[7].bounding_box.y, 9.0);
        assert_eq!(items[7].bounding_box.width, 300.0);
        assert_eq!(items[7].bounding_box.height, 18.0);
        match &items[7].config {
            render_commands::RenderCommandConfig::Rectangle(rect) => {
                assert_eq!(rect.color.r, 85.0);
                assert_eq!(rect.color.g, 85.0);
                assert_eq!(rect.color.b, 85.0);
                assert_eq!(rect.color.a, 255.0);
                assert_eq!(rect.corner_radii.top_left, 9.0);
                assert_eq!(rect.corner_radii.top_right, 9.0);
                assert_eq!(rect.corner_radii.bottom_left, 9.0);
                assert_eq!(rect.corner_radii.bottom_right, 9.0);
            }
            _ => panic!("Expected Rectangle config for item 7"),
        }

        // Tower progress bar
        assert_eq!(items[8].bounding_box.x, 699.99146);
        assert_eq!(items[8].bounding_box.y, 9.0);
        assert_eq!(items[8].bounding_box.width, 300.0);
        assert_eq!(items[8].bounding_box.height, 18.0);
        match &items[8].config {
            render_commands::RenderCommandConfig::Rectangle(rect) => {
                assert_eq!(rect.color.r, 69.0);
                assert_eq!(rect.color.g, 168.0);
                assert_eq!(rect.color.b, 90.0);
                assert_eq!(rect.color.a, 255.0);
                assert_eq!(rect.corner_radii.top_left, 9.0);
                assert_eq!(rect.corner_radii.top_right, 9.0);
                assert_eq!(rect.corner_radii.bottom_left, 9.0);
                assert_eq!(rect.corner_radii.bottom_right, 9.0);
            }
            _ => panic!("Expected Rectangle config for item 8"),
        }
    }

    #[rustfmt::skip]
    #[test]
    fn test_floating() {
        let mut ply = Ply::<()>::new_headless(Dimensions::new(1000.0, 1000.0));

        let mut ui = ply.begin();

        ui.set_measure_text_function(|_, _| {
            Dimensions::new(100.0, 24.0)
        });

        ui.element().width(fixed!(20.0)).height(fixed!(20.0))
            .layout(|l| l.align(crate::align::AlignX::CenterX, crate::align::AlignY::CenterY))
            .floating(|f| f
                .attach_root()
                .anchor((crate::align::AlignX::CenterX, crate::align::AlignY::CenterY), (crate::align::AlignX::Left, crate::align::AlignY::Top))
                .offset(100.0, 150.0)
                .passthrough()
                .z_index(110)
            )
            .corner_radius(10.0)
            .background_color(0x4488DD)
            .children(|ui| {
                ui.text("Re", |t| t
                    .font_size(6)
                    .color(0xFFFFFF)
                );
            });

        let items = ui.eval();

        for item in &items {
            println!(
                "id: {}\nbbox: {:?}\nconfig: {:?}",
                item.id, item.bounding_box, item.config,
            );
        }

        assert_eq!(items.len(), 2);

        assert_eq!(items[0].bounding_box.x, 90.0);
        assert_eq!(items[0].bounding_box.y, 140.0);
        assert_eq!(items[0].bounding_box.width, 20.0);
        assert_eq!(items[0].bounding_box.height, 20.0);
        match &items[0].config {
            render_commands::RenderCommandConfig::Rectangle(rect) => {
                assert_eq!(rect.color.r, 68.0);
                assert_eq!(rect.color.g, 136.0);
                assert_eq!(rect.color.b, 221.0);
                assert_eq!(rect.color.a, 255.0);
                assert_eq!(rect.corner_radii.top_left, 10.0);
                assert_eq!(rect.corner_radii.top_right, 10.0);
                assert_eq!(rect.corner_radii.bottom_left, 10.0);
                assert_eq!(rect.corner_radii.bottom_right, 10.0);
            }
            _ => panic!("Expected Rectangle config for item 0"),
        }

        assert_eq!(items[1].bounding_box.x, 50.0);
        assert_eq!(items[1].bounding_box.y, 138.0);
        assert_eq!(items[1].bounding_box.width, 100.0);
        assert_eq!(items[1].bounding_box.height, 24.0);
        match &items[1].config {
            render_commands::RenderCommandConfig::Text(text) => {
                assert_eq!(text.text, "Re");
                assert_eq!(text.color.r, 255.0);
                assert_eq!(text.color.g, 255.0);
                assert_eq!(text.color.b, 255.0);
                assert_eq!(text.color.a, 255.0);
                assert_eq!(text.font_size, 6);
            }
            _ => panic!("Expected Text config for item 1"),
        }
    }

    #[rustfmt::skip]
    #[test]
    fn test_simple_text_measure() {
        let mut ply = Ply::<()>::new_headless(Dimensions::new(800.0, 600.0));

        ply.set_measure_text_function(|_text, _config| {
            Dimensions::default()
        });

        let mut ui = ply.begin();

        ui.element()
            .id("parent_rect")
            .width(Sizing::Fixed(100.0))
            .height(Sizing::Fixed(100.0))
            .layout(|l| l
                .padding(Padding::all(10))
            )
            .background_color(Color::rgb(255., 255., 255.))
            .children(|ui| {
                ui.text(&format!("{}", 1234), |t| t
                    .color(Color::rgb(255., 255., 255.))
                    .font_size(24)
                );
            });

        let _items = ui.eval();
    }

    #[rustfmt::skip]
    #[test]
    fn test_shader_begin_end() {
        use shaders::ShaderAsset;

        let test_shader = ShaderAsset::Source {
            file_name: "test_effect.glsl",
            fragment: "#version 100\nprecision lowp float;\nvarying vec2 uv;\nuniform sampler2D Texture;\nvoid main() { gl_FragColor = texture2D(Texture, uv); }",
        };

        let mut ply = Ply::<()>::new_headless(Dimensions::new(800.0, 600.0));
        ply.set_measure_text_function(|_, _| Dimensions::new(100.0, 24.0));

        let mut ui = ply.begin();

        // Element with a group shader containing children
        ui.element()
            .width(fixed!(200.0)).height(fixed!(200.0))
            .background_color(0xFF0000)
            .shader(&test_shader, |s| {
                s.uniform("time", 1.0f32);
            })
            .children(|ui| {
                ui.element()
                    .width(fixed!(100.0)).height(fixed!(100.0))
                    .background_color(0x00FF00)
                    .empty();
            });

        let items = ui.eval();

        for (i, item) in items.iter().enumerate() {
            println!(
                "[{}] config: {:?}, bbox: {:?}",
                i, item.config, item.bounding_box,
            );
        }

        // Expected order (GroupBegin now wraps the entire element group):
        // 0: GroupBegin
        // 1: Rectangle (parent background)
        // 2: Rectangle (child)
        // 3: GroupEnd
        assert!(items.len() >= 4, "Expected at least 4 items, got {}", items.len());

        match &items[0].config {
            render_commands::RenderCommandConfig::GroupBegin { shader, visual_rotation } => {
                let config = shader.as_ref().expect("GroupBegin should have shader config");
                assert!(!config.fragment.is_empty(), "GroupBegin should have fragment source");
                assert_eq!(config.uniforms.len(), 1);
                assert_eq!(config.uniforms[0].name, "time");
                assert!(visual_rotation.is_none(), "Shader-only group should have no visual_rotation");
            }
            other => panic!("Expected GroupBegin for item 0, got {:?}", other),
        }

        match &items[1].config {
            render_commands::RenderCommandConfig::Rectangle(rect) => {
                assert_eq!(rect.color.r, 255.0);
                assert_eq!(rect.color.g, 0.0);
                assert_eq!(rect.color.b, 0.0);
            }
            other => panic!("Expected Rectangle for item 1, got {:?}", other),
        }

        match &items[2].config {
            render_commands::RenderCommandConfig::Rectangle(rect) => {
                assert_eq!(rect.color.r, 0.0);
                assert_eq!(rect.color.g, 255.0);
                assert_eq!(rect.color.b, 0.0);
            }
            other => panic!("Expected Rectangle for item 2, got {:?}", other),
        }

        match &items[3].config {
            render_commands::RenderCommandConfig::GroupEnd => {}
            other => panic!("Expected GroupEnd for item 3, got {:?}", other),
        }
    }

    #[rustfmt::skip]
    #[test]
    fn test_multiple_shaders_nested() {
        use shaders::ShaderAsset;

        let shader_a = ShaderAsset::Source {
            file_name: "shader_a.glsl",
            fragment: "#version 100\nprecision lowp float;\nvoid main() { gl_FragColor = vec4(1.0); }",
        };
        let shader_b = ShaderAsset::Source {
            file_name: "shader_b.glsl",
            fragment: "#version 100\nprecision lowp float;\nvoid main() { gl_FragColor = vec4(0.5); }",
        };

        let mut ply = Ply::<()>::new_headless(Dimensions::new(800.0, 600.0));
        ply.set_measure_text_function(|_, _| Dimensions::new(100.0, 24.0));

        let mut ui = ply.begin();

        // Element with two group shaders
        ui.element()
            .width(fixed!(200.0)).height(fixed!(200.0))
            .background_color(0xFFFFFF)
            .shader(&shader_a, |s| { s.uniform("val", 1.0f32); })
            .shader(&shader_b, |s| { s.uniform("val", 2.0f32); })
            .children(|ui| {
                ui.element()
                    .width(fixed!(50.0)).height(fixed!(50.0))
                    .background_color(0x0000FF)
                    .empty();
            });

        let items = ui.eval();

        for (i, item) in items.iter().enumerate() {
            println!("[{}] config: {:?}", i, item.config);
        }

        // Expected order (GroupBegin wraps before element drawing):
        // 0: GroupBegin(shader_b) — outermost, wraps everything
        // 1: GroupBegin(shader_a) — innermost, wraps element + children
        // 2: Rectangle (parent)
        // 3: Rectangle (child)
        // 4: GroupEnd — closes shader_a
        // 5: GroupEnd — closes shader_b
        assert!(items.len() >= 6, "Expected at least 6 items, got {}", items.len());

        match &items[0].config {
            render_commands::RenderCommandConfig::GroupBegin { shader, .. } => {
                let config = shader.as_ref().unwrap();
                // shader_b is outermost
                assert!(config.fragment.contains("0.5"), "Expected shader_b fragment");
            }
            other => panic!("Expected GroupBegin(shader_b) for item 0, got {:?}", other),
        }
        match &items[1].config {
            render_commands::RenderCommandConfig::GroupBegin { shader, .. } => {
                let config = shader.as_ref().unwrap();
                // shader_a is innermost
                assert!(config.fragment.contains("1.0"), "Expected shader_a fragment");
            }
            other => panic!("Expected GroupBegin(shader_a) for item 1, got {:?}", other),
        }
        match &items[2].config {
            render_commands::RenderCommandConfig::Rectangle(_) => {}
            other => panic!("Expected Rectangle for item 2, got {:?}", other),
        }
        match &items[3].config {
            render_commands::RenderCommandConfig::Rectangle(_) => {}
            other => panic!("Expected Rectangle for item 3, got {:?}", other),
        }
        match &items[4].config {
            render_commands::RenderCommandConfig::GroupEnd => {}
            other => panic!("Expected GroupEnd for item 4, got {:?}", other),
        }
        match &items[5].config {
            render_commands::RenderCommandConfig::GroupEnd => {}
            other => panic!("Expected GroupEnd for item 5, got {:?}", other),
        }
    }

    #[rustfmt::skip]
    #[test]
    fn test_effect_on_render_command() {
        use shaders::ShaderAsset;

        let effect_shader = ShaderAsset::Source {
            file_name: "gradient.glsl",
            fragment: "#version 100\nprecision lowp float;\nvoid main() { gl_FragColor = vec4(1.0); }",
        };

        let mut ply = Ply::<()>::new_headless(Dimensions::new(800.0, 600.0));

        let mut ui = ply.begin();

        ui.element()
            .width(fixed!(200.0)).height(fixed!(100.0))
            .background_color(0xFF0000)
            .effect(&effect_shader, |s| {
                s.uniform("color_a", [1.0f32, 0.0, 0.0, 1.0])
                 .uniform("color_b", [0.0f32, 0.0, 1.0, 1.0]);
            })
            .empty();

        let items = ui.eval();

        assert_eq!(items.len(), 1, "Expected 1 item, got {}", items.len());
        assert_eq!(items[0].effects.len(), 1, "Expected 1 effect");
        assert_eq!(items[0].effects[0].uniforms.len(), 2);
        assert_eq!(items[0].effects[0].uniforms[0].name, "color_a");
        assert_eq!(items[0].effects[0].uniforms[1].name, "color_b");
    }

    #[rustfmt::skip]
    #[test]
    fn test_visual_rotation_emits_group() {
        let mut ply = Ply::<()>::new_headless(Dimensions::new(800.0, 600.0));
        let mut ui = ply.begin();

        ui.element()
            .width(fixed!(100.0)).height(fixed!(50.0))
            .background_color(0xFF0000)
            .rotate_visual(|r| r.degrees(45.0))
            .empty();

        let items = ui.eval();

        // Expected: GroupBegin, Rectangle, GroupEnd
        assert_eq!(items.len(), 3, "Expected 3 items, got {}", items.len());

        match &items[0].config {
            render_commands::RenderCommandConfig::GroupBegin { shader, visual_rotation } => {
                assert!(shader.is_none(), "Rotation-only group should have no shader");
                let vr = visual_rotation.as_ref().expect("Should have visual_rotation");
                assert!((vr.rotation_radians - 45.0_f32.to_radians()).abs() < 0.001);
                assert_eq!(vr.pivot_x, 0.5);
                assert_eq!(vr.pivot_y, 0.5);
                assert!(!vr.flip_x);
                assert!(!vr.flip_y);
            }
            other => panic!("Expected GroupBegin for item 0, got {:?}", other),
        }

        match &items[1].config {
            render_commands::RenderCommandConfig::Rectangle(_) => {}
            other => panic!("Expected Rectangle for item 1, got {:?}", other),
        }

        match &items[2].config {
            render_commands::RenderCommandConfig::GroupEnd => {}
            other => panic!("Expected GroupEnd for item 2, got {:?}", other),
        }
    }

    #[rustfmt::skip]
    #[test]
    fn test_visual_rotation_with_shader_merged() {
        use shaders::ShaderAsset;

        let test_shader = ShaderAsset::Source {
            file_name: "merge_test.glsl",
            fragment: "#version 100\nprecision lowp float;\nvoid main() { gl_FragColor = vec4(1.0); }",
        };

        let mut ply = Ply::<()>::new_headless(Dimensions::new(800.0, 600.0));
        let mut ui = ply.begin();

        // Both shader and visual rotation — should emit ONE GroupBegin
        ui.element()
            .width(fixed!(100.0)).height(fixed!(100.0))
            .background_color(0xFF0000)
            .shader(&test_shader, |s| { s.uniform("v", 1.0f32); })
            .rotate_visual(|r| r.degrees(30.0).pivot(0.0, 0.0))
            .empty();

        let items = ui.eval();

        // Expected: GroupBegin (with shader + rotation), Rectangle, GroupEnd
        assert_eq!(items.len(), 3, "Expected 3 items (merged), got {}", items.len());

        match &items[0].config {
            render_commands::RenderCommandConfig::GroupBegin { shader, visual_rotation } => {
                assert!(shader.is_some(), "Merged group should have shader");
                let vr = visual_rotation.as_ref().expect("Merged group should have visual_rotation");
                assert!((vr.rotation_radians - 30.0_f32.to_radians()).abs() < 0.001);
                assert_eq!(vr.pivot_x, 0.0);
                assert_eq!(vr.pivot_y, 0.0);
            }
            other => panic!("Expected GroupBegin for item 0, got {:?}", other),
        }
    }

    #[rustfmt::skip]
    #[test]
    fn test_visual_rotation_with_multiple_shaders() {
        use shaders::ShaderAsset;

        let shader_a = ShaderAsset::Source {
            file_name: "vr_a.glsl",
            fragment: "#version 100\nprecision lowp float;\nvoid main() { gl_FragColor = vec4(1.0); }",
        };
        let shader_b = ShaderAsset::Source {
            file_name: "vr_b.glsl",
            fragment: "#version 100\nprecision lowp float;\nvoid main() { gl_FragColor = vec4(0.5); }",
        };

        let mut ply = Ply::<()>::new_headless(Dimensions::new(800.0, 600.0));
        let mut ui = ply.begin();

        ui.element()
            .width(fixed!(100.0)).height(fixed!(100.0))
            .background_color(0xFF0000)
            .shader(&shader_a, |s| { s.uniform("v", 1.0f32); })
            .shader(&shader_b, |s| { s.uniform("v", 2.0f32); })
            .rotate_visual(|r| r.degrees(90.0))
            .empty();

        let items = ui.eval();

        // Expected: GroupBegin(shader_b + rotation), GroupBegin(shader_a), Rect, GroupEnd, GroupEnd
        assert!(items.len() >= 5, "Expected at least 5 items, got {}", items.len());

        // Outermost GroupBegin carries both shader_b and visual_rotation
        match &items[0].config {
            render_commands::RenderCommandConfig::GroupBegin { shader, visual_rotation } => {
                assert!(shader.is_some(), "Outermost should have shader");
                assert!(visual_rotation.is_some(), "Outermost should have visual_rotation");
            }
            other => panic!("Expected GroupBegin for item 0, got {:?}", other),
        }

        // Inner GroupBegin has shader only, no rotation
        match &items[1].config {
            render_commands::RenderCommandConfig::GroupBegin { shader, visual_rotation } => {
                assert!(shader.is_some(), "Inner should have shader");
                assert!(visual_rotation.is_none(), "Inner should NOT have visual_rotation");
            }
            other => panic!("Expected GroupBegin for item 1, got {:?}", other),
        }
    }

    #[rustfmt::skip]
    #[test]
    fn test_visual_rotation_noop_skipped() {
        let mut ply = Ply::<()>::new_headless(Dimensions::new(800.0, 600.0));
        let mut ui = ply.begin();

        // 0° rotation with no flips — should be optimized away
        ui.element()
            .width(fixed!(100.0)).height(fixed!(100.0))
            .background_color(0xFF0000)
            .rotate_visual(|r| r.degrees(0.0))
            .empty();

        let items = ui.eval();

        // Should be just the rectangle, no GroupBegin/End
        assert_eq!(items.len(), 1, "Noop rotation should produce 1 item, got {}", items.len());
        match &items[0].config {
            render_commands::RenderCommandConfig::Rectangle(_) => {}
            other => panic!("Expected Rectangle, got {:?}", other),
        }
    }

    #[rustfmt::skip]
    #[test]
    fn test_visual_rotation_flip_only() {
        let mut ply = Ply::<()>::new_headless(Dimensions::new(800.0, 600.0));
        let mut ui = ply.begin();

        // 0° but flip_x — NOT a noop, should emit group
        ui.element()
            .width(fixed!(100.0)).height(fixed!(100.0))
            .background_color(0xFF0000)
            .rotate_visual(|r| r.flip_x())
            .empty();

        let items = ui.eval();

        // GroupBegin, Rectangle, GroupEnd
        assert_eq!(items.len(), 3, "Flip-only should produce 3 items, got {}", items.len());
        match &items[0].config {
            render_commands::RenderCommandConfig::GroupBegin { visual_rotation, .. } => {
                let vr = visual_rotation.as_ref().expect("Should have rotation config");
                assert!(vr.flip_x);
                assert!(!vr.flip_y);
                assert_eq!(vr.rotation_radians, 0.0);
            }
            other => panic!("Expected GroupBegin, got {:?}", other),
        }
    }

    #[rustfmt::skip]
    #[test]
    fn test_visual_rotation_preserves_bounding_box() {
        let mut ply = Ply::<()>::new_headless(Dimensions::new(800.0, 600.0));
        let mut ui = ply.begin();

        ui.element()
            .width(fixed!(200.0)).height(fixed!(100.0))
            .background_color(0xFF0000)
            .rotate_visual(|r| r.degrees(45.0))
            .empty();

        let items = ui.eval();

        // The rectangle inside should keep original dimensions (layout unaffected)
        let rect = &items[1]; // Rectangle is after GroupBegin
        assert_eq!(rect.bounding_box.width, 200.0);
        assert_eq!(rect.bounding_box.height, 100.0);
    }

    #[rustfmt::skip]
    #[test]
    fn test_visual_rotation_config_values() {
        let mut ply = Ply::<()>::new_headless(Dimensions::new(800.0, 600.0));
        let mut ui = ply.begin();

        ui.element()
            .width(fixed!(100.0)).height(fixed!(100.0))
            .background_color(0xFF0000)
            .rotate_visual(|r| r
                .radians(std::f32::consts::FRAC_PI_2)
                .pivot(0.25, 0.75)
                .flip_x()
                .flip_y()
            )
            .empty();

        let items = ui.eval();

        match &items[0].config {
            render_commands::RenderCommandConfig::GroupBegin { visual_rotation, .. } => {
                let vr = visual_rotation.as_ref().unwrap();
                assert!((vr.rotation_radians - std::f32::consts::FRAC_PI_2).abs() < 0.001);
                assert_eq!(vr.pivot_x, 0.25);
                assert_eq!(vr.pivot_y, 0.75);
                assert!(vr.flip_x);
                assert!(vr.flip_y);
            }
            other => panic!("Expected GroupBegin, got {:?}", other),
        }
    }

    // =====================================================================
    // Shape rotation tests (Phase 2)
    // =====================================================================

    #[rustfmt::skip]
    #[test]
    fn test_shape_rotation_emits_with_rotation() {
        let mut ply = Ply::<()>::new_headless(Dimensions::new(800.0, 600.0));
        let mut ui = ply.begin();

        ui.element()
            .width(fixed!(100.0)).height(fixed!(50.0))
            .background_color(0xFF0000)
            .rotate_shape(|r| r.degrees(45.0))
            .empty();

        let items = ui.eval();

        // Should produce a single Rectangle with shape_rotation
        assert_eq!(items.len(), 1, "Expected 1 item, got {}", items.len());
        let sr = items[0].shape_rotation.as_ref().expect("Should have shape_rotation");
        assert!((sr.rotation_radians - 45.0_f32.to_radians()).abs() < 0.001);
        assert!(!sr.flip_x);
        assert!(!sr.flip_y);
    }

    #[rustfmt::skip]
    #[test]
    fn test_shape_rotation_aabb_90_degrees() {
        // 90° rotation of a 200×100 rect → AABB should be 100×200
        let mut ply = Ply::<()>::new_headless(Dimensions::new(800.0, 600.0));
        let mut ui = ply.begin();

        ui.element().width(grow!()).height(grow!())
            .layout(|l| l)
            .children(|ui| {
                ui.element()
                    .width(fixed!(200.0)).height(fixed!(100.0))
                    .background_color(0xFF0000)
                    .rotate_shape(|r| r.degrees(90.0))
                    .empty();
            });

        let items = ui.eval();

        // Find the rectangle
        let rect = items.iter().find(|i| matches!(i.config, render_commands::RenderCommandConfig::Rectangle(_))).unwrap();
        // The bounding box should have original dims (centered in AABB)
        assert!((rect.bounding_box.width - 200.0).abs() < 0.1, "width should be 200, got {}", rect.bounding_box.width);
        assert!((rect.bounding_box.height - 100.0).abs() < 0.1, "height should be 100, got {}", rect.bounding_box.height);
    }

    #[rustfmt::skip]
    #[test]
    fn test_shape_rotation_aabb_45_degrees_sharp() {
        // 45° rotation of a 100×100 sharp rect → AABB ≈ 141.4×141.4
        let mut ply = Ply::<()>::new_headless(Dimensions::new(800.0, 600.0));
        let mut ui = ply.begin();

        // We need a parent to see the AABB effect on sibling positioning
        ui.element().width(grow!()).height(grow!())
            .layout(|l| l.direction(layout::LayoutDirection::LeftToRight))
            .children(|ui| {
                ui.element()
                    .width(fixed!(100.0)).height(fixed!(100.0))
                    .background_color(0xFF0000)
                    .rotate_shape(|r| r.degrees(45.0))
                    .empty();

                // Second element — its x-position should be offset by ~141.4
                ui.element()
                    .width(fixed!(50.0)).height(fixed!(50.0))
                    .background_color(0x00FF00)
                    .empty();
            });

        let items = ui.eval();

        // Find the green rectangle (second one)
        let rects: Vec<_> = items.iter()
            .filter(|i| matches!(i.config, render_commands::RenderCommandConfig::Rectangle(_)))
            .collect();
        assert!(rects.len() >= 2, "Expected at least 2 rectangles, got {}", rects.len());

        let expected_aabb_w = (2.0_f32.sqrt()) * 100.0; // ~141.42
        let green_x = rects[1].bounding_box.x;
        // Green rect starts at AABB width (since parent starts at x=0)
        assert!((green_x - expected_aabb_w).abs() < 1.0,
            "Green rect x should be ~{}, got {}", expected_aabb_w, green_x);
    }

    #[rustfmt::skip]
    #[test]
    fn test_shape_rotation_aabb_45_degrees_rounded() {
        // 45° rotation of a 100×100 rect with corner radius 10 →
        // AABB = |(100-20)cos45| + |(100-20)sin45| + 20 ≈ 133.14
        let mut ply = Ply::<()>::new_headless(Dimensions::new(800.0, 600.0));
        let mut ui = ply.begin();

        ui.element().width(grow!()).height(grow!())
            .layout(|l| l.direction(layout::LayoutDirection::LeftToRight))
            .children(|ui| {
                ui.element()
                    .width(fixed!(100.0)).height(fixed!(100.0))
                    .corner_radius(10.0)
                    .background_color(0xFF0000)
                    .rotate_shape(|r| r.degrees(45.0))
                    .empty();

                ui.element()
                    .width(fixed!(50.0)).height(fixed!(50.0))
                    .background_color(0x00FF00)
                    .empty();
            });

        let items = ui.eval();

        let rects: Vec<_> = items.iter()
            .filter(|i| matches!(i.config, render_commands::RenderCommandConfig::Rectangle(_)))
            .collect();
        assert!(rects.len() >= 2);

        // Expected: |(100-20)·cos45| + |(100-20)·sin45| + 20 = 80·√2 + 20 ≈ 133.14
        let expected_aabb_w = 80.0 * 2.0_f32.sqrt() + 20.0;
        let green_x = rects[1].bounding_box.x;
        // Green rect starts at AABB width (since parent starts at x=0)
        assert!((green_x - expected_aabb_w).abs() < 1.0,
            "Green rect x should be ~{}, got {}", expected_aabb_w, green_x);
    }

    #[rustfmt::skip]
    #[test]
    fn test_shape_rotation_noop_no_aabb_change() {
        // 0° with no flip = noop, should not change dimensions
        let mut ply = Ply::<()>::new_headless(Dimensions::new(800.0, 600.0));
        let mut ui = ply.begin();

        ui.element()
            .width(fixed!(100.0)).height(fixed!(50.0))
            .background_color(0xFF0000)
            .rotate_shape(|r| r.degrees(0.0))
            .empty();

        let items = ui.eval();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].bounding_box.width, 100.0);
        assert_eq!(items[0].bounding_box.height, 50.0);
        // shape_rotation should still be present (renderer filters noop)
        // Actually noop is filtered at engine level, so it should be None
        assert!(items[0].shape_rotation.is_none(), "Noop shape rotation should be filtered");
    }

    #[rustfmt::skip]
    #[test]
    fn test_shape_rotation_flip_only() {
        // flip_x with 0° — NOT noop, but doesn't change AABB
        let mut ply = Ply::<()>::new_headless(Dimensions::new(800.0, 600.0));
        let mut ui = ply.begin();

        ui.element()
            .width(fixed!(100.0)).height(fixed!(50.0))
            .background_color(0xFF0000)
            .rotate_shape(|r| r.flip_x())
            .empty();

        let items = ui.eval();
        assert_eq!(items.len(), 1);
        let sr = items[0].shape_rotation.as_ref().expect("flip_x should produce shape_rotation");
        assert!(sr.flip_x);
        assert!(!sr.flip_y);
        // AABB unchanged for flip-only
        assert_eq!(items[0].bounding_box.width, 100.0);
        assert_eq!(items[0].bounding_box.height, 50.0);
    }

    #[rustfmt::skip]
    #[test]
    fn test_shape_rotation_180_no_aabb_change() {
        // 180° rotation → AABB same as original
        let mut ply = Ply::<()>::new_headless(Dimensions::new(800.0, 600.0));
        let mut ui = ply.begin();

        ui.element()
            .width(fixed!(200.0)).height(fixed!(100.0))
            .background_color(0xFF0000)
            .rotate_shape(|r| r.degrees(180.0))
            .empty();

        let items = ui.eval();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].bounding_box.width, 200.0);
        assert_eq!(items[0].bounding_box.height, 100.0);
    }

    // =====================================================================
    // Math tests
    // =====================================================================

    #[test]
    fn test_classify_angle() {
        use math::{classify_angle, AngleType};
        assert_eq!(classify_angle(0.0), AngleType::Zero);
        assert_eq!(classify_angle(std::f32::consts::TAU), AngleType::Zero);
        assert_eq!(classify_angle(-std::f32::consts::TAU), AngleType::Zero);
        assert_eq!(classify_angle(std::f32::consts::FRAC_PI_2), AngleType::Right90);
        assert_eq!(classify_angle(std::f32::consts::PI), AngleType::Straight180);
        assert_eq!(classify_angle(3.0 * std::f32::consts::FRAC_PI_2), AngleType::Right270);
        match classify_angle(1.0) {
            AngleType::Arbitrary(v) => assert!((v - 1.0).abs() < 0.01),
            other => panic!("Expected Arbitrary, got {:?}", other),
        }
    }

    #[test]
    fn test_compute_rotated_aabb_zero() {
        use math::compute_rotated_aabb;
        use engine::CornerRadius;
        let cr = CornerRadius::default();
        let (w, h) = compute_rotated_aabb(100.0, 50.0, &cr, 0.0);
        assert_eq!(w, 100.0);
        assert_eq!(h, 50.0);
    }

    #[test]
    fn test_compute_rotated_aabb_90() {
        use math::compute_rotated_aabb;
        use engine::CornerRadius;
        let cr = CornerRadius::default();
        let (w, h) = compute_rotated_aabb(200.0, 100.0, &cr, std::f32::consts::FRAC_PI_2);
        assert!((w - 100.0).abs() < 0.1, "w should be 100, got {}", w);
        assert!((h - 200.0).abs() < 0.1, "h should be 200, got {}", h);
    }

    #[test]
    fn test_compute_rotated_aabb_45_sharp() {
        use math::compute_rotated_aabb;
        use engine::CornerRadius;
        let cr = CornerRadius::default();
        let theta = std::f32::consts::FRAC_PI_4;
        let (w, h) = compute_rotated_aabb(100.0, 100.0, &cr, theta);
        let expected = 100.0 * 2.0_f32.sqrt();
        assert!((w - expected).abs() < 0.5, "w should be ~{}, got {}", expected, w);
        assert!((h - expected).abs() < 0.5, "h should be ~{}, got {}", expected, h);
    }

    #[test]
    fn test_compute_rotated_aabb_45_rounded() {
        use math::compute_rotated_aabb;
        use engine::CornerRadius;
        let cr = CornerRadius { top_left: 10.0, top_right: 10.0, bottom_left: 10.0, bottom_right: 10.0 };
        let theta = std::f32::consts::FRAC_PI_4;
        let (w, h) = compute_rotated_aabb(100.0, 100.0, &cr, theta);
        let expected = 80.0 * 2.0_f32.sqrt() + 20.0; // ~133.14
        assert!((w - expected).abs() < 0.5, "w should be ~{}, got {}", expected, w);
        assert!((h - expected).abs() < 0.5, "h should be ~{}, got {}", expected, h);
    }

    #[test]
    fn test_on_press_callback_fires() {
        use std::cell::RefCell;
        use std::rc::Rc;

        let mut ply = Ply::<()>::new_headless(Dimensions::new(800.0, 600.0));
        let press_count = Rc::new(RefCell::new(0u32));
        let release_count = Rc::new(RefCell::new(0u32));

        // Frame 1: lay out a 100x100 element and eval to establish bounding boxes
        {
            let mut ui = ply.begin();
            ui.element()
                .id("btn")
                .width(fixed!(100.0))
                .height(fixed!(100.0))
                .empty();
            ui.eval();
        }

        // Frame 2: add press callbacks
        {
            let pc = press_count.clone();
            let rc = release_count.clone();
            let mut ui = ply.begin();
            ui.element()
                .id("btn")
                .width(fixed!(100.0))
                .height(fixed!(100.0))
                .on_press(move |_| { *pc.borrow_mut() += 1; })
                .on_release(move |_| { *rc.borrow_mut() += 1; })
                .empty();
            ui.eval();
        }

        // Simulate pointer press at (50, 50) — inside the element
        ply.context.set_pointer_state(Vector2::new(50.0, 50.0), true);
        assert_eq!(*press_count.borrow(), 1, "on_press should fire once");
        assert_eq!(*release_count.borrow(), 0, "on_release should not fire yet");

        // Simulate pointer release
        ply.context.set_pointer_state(Vector2::new(50.0, 50.0), false);
        assert_eq!(*release_count.borrow(), 1, "on_release should fire once");
    }

    #[test]
    fn test_pressed_query() {
        let mut ply = Ply::<()>::new_headless(Dimensions::new(800.0, 600.0));

        // Frame 1: layout
        {
            let mut ui = ply.begin();
            ui.element()
                .id("btn")
                .width(fixed!(100.0))
                .height(fixed!(100.0))
                .empty();
            ui.eval();
        }

        // Simulate pointer press at (50, 50)
        ply.context.set_pointer_state(Vector2::new(50.0, 50.0), true);

        // Frame 2: check pressed() during layout
        {
            let mut ui = ply.begin();
            ui.element()
                .id("btn")
                .width(fixed!(100.0))
                .height(fixed!(100.0))
                .children(|ui| {
                    assert!(ui.pressed(), "element should report as pressed");
                });
            ui.eval();
        }
    }

    #[test]
    fn test_tab_navigation_cycles_focus() {
        let mut ply = Ply::<()>::new_headless(Dimensions::new(800.0, 600.0));

        // Frame 1: create 3 focusable elements
        {
            let mut ui = ply.begin();
            ui.element()
                .id("a")
                .width(fixed!(100.0))
                .height(fixed!(50.0))
                .accessibility(|a| a.button("A"))
                .empty();
            ui.element()
                .id("b")
                .width(fixed!(100.0))
                .height(fixed!(50.0))
                .accessibility(|a| a.button("B"))
                .empty();
            ui.element()
                .id("c")
                .width(fixed!(100.0))
                .height(fixed!(50.0))
                .accessibility(|a| a.button("C"))
                .empty();
            ui.eval();
        }

        let id_a = Id::from("a").id;
        let id_b = Id::from("b").id;
        let id_c = Id::from("c").id;

        // No focus initially
        assert_eq!(ply.focused_element(), None);

        // Tab → focus A
        ply.context.cycle_focus(false);
        assert_eq!(ply.context.focused_element_id, id_a);

        // Tab → focus B
        ply.context.cycle_focus(false);
        assert_eq!(ply.context.focused_element_id, id_b);

        // Tab → focus C
        ply.context.cycle_focus(false);
        assert_eq!(ply.context.focused_element_id, id_c);

        // Tab → wrap to A
        ply.context.cycle_focus(false);
        assert_eq!(ply.context.focused_element_id, id_a);

        // Shift+Tab → wrap to C
        ply.context.cycle_focus(true);
        assert_eq!(ply.context.focused_element_id, id_c);
    }

    #[test]
    fn test_tab_index_ordering() {
        let mut ply = Ply::<()>::new_headless(Dimensions::new(800.0, 600.0));

        // Frame 1: create elements with explicit tab indices (reverse order)
        {
            let mut ui = ply.begin();
            ui.element()
                .id("third")
                .width(fixed!(100.0))
                .height(fixed!(50.0))
                .accessibility(|a| a.button("Third").tab_index(3))
                .empty();
            ui.element()
                .id("first")
                .width(fixed!(100.0))
                .height(fixed!(50.0))
                .accessibility(|a| a.button("First").tab_index(1))
                .empty();
            ui.element()
                .id("second")
                .width(fixed!(100.0))
                .height(fixed!(50.0))
                .accessibility(|a| a.button("Second").tab_index(2))
                .empty();
            ui.eval();
        }

        let id_first = Id::from("first").id;
        let id_second = Id::from("second").id;
        let id_third = Id::from("third").id;

        // Tab ordering should follow tab_index, not insertion order
        ply.context.cycle_focus(false);
        assert_eq!(ply.context.focused_element_id, id_first);
        ply.context.cycle_focus(false);
        assert_eq!(ply.context.focused_element_id, id_second);
        ply.context.cycle_focus(false);
        assert_eq!(ply.context.focused_element_id, id_third);
    }

    #[test]
    fn test_arrow_key_navigation() {
        let mut ply = Ply::<()>::new_headless(Dimensions::new(800.0, 600.0));
        use engine::ArrowDirection;

        let id_a = Id::from("a").id;
        let id_b = Id::from("b").id;

        // Frame 1: create two elements with arrow overrides
        {
            let mut ui = ply.begin();
            ui.element()
                .id("a")
                .width(fixed!(100.0))
                .height(fixed!(50.0))
                .accessibility(|a| a.button("A").focus_right("b"))
                .empty();
            ui.element()
                .id("b")
                .width(fixed!(100.0))
                .height(fixed!(50.0))
                .accessibility(|a| a.button("B").focus_left("a"))
                .empty();
            ui.eval();
        }

        // Focus A first
        ply.context.set_focus(id_a);
        assert_eq!(ply.context.focused_element_id, id_a);

        // Arrow right → B
        ply.context.arrow_focus(ArrowDirection::Right);
        assert_eq!(ply.context.focused_element_id, id_b);

        // Arrow left → A
        ply.context.arrow_focus(ArrowDirection::Left);
        assert_eq!(ply.context.focused_element_id, id_a);

        // Arrow up → no override, stays on A
        ply.context.arrow_focus(ArrowDirection::Up);
        assert_eq!(ply.context.focused_element_id, id_a);
    }

    #[test]
    fn test_focused_query() {
        let mut ply = Ply::<()>::new_headless(Dimensions::new(800.0, 600.0));

        let id_a = Id::from("a").id;

        // Frame 1: layout + set focus
        {
            let mut ui = ply.begin();
            ui.element()
                .id("a")
                .width(fixed!(100.0))
                .height(fixed!(50.0))
                .accessibility(|a| a.button("A"))
                .empty();
            ui.eval();
        }

        ply.context.set_focus(id_a);

        // Frame 2: check focused() during layout
        {
            let mut ui = ply.begin();
            ui.element()
                .id("a")
                .width(fixed!(100.0))
                .height(fixed!(50.0))
                .accessibility(|a| a.button("A"))
                .children(|ui| {
                    assert!(ui.focused(), "element should report as focused");
                });
            ui.eval();
        }
    }

    #[test]
    fn test_on_focus_callback_fires_on_tab() {
        use std::cell::RefCell;
        use std::rc::Rc;

        let mut ply = Ply::<()>::new_headless(Dimensions::new(800.0, 600.0));
        let focus_a = Rc::new(RefCell::new(0u32));
        let unfocus_a = Rc::new(RefCell::new(0u32));
        let focus_b = Rc::new(RefCell::new(0u32));

        // Frame 1: create focusable elements with on_focus/on_unfocus
        {
            let fa = focus_a.clone();
            let ua = unfocus_a.clone();
            let fb = focus_b.clone();
            let mut ui = ply.begin();
            ui.element()
                .id("a")
                .width(fixed!(100.0))
                .height(fixed!(50.0))
                .accessibility(|a| a.button("A"))
                .on_focus(move |_| { *fa.borrow_mut() += 1; })
                .on_unfocus(move |_| { *ua.borrow_mut() += 1; })
                .empty();
            ui.element()
                .id("b")
                .width(fixed!(100.0))
                .height(fixed!(50.0))
                .accessibility(|a| a.button("B"))
                .on_focus(move |_| { *fb.borrow_mut() += 1; })
                .empty();
            ui.eval();
        }

        // Tab → focus A
        ply.context.cycle_focus(false);
        assert_eq!(*focus_a.borrow(), 1, "on_focus should fire for A");
        assert_eq!(*unfocus_a.borrow(), 0, "on_unfocus should not fire yet");

        // Tab → focus B (unfocus A)
        ply.context.cycle_focus(false);
        assert_eq!(*unfocus_a.borrow(), 1, "on_unfocus should fire for A");
        assert_eq!(*focus_b.borrow(), 1, "on_focus should fire for B");
    }

    #[test]
    fn test_on_focus_callback_fires_on_set_focus() {
        use std::cell::RefCell;
        use std::rc::Rc;

        let mut ply = Ply::<()>::new_headless(Dimensions::new(800.0, 600.0));
        let focus_count = Rc::new(RefCell::new(0u32));
        let unfocus_count = Rc::new(RefCell::new(0u32));

        let id_a = Id::from("a").id;

        // Frame 1
        {
            let fc = focus_count.clone();
            let uc = unfocus_count.clone();
            let mut ui = ply.begin();
            ui.element()
                .id("a")
                .width(fixed!(100.0))
                .height(fixed!(50.0))
                .accessibility(|a| a.button("A"))
                .on_focus(move |_| { *fc.borrow_mut() += 1; })
                .on_unfocus(move |_| { *uc.borrow_mut() += 1; })
                .empty();
            ui.eval();
        }

        // Programmatic set_focus
        ply.context.set_focus(id_a);
        assert_eq!(*focus_count.borrow(), 1, "on_focus should fire on set_focus");

        // clear_focus
        ply.context.clear_focus();
        assert_eq!(*unfocus_count.borrow(), 1, "on_unfocus should fire on clear_focus");
    }

    #[test]
    fn test_focus_ring_render_command() {
        use render_commands::RenderCommandConfig;

        let mut ply = Ply::<()>::new_headless(Dimensions::new(800.0, 600.0));
        let id_a = Id::from("a").id;

        // Frame 1: layout
        {
            let mut ui = ply.begin();
            ui.element()
                .id("a")
                .width(fixed!(100.0))
                .height(fixed!(50.0))
                .corner_radius(8.0)
                .accessibility(|a| a.button("A"))
                .empty();
            ui.eval();
        }

        // Set focus
        ply.context.set_focus(id_a);

        // Frame 2: eval to get render commands with focus ring
        {
            let mut ui = ply.begin();
            ui.element()
                .id("a")
                .width(fixed!(100.0))
                .height(fixed!(50.0))
                .corner_radius(8.0)
                .accessibility(|a| a.button("A"))
                .empty();
            let items = ui.eval();

            // Look for a border render command with z_index 32764 (the focus ring)
            let focus_ring = items.iter().find(|cmd| {
                cmd.z_index == 32764 && matches!(cmd.config, RenderCommandConfig::Border(_))
            });
            assert!(focus_ring.is_some(), "Focus ring border should be in render commands");

            let ring = focus_ring.unwrap();
            // Focus ring should be expanded by 2px per side
            assert!(ring.bounding_box.width > 100.0, "Focus ring should be wider than element");
            assert!(ring.bounding_box.height > 50.0, "Focus ring should be taller than element");
        }
    }
}
