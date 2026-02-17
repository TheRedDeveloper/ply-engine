//! Native platform accessibility via AccessKit.
//!
//! On non-WASM platforms, this module creates an AccessKit adapter that exposes
//! Ply's accessible elements to system screen readers (Orca on Linux, VoiceOver
//! on macOS, Narrator/NVDA on Windows).
//!
//! Thread safety: AccessKit handler traits are called from another thread on
//! some platforms (notably Linux/AT-SPI). We use `Arc<Mutex<>>` for shared
//! state between the main loop and the adapter thread.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use accesskit::{
    Action, ActionHandler, ActionRequest, ActivationHandler, Live, Node, NodeId, Role, Toggled,
    Tree, TreeId, TreeUpdate,
};
#[cfg(target_os = "linux")]
use accesskit::DeactivationHandler;

#[allow(unused_imports)]
use crate::accessibility::{AccessibilityConfig, AccessibilityRole, LiveRegionMode};

// ============================================================================
// Constants
// ============================================================================

/// Sentinel NodeId for the root window node.
/// We use u64::MAX to avoid collision with element IDs (which are u32 hash values).
const ROOT_NODE_ID: NodeId = NodeId(u64::MAX);

/// Sentinel NodeId for the document container node.
/// Placed as the sole child of the root window, all accessible elements are its children.
/// This enables structural navigation in screen readers (e.g. Orca's Insert+Z).
const DOCUMENT_NODE_ID: NodeId = NodeId(u64::MAX - 1);

// ============================================================================
// Role mapping: Ply AccessibilityRole → AccessKit Role
// ============================================================================

fn map_role(role: &AccessibilityRole) -> Role {
    match role {
        AccessibilityRole::None => Role::Unknown,
        AccessibilityRole::Button => Role::Button,
        AccessibilityRole::Link => Role::Link,
        AccessibilityRole::Heading { .. } => Role::Heading,
        AccessibilityRole::Label => Role::Label,
        AccessibilityRole::StaticText => Role::Label,
        AccessibilityRole::TextInput => Role::TextInput,
        AccessibilityRole::TextArea => Role::MultilineTextInput,
        AccessibilityRole::Checkbox => Role::CheckBox,
        AccessibilityRole::RadioButton => Role::RadioButton,
        AccessibilityRole::Slider => Role::Slider,
        AccessibilityRole::Group => Role::Group,
        AccessibilityRole::List => Role::List,
        AccessibilityRole::ListItem => Role::ListItem,
        AccessibilityRole::Menu => Role::Menu,
        AccessibilityRole::MenuItem => Role::MenuItem,
        AccessibilityRole::MenuBar => Role::MenuBar,
        AccessibilityRole::Tab => Role::Tab,
        AccessibilityRole::TabList => Role::TabList,
        AccessibilityRole::TabPanel => Role::TabPanel,
        AccessibilityRole::Dialog => Role::Dialog,
        AccessibilityRole::AlertDialog => Role::AlertDialog,
        AccessibilityRole::Toolbar => Role::Toolbar,
        AccessibilityRole::Image => Role::Image,
        AccessibilityRole::ProgressBar => Role::ProgressIndicator,
    }
}

// ============================================================================
// Build an AccessKit Node from a Ply AccessibilityConfig
// ============================================================================

fn build_node(config: &AccessibilityConfig) -> Node {
    let role = map_role(&config.role);
    let mut node = Node::new(role);

    // Label
    if !config.label.is_empty() {
        node.set_label(config.label.as_str());
    }

    // Description
    if !config.description.is_empty() {
        node.set_description(config.description.as_str());
    }

    // Value (text value for sliders, progress bars, etc.)
    if !config.value.is_empty() {
        node.set_value(config.value.as_str());
    }

    // Numeric value bounds (for sliders)
    if let Some(min) = config.value_min {
        node.set_min_numeric_value(min as f64);
    }
    if let Some(max) = config.value_max {
        node.set_max_numeric_value(max as f64);
    }

    // If we have a numeric value, try to parse it
    if !config.value.is_empty() {
        if let Ok(num) = config.value.parse::<f64>() {
            node.set_numeric_value(num);
        }
    }

    // Heading level
    if let AccessibilityRole::Heading { level } = &config.role {
        node.set_level(*level as usize);
    }

    // Checked/toggled state (checkboxes, radio buttons)
    if let Some(checked) = config.checked {
        node.set_toggled(if checked {
            Toggled::True
        } else {
            Toggled::False
        });
    }

    // Live region
    match config.live_region {
        LiveRegionMode::Off => {}
        LiveRegionMode::Polite => {
            node.set_live(Live::Polite);
        }
        LiveRegionMode::Assertive => {
            node.set_live(Live::Assertive);
        }
    }

    // Declare supported actions based on role
    if config.focusable {
        node.add_action(Action::Focus);
    }
    match config.role {
        AccessibilityRole::Button | AccessibilityRole::Link | AccessibilityRole::MenuItem => {
            node.add_action(Action::Click);
        }
        AccessibilityRole::Checkbox | AccessibilityRole::RadioButton => {
            node.add_action(Action::Click);
        }
        AccessibilityRole::Slider => {
            node.add_action(Action::Increment);
            node.add_action(Action::Decrement);
            node.add_action(Action::SetValue);
        }
        _ => {}
    }

    node
}

// ============================================================================
// Build a full TreeUpdate from Ply's accessibility data
// ============================================================================

fn build_tree_update(
    configs: &HashMap<u32, AccessibilityConfig>,
    element_order: &[u32],
    focused_id: u32,
    include_tree: bool,
) -> TreeUpdate {
    let mut nodes: Vec<(NodeId, Node)> = Vec::with_capacity(element_order.len() + 2);

    // Collect child NodeIds for the document container
    let child_ids: Vec<NodeId> = element_order
        .iter()
        .filter(|id| configs.contains_key(id))
        .map(|&id| NodeId(id as u64))
        .collect();

    // Root window → Document → accessible elements
    let mut root_node = Node::new(Role::Window);
    root_node.set_label("Ply Application");
    root_node.set_children(vec![DOCUMENT_NODE_ID]);
    nodes.push((ROOT_NODE_ID, root_node));

    // Document container enables structural navigation in screen readers
    let mut doc_node = Node::new(Role::Document);
    doc_node.set_children(child_ids);
    nodes.push((DOCUMENT_NODE_ID, doc_node));

    // Build child nodes
    for &elem_id in element_order {
        if let Some(config) = configs.get(&elem_id) {
            let node = build_node(config);
            nodes.push((NodeId(elem_id as u64), node));
        }
    }

    // Determine focus: if focused_id is 0 (no focus), point to root
    let focus = if focused_id != 0 && configs.contains_key(&focused_id) {
        NodeId(focused_id as u64)
    } else {
        ROOT_NODE_ID
    };

    let tree = if include_tree {
        let mut t = Tree::new(ROOT_NODE_ID);
        t.toolkit_name = Some("Ply Engine".to_string());
        t.toolkit_version = Some(env!("CARGO_PKG_VERSION").to_string());
        Some(t)
    } else {
        None
    };

    TreeUpdate {
        nodes,
        tree,
        tree_id: TreeId::ROOT,
        focus,
    }
}

// ============================================================================
// Handler implementations
// ============================================================================

/// ActivationHandler: called when an assistive technology activates.
/// Holds a pre-built initial tree so the adapter is immediately ready.
struct PlyActivationHandler {
    initial_tree: Mutex<Option<TreeUpdate>>,
}

impl ActivationHandler for PlyActivationHandler {
    fn request_initial_tree(&mut self) -> Option<TreeUpdate> {
        self.initial_tree
            .lock()
            .ok()
            .and_then(|mut t| t.take())
    }
}

/// ActionHandler: queues incoming screen reader action requests for processing
/// on the main thread during the next eval() cycle.
struct PlyActionHandler {
    queue: Arc<Mutex<Vec<ActionRequest>>>,
}

impl ActionHandler for PlyActionHandler {
    fn do_action(&mut self, request: ActionRequest) {
        if let Ok(mut q) = self.queue.lock() {
            q.push(request);
        }
    }
}

/// DeactivationHandler: called when the assistive technology disconnects.
/// Only used on Linux (AT-SPI); macOS and Windows adapters don't require one.
#[cfg(target_os = "linux")]
struct PlyDeactivationHandler;

#[cfg(target_os = "linux")]
impl DeactivationHandler for PlyDeactivationHandler {
    fn deactivate_accessibility(&mut self) {
        // Nothing to clean up
    }
}

// ============================================================================
// Platform adapter wrapper
// ============================================================================

enum PlatformAdapter {
    #[cfg(target_os = "linux")]
    Unix(accesskit_unix::Adapter),
    #[cfg(target_os = "macos")]
    MacOs(accesskit_macos::SubclassingAdapter),
    #[cfg(target_os = "windows")]
    /// Marker — actual adapter lives in the `WINDOWS_A11Y` static so the
    /// wndproc hook (a plain `fn` pointer) can access it for `WM_GETOBJECT`.
    Windows,
    /// Fallback for platforms without an adapter (e.g. Android/iOS in the future).
    None,
}

// ============================================================================
// Windows: Static adapter state + subclass proc for WM_GETOBJECT
// ============================================================================

#[cfg(target_os = "windows")]
struct WindowsA11yState {
    adapter: accesskit_windows::Adapter,
    activation_handler: PlyActivationHandler,
}

/// The Windows AccessKit adapter must be accessible from the window subclass
/// procedure (a plain `extern "system"` callback), so we store it in a static.
/// The `Mutex` is released *before* calling `.into()` on `handle_wm_getobject`'s
/// return value, which may trigger nested `WM_GETOBJECT` — avoiding deadlock.
#[cfg(target_os = "windows")]
static WINDOWS_A11Y: std::sync::Mutex<Option<WindowsA11yState>> = std::sync::Mutex::new(None);

// Win32 FFI for window subclassing (comctl32.dll).
#[cfg(target_os = "windows")]
#[link(name = "comctl32")]
extern "system" {
    fn SetWindowSubclass(
        hwnd: isize,
        pfn_subclass: unsafe extern "system" fn(isize, u32, usize, isize, usize, usize) -> isize,
        uid_subclass: usize,
        dw_ref_data: usize,
    ) -> i32;
    fn DefSubclassProc(hwnd: isize, msg: u32, wparam: usize, lparam: isize) -> isize;
}

/// Subclass procedure installed on miniquad's window. Forwards
/// `WM_GETOBJECT` to AccessKit and relays focus changes to the adapter.
/// All other messages are passed through to the original window procedure.
#[cfg(target_os = "windows")]
unsafe extern "system" fn a11y_subclass_proc(
    hwnd: isize,
    msg: u32,
    wparam: usize,
    lparam: isize,
    _uid_subclass: usize,
    _dw_ref_data: usize,
) -> isize {
    const WM_GETOBJECT: u32 = 0x003D;
    const WM_SETFOCUS: u32 = 0x0007;
    const WM_KILLFOCUS: u32 = 0x0008;

    match msg {
        WM_GETOBJECT => {
            // Acquire lock → call handle_wm_getobject → release lock → .into()
            // The .into() may trigger a nested WM_GETOBJECT, so the lock must
            // be released first.
            let pending = {
                if let Ok(mut guard) = WINDOWS_A11Y.lock() {
                    if let Some(state) = guard.as_mut() {
                        state.adapter.handle_wm_getobject(
                            accesskit_windows::WPARAM(wparam),
                            accesskit_windows::LPARAM(lparam),
                            &mut state.activation_handler,
                        )
                    } else {
                        None
                    }
                } else {
                    None
                }
            };
            // Lock released — safe to call .into() which may trigger nested WM_GETOBJECT
            if let Some(r) = pending {
                let lresult: accesskit_windows::LRESULT = r.into();
                return lresult.0;
            }
            DefSubclassProc(hwnd, msg, wparam, lparam)
        }
        WM_SETFOCUS | WM_KILLFOCUS => {
            let is_focused = msg == WM_SETFOCUS;
            let pending = {
                if let Ok(mut guard) = WINDOWS_A11Y.lock() {
                    if let Some(state) = guard.as_mut() {
                        state.adapter.update_window_focus_state(is_focused)
                    } else {
                        None
                    }
                } else {
                    None
                }
            };
            if let Some(events) = pending {
                events.raise();
            }
            // Always pass focus messages to the original wndproc
            DefSubclassProc(hwnd, msg, wparam, lparam)
        }
        _ => DefSubclassProc(hwnd, msg, wparam, lparam),
    }
}

// ============================================================================
// Linux: Ensure ScreenReaderEnabled is set on the AT-SPI bus
// ============================================================================

/// On some non-GNOME Wayland compositors, Orca does not set the
/// `org.a11y.Status.ScreenReaderEnabled` property on the session D-Bus bus.
/// AccessKit only creates its AT-SPI bus connection when this property is `true`.
/// This function checks the property and sets it if `IsEnabled` is `true` but
/// `ScreenReaderEnabled` is `false`.
#[cfg(target_os = "linux")]
fn ensure_screen_reader_enabled() {
    use std::process::Command;

    // Check current value of ScreenReaderEnabled
    let sr_output = Command::new("busctl")
        .args([
            "--user",
            "get-property",
            "org.a11y.Bus",
            "/org/a11y/bus",
            "org.a11y.Status",
            "ScreenReaderEnabled",
        ])
        .output();

    let sr_enabled = match &sr_output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            stdout.trim() == "b true"
        }
        Err(_) => return, // busctl not available — nothing we can do
    };

    if sr_enabled {
        // Already true — AccessKit should activate on its own
        return;
    }

    // Check if AT-SPI is enabled at all (IsEnabled)
    let is_output = Command::new("busctl")
        .args([
            "--user",
            "get-property",
            "org.a11y.Bus",
            "/org/a11y/bus",
            "org.a11y.Status",
            "IsEnabled",
        ])
        .output();

    let is_enabled = match &is_output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            stdout.trim() == "b true"
        }
        Err(_) => return,
    };

    if !is_enabled {
        // AT-SPI is not enabled; don't force ScreenReaderEnabled
        return;
    }

    // IsEnabled=true but ScreenReaderEnabled=false.
    // Set ScreenReaderEnabled=true to trigger AccessKit activation.
    let _ = Command::new("busctl")
        .args([
            "--user",
            "set-property",
            "org.a11y.Bus",
            "/org/a11y/bus",
            "org.a11y.Status",
            "ScreenReaderEnabled",
            "b",
            "true",
        ])
        .output();
}

// ============================================================================
// NativeAccessibilityState (owned by Ply)
// ============================================================================

pub struct NativeAccessibilityState {
    adapter: PlatformAdapter,
    action_queue: Arc<Mutex<Vec<ActionRequest>>>,
    initialized: bool,
}

impl Default for NativeAccessibilityState {
    fn default() -> Self {
        Self {
            adapter: PlatformAdapter::None,
            action_queue: Arc::new(Mutex::new(Vec::new())),
            initialized: false,
        }
    }
}

