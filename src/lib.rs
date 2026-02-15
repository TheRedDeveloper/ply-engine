#![cfg_attr(not(feature = "std"), no_std)]

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

use core::marker::PhantomData;

use id::Id;
use math::{Dimensions, Vector2};
use render_commands::RenderCommand;

pub use color::Color;

#[cfg(feature = "std")]
use text::TextConfig;

use text::TextElementConfig;
#[derive(Clone)]
pub struct Declaration<'render, ImageElementData: 'render, CustomElementData: 'render> {
    id: Option<Id>,
    inner: engine::ElementDeclaration,
    _phantom: PhantomData<(&'render CustomElementData, &'render ImageElementData)>,
}

impl<'render, ImageElementData: 'render, CustomElementData: 'render>
    Declaration<'render, ImageElementData, CustomElementData>
{
    #[inline]
    pub fn new() -> Self {
        Self {
            id: None,
            inner: engine::ElementDeclaration::default(),
            _phantom: PhantomData,
        }
    }

    #[inline]
    pub fn background_color(&mut self, color: Color) -> &mut Self {
        self.inner.background_color = color;
        self
    }

    /// Sets aspect ratio for image elements.
    #[inline]
    pub fn aspect_ratio(&mut self, aspect_ratio: f32) -> &mut Self {
        self.inner.aspect_ratio = aspect_ratio;
        self
    }

    #[inline]
    pub fn clip(&mut self, horizontal: bool, vertical: bool, child_offset: Vector2) -> &mut Self {
        self.inner.clip.horizontal = horizontal;
        self.inner.clip.vertical = vertical;
        self.inner.clip.child_offset = child_offset;
        self
    }

    #[inline]
    pub fn id(&mut self, id: Id) -> &mut Self {
        self.id = Some(id);
        self
    }

    #[inline]
    pub fn custom_element(&mut self, data: &'render CustomElementData) -> &mut Self {
        self.inner.custom_data = data as *const CustomElementData as usize;
        self
    }

    #[inline]
    pub fn layout(
        &mut self,
    ) -> layout::LayoutBuilder<'_, 'render, ImageElementData, CustomElementData> {
        layout::LayoutBuilder::new(self)
    }

    #[inline]
    pub fn image(
        &mut self,
    ) -> elements::ImageBuilder<'_, 'render, ImageElementData, CustomElementData> {
        elements::ImageBuilder::new(self)
    }

    #[inline]
    pub fn floating(
        &mut self,
    ) -> elements::FloatingBuilder<'_, 'render, ImageElementData, CustomElementData> {
        elements::FloatingBuilder::new(self)
    }

    #[inline]
    pub fn border(
        &mut self,
    ) -> elements::BorderBuilder<'_, 'render, ImageElementData, CustomElementData> {
        elements::BorderBuilder::new(self)
    }

    #[inline]
    pub fn corner_radius(
        &mut self,
    ) -> elements::CornerRadiusBuilder<'_, 'render, ImageElementData, CustomElementData> {
        elements::CornerRadiusBuilder::new(self)
    }
}

