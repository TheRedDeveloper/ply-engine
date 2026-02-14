#![cfg_attr(not(feature = "std"), no_std)]

pub mod bindings;
pub mod color;
pub mod elements;
pub mod errors;
pub mod id;
pub mod layout;
pub mod math;
pub mod render_commands;
pub mod text;
pub mod renderer;
#[cfg(feature = "text-styling")]
pub mod text_styling;

mod mem;
use core::marker::PhantomData;

pub use crate::bindings::*;
use errors::Error;
use id::Id;
use math::{BoundingBox, Dimensions, Vector2};
use render_commands::RenderCommand;

pub use color::Color;

#[cfg(feature = "std")]
use text::TextConfig;
#[cfg(feature = "std")]
use std::{
    cell::Cell,
    sync::{Mutex, MutexGuard, OnceLock},
};

use text::TextElementConfig;
#[derive(Copy, Clone)]
pub struct Declaration<'render, ImageElementData: 'render, CustomElementData: 'render> {
    id: Option<Id>,
    inner: Clay_ElementDeclaration,
    _phantom: PhantomData<(&'render CustomElementData, &'render ImageElementData)>,
}

impl<'render, ImageElementData: 'render, CustomElementData: 'render>
    Declaration<'render, ImageElementData, CustomElementData>
{
    #[inline]
    pub fn new() -> Self {
        let mut zeroed: Self = crate::mem::zeroed_init();
        zeroed.id = None; // Zeroed doesn't mean None for Option
        zeroed
    }

    #[inline]
    pub fn background_color(&mut self, color: Color) -> &mut Self {
        self.inner.backgroundColor = color.into();
        self
    }

    /// Sets aspect ratio for image elements.
    #[inline]
    pub fn aspect_ratio(&mut self, aspect_ratio: f32) -> &mut Self {
        self.inner.aspectRatio.aspectRatio = aspect_ratio;
        self
    }

    #[inline]
    pub fn clip(&mut self, horizontal: bool, vertical: bool, child_offset: Vector2) -> &mut Self {
        self.inner.clip.horizontal = horizontal;
        self.inner.clip.vertical = vertical;
        self.inner.clip.childOffset = child_offset.into();
        self
    }

    #[inline]
    pub fn id(&mut self, id: Id) -> &mut Self {
        self.id = Some(id);
        self
    }

    #[inline]
    pub fn custom_element(&mut self, data: &'render CustomElementData) -> &mut Self {
        self.inner.custom.customData = data as *const CustomElementData as _;
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

#[cfg(feature = "std")]
unsafe extern "C" fn measure_text_trampoline_user_data<'a, F, T>(
    text_slice: Clay_StringSlice,
    config: *mut Clay_TextElementConfig,
    user_data: *mut core::ffi::c_void,
) -> Clay_Dimensions
where
    F: Fn(&str, &TextConfig, &'a mut T) -> Dimensions + 'a,
    T: 'a,
{
    let text = core::str::from_utf8_unchecked(core::slice::from_raw_parts(
        text_slice.chars as *const u8,
        text_slice.length as _,
    ));

    let closure_and_data: &mut (F, T) = &mut *(user_data as *mut (F, T));
    let text_config = TextConfig::from(*config);
    let (callback, data) = closure_and_data;
    callback(text, &text_config, data).into()
}

#[cfg(feature = "std")]
unsafe extern "C" fn measure_text_trampoline<'a, F>(
    text_slice: Clay_StringSlice,
    config: *mut Clay_TextElementConfig,
    user_data: *mut core::ffi::c_void,
) -> Clay_Dimensions
where
    F: Fn(&str, &TextConfig) -> Dimensions + 'a,
{
    let text = core::str::from_utf8_unchecked(core::slice::from_raw_parts(
        text_slice.chars as *const u8,
        text_slice.length as _,
    ));

    let tuple = &*(user_data as *const (F, usize));
    let text_config = TextConfig::from(*config);
    (tuple.0)(text, &text_config).into()
}

unsafe extern "C" fn error_handler(error_data: Clay_ErrorData) {
    let error: Error = error_data.into();
    panic!("Clay Error: (type: {:?}) {}", error.type_, error.text);
}

#[cfg(feature = "std")]
thread_local! {
    static CLAY_FFI_GUARD_DEPTH: Cell<u32> = const { Cell::new(0) };
}

#[cfg(feature = "std")]
fn clay_ffi_mutex() -> &'static Mutex<()> {
    static CLAY_FFI_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();
    CLAY_FFI_MUTEX.get_or_init(|| Mutex::new(()))
}