impl NativeAccessibilityState {
    fn initialize(
        &mut self,
        configs: &HashMap<u32, AccessibilityConfig>,
        element_order: &[u32],
        focused_id: u32,
    ) {
        let queue = self.action_queue.clone();
        let initial_tree = build_tree_update(configs, element_order, focused_id, true);

        #[cfg(target_os = "linux")]
        {
            let activation_handler = PlyActivationHandler {
                initial_tree: Mutex::new(Some(initial_tree)),
            };
            let mut adapter = accesskit_unix::Adapter::new(
                activation_handler,
                PlyActionHandler { queue },
                PlyDeactivationHandler,
            );
            // Tell the adapter our window currently has focus
            adapter.update_window_focus_state(true);
            self.adapter = PlatformAdapter::Unix(adapter);

            // Workaround: On some Wayland compositors (e.g. Hyprland), Orca does
            // not set the `ScreenReaderEnabled` D-Bus property to `true` even when
            // running. AccessKit only activates its AT-SPI adapter when this
            // property is `true`. We spawn a background thread that checks the
            // property and sets it if needed, which triggers AccessKit's internal
            // PropertyChanged listener to activate.
            std::thread::spawn(|| {
                // Give AccessKit's event loop time to subscribe to PropertyChanged
                std::thread::sleep(std::time::Duration::from_millis(200));
                ensure_screen_reader_enabled();
            });
        }

        #[cfg(target_os = "macos")]
        {
            // macOS: Use SubclassingAdapter which dynamically subclasses the miniquad
            // NSView to handle NSAccessibility protocol callbacks from VoiceOver.
            //
            // `apple_view()` returns an ObjcId (*mut c_void) for the miniquad view.
            // SubclassingAdapter overrides accessibilityChildren, accessibilityFocusedUIElement,
            // and accessibilityHitTest: on the view's class.
            let view = macroquad::miniquad::window::apple_view() as *mut std::ffi::c_void;
            let activation_handler = PlyActivationHandler {
                initial_tree: Mutex::new(Some(initial_tree)),
            };
            let mut adapter = unsafe {
                accesskit_macos::SubclassingAdapter::new(
                    view,
                    activation_handler,
                    PlyActionHandler { queue },
                )
            };
            // Notify the adapter that our view currently has focus
            if let Some(events) = adapter.update_view_focus_state(true) {
                events.raise();
            }
            self.adapter = PlatformAdapter::MacOs(adapter);
        }

        #[cfg(target_os = "windows")]
        {
            // Windows: Use the raw Adapter with a window subclass to intercept
            // WM_GETOBJECT messages sent by screen readers (Narrator/NVDA).
            //
            // We cannot use AccessKit's SubclassingAdapter because miniquad
            // calls ShowWindow() before our code runs, and SubclassingAdapter
            // panics if the window is already visible (IsWindowVisible check).
            //
            // Instead we use SetWindowSubclass (comctl32) to install our own
            // subclass procedure that forwards WM_GETOBJECT to AccessKit.
            // The adapter and activation handler live in the WINDOWS_A11Y static
            // so the subclass proc (a plain extern "system" fn) can access them.
            let hwnd_ptr = macroquad::miniquad::window::windows_hwnd();
            let hwnd = accesskit_windows::HWND(hwnd_ptr);
            let adapter = accesskit_windows::Adapter::new(
                hwnd,
                true, // window starts focused
                PlyActionHandler { queue },
            );
            let activation_handler = PlyActivationHandler {
                initial_tree: Mutex::new(Some(initial_tree)),
            };
            *WINDOWS_A11Y.lock().unwrap() = Some(WindowsA11yState {
                adapter,
                activation_handler,
            });
            // Install the subclass so WM_GETOBJECT is forwarded to AccessKit
            unsafe {
                SetWindowSubclass(
                    hwnd_ptr as isize,
                    a11y_subclass_proc,
                    0xA11E, // arbitrary subclass ID
                    0,
                );
            }
            self.adapter = PlatformAdapter::Windows;
        }

        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            let _ = (queue, initial_tree);
            self.adapter = PlatformAdapter::None;
        }

        self.initialized = true;
    }
}

/// An action requested by the screen reader, to be processed by the engine.
pub enum PendingA11yAction {
    /// Set keyboard focus to the element with this u32 ID.
    Focus(u32),
    /// Fire the on_press callback for the element with this u32 ID.
    Click(u32),
}