impl<ImageElementData, CustomElementData> Default
    for Declaration<'_, ImageElementData, CustomElementData>
{
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
pub struct Clay {
    context: engine::ClayContext,
}

pub struct ClayLayoutScope<'clay, 'render, ImageElementData, CustomElementData> {
    clay: &'clay mut Clay,
    _phantom: core::marker::PhantomData<(&'render ImageElementData, &'render CustomElementData)>,
    dropped: bool,
    #[cfg(feature = "std")]
    owned_strings: std::vec::Vec<std::string::String>,
}

impl<'render, 'clay: 'render, ImageElementData: 'render, CustomElementData: 'render>
    ClayLayoutScope<'clay, 'render, ImageElementData, CustomElementData>
{
    /// Create an element, passing its config and a function to add children
    pub fn with<
        F: FnOnce(&mut ClayLayoutScope<'clay, 'render, ImageElementData, CustomElementData>),
    >(
        &mut self,
        declaration: &Declaration<'render, ImageElementData, CustomElementData>,
        f: F,
    ) {
        if let Some(id) = declaration.id {
            self.clay.context.open_element_with_id(id.id);
        } else {
            self.clay.context.open_element();
        }
        self.clay.context.configure_open_element(&declaration.inner);

        f(self);

        self.clay.context.close_element();
    }

    pub fn with_styling<
        G: FnOnce(
            &mut ClayLayoutScope<'clay, 'render, ImageElementData, CustomElementData>,
        ) -> Declaration<'render, ImageElementData, CustomElementData>,
        F: FnOnce(&mut ClayLayoutScope<'clay, 'render, ImageElementData, CustomElementData>),
    >(
        &mut self,
        g: G,
        f: F,
    ) {
        let declaration = g(self);

        if let Some(id) = declaration.id {
            self.clay.context.open_element_with_id(id.id);
        } else {
            self.clay.context.open_element();
        }
        self.clay.context.configure_open_element(&declaration.inner);

        f(self);

        self.clay.context.close_element();
    }

    pub fn end(
        &mut self,
    ) -> impl Iterator<Item = RenderCommand<'render, ImageElementData, CustomElementData>> {
        self.dropped = true;
        let commands = self.clay.context.end_layout();
        let mut result = Vec::new();
        for cmd in commands {
            result.push(unsafe { RenderCommand::from_engine_render_command(cmd) });
        }
        result.into_iter()
    }

    /// Adds a text element to the current open element or to the root layout.
    /// The string data is copied and stored.
    /// For string literals, use `text_literal()` for better performance (avoids copying).
    /// For dynamic strings, use `text_string()`.
    #[cfg(feature = "std")]
    pub fn text(&mut self, text: &str, config: TextElementConfig) {
        let owned = std::string::String::from(text);
        self.text_string(owned, config);
    }

    /// Adds a text element from a string that must live until fully used.
    /// Only available in no_std - you must ensure the string lives long enough.
    #[cfg(not(feature = "std"))]
    pub fn text(&mut self, text: &'render str, config: TextElementConfig) {
        let text_config_index = self.clay.context.store_text_element_config(config.into_internal());
        self.clay.context.open_text_element(
            text.as_ptr() as usize,
            text.len() as i32,
            false,
            text_config_index,
        );
    }

    /// Adds a text element from a static string literal without copying.
    pub fn text_literal(&mut self, text: &'static str, config: TextElementConfig) {
        let text_config_index = self.clay.context.store_text_element_config(config.into_internal());
        self.clay.context.open_text_element(
            text.as_ptr() as usize,
            text.len() as i32,
            true,
            text_config_index,
        );
    }

    /// Adds a text element from an owned string that will be stored.
    #[cfg(feature = "std")]
    pub fn text_string(&mut self, text: std::string::String, config: TextElementConfig) {
        let ptr = text.as_ptr() as usize;
        let len = text.len() as i32;
        self.owned_strings.push(text);
        let text_config_index = self.clay.context.store_text_element_config(config.into_internal());
        self.clay.context.open_text_element(ptr, len, false, text_config_index);
    }

    pub fn hovered(&self) -> bool {
        self.clay.context.hovered()
    }

    pub fn on_hover<F>(&mut self, callback: F)
    where
        F: FnMut(engine::ElementId, engine::PointerData) + 'static,
    {
        self.clay.context.on_hover(Box::new(callback));
    }

    pub fn scroll_offset(&self) -> Vector2 {
        self.clay.context.get_scroll_offset()
    }
}

impl<'clay, 'render, ImageElementData, CustomElementData> core::ops::Deref
    for ClayLayoutScope<'clay, 'render, ImageElementData, CustomElementData>
{
    type Target = Clay;

    fn deref(&self) -> &Self::Target {
        self.clay
    }
}

impl<'clay, 'render, ImageElementData, CustomElementData> core::ops::DerefMut
    for ClayLayoutScope<'clay, 'render, ImageElementData, CustomElementData>
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.clay
    }
}

impl<ImageElementData, CustomElementData> Drop
    for ClayLayoutScope<'_, '_, ImageElementData, CustomElementData>
{
    fn drop(&mut self) {
        if !self.dropped {
            self.clay.context.end_layout();
        }
    }
}

impl Clay {
    pub fn begin<'render, ImageElementData: 'render, CustomElementData: 'render>(
        &mut self,
    ) -> ClayLayoutScope<'_, 'render, ImageElementData, CustomElementData> {
        self.context.begin_layout();
        ClayLayoutScope {
            clay: self,
            _phantom: core::marker::PhantomData,
            dropped: false,
            #[cfg(feature = "std")]
            owned_strings: std::vec::Vec::new(),
        }
    }

    #[cfg(feature = "std")]
    pub fn new(dimensions: Dimensions) -> Self {
        Self {
            context: engine::ClayContext::new(dimensions),
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

    #[cfg(feature = "std")]
    /// Z-sorted list of element IDs that the cursor is currently over
    pub fn pointer_over_ids(&self) -> Vec<Id> {
        self.context
            .get_pointer_over_ids()
            .iter()
            .map(|&id| Id { id })
            .collect()
    }

    /// Set the callback for text measurement with user data
    #[cfg(feature = "std")]
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
            move |text: &str, config: &engine::InternalTextElementConfig| -> Dimensions {
                let text_config = TextConfig::from_internal(config);
                callback(text, &text_config, &mut data.borrow_mut())
            },
        ));
    }

    /// Set the callback for text measurement
    #[cfg(feature = "std")]
    pub fn set_measure_text_function<F>(&mut self, callback: F)
    where
        F: Fn(&str, &TextConfig) -> Dimensions + 'static,
    {
        self.context.set_measure_text_function(Box::new(
            move |text: &str, config: &engine::InternalTextElementConfig| -> Dimensions {
                let text_config = TextConfig::from_internal(config);
                callback(text, &text_config)
            },
        ));
    }

    /// Sets the maximum number of elements that clay supports
    /// **Use only if you know what you are doing or you're getting errors from clay**
    pub fn max_element_count(&mut self, max_element_count: u32) {
        self.context.set_max_element_count(max_element_count as i32);
    }

    /// Sets the capacity of the cache used for text in the measure text function
    /// **Use only if you know what you are doing or you're getting errors from clay**
    pub fn max_measure_text_cache_word_count(&mut self, count: u32) {
        self.context.set_max_measure_text_cache_word_count(count as i32);
    }

    /// Enables or disables the debug mode of clay
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

    /// Updates the state of the pointer for clay. Used to update scroll containers and for
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
}


#[cfg(test)]
mod tests {
    use super::*;
    use color::Color;
    use layout::{Padding, Sizing};

