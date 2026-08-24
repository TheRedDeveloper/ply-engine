//! Pointer input data and state representations.

use crate::math::Vector2;

/// Data representing the current state and position of a pointer (mouse cursor or touch).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointerData {
    pub position: Vector2,
    pub state: PointerState,
}

impl Default for PointerData {
    fn default() -> Self {
        Self {
            position: Vector2::default(),
            state: PointerState::default(),
        }
    }
}

/// The interaction state of a pointer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PointerState {
    #[default]
    Idle,
    PressedThisFrame,
    Pressed,
    ReleasedThisFrame,
}

impl PointerData {
    /// Check if pointer is currently pressed
    pub fn pressed(&self) -> bool {
        matches!(self.state, PointerState::Pressed | PointerState::PressedThisFrame)
    }

    /// Check if pointer is currently released
    pub fn released(&self) -> bool {
        matches!(self.state, PointerState::Idle | PointerState::ReleasedThisFrame)
    }

    /// Check if pointer was just pressed this frame
    pub fn just_pressed(&self) -> bool {
        self.state == PointerState::PressedThisFrame
    }

    /// Check if pointer is just released this frame
    pub fn just_released(&self) -> bool {
        self.state == PointerState::ReleasedThisFrame
    }
}