//! Web/WASM accessibility bridge.
//!
//! When compiled for `wasm32`, this module creates a hidden DOM tree that mirrors
//! Ply's accessible elements. Screen readers interact with the hidden DOM while
//! visual rendering stays on the canvas.
//!
//! The bridge uses macroquad's plugin system
//! The JS side lives in `ply_bundle.js` (`ply_accessibility` plugin).

#[cfg(target_arch = "wasm32")]
use crate::accessibility::{AccessibilityRole, LiveRegionMode};

#[cfg(target_arch = "wasm32")]
use std::collections::HashSet;

// ============================================================================
// JS plugin function declarations (resolved at WASM load time)
// ============================================================================

#[cfg(target_arch = "wasm32")]
extern "C" {
    fn ply_a11y_init();
    fn ply_a11y_upsert_node(
        id: u32,
        role_ptr: *const u8,
        role_len: u32,
        label_ptr: *const u8,
        label_len: u32,
        tab_index: i32,
    );
    fn ply_a11y_set_heading_level(id: u32, level: u32);
    fn ply_a11y_set_checked(id: u32, checked: u32);
    fn ply_a11y_set_value(
        id: u32,
        value_ptr: *const u8,
        value_len: u32,
        min: f32,
        max: f32,
    );
    fn ply_a11y_set_live(id: u32, mode: u32);
    fn ply_a11y_remove_node(id: u32);
    fn ply_a11y_set_focus(id: u32);
    fn ply_a11y_clear();
    fn ply_a11y_announce(id: u32, text_ptr: *const u8, text_len: u32);
    fn ply_a11y_set_description(id: u32, desc_ptr: *const u8, desc_len: u32);
    fn ply_a11y_reorder(ids_ptr: *const u32, count: u32);
}

// ============================================================================
// Role mapping
// ============================================================================

#[cfg(target_arch = "wasm32")]
fn role_to_aria_string(role: &AccessibilityRole) -> &'static str {
    match role {
        AccessibilityRole::None => "none",
        AccessibilityRole::Button => "button",
        AccessibilityRole::Link => "link",
        AccessibilityRole::Heading { .. } => "heading",
        AccessibilityRole::Label => "note",
        AccessibilityRole::StaticText => "none",
        AccessibilityRole::TextInput => "textbox",
        AccessibilityRole::TextArea => "textbox",
        AccessibilityRole::Checkbox => "checkbox",
        AccessibilityRole::RadioButton => "radio",
        AccessibilityRole::Slider => "slider",
        AccessibilityRole::Group => "group",
        AccessibilityRole::List => "list",
        AccessibilityRole::ListItem => "listitem",
        AccessibilityRole::Menu => "menu",
        AccessibilityRole::MenuItem => "menuitem",
        AccessibilityRole::MenuBar => "menubar",
        AccessibilityRole::Tab => "tab",
        AccessibilityRole::TabList => "tablist",
        AccessibilityRole::TabPanel => "tabpanel",
        AccessibilityRole::Dialog => "dialog",
        AccessibilityRole::AlertDialog => "alertdialog",
        AccessibilityRole::Toolbar => "toolbar",
        AccessibilityRole::Image => "img",
        AccessibilityRole::ProgressBar => "progressbar",
    }
}

// ============================================================================
// Sync state
// ============================================================================

#[cfg(target_arch = "wasm32")]
pub struct WebAccessibilityState {
    initialized: bool,
    previous_ids: HashSet<u32>,
    previous_focus: u32,
    previous_order: Vec<u32>,
}

#[cfg(target_arch = "wasm32")]
impl Default for WebAccessibilityState {
    fn default() -> Self {
        Self {
            initialized: false,
            previous_ids: HashSet::new(),
            previous_focus: 0,
            previous_order: Vec::new(),
        }
    }
}

// ============================================================================
// Sync function (called each frame after layout)
// ============================================================================

#[cfg(target_arch = "wasm32")]
pub fn sync_accessibility_tree(
    state: &mut WebAccessibilityState,
    accessibility_configs: &std::collections::HashMap<u32, crate::accessibility::AccessibilityConfig>,
    accessibility_element_order: &[u32],
    focused_element_id: u32,
) {
    // Initialize the hidden DOM root on first call
    if !state.initialized {
        unsafe { ply_a11y_init(); }
        state.initialized = true;
    }

    // Track which IDs exist this frame
    let mut current_ids = HashSet::with_capacity(accessibility_configs.len());

    // Iterate in layout order (not HashMap order)
    for &elem_id in accessibility_element_order {
        let config = match accessibility_configs.get(&elem_id) {
            Some(c) => c,
            None => continue,
        };
        current_ids.insert(elem_id);

        let role_str = role_to_aria_string(&config.role);
        let tab_index = if config.focusable {
            config.tab_index.unwrap_or(0)
        } else {
            -1
        };

        unsafe {
            ply_a11y_upsert_node(
                elem_id,
                role_str.as_ptr(),
                role_str.len() as u32,
                config.label.as_ptr(),
                config.label.len() as u32,
                tab_index,
            );
        }

        // Heading level
        if let AccessibilityRole::Heading { level } = &config.role {
            unsafe { ply_a11y_set_heading_level(elem_id, *level as u32); }
        }

        // Checked state
        if let Some(checked) = config.checked {
            unsafe { ply_a11y_set_checked(elem_id, if checked { 1 } else { 0 }); }
        }

        // Value (for sliders, progress bars)
        if !config.value.is_empty() {
            unsafe {
                ply_a11y_set_value(
                    elem_id,
                    config.value.as_ptr(),
                    config.value.len() as u32,
                    config.value_min.unwrap_or(f32::NAN),
                    config.value_max.unwrap_or(f32::NAN),
                );
            }
        }

        // Description
        if !config.description.is_empty() {
            unsafe {
                ply_a11y_set_description(
                    elem_id,
                    config.description.as_ptr(),
                    config.description.len() as u32,
                );
            }
        }

        // Live region
        let live_mode = match config.live_region {
            LiveRegionMode::Off => 0u32,
            LiveRegionMode::Polite => 1,
            LiveRegionMode::Assertive => 2,
        };
        if live_mode > 0 {
            unsafe { ply_a11y_set_live(elem_id, live_mode); }
        }
    }

    // Remove nodes that existed last frame but not this frame
    for old_id in &state.previous_ids {
        if !current_ids.contains(old_id) {
            unsafe { ply_a11y_remove_node(*old_id); }
        }
    }

    // Reorder DOM children to match layout order (only when order changes)
    if accessibility_element_order != state.previous_order.as_slice() {
        unsafe {
            ply_a11y_reorder(
                accessibility_element_order.as_ptr(),
                accessibility_element_order.len() as u32,
            );
        }
        state.previous_order = accessibility_element_order.to_vec();
    }

    // Sync focus: update aria-activedescendant on the canvas
    if focused_element_id != state.previous_focus {
        unsafe { ply_a11y_set_focus(focused_element_id); }
        state.previous_focus = focused_element_id;
    }

    state.previous_ids = current_ids;
}