#[cfg(feature = "std")]
struct ClayFfiGuard {
    _guard: Option<MutexGuard<'static, ()>>,
}

#[cfg(feature = "std")]
impl ClayFfiGuard {
    fn acquire() -> Self {
        let already_locked = CLAY_FFI_GUARD_DEPTH.with(|depth| {
            let current = depth.get();
            depth.set(current + 1);
            current > 0
        });

        if already_locked {
            Self { _guard: None }
        } else {
            let guard = clay_ffi_mutex().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            Self {
                _guard: Some(guard),
            }
        }
    }
}

#[cfg(feature = "std")]
impl Drop for ClayFfiGuard {
    fn drop(&mut self) {
        CLAY_FFI_GUARD_DEPTH.with(|depth| {
            depth.set(depth.get().saturating_sub(1));
        });
    }
}

#[cfg(not(feature = "std"))]
struct ClayFfiGuard;

#[cfg(not(feature = "std"))]
impl ClayFfiGuard {
    fn acquire() -> Self {
        Self
    }
}

#[allow(dead_code)]
pub struct Clay {
    /// Memory used internally by clay
    #[cfg(feature = "std")]
    _memory: Vec<u8>,
    context: *mut Clay_Context,
    /// Memory used internally by clay. The caller is responsible for managing this memory in
    /// no_std case.
    #[cfg(not(feature = "std"))]
    _memory: *const core::ffi::c_void,
    /// Stores the raw pointer to the callback data for later cleanup
    text_measure_callback: Option<*const core::ffi::c_void>,
}

pub struct ClayLayoutScope<'clay, 'render, ImageElementData, CustomElementData> {
    clay: &'clay mut Clay,
    _phantom: core::marker::PhantomData<(&'render ImageElementData, &'render CustomElementData)>,
    dropped: bool,
    #[cfg(feature = "std")]
    owned_strings: core::cell::RefCell<std::vec::Vec<std::string::String>>,
}

impl<'render, 'clay: 'render, ImageElementData: 'render, CustomElementData: 'render>
    ClayLayoutScope<'clay, 'render, ImageElementData, CustomElementData>
{
    /// Create an element, passing its config and a function to add childrens
    pub fn with<
        F: FnOnce(&mut ClayLayoutScope<'clay, 'render, ImageElementData, CustomElementData>),
    >(
        &mut self,
        declaration: &Declaration<'render, ImageElementData, CustomElementData>,
        f: F,
    ) {
        let _ffi_guard = ClayFfiGuard::acquire();
        unsafe {
            Clay_SetCurrentContext(self.clay.context);
            if let Some(id) = declaration.id {
                Clay__OpenElementWithId(id.id);
            } else {
                Clay__OpenElement();
            }
            Clay__ConfigureOpenElement(declaration.inner);
        }

        f(self);

        unsafe {
            Clay__CloseElement();
        }
    }

    pub fn with_styling<
        G: FnOnce(
            &ClayLayoutScope<'clay, 'render, ImageElementData, CustomElementData>,
        ) -> Declaration<'render, ImageElementData, CustomElementData>,
        F: FnOnce(&ClayLayoutScope<'clay, 'render, ImageElementData, CustomElementData>),
    >(
        &self,
        g: G,
        f: F,
    ) {
        let _ffi_guard = ClayFfiGuard::acquire();
        unsafe {
            Clay_SetCurrentContext(self.clay.context);
        }
        
        let declaration = g(self);
        
        unsafe {
            if let Some(id) = declaration.id {
                Clay__OpenElementWithId(id.id);
            } else {
                Clay__OpenElement();
            }
            Clay__ConfigureOpenElement(declaration.inner);
        }

        f(self);

        unsafe {
            Clay__CloseElement();
        }
    }

    pub fn end(
        &mut self,
    ) -> impl Iterator<Item = RenderCommand<'render, ImageElementData, CustomElementData>> {
        let _ffi_guard = ClayFfiGuard::acquire();
        let array = unsafe {
            Clay_SetCurrentContext(self.clay.context);
            Clay_EndLayout()
        };
        self.dropped = true;
        let slice = unsafe { core::slice::from_raw_parts(array.internalArray, array.length as _) };
        slice
            .iter()
            .map(|command| unsafe { RenderCommand::from_clay_render_command(*command) })
    }

    /// Adds a text element to the current open element or to the root layout.
    /// The string data is copied and stored.
    /// For string literals, use `text_literal()` for better performance (avoids copying).
    /// For dynamic strings, use `text_string()`.
    #[cfg(feature = "std")]
    pub fn text(&self, text: &str, config: TextElementConfig) {
        let owned = std::string::String::from(text);
        self.text_string(owned, config);
    }

    /// Adds a text element from a string that must live until fully used.
    /// Only available in no_std - you must ensure the string lives long enough.
    #[cfg(not(feature = "std"))]
    pub fn text(&self, text: &'render str, config: TextElementConfig) {
        let _ffi_guard = ClayFfiGuard::acquire();
        unsafe {
            Clay_SetCurrentContext(self.clay.context);
            Clay__OpenTextElement(text.into(), config.into())
        };
    }

    /// Adds a text element from a static string literal without copying.
    pub fn text_literal(&self, text: &'static str, config: TextElementConfig) {
        let _ffi_guard = ClayFfiGuard::acquire();
        let clay_string = Clay_String {
            isStaticallyAllocated: true,
            length: text.len() as _,
            chars: text.as_ptr() as _,
        };
        unsafe {
            Clay_SetCurrentContext(self.clay.context);
            Clay__OpenTextElement(clay_string, config.into())
        };
    }

    /// Adds a text element from an owned string that will be stored.
    #[cfg(feature = "std")]
    pub fn text_string(&self, text: std::string::String, config: TextElementConfig) {
        let _ffi_guard = ClayFfiGuard::acquire();
        let mut owned_strings = self.owned_strings.borrow_mut();
        owned_strings.push(text);
        let text_ref = owned_strings.last().unwrap();

        let clay_string = Clay_String {
            isStaticallyAllocated: false,
            length: text_ref.len() as _,
            chars: text_ref.as_ptr() as _,
        };
        unsafe {
            Clay_SetCurrentContext(self.clay.context);
            Clay__OpenTextElement(clay_string, config.into())
        };
    }

    pub fn hovered(&self) -> bool {
        let _ffi_guard = ClayFfiGuard::acquire();
        unsafe {
            Clay_SetCurrentContext(self.clay.context);
            Clay_Hovered()
        }
    }

    pub fn on_hover<F, T>(&self, callback: F, user_data: T)
    where
        F: Fn(Id, Clay_PointerData, &mut T) + 'static
    {
        let _ffi_guard = ClayFfiGuard::acquire();
        let boxed = Box::new((callback, user_data));
        let user_data_ptr = Box::into_raw(boxed) as *mut core::ffi::c_void;

        unsafe extern "C" fn trampoline<F, T>(
            element_id: Clay_ElementId,
            pointer_data: Clay_PointerData,
            user_data: *mut core::ffi::c_void,
        )
        where
            F: Fn(Id, Clay_PointerData, &mut T) + 'static,
        {
            let (callback, data) = &mut *(user_data as *mut (F, T));
            let id = Id { id: element_id };
            callback(id, pointer_data, data);
        }

        unsafe {
            Clay_SetCurrentContext(self.clay.context);
            Clay_OnHover(
                Some(trampoline::<F, T>),
                user_data_ptr,
            );
        }
    }

    pub fn scroll_offset(&self) -> Vector2 {
        let _ffi_guard = ClayFfiGuard::acquire();
        unsafe {
            Clay_SetCurrentContext(self.clay.context);
            Clay_GetScrollOffset().into()
        }
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
            let _ffi_guard = ClayFfiGuard::acquire();
            unsafe {
                Clay_SetCurrentContext(self.clay.context);
                Clay_EndLayout();
            }
        }
    }
}

impl Clay {
    pub fn begin<'render, ImageElementData: 'render, CustomElementData: 'render>(
        &mut self,
    ) -> ClayLayoutScope<'_, 'render, ImageElementData, CustomElementData> {
        let _ffi_guard = ClayFfiGuard::acquire();
        unsafe {
            Clay_SetCurrentContext(self.context);
            Clay_BeginLayout()
        };
        ClayLayoutScope {
            clay: self,
            _phantom: core::marker::PhantomData,
            dropped: false,
            #[cfg(feature = "std")]
            owned_strings: core::cell::RefCell::new(std::vec::Vec::new()),
        }
    }

    #[cfg(feature = "std")]
    pub fn new(dimensions: Dimensions) -> Self {
        let _ffi_guard = ClayFfiGuard::acquire();
        let memory_size = Self::required_memory_size();
        let memory = vec![0; memory_size];
        let context;

        unsafe {
            let arena =
                Clay_CreateArenaWithCapacityAndMemory(memory_size as _, memory.as_ptr() as _);

            context = Clay_Initialize(
                arena,
                dimensions.into(),
                Clay_ErrorHandler {
                    errorHandlerFunction: Some(error_handler),
                    userData: std::ptr::null_mut(),
                },
            );
        }

        Self {
            _memory: memory,
            context,
            text_measure_callback: None,
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
        id::Id::new_index_local(label, 0)
    }

    /// Generates a locally unique indexed ID based on the given `label` and `index`.
    ///
    /// This is useful for differentiating elements within a local scope while keeping their labels consistent.
    #[inline]
    pub fn id_index_local(&self, label: &'static str, index: u32) -> id::Id {
        id::Id::new_index_local(label, index)
    }

    pub fn pointer_over(&self, cfg: Id) -> bool {
        unsafe { Clay_PointerOver(cfg.id) }
    }

    #[cfg(feature = "std")]
    /// Z-sorted list of element IDs that the cursor is currently over
    pub fn pointer_over_ids(&self) -> Vec<Id> {
        unsafe {
            let array = Clay_GetPointerOverIds();
            let slice = core::slice::from_raw_parts(array.internalArray, array.length as _);
            slice.iter().map(|&id| Id { id }).collect()
        }
    }

    #[cfg(not(feature = "std"))]
    pub unsafe fn new_with_memory(dimensions: Dimensions, memory: *mut core::ffi::c_void) -> Self {
        let memory_size = Self::required_memory_size();
        let arena = Clay_CreateArenaWithCapacityAndMemory(memory_size as _, memory);

        let context = Clay_Initialize(
            arena,
            dimensions.into(),
            Clay_ErrorHandler {
                errorHandlerFunction: Some(error_handler),
                userData: core::ptr::null_mut(),
            },
        );

        Self {
            _memory: memory,
            context,
            text_measure_callback: None,
        }
    }

    /// Wrapper for `Clay_MinMemorySize`, returns the minimum required memory by clay
    pub fn required_memory_size() -> usize {
        unsafe { Clay_MinMemorySize() as usize }
    }

    /// Set the callback for text measurement with user data
    #[cfg(feature = "std")]
    pub fn set_measure_text_function_user_data<'clay, F, T>(
        &'clay mut self,
        userdata: T,
        callback: F,
    ) where
        F: Fn(&str, &TextConfig, &'clay mut T) -> Dimensions + 'static,
        T: 'clay,
    {
        // Box the callback and userdata together
        let boxed = Box::new((callback, userdata));

        // Get a raw pointer to the boxed data
        let user_data_ptr = Box::into_raw(boxed) as _;

        // Register the callback with the external C function
        unsafe {
            Self::set_measure_text_function_unsafe(
                measure_text_trampoline_user_data::<F, T>,
                user_data_ptr,
            );
        }

        // Store the raw pointer for later cleanup
        self.text_measure_callback = Some(user_data_ptr as *const core::ffi::c_void);
    }

    /// Set the callback for text measurement
    #[cfg(feature = "std")]
    pub fn set_measure_text_function<F>(&mut self, callback: F)
    where
        F: Fn(&str, &TextConfig) -> Dimensions + 'static,
    {
        // Box the callback and userdata together
        // Tuple here is to prevent Rust ZST optimization from breaking getting a raw pointer
        let boxed = Box::new((callback, 0usize));

        // Get a raw pointer to the boxed data
        let user_data_ptr = Box::into_raw(boxed) as *mut core::ffi::c_void;

        // Register the callback with the external C function
        unsafe {
            Self::set_measure_text_function_unsafe(measure_text_trampoline::<F>, user_data_ptr);
        }

        // Store the raw pointer for later cleanup
        self.text_measure_callback = Some(user_data_ptr as *const core::ffi::c_void);
    }

    /// Set the callback for text measurement with user data.
    /// # Safety
    /// This function is unsafe because it sets a callback function without any error checking
    pub unsafe fn set_measure_text_function_unsafe(
        callback: unsafe extern "C" fn(
            Clay_StringSlice,
            *mut Clay_TextElementConfig,
            *mut core::ffi::c_void,
        ) -> Clay_Dimensions,
        user_data: *mut core::ffi::c_void,
    ) {
        Clay_SetMeasureTextFunction(Some(callback), user_data);
    }

    /// Sets the maximum number of element that clay supports
    /// **Use only if you know what you are doing or your getting errors from clay**
    pub fn max_element_count(&mut self, max_element_count: u32) {
        unsafe {
            Clay_SetMaxElementCount(max_element_count as _);
        }
    }
    /// Sets the capacity of the cache used for text in the measure text function
    /// **Use only if you know what you are doing or your getting errors from clay**
    pub fn max_measure_text_cache_word_count(&self, count: u32) {
        unsafe {
            Clay_SetMaxElementCount(count as _);
        }
    }

    /// Enables or disables the debug mode of clay
    pub fn set_debug_mode(&self, enable: bool) {
        unsafe {
            Clay_SetDebugModeEnabled(enable);
        }
    }

    /// Returns if debug mode is enabled
    pub fn is_debug_mode(&self) -> bool {
        unsafe { Clay_IsDebugModeEnabled() }
    }

    /// Enables or disables culling
    pub fn set_culling(&self, enable: bool) {
        unsafe {
            Clay_SetCullingEnabled(enable);
        }
    }

    /// Sets the dimensions of the global layout, use if, for example the window size you render to
    /// changed
    pub fn set_layout_dimensions(&self, dimensions: Dimensions) {
        unsafe {
            Clay_SetLayoutDimensions(dimensions.into());
        }
    }
    /// Updates the state of the pointer for clay. Used to update scroll containers and for
    /// interactions functions
    pub fn pointer_state(&self, position: Vector2, is_down: bool) {
        unsafe {
            Clay_SetPointerState(position.into(), is_down);
        }
    }
    pub fn update_scroll_containers(
        &self,
        drag_scrolling_enabled: bool,
        scroll_delta: Vector2,
        delta_time: f32,
    ) {
        unsafe {
            Clay_UpdateScrollContainers(drag_scrolling_enabled, scroll_delta.into(), delta_time);
        }
    }

    /// Returns if the current element you are creating is hovered
    pub fn hovered(&self) -> bool {
        unsafe { Clay_Hovered() }
    }

    fn element_data(id: Id) -> Clay_ElementData {
        unsafe { Clay_GetElementData(id.id) }
    }

    pub fn bounding_box(&self, id: Id) -> Option<BoundingBox> {
        let element_data = Self::element_data(id);

        if element_data.found {
            Some(element_data.boundingBox.into())
        } else {
            None
        }
    }
    pub fn scroll_container_data(&self, id: Id) -> Option<Clay_ScrollContainerData> {
        let _ffi_guard = ClayFfiGuard::acquire();
        unsafe {
            Clay_SetCurrentContext(self.context);
            let scroll_container_data = Clay_GetScrollContainerData(id.id);

            if scroll_container_data.found {
                Some(scroll_container_data)
            } else {
                None
            }
        }
    }
}

#[cfg(feature = "std")]
impl Drop for Clay {
    fn drop(&mut self) {
        let _ffi_guard = ClayFfiGuard::acquire();
        unsafe {
            if let Some(ptr) = self.text_measure_callback {
                let _ = Box::from_raw(ptr as *mut (usize, usize));
            }

            if Clay_GetCurrentContext() == self.context {
                Clay_SetCurrentContext(core::ptr::null_mut() as _);
            }
        }
    }
}

impl From<&str> for Clay_String {
    fn from(value: &str) -> Self {
        Self {
            isStaticallyAllocated: true,
            length: value.len() as _,
            chars: value.as_ptr() as _,
        }
    }
}

impl From<Clay_String> for &str {
    fn from(value: Clay_String) -> Self {
        unsafe {
            core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                value.chars as *const u8,
                value.length as _,
            ))
        }
    }
}

impl From<Clay_StringSlice> for &str {
    fn from(value: Clay_StringSlice) -> Self {
        unsafe {
            core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                value.chars as *const u8,
                value.length as _,
            ))
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
