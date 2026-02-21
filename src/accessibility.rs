use crate::id::Id;

/// Defines the semantic role of a UI element for screen readers and assistive technologies.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum AccessibilityRole {
    #[default]
    None,
    // Interactive
    Button,
    Link,
    // Text
    Heading {
        level: u8,
    },
    Label,
    StaticText,
    // Input
    TextInput,
    TextArea,
    Checkbox,
    RadioButton,
    Slider,
    // Containers
    Group,
    List,
    ListItem,
    Menu,
    MenuItem,
    MenuBar,
    Tab,
    TabList,
    TabPanel,
    Dialog,
    AlertDialog,
    Toolbar,
    // Media
    Image,
    ProgressBar,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LiveRegionMode {
    /// No live announcements.
    #[default]
    Off,
    /// Screen reader announces changes politely (waits for current speech to finish).
    Polite,
    /// Screen reader announces changes immediately (interrupts current speech).
    Assertive,
}

#[derive(Debug, Clone, Default)]
pub struct AccessibilityConfig {
    pub focusable: bool,
    pub role: AccessibilityRole,
    pub label: String,
    pub description: String,
    pub value: String,
    pub value_min: Option<f32>,
    pub value_max: Option<f32>,
    pub checked: Option<bool>,
    pub tab_index: Option<i32>,
    pub focus_right: Option<u32>,
    pub focus_left: Option<u32>,
    pub focus_up: Option<u32>,
    pub focus_down: Option<u32>,
    pub show_ring: bool,
    pub live_region: LiveRegionMode,
}

impl AccessibilityConfig {
    pub fn new() -> Self {
        Self {
            show_ring: true,
            ..Default::default()
        }
    }
}

pub struct AccessibilityBuilder {
    pub(crate) config: AccessibilityConfig,
}

impl AccessibilityBuilder {
    pub(crate) fn new() -> Self {
        Self {
            config: AccessibilityConfig::new(),
        }
    }

    /// Marks this element as focusable (adds to tab order).
    pub fn focusable(&mut self) -> &mut Self {
        self.config.focusable = true;
        self
    }

    /// Sets role = Button and label in one call.
    pub fn button(&mut self, label: &str) -> &mut Self {
        self.config.role = AccessibilityRole::Button;
        self.config.label = label.to_string();
        self.config.focusable = true;
        self
    }

    /// Sets role = Heading with the given level (1–6) and label.
    pub fn heading(&mut self, label: &str, level: u8) -> &mut Self {
        self.config.role = AccessibilityRole::Heading { level };
        self.config.label = label.to_string();
        self
    }

    /// Sets role = Link and label.
    pub fn link(&mut self, label: &str) -> &mut Self {
        self.config.role = AccessibilityRole::Link;
        self.config.label = label.to_string();
        self.config.focusable = true;
        self
    }

    /// Sets role = StaticText and label. For read-only informational text.
    pub fn static_text(&mut self, label: &str) -> &mut Self {
        self.config.role = AccessibilityRole::StaticText;
        self.config.label = label.to_string();
        self
    }

    /// Sets role = Checkbox, label, and focusable.
    pub fn checkbox(&mut self, label: &str) -> &mut Self {
        self.config.role = AccessibilityRole::Checkbox;
        self.config.label = label.to_string();
        self.config.focusable = true;
        self
    }

    /// Sets role = Slider, label, and focusable.
    pub fn slider(&mut self, label: &str) -> &mut Self {
        self.config.role = AccessibilityRole::Slider;
        self.config.label = label.to_string();
        self.config.focusable = true;
        self
    }

    /// Sets role = Image with an alt-text label.
    pub fn image(&mut self, alt: &str) -> &mut Self {
        self.config.role = AccessibilityRole::Image;
        self.config.label = alt.to_string();
        self
    }

    /// Sets the role explicitly.
    pub fn role(&mut self, role: AccessibilityRole) -> &mut Self {
        self.config.role = role;
        self
    }

    /// Sets the accessible label.
    pub fn label(&mut self, label: &str) -> &mut Self {
        self.config.label = label.to_string();
        self
    }

    /// Sets the accessible description.
    pub fn description(&mut self, desc: &str) -> &mut Self {
        self.config.description = desc.to_string();
        self
    }

    /// Sets the current value (for sliders, progress bars, etc.).
    pub fn value(&mut self, value: &str) -> &mut Self {
        self.config.value = value.to_string();
        self
    }