    #[rustfmt::skip]
    #[test]
    fn test_begin() {
        let mut clay = Clay::new(Dimensions::new(800.0, 600.0));

        clay.set_measure_text_function(|_, _| {
            Dimensions::new(100.0, 24.0)
        });

        let mut clay = clay.begin::<(), ()>();

        clay.with(&Declaration::new()
            .layout()
                .width(Sizing::Fixed(100.0))
                .height(Sizing::Fixed(100.0))
                .end()
            .background_color(Color::rgb(255., 255., 255.)), |clay|
        {
            clay.with(&Declaration::new()
                .layout()
                    .width(Sizing::Fixed(100.0))
                    .height(Sizing::Fixed(100.0))
                    .end()
                .background_color(Color::rgb(255., 255., 255.)), |clay|
            {
                clay.with(&Declaration::new()
                    .layout()
                        .width(Sizing::Fixed(100.0))
                        .height(Sizing::Fixed(100.0))
                        .end()
                    .background_color(Color::rgb(255., 255., 255.)), |clay|
                    {
                        clay.text_literal("test", TextConfig::new()
                            .color(Color::rgb(255., 255., 255.))
                            .font_size(24)
                            .end());
                    },
                );
            });
        });

        clay.with(&Declaration::new()
            .layout()
                .end()
            .border()
                .color(Color::rgb(255., 255., 0.))
                .all_directions(2)
                .end()
            .corner_radius().all(10.0).end(), |clay|
        {
            clay.with(&Declaration::new()
                .layout()
                    .width(Sizing::Fixed(50.0))
                    .height(Sizing::Fixed(50.0))
                    .end()
                .background_color(Color::rgb(0., 255., 255.)), |_clay| {},
            );
        });

        let items = clay.end().collect::<Vec<_>>();

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
        let mut clay = Clay::new(Dimensions::new(1000.0, 1000.0));

        let mut clay = clay.begin::<(), ()>();

        clay.set_measure_text_function(|_, _| {
            Dimensions::new(100.0, 24.0)
        });

        for &(label, level) in &[("Road", 1), ("Wall", 2), ("Tower", 3)] {
            clay.with(
                &Declaration::new()
                    .layout()
                        .width(grow!())
                        .height(fixed!(36.0))
                        .direction(crate::layout::LayoutDirection::LeftToRight)
                        .child_gap(12)
                        .child_alignment(crate::layout::Alignment::new(
                            crate::layout::LayoutAlignmentX::Left,
                            crate::layout::LayoutAlignmentY::Center,
                        ))
                        .end(),
                |clay| {
                    clay.text_literal(label,
                        TextConfig::new().font_size(18).color(Color::u_rgb(0xFF, 0xFF, 0xFF)).end());
                    clay.with(
                        &Declaration::new()
                            .layout().width(grow!()).height(fixed!(18.0)).end()
                            .corner_radius().all(9.0).end()
                            .background_color(Color::u_rgb(0x55, 0x55, 0x55)),
                        |clay| {
                            clay.with(
                                &Declaration::new()
                                    .layout()
                                        .width(fixed!(300.0 * level as f32 / 3.0))
                                        .height(grow!())
                                    .end()
                                    .corner_radius().all(9.0).end()
                                    .background_color(Color::u_rgb(0x45, 0xA8, 0x5A)),
                                |_| {},
                            );
                        },
                    );
                },
            );
        }

        let items = clay.end().collect::<Vec<_>>();

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
        let mut clay = Clay::new(Dimensions::new(1000.0, 1000.0));

        let mut clay = clay.begin::<(), ()>();

        clay.set_measure_text_function(|_, _| {
            Dimensions::new(100.0, 24.0)
        });

        clay.with(
            &Declaration::new()
                .layout()
                    .width(fixed!(20.0))
                    .height(fixed!(20.0))
                    .child_alignment(crate::layout::Alignment::new(
                        crate::layout::LayoutAlignmentX::Center,
                        crate::layout::LayoutAlignmentY::Center,
                    ))
                .end()
                .floating()
                    .attach_to(crate::elements::FloatingAttachToElement::Root)
                    .attach_points(
                        crate::elements::FloatingAttachPointType::CenterCenter,
                        crate::elements::FloatingAttachPointType::LeftTop,
                    )
                    .offset(Vector2::new(100.0, 150.0))
                    .pointer_capture_mode(crate::elements::PointerCaptureMode::Passthrough)
                    .z_index(110)
                .end()
                .corner_radius().all(10.0).end()
                .background_color(Color::u_rgb(0x44, 0x88, 0xDD)),
            |clay| {
                clay.text_literal(
                    "Re",
                    TextConfig::new()
                        .font_size(6)
                        .color(Color::u_rgb(0xFF, 0xFF, 0xFF))
                        .end(),
                );
            },
        );

        let items = clay.end().collect::<Vec<_>>();

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
        let mut clay = Clay::new(Dimensions::new(800.0, 600.0));

        clay.set_measure_text_function(|_text, _config| {
            Dimensions::default()
        });

        let mut clay = clay.begin::<(), ()>();

        clay.with(&Declaration::new()
            .id(clay.id("parent_rect"))
            .layout()
                .width(Sizing::Fixed(100.0))
                .height(Sizing::Fixed(100.0))
                .padding(Padding::all(10))
                .end()
            .background_color(Color::rgb(255., 255., 255.)), |clay|
        {
            clay.text_literal("test", TextConfig::new()
                .color(Color::rgb(255., 255., 255.))
                .font_size(24)
                .end());

            clay.text(&format!("dynamic str {}", 1234), TextConfig::new()
                .color(Color::rgb(255., 255., 255.))
                .font_size(24)
                .end());

            clay.text_string(format!("String {}", 1234), TextConfig::new()
                .color(Color::rgb(255., 255., 255.))
                .font_size(24)
                .end());
        });

        let _items = clay.end();
    }
}