// ============================================================================
// Sync function (called each frame after layout, mirrors the web version)
// ============================================================================

/// Synchronise Ply's accessibility state with the platform screen reader.
///
/// This is the native equivalent of `accessibility_web::sync_accessibility_tree`.
/// It is called from `Ply::eval()` on every frame when the `native-a11y` feature
/// is enabled and we are **not** compiling for WASM.
///
/// Returns a list of actions requested by the screen reader (focus changes,
/// clicks) that the engine should process.
pub fn sync_accessibility_tree(
    state: &mut NativeAccessibilityState,
    accessibility_configs: &HashMap<u32, AccessibilityConfig>,
    accessibility_element_order: &[u32],
    focused_element_id: u32,
) -> Vec<PendingA11yAction> {
    // Lazy-initialize the platform adapter on first call
    if !state.initialized {
        state.initialize(accessibility_configs, accessibility_element_order, focused_element_id);
    }

    // Process any queued action requests from the screen reader
    let pending_actions: Vec<ActionRequest> = {
        if let Ok(mut q) = state.action_queue.lock() {
            q.drain(..).collect()
        } else {
            Vec::new()
        }
    };

    // Convert AccessKit actions into engine-level actions
    let mut result = Vec::new();
    for action in &pending_actions {
        // Skip sentinel nodes (root window, document container)
        let target = action.target_node.0;
        if target == ROOT_NODE_ID.0 || target == DOCUMENT_NODE_ID.0 {
            continue;
        }
        let target_id = target as u32;
        match action.action {
            Action::Focus => {
                result.push(PendingA11yAction::Focus(target_id));
            }
            Action::Click => {
                result.push(PendingA11yAction::Click(target_id));
            }
            _ => {}
        }
    }

    // Build and push the tree update to the platform adapter
    let update = build_tree_update(
        accessibility_configs,
        accessibility_element_order,
        focused_element_id,
        false,
    );

    match &mut state.adapter {
        #[cfg(target_os = "linux")]
        PlatformAdapter::Unix(adapter) => {
            adapter.update_if_active(|| update);
        }
        #[cfg(target_os = "macos")]
        PlatformAdapter::MacOs(adapter) => {
            if let Some(events) = adapter.update_if_active(|| update) {
                events.raise();
            }
        }
        #[cfg(target_os = "windows")]
        PlatformAdapter::Windows => {
            // Access the adapter through the static (same one the wndproc hook uses)
            let pending = {
                let mut guard = WINDOWS_A11Y.lock().unwrap();
                if let Some(state) = guard.as_mut() {
                    state.adapter.update_if_active(|| update)
                } else {
                    None
                }
            };
            if let Some(events) = pending {
                events.raise();
            }
        }
        PlatformAdapter::None => {
            let _ = update;
        }
    }

    result
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::accessibility::{AccessibilityConfig, AccessibilityRole, LiveRegionMode};

    fn make_config(role: AccessibilityRole, label: &str) -> AccessibilityConfig {
        AccessibilityConfig {
            focusable: true,
            role,
            label: label.to_string(),
            show_ring: true,
            ..Default::default()
        }
    }

    #[test]
    fn role_mapping_covers_all_variants() {
        // Ensure every AccessibilityRole maps to a non-panicking Role
        let roles = vec![
            AccessibilityRole::None,
            AccessibilityRole::Button,
            AccessibilityRole::Link,
            AccessibilityRole::Heading { level: 1 },
            AccessibilityRole::Label,
            AccessibilityRole::StaticText,
            AccessibilityRole::TextInput,
            AccessibilityRole::TextArea,
            AccessibilityRole::Checkbox,
            AccessibilityRole::RadioButton,
            AccessibilityRole::Slider,
            AccessibilityRole::Group,
            AccessibilityRole::List,
            AccessibilityRole::ListItem,
            AccessibilityRole::Menu,
            AccessibilityRole::MenuItem,
            AccessibilityRole::MenuBar,
            AccessibilityRole::Tab,
            AccessibilityRole::TabList,
            AccessibilityRole::TabPanel,
            AccessibilityRole::Dialog,
            AccessibilityRole::AlertDialog,
            AccessibilityRole::Toolbar,
            AccessibilityRole::Image,
            AccessibilityRole::ProgressBar,
        ];
        for role in roles {
            let _ = map_role(&role);
        }
    }

    #[test]
    fn build_node_button() {
        let config = make_config(AccessibilityRole::Button, "Click me");
        let node = build_node(&config);
        assert_eq!(node.role(), Role::Button);
        assert_eq!(node.label(), Some("Click me"));
    }

    #[test]
    fn build_node_heading_with_level() {
        let config = make_config(AccessibilityRole::Heading { level: 2 }, "Section");
        let node = build_node(&config);
        assert_eq!(node.role(), Role::Heading);
        assert_eq!(node.level(), Some(2));
        assert_eq!(node.label(), Some("Section"));
    }

    #[test]
    fn build_node_checkbox_toggled() {
        let mut config = make_config(AccessibilityRole::Checkbox, "Agree");
        config.checked = Some(true);
        let node = build_node(&config);
        assert_eq!(node.role(), Role::CheckBox);
        assert_eq!(node.toggled(), Some(Toggled::True));
    }

    #[test]
    fn build_node_slider_values() {
        let mut config = make_config(AccessibilityRole::Slider, "Volume");
        config.value = "50".to_string();
        config.value_min = Some(0.0);
        config.value_max = Some(100.0);
        let node = build_node(&config);
        assert_eq!(node.role(), Role::Slider);
        assert_eq!(node.numeric_value(), Some(50.0));
        assert_eq!(node.min_numeric_value(), Some(0.0));
        assert_eq!(node.max_numeric_value(), Some(100.0));
    }

    #[test]
    fn build_node_live_region() {
        let mut config = make_config(AccessibilityRole::Label, "Status");
        config.live_region = LiveRegionMode::Polite;
        let node = build_node(&config);
        assert_eq!(node.live(), Some(Live::Polite));
    }

    #[test]
    fn build_node_description() {
        let mut config = make_config(AccessibilityRole::Button, "Submit");
        config.description = "Submit the form".to_string();
        let node = build_node(&config);
        assert_eq!(node.description(), Some("Submit the form"));
    }

    #[test]
    fn build_tree_update_structure() {
        let mut configs = HashMap::new();
        configs.insert(101, make_config(AccessibilityRole::Button, "OK"));
        configs.insert(102, make_config(AccessibilityRole::Button, "Cancel"));

        let order = vec![101, 102];
        let update = build_tree_update(&configs, &order, 101, true);

        // Should have root + document + 2 children = 4 nodes
        assert_eq!(update.nodes.len(), 4);

        // Root should be first
        assert_eq!(update.nodes[0].0, ROOT_NODE_ID);
        assert_eq!(update.nodes[0].1.role(), Role::Window);

        // Document container should be second
        assert_eq!(update.nodes[1].0, DOCUMENT_NODE_ID);
        assert_eq!(update.nodes[1].1.role(), Role::Document);

        // Focus should be on element 101
        assert_eq!(update.focus, NodeId(101));

        // Tree metadata
        let tree = update.tree.as_ref().unwrap();
        assert_eq!(tree.root, ROOT_NODE_ID);
        assert_eq!(tree.toolkit_name, Some("Ply Engine".to_string()));
    }

    #[test]
    fn build_tree_update_no_focus() {
        let configs = HashMap::new();
        let order = vec![];
        let update = build_tree_update(&configs, &order, 0, true);

        // Only root + document nodes
        assert_eq!(update.nodes.len(), 2);
        // Focus falls back to root
        assert_eq!(update.focus, ROOT_NODE_ID);
    }

    #[test]
    fn default_state_is_uninitialized() {
        let state = NativeAccessibilityState::default();
        assert!(!state.initialized);
    }
}
