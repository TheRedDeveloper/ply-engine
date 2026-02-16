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
pub mod renderer;
#[cfg(feature = "text-styling")]
pub mod text_styling;
#[cfg(feature = "built-in-shaders")]
pub mod built_in_shaders;

use id::Id;
use math::{Dimensions, Vector2};
use render_commands::RenderCommand;
use text::TextConfig;

pub use color::Color;

#[allow(dead_code)]
pub struct Ply<CustomElementData: Clone + Default + std::fmt::Debug = ()> {
    context: engine::PlyContext<CustomElementData>,
    headless: bool,
}

pub struct PlyLayoutScope<'ply, CustomElementData: Clone + Default + std::fmt::Debug = ()> {
    ply: &'ply mut Ply<CustomElementData>,
}

/// Builder for creating elements with closure-based syntax.
/// Methods return `self` by value for chaining. Finalize with `.children()` or `.empty()`.
pub struct ElementBuilder<'ply, CustomElementData: Clone + Default + std::fmt::Debug = ()> {
    ply: &'ply mut Ply<CustomElementData>,
    inner: engine::ElementDeclaration<CustomElementData>,
    id: Option<Id>,
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

    /// Shorthand alias for `background_color`.
    #[inline]
    pub fn color(self, color: impl Into<Color>) -> Self {
        self.background_color(color)
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
    #[inline]
    pub fn image(mut self, data: &'static renderer::GraphicAsset) -> Self {
        self.inner.image_data = Some(data);
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

    /// Finalizes the element with children defined in a closure.
    pub fn children(self, f: impl FnOnce(&mut Ply<CustomElementData>)) -> Id {
        let ElementBuilder { ply, inner, id } = self;
        if let Some(ref id) = id {
            ply.context.open_element_with_id(id);
        } else {
            ply.context.open_element();
        }
        ply.context.configure_open_element(&inner);
        let element_id = ply.context.get_open_element_id();

        f(ply);

        ply.context.close_element();

        Id { id: element_id, ..Default::default() }
    }

    /// Finalizes the element with no children.
    pub fn empty(self) -> Id {
        self.children(|_| {})
    }
}

impl<'ply, CustomElementData: Clone + Default + std::fmt::Debug> core::ops::Deref
    for PlyLayoutScope<'ply, CustomElementData>
{
    type Target = Ply<CustomElementData>;

    fn deref(&self) -> &Self::Target {
        self.ply
    }
}

impl<'ply, CustomElementData: Clone + Default + std::fmt::Debug> core::ops::DerefMut
    for PlyLayoutScope<'ply, CustomElementData>
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.ply
    }
}

impl<CustomElementData: Clone + Default + std::fmt::Debug> Ply<CustomElementData> {
    pub fn begin(
        &mut self,
    ) -> PlyLayoutScope<'_, CustomElementData> {
        if !self.headless {
            self.context.set_layout_dimensions(Dimensions::new(
                macroquad::prelude::screen_width(),
                macroquad::prelude::screen_height(),
            ));
        }

        // Auto-update pointer state from macroquad
        if !self.headless {
            let (mx, my) = macroquad::prelude::mouse_position();
            let is_down = macroquad::prelude::is_mouse_button_down(
                macroquad::prelude::MouseButton::Left,
            );
            self.context.set_pointer_state(Vector2::new(mx, my), is_down);

            let (scroll_x, scroll_y) = macroquad::prelude::mouse_wheel();
            const SCROLL_SPEED: f32 = 20.0;
            self.context.update_scroll_containers(
                true,
                Vector2::new(scroll_x * SCROLL_SPEED, scroll_y * SCROLL_SPEED),
                macroquad::prelude::get_frame_time(),
            );
        }

        self.context.begin_layout();
        PlyLayoutScope {
            ply: self,
        }
    }

    /// Creates a new element builder for configuring and adding an element.
    /// Finalize with `.children(|ui| {...})` or `.empty()`.
    pub fn element(&mut self) -> ElementBuilder<'_, CustomElementData> {
        ElementBuilder {
            ply: self,
            inner: engine::ElementDeclaration::default(),
            id: None,
        }
    }

    /// Adds a text element to the current open element or to the root layout.
    pub fn text(&mut self, text: &str, config_fn: impl FnOnce(&mut TextConfig) -> &mut TextConfig) {
        let mut config = TextConfig::new();
        config_fn(&mut config);
        let text_config_index = self.context.store_text_element_config(config);
        self.context.open_text_element(text, text_config_index);
    }

    pub fn on_hover<F>(&mut self, callback: F)
    where
        F: FnMut(Id, engine::PointerData) + 'static,
    {
        self.context.on_hover(Box::new(callback));
    }

    pub fn scroll_offset(&self) -> Vector2 {
        self.context.get_scroll_offset()
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
        }
    }

    /// Generates a locally unique ID based on the given `label`.
    ///
    /// The ID is unique within a specific local scope but not globally.
    #[inline]
    pub fn id_local(&self, label: &'static str) -> id::Id {
        let parent_id = self.context.get_parent_element_id();
        id::Id::new_index_local_with_parent(label, 0, parent_id)
    }

    /// Generates a locally unique indexed ID based on the given `label` and `index`.
    ///
    /// This is useful for differentiating elements within a local scope while keeping their labels consistent.
    #[inline]
    pub fn id_index_local(&self, label: &'static str, index: u32) -> id::Id {
        let parent_id = self.context.get_parent_element_id();
        id::Id::new_index_local_with_parent(label, index, parent_id)
    }

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

    /// Returns if the current element you are creating is hovered
    pub fn hovered(&self) -> bool {
        self.context.hovered()
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
        result
    }
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
                    .align(crate::layout::LayoutAlignmentX::Left, crate::layout::LayoutAlignmentY::Center)
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
            .layout(|l| l.align(crate::layout::LayoutAlignmentX::Center, crate::layout::LayoutAlignmentY::Center))
            .floating(|f| f
                .attach(crate::elements::FloatingAttachToElement::Root)
                .anchor(crate::elements::FloatingAttachPointType::CenterCenter, crate::elements::FloatingAttachPointType::LeftTop)
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
}