    /// Sets the minimum value.
    pub fn value_min(&mut self, min: f32) -> &mut Self {
        self.config.value_min = Some(min);
        self
    }

    /// Sets the maximum value.
    pub fn value_max(&mut self, max: f32) -> &mut Self {
        self.config.value_max = Some(max);
        self
    }

    /// Sets the checked state (for checkboxes/radio buttons).
    pub fn checked(&mut self, checked: bool) -> &mut Self {
        self.config.checked = Some(checked);
        self
    }

    /// Sets the explicit tab index. Elements without a tab_index
    /// follow insertion order.
    pub fn tab_index(&mut self, index: i32) -> &mut Self {
        self.config.tab_index = Some(index);
        self
    }

    /// When the right arrow key is pressed while this element is focused,
    /// focus moves to the given target element.
    pub fn focus_right(&mut self, target: impl Into<Id>) -> &mut Self {
        self.config.focus_right = Some(target.into().id);
        self
    }

    /// When the left arrow key is pressed while this element is focused,
    /// focus moves to the given target element.
    pub fn focus_left(&mut self, target: impl Into<Id>) -> &mut Self {
        self.config.focus_left = Some(target.into().id);
        self
    }

    /// When the up arrow key is pressed while this element is focused,
    /// focus moves to the given target element.
    pub fn focus_up(&mut self, target: impl Into<Id>) -> &mut Self {
        self.config.focus_up = Some(target.into().id);
        self
    }

    /// When the down arrow key is pressed while this element is focused,
    /// focus moves to the given target element.
    pub fn focus_down(&mut self, target: impl Into<Id>) -> &mut Self {
        self.config.focus_down = Some(target.into().id);
        self
    }

    /// Disables the automatic focus ring on this element.
    pub fn disable_ring(&mut self) -> &mut Self {
        self.config.show_ring = false;
        self
    }

    /// Sets the live region to polite — screen reader announces changes on next idle.
    pub fn live_region_polite(&mut self) -> &mut Self {
        self.config.live_region = LiveRegionMode::Polite;
        self
    }

    /// Sets the live region to assertive — screen reader interrupts to announce changes.
    pub fn live_region_assertive(&mut self) -> &mut Self {
        self.config.live_region = LiveRegionMode::Assertive;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_button_sets_role_and_focusable() {
        let mut builder = AccessibilityBuilder::new();
        builder.button("Submit");
        assert_eq!(builder.config.role, AccessibilityRole::Button);
        assert_eq!(builder.config.label, "Submit");
        assert!(builder.config.focusable);
        assert!(builder.config.show_ring); // default on
    }

    #[test]
    fn builder_heading_sets_level() {
        let mut builder = AccessibilityBuilder::new();
        builder.heading("Settings", 2);
        assert_eq!(
            builder.config.role,
            AccessibilityRole::Heading { level: 2 }
        );
        assert_eq!(builder.config.label, "Settings");
    }

    #[test]
    fn builder_disable_ring() {
        let mut builder = AccessibilityBuilder::new();
        builder.focusable().disable_ring();
        assert!(builder.config.focusable);
        assert!(!builder.config.show_ring);
    }

    #[test]
    fn builder_focus_directions() {
        let mut builder = AccessibilityBuilder::new();
        builder
            .focusable()
            .focus_right(("next", 0u32))
            .focus_left(("prev", 0u32))
            .focus_up(("above", 0u32))
            .focus_down(("below", 0u32));

        assert_eq!(builder.config.focus_right, Some(Id::from(("next", 0u32)).id));
        assert_eq!(builder.config.focus_left, Some(Id::from(("prev", 0u32)).id));
        assert_eq!(builder.config.focus_up, Some(Id::from(("above", 0u32)).id));
        assert_eq!(builder.config.focus_down, Some(Id::from(("below", 0u32)).id));
    }

    #[test]
    fn builder_slider_properties() {
        let mut builder = AccessibilityBuilder::new();
        builder
            .role(AccessibilityRole::Slider)
            .label("Volume")
            .description("Adjusts the master volume from 0 to 100")
            .value("75")
            .value_min(0.0)
            .value_max(100.0);

        assert_eq!(builder.config.role, AccessibilityRole::Slider);
        assert_eq!(builder.config.label, "Volume");
        assert_eq!(builder.config.value, "75");
        assert_eq!(builder.config.value_min, Some(0.0));
        assert_eq!(builder.config.value_max, Some(100.0));
    }
}
