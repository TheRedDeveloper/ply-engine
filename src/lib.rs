pub mod color;
pub mod elements;
pub mod engine;
pub mod errors;
pub mod id;
pub mod layout;
pub mod math;
pub mod render_commands;
pub mod text;
pub mod renderer;
#[cfg(feature = "text-styling")]
pub mod text_styling;

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
pub struct ElementBuilder<CustomElementData: Clone + Default + std::fmt::Debug = ()> {
    ply_ptr: *mut Ply<CustomElementData>,
    inner: engine::ElementDeclaration<CustomElementData>,
    id: Option<Id>,
}

impl<CustomElementData: Clone + Default + std::fmt::Debug>
    ElementBuilder<CustomElementData>
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
    #[inline]
    pub fn id(mut self, id: Id) -> Self {
        self.id = Some(id);
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
    pub fn image(mut self, data: &'static renderer::Asset) -> Self {
        self.inner.image_data = Some(data);
        self
    }

    /// Finalizes the element with children defined in a closure.
    pub fn children(self, f: impl FnOnce(&mut Ply<CustomElementData>)) -> Id {
        // SAFETY: The raw pointer was obtained from a valid &mut Ply reference
        // in element(). The Ply instance remains valid for the duration of this call.
        unsafe {
            let ply = &mut *self.ply_ptr;
            if let Some(ref id) = self.id {
                ply.context.open_element_with_id(&id.id);
            } else {
                ply.context.open_element();
            }
            ply.context.configure_open_element(&self.inner);
            let element_id = ply.context.get_open_element_id();

            f(ply);

            ply.context.close_element();

            Id { id: engine::ElementId { id: element_id, ..Default::default() } }
        }
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
    pub fn element(&mut self) -> ElementBuilder<CustomElementData> {
        ElementBuilder {
            ply_ptr: self as *mut _,
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
        F: FnMut(engine::ElementId, engine::PointerData) + 'static,
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

    /// Generates a unique ID based on the given `label`.
    ///
    /// This ID is global and must be unique across the entire scope.
    #[inline]
    pub fn id(&self, label: &'static str) -> id::Id {
        id::Id::new(label)
    }

    /// Generates a unique indexed ID based on the given `label` and `index`.
    ///
    /// This is useful when multiple elements share the same label but need distinct IDs.
    #[inline]
    pub fn id_index(&self, label: &'static str, index: u32) -> id::Id {
        id::Id::new_index(label, index)
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
        self.context.pointer_over(cfg.id)
    }

    /// Z-sorted list of element IDs that the cursor is currently over
    pub fn pointer_over_ids(&self) -> Vec<Id> {
        self.context
            .get_pointer_over_ids()
            .iter()
            .map(|id| Id { id: id.clone() })
            .collect()
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
        self.context.get_element_data(id.id)
    }

    pub fn scroll_container_data(&self, id: Id) -> Option<engine::ScrollContainerData> {
        let data = self.context.get_scroll_container_data(id.id);
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
            .id(ui.id("parent_rect"))
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
}
