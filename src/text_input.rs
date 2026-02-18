use crate::color::Color;

/// Persistent text editing state per text input element.
/// Keyed by element `u32` ID in `PlyContext::text_edit_states`.
#[derive(Debug, Clone)]
pub struct TextEditState {
    /// The current text content.
    pub text: String,
    /// Character index of the cursor (0 = before first char, text.chars().count() = after last).
    pub cursor_pos: usize,
    /// When `Some`, defines the anchor of a selection range (anchor..cursor_pos or cursor_pos..anchor).
    pub selection_anchor: Option<usize>,
    /// Horizontal scroll offset (pixels) when text overflows the bounding box.
    pub scroll_offset: f32,
    /// Vertical scroll offset (pixels) for multiline text inputs.
    pub scroll_offset_y: f32,
    /// Timer for cursor blink animation (seconds).
    pub cursor_blink_timer: f64,
    /// Timestamp of last click (for double-click detection).
    pub last_click_time: f64,
    /// Element ID of last click (for double-click detection).
    pub last_click_element: u32,
}

impl Default for TextEditState {
    fn default() -> Self {
        Self {
            text: String::new(),
            cursor_pos: 0,
            selection_anchor: None,
            scroll_offset: 0.0,
            scroll_offset_y: 0.0,
            cursor_blink_timer: 0.0,
            last_click_time: 0.0,
            last_click_element: 0,
        }
    }
}

impl TextEditState {
    /// Returns the ordered selection range `(start, end)` if a selection is active.
    pub fn selection_range(&self) -> Option<(usize, usize)> {
        self.selection_anchor.map(|anchor| {
            let start = anchor.min(self.cursor_pos);
            let end = anchor.max(self.cursor_pos);
            (start, end)
        })
    }

    /// Returns the selected text, or empty string if no selection.
    pub fn selected_text(&self) -> &str {
        if let Some((start, end)) = self.selection_range() {
            let byte_start = char_index_to_byte(&self.text, start);
            let byte_end = char_index_to_byte(&self.text, end);
            &self.text[byte_start..byte_end]
        } else {
            ""
        }
    }

    /// Delete the current selection and place cursor at the start.
    /// Returns true if a selection was deleted.
    pub fn delete_selection(&mut self) -> bool {
        if let Some((start, end)) = self.selection_range() {
            let byte_start = char_index_to_byte(&self.text, start);
            let byte_end = char_index_to_byte(&self.text, end);
            self.text.drain(byte_start..byte_end);
            self.cursor_pos = start;
            self.selection_anchor = None;
            true
        } else {
            false
        }
    }

    /// Insert text at the current cursor position, replacing any selection.
    /// Respects max_length if provided.
    pub fn insert_text(&mut self, s: &str, max_length: Option<usize>) {
        self.delete_selection();
        let char_count = self.text.chars().count();
        let insert_count = s.chars().count();
        let allowed = if let Some(max) = max_length {
            if char_count >= max {
                0
            } else {
                insert_count.min(max - char_count)
            }
        } else {
            insert_count
        };
        if allowed == 0 {
            return;
        }
        let insert_str: String = s.chars().take(allowed).collect();
        let byte_pos = char_index_to_byte(&self.text, self.cursor_pos);
        self.text.insert_str(byte_pos, &insert_str);
        self.cursor_pos += allowed;
        self.reset_blink();
    }

    /// Move cursor left by one character.
    pub fn move_left(&mut self, shift: bool) {
        if !shift {
            // If there's a selection and no shift, collapse to start
            if let Some((start, _end)) = self.selection_range() {
                self.cursor_pos = start;
                self.selection_anchor = None;
                self.reset_blink();
                return;
            }
        }
        if self.cursor_pos > 0 {
            if shift && self.selection_anchor.is_none() {
                self.selection_anchor = Some(self.cursor_pos);
            }
            self.cursor_pos -= 1;
            if shift {
                // If anchor equals cursor, clear selection
                if self.selection_anchor == Some(self.cursor_pos) {
                    self.selection_anchor = None;
                }
            }
        }
        if !shift {
            self.selection_anchor = None;
        }
        self.reset_blink();
    }

    /// Move cursor right by one character.
    pub fn move_right(&mut self, shift: bool) {
        let len = self.text.chars().count();
        if !shift {
            // If there's a selection and no shift, collapse to end
            if let Some((_start, end)) = self.selection_range() {
                self.cursor_pos = end;
                self.selection_anchor = None;
                self.reset_blink();
                return;
            }
        }
        if self.cursor_pos < len {
            if shift && self.selection_anchor.is_none() {
                self.selection_anchor = Some(self.cursor_pos);
            }
            self.cursor_pos += 1;
            if shift {
                if self.selection_anchor == Some(self.cursor_pos) {
                    self.selection_anchor = None;
                }
            }
        }
        if !shift {
            self.selection_anchor = None;
        }
        self.reset_blink();
    }

    /// Move cursor to the start of the previous word.
    pub fn move_word_left(&mut self, shift: bool) {
        if shift && self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor_pos);
        }
        self.cursor_pos = find_word_boundary_left(&self.text, self.cursor_pos);
        if !shift {
            self.selection_anchor = None;
        } else if self.selection_anchor == Some(self.cursor_pos) {
            self.selection_anchor = None;
        }
        self.reset_blink();
    }

    /// Move cursor to the end of the next word.
    pub fn move_word_right(&mut self, shift: bool) {
        if shift && self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor_pos);
        }
        self.cursor_pos = find_word_boundary_right(&self.text, self.cursor_pos);
        if !shift {
            self.selection_anchor = None;
        } else if self.selection_anchor == Some(self.cursor_pos) {
            self.selection_anchor = None;
        }
        self.reset_blink();
    }

    /// Move cursor to start of line.
    pub fn move_home(&mut self, shift: bool) {
        if shift && self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor_pos);
        }
        self.cursor_pos = 0;
        if !shift {
            self.selection_anchor = None;
        } else if self.selection_anchor == Some(0) {
            self.selection_anchor = None;
        }
        self.reset_blink();
    }

    /// Move cursor to end of line.
    pub fn move_end(&mut self, shift: bool) {
        let len = self.text.chars().count();
        if shift && self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor_pos);
        }
        self.cursor_pos = len;
        if !shift {
            self.selection_anchor = None;
        } else if self.selection_anchor == Some(len) {
            self.selection_anchor = None;
        }
        self.reset_blink();
    }

    /// Select all text.
    pub fn select_all(&mut self) {
        let len = self.text.chars().count();
        if len > 0 {
            self.selection_anchor = Some(0);
            self.cursor_pos = len;
        }
        self.reset_blink();
    }

    /// Delete character before cursor (Backspace).
    pub fn backspace(&mut self) {
        if self.delete_selection() {
            return;
        }
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
            let byte_pos = char_index_to_byte(&self.text, self.cursor_pos);
            let next_byte = char_index_to_byte(&self.text, self.cursor_pos + 1);
            self.text.drain(byte_pos..next_byte);
        }
        self.reset_blink();
    }

    /// Delete character after cursor (Delete key).
    pub fn delete_forward(&mut self) {
        if self.delete_selection() {
            return;
        }
        let len = self.text.chars().count();
        if self.cursor_pos < len {
            let byte_pos = char_index_to_byte(&self.text, self.cursor_pos);
            let next_byte = char_index_to_byte(&self.text, self.cursor_pos + 1);
            self.text.drain(byte_pos..next_byte);
        }
        self.reset_blink();
    }

    /// Delete the word before the cursor (Ctrl+Backspace).
    pub fn backspace_word(&mut self) {
        if self.delete_selection() {
            return;
        }
        let target = find_word_boundary_left(&self.text, self.cursor_pos);
        let byte_start = char_index_to_byte(&self.text, target);
        let byte_end = char_index_to_byte(&self.text, self.cursor_pos);
        self.text.drain(byte_start..byte_end);
        self.cursor_pos = target;
        self.reset_blink();
    }

    /// Delete the word after the cursor (Ctrl+Delete).
    pub fn delete_word_forward(&mut self) {
        if self.delete_selection() {
            return;
        }
        let target = find_word_boundary_right(&self.text, self.cursor_pos);
        let byte_start = char_index_to_byte(&self.text, self.cursor_pos);
        let byte_end = char_index_to_byte(&self.text, target);
        self.text.drain(byte_start..byte_end);
        self.reset_blink();
    }

    /// Set cursor position from a click at pixel x within the element.
    /// `char_x_positions` should be a sorted list of x-positions for each character boundary
    /// (index 0 = left edge of first char, index n = right edge of last char).
    pub fn click_to_cursor(&mut self, click_x: f32, char_x_positions: &[f32], shift: bool) {
        let new_pos = find_nearest_char_boundary(click_x, char_x_positions);
        if shift {
            if self.selection_anchor.is_none() {
                self.selection_anchor = Some(self.cursor_pos);
            }
        } else {
            self.selection_anchor = None;
        }
        self.cursor_pos = new_pos;
        if shift {
            if self.selection_anchor == Some(self.cursor_pos) {
                self.selection_anchor = None;
            }
        }
        self.reset_blink();
    }

    /// Select the word at the given character position (for double-click).
    pub fn select_word_at(&mut self, char_pos: usize) {
        let (start, end) = find_word_at(&self.text, char_pos);
        if start != end {
            self.selection_anchor = Some(start);
            self.cursor_pos = end;
        }
        self.reset_blink();
    }

    /// Reset blink timer so cursor is immediately visible.
    pub fn reset_blink(&mut self) {
        self.cursor_blink_timer = 0.0;
    }

    /// Returns whether the cursor should be visible based on blink timer.
    pub fn cursor_visible(&self) -> bool {
        (self.cursor_blink_timer % 1.06) < 0.53
    }

    /// Update scroll offset to ensure cursor is visible within `visible_width`.
    /// `cursor_x` is the pixel x-position of the cursor relative to text start.
    pub fn ensure_cursor_visible(&mut self, cursor_x: f32, visible_width: f32) {
        if cursor_x - self.scroll_offset > visible_width {
            self.scroll_offset = cursor_x - visible_width;
        }
        if cursor_x - self.scroll_offset < 0.0 {
            self.scroll_offset = cursor_x;
        }
        // Clamp scroll_offset to valid range
        if self.scroll_offset < 0.0 {
            self.scroll_offset = 0.0;
        }
    }

    /// Update vertical scroll offset to keep cursor visible in multiline mode.
    /// `cursor_line` is the 0-based line index the cursor is on.
    /// `line_height` is pixel height per line. `visible_height` is the element height.
    pub fn ensure_cursor_visible_vertical(&mut self, cursor_line: usize, line_height: f32, visible_height: f32) {
        let cursor_y = cursor_line as f32 * line_height;
        let cursor_bottom = cursor_y + line_height;
        if cursor_bottom - self.scroll_offset_y > visible_height {
            self.scroll_offset_y = cursor_bottom - visible_height;
        }
        if cursor_y - self.scroll_offset_y < 0.0 {
            self.scroll_offset_y = cursor_y;
        }
        if self.scroll_offset_y < 0.0 {
            self.scroll_offset_y = 0.0;
        }
    }

    /// Move cursor to the start of the current line (Home in multiline mode).
    pub fn move_line_home(&mut self, shift: bool) {
        if shift && self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor_pos);
        }
        let target = line_start_char_pos(&self.text, self.cursor_pos);
        self.cursor_pos = target;
        if !shift {
            self.selection_anchor = None;
        } else if self.selection_anchor == Some(self.cursor_pos) {
            self.selection_anchor = None;
        }
        self.reset_blink();
    }

    /// Move cursor to the end of the current line (End in multiline mode).
    pub fn move_line_end(&mut self, shift: bool) {
        if shift && self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor_pos);
        }
        let target = line_end_char_pos(&self.text, self.cursor_pos);
        self.cursor_pos = target;
        if !shift {
            self.selection_anchor = None;
        } else if self.selection_anchor == Some(self.cursor_pos) {
            self.selection_anchor = None;
        }
        self.reset_blink();
    }

    /// Move cursor up one line (multiline only).
    pub fn move_up(&mut self, shift: bool) {
        let (line, col) = line_and_column(&self.text, self.cursor_pos);
        if line == 0 {
            // Already on first line — move to start
            if shift && self.selection_anchor.is_none() {
                self.selection_anchor = Some(self.cursor_pos);
            }
            self.cursor_pos = 0;
            if !shift {
                self.selection_anchor = None;
            } else if self.selection_anchor == Some(self.cursor_pos) {
                self.selection_anchor = None;
            }
            self.reset_blink();
            return;
        }
        if shift && self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor_pos);
        }
        self.cursor_pos = char_pos_from_line_col(&self.text, line - 1, col);
        if !shift {
            self.selection_anchor = None;
        } else if self.selection_anchor == Some(self.cursor_pos) {
            self.selection_anchor = None;
        }
        self.reset_blink();
    }

    /// Move cursor down one line (multiline only).
    pub fn move_down(&mut self, shift: bool) {
        let (line, col) = line_and_column(&self.text, self.cursor_pos);
        let line_count = self.text.chars().filter(|&c| c == '\n').count() + 1;
        if line >= line_count - 1 {
            // Already on last line — move to end
            if shift && self.selection_anchor.is_none() {
                self.selection_anchor = Some(self.cursor_pos);
            }
            self.cursor_pos = self.text.chars().count();
            if !shift {
                self.selection_anchor = None;
            } else if self.selection_anchor == Some(self.cursor_pos) {
                self.selection_anchor = None;
            }
            self.reset_blink();
            return;
        }
        if shift && self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor_pos);
        }
        self.cursor_pos = char_pos_from_line_col(&self.text, line + 1, col);
        if !shift {
            self.selection_anchor = None;
        } else if self.selection_anchor == Some(self.cursor_pos) {
            self.selection_anchor = None;
        }
        self.reset_blink();
    }
}

// =============================================================================
// Visual configuration for text input elements
// =============================================================================

/// Configuration for a text input element's visual appearance.
/// Stored per-frame in `PlyContext::text_input_configs`.
#[derive(Debug, Clone)]
pub struct TextInputConfig {
    /// Placeholder text shown when input is empty.
    pub placeholder: String,
    /// Maximum number of characters allowed. `None` = unlimited.
    pub max_length: Option<usize>,
    /// When true, characters are displayed as `•`.
    pub is_password: bool,
    /// When true, the input supports multiple lines (Enter inserts newline).
    pub is_multiline: bool,
    /// Font ID for the text (matches the user's font registry).
    pub font_id: u16,
    /// Font size in pixels.
    pub font_size: u16,
    /// Color of the input text.
    pub text_color: Color,
    /// Color of the placeholder text.
    pub placeholder_color: Color,
    /// Color of the cursor line.
    pub cursor_color: Color,
    /// Color of the selection highlight rectangle.
    pub selection_color: Color,
}

impl Default for TextInputConfig {
    fn default() -> Self {
        Self {
            placeholder: String::new(),
            max_length: None,
            is_password: false,
            is_multiline: false,
            font_id: 0,
            font_size: 16,
            text_color: Color::rgba(1.0, 1.0, 1.0, 1.0),
            placeholder_color: Color::rgba(0.5, 0.5, 0.5, 1.0),
            cursor_color: Color::rgba(1.0, 1.0, 1.0, 1.0),
            selection_color: Color::rgba(0.27, 0.51, 0.71, 0.5),
        }
    }
}

/// Builder for configuring a text input element via closure.
pub struct TextInputBuilder {
    pub(crate) config: TextInputConfig,
    pub(crate) on_changed_fn: Option<Box<dyn FnMut(&str) + 'static>>,
    pub(crate) on_submit_fn: Option<Box<dyn FnMut(&str) + 'static>>,
}

impl TextInputBuilder {
    pub(crate) fn new() -> Self {
        Self {
            config: TextInputConfig::default(),
            on_changed_fn: None,
            on_submit_fn: None,
        }
    }

    /// Sets the placeholder text shown when the input is empty.
    #[inline]
    pub fn placeholder(&mut self, text: &str) -> &mut Self {
        self.config.placeholder = text.to_string();
        self
    }

    /// Sets the maximum number of characters allowed.
    #[inline]
    pub fn max_length(&mut self, len: usize) -> &mut Self {
        self.config.max_length = Some(len);
        self
    }

    /// Enables password mode (characters shown as dots).
    #[inline]
    pub fn password(&mut self, enabled: bool) -> &mut Self {
        self.config.is_password = enabled;
        self
    }

    /// Enables multiline mode (Enter inserts newline, up/down arrows navigate lines).
    #[inline]
    pub fn multiline(&mut self, enabled: bool) -> &mut Self {
        self.config.is_multiline = enabled;
        self
    }

    /// Sets the font ID.
    #[inline]
    pub fn font_id(&mut self, id: u16) -> &mut Self {
        self.config.font_id = id;
        self
    }

    /// Sets the font size.
    #[inline]
    pub fn font_size(&mut self, size: u16) -> &mut Self {
        self.config.font_size = size;
        self
    }

    /// Sets the text color.
    #[inline]
    pub fn text_color(&mut self, color: impl Into<Color>) -> &mut Self {
        self.config.text_color = color.into();
        self
    }

    /// Sets the placeholder text color.
    #[inline]
    pub fn placeholder_color(&mut self, color: impl Into<Color>) -> &mut Self {
        self.config.placeholder_color = color.into();
        self
    }

    /// Sets the cursor color.
    #[inline]
    pub fn cursor_color(&mut self, color: impl Into<Color>) -> &mut Self {
        self.config.cursor_color = color.into();
        self
    }

    /// Sets the selection highlight color.
    #[inline]
    pub fn selection_color(&mut self, color: impl Into<Color>) -> &mut Self {
        self.config.selection_color = color.into();
        self
    }

    /// Registers a callback fired whenever the text content changes.
    #[inline]
    pub fn on_changed<F>(&mut self, callback: F) -> &mut Self
    where
        F: FnMut(&str) + 'static,
    {
        self.on_changed_fn = Some(Box::new(callback));
        self
    }

    /// Registers a callback fired when the user presses Enter.
    #[inline]
    pub fn on_submit<F>(&mut self, callback: F) -> &mut Self
    where
        F: FnMut(&str) + 'static,
    {
        self.on_submit_fn = Some(Box::new(callback));
        self
    }
}

// =============================================================================
// Helper functions
// =============================================================================

/// Convert a character index to a byte index in the string.
pub fn char_index_to_byte(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(byte_pos, _)| byte_pos)
        .unwrap_or(s.len())
}

// =============================================================================
// Multiline helper functions
// =============================================================================

/// Find the char index of the start of the line containing `char_pos`.
/// A "line" is delimited by '\n'. Returns 0 for the first line.
pub fn line_start_char_pos(text: &str, char_pos: usize) -> usize {
    let chars: Vec<char> = text.chars().collect();
    let mut i = char_pos;
    while i > 0 && chars[i - 1] != '\n' {
        i -= 1;
    }
    i
}

/// Find the char index of the end of the line containing `char_pos`.
/// Returns the position just before the '\n' or at text end.
pub fn line_end_char_pos(text: &str, char_pos: usize) -> usize {
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut i = char_pos;
    while i < len && chars[i] != '\n' {
        i += 1;
    }
    i
}

/// Returns (line_index, column) for a given char position.
/// Lines are 0-indexed, split by '\n'.
pub fn line_and_column(text: &str, char_pos: usize) -> (usize, usize) {
    let mut line = 0;
    let mut col = 0;
    for (i, ch) in text.chars().enumerate() {
        if i == char_pos {
            return (line, col);
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }
    (line, col)
}

/// Convert a (line, column) pair to a character position.
/// If the column exceeds the line length, clamps to line end.
pub fn char_pos_from_line_col(text: &str, target_line: usize, target_col: usize) -> usize {
    let mut line = 0;
    let mut col = 0;
    for (i, ch) in text.chars().enumerate() {
        if line == target_line && col == target_col {
            return i;
        }
        if ch == '\n' {
            if line == target_line {
                // Column exceeds this line length; return end of this line
                return i;
            }
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }
    // If target is beyond text, return text length
    text.chars().count()
}

/// Split text into lines (by '\n'), returning each line's text
/// and the global char index where it starts.
pub fn split_lines(text: &str) -> Vec<(usize, &str)> {
    let mut result = Vec::new();
    let mut char_start = 0;
    let mut byte_start = 0;
    for (byte_idx, ch) in text.char_indices() {
        if ch == '\n' {
            result.push((char_start, &text[byte_start..byte_idx]));
            char_start += text[byte_start..byte_idx].chars().count() + 1; // +1 for '\n'
            byte_start = byte_idx + 1; // '\n' is 1 byte
        }
    }
    // Last line (after final '\n' or entire text if no '\n')
    result.push((char_start, &text[byte_start..]));
    result
}

// =============================================================================
// Word wrapping
// =============================================================================

/// A single visual line after word-wrapping.
#[derive(Debug, Clone)]
pub struct VisualLine {
    /// The text content of this visual line.
    pub text: String,
    /// The global character index where this visual line starts in the full text.
    pub global_char_start: usize,
    /// Number of characters in this visual line.
    pub char_count: usize,
}

/// Word-wrap text into visual lines that fit within `max_width`.
/// Splits on '\n' first (hard breaks), then wraps long lines at word boundaries.
/// If `max_width <= 0`, no wrapping occurs (equivalent to `split_lines`).
pub fn wrap_lines(
    text: &str,
    max_width: f32,
    font_id: u16,
    font_size: u16,
    measure_fn: &dyn Fn(&str, &crate::text::TextConfig) -> crate::math::Dimensions,
) -> Vec<VisualLine> {
    let config = crate::text::TextConfig {
        font_id,
        font_size,
        ..Default::default()
    };

    let hard_lines = split_lines(text);
    let mut result = Vec::new();

    for (global_start, line_text) in hard_lines {
        if line_text.is_empty() {
            result.push(VisualLine {
                text: String::new(),
                global_char_start: global_start,
                char_count: 0,
            });
            continue;
        }

        if max_width <= 0.0 {
            // No wrapping
            result.push(VisualLine {
                text: line_text.to_string(),
                global_char_start: global_start,
                char_count: line_text.chars().count(),
            });
            continue;
        }

        // Check if the whole line fits
        let full_width = measure_fn(line_text, &config).width;
        if full_width <= max_width {
            result.push(VisualLine {
                text: line_text.to_string(),
                global_char_start: global_start,
                char_count: line_text.chars().count(),
            });
            continue;
        }

        // Need to wrap this line
        let chars: Vec<char> = line_text.chars().collect();
        let total_chars = chars.len();
        let mut line_char_start = 0; // index within chars[]

        while line_char_start < total_chars {
            // Find how many characters fit in max_width
            let mut fit_count = 0;
            for i in 1..=(total_chars - line_char_start) {
                let substr: String = chars[line_char_start..line_char_start + i].iter().collect();
                let w = measure_fn(&substr, &config).width;
                if w > max_width {
                    break;
                }
                fit_count = i;
            }

            if fit_count == 0 {
                // Even a single character doesn't fit; force at least one character
                fit_count = 1;
            }

            if line_char_start + fit_count < total_chars {
                // Try to break at a word boundary (last space within fit_count)
                let mut break_at = fit_count;
                let mut found_space = false;
                for j in (1..=fit_count).rev() {
                    if chars[line_char_start + j - 1] == ' ' {
                        break_at = j;
                        found_space = true;
                        break;
                    }
                }
                // If we found a space, break there; otherwise force character-level break
                let wrap_count = if found_space { break_at } else { fit_count };
                let segment: String = chars[line_char_start..line_char_start + wrap_count].iter().collect();
                result.push(VisualLine {
                    text: segment,
                    global_char_start: global_start + line_char_start,
                    char_count: wrap_count,
                });
                line_char_start += wrap_count;
                // Skip leading space on the next line if we broke at a space
                if found_space && line_char_start < total_chars && chars[line_char_start] == ' ' {
                    // Don't skip — the space is already consumed in the segment above
                    // Actually, break_at includes the space. Let's keep it as-is for now.
                }
            } else {
                // Remaining text fits
                let segment: String = chars[line_char_start..].iter().collect();
                let count = total_chars - line_char_start;
                result.push(VisualLine {
                    text: segment,
                    global_char_start: global_start + line_char_start,
                    char_count: count,
                });
                line_char_start = total_chars;
            }
        }
    }

    // Ensure at least one visual line
    if result.is_empty() {
        result.push(VisualLine {
            text: String::new(),
            global_char_start: 0,
            char_count: 0,
        });
    }

    result
}

/// Given visual lines and a global cursor position, return (visual_line_index, column_in_visual_line).
pub fn cursor_to_visual_pos(visual_lines: &[VisualLine], cursor_pos: usize) -> (usize, usize) {
    for (i, vl) in visual_lines.iter().enumerate() {
        let line_end = vl.global_char_start + vl.char_count;
        if cursor_pos < line_end || i == visual_lines.len() - 1 {
            return (i, cursor_pos.saturating_sub(vl.global_char_start));
        }
        // If cursor_pos == line_end and this isn't the last line, it could be at the
        // start of the next line OR the end of this one. For wrapped lines (no \n),
        // prefer placing it at the start of the next line.
        if cursor_pos == line_end {
            // Check if next line continues from this one (wrapped) or is a new paragraph
            if i + 1 < visual_lines.len() {
                let next = &visual_lines[i + 1];
                if next.global_char_start == line_end {
                    // Wrapped continuation — cursor goes to start of next visual line
                    return (i + 1, 0);
                }
                // Hard break (\n between them) — cursor at end of this line
                return (i, cursor_pos - vl.global_char_start);
            }
            return (i, cursor_pos - vl.global_char_start);
        }
    }
    (0, 0)
}

/// Navigate cursor one visual line up. Returns the new global cursor position.
/// `col` is the desired column (preserved across up/down moves).
pub fn visual_move_up(visual_lines: &[VisualLine], cursor_pos: usize) -> usize {
    let (line, col) = cursor_to_visual_pos(visual_lines, cursor_pos);
    if line == 0 {
        return 0; // Already on first visual line → move to start
    }
    let target_line = &visual_lines[line - 1];
    let new_col = col.min(target_line.char_count);
    target_line.global_char_start + new_col
}

/// Navigate cursor one visual line down. Returns the new global cursor position.
pub fn visual_move_down(visual_lines: &[VisualLine], cursor_pos: usize, text_len: usize) -> usize {
    let (line, col) = cursor_to_visual_pos(visual_lines, cursor_pos);
    if line >= visual_lines.len() - 1 {
        return text_len; // Already on last visual line → move to end
    }
    let target_line = &visual_lines[line + 1];
    let new_col = col.min(target_line.char_count);
    target_line.global_char_start + new_col
}

/// Move to start of current visual line. Returns the new global cursor position.
pub fn visual_line_home(visual_lines: &[VisualLine], cursor_pos: usize) -> usize {
    let (line, _col) = cursor_to_visual_pos(visual_lines, cursor_pos);
    visual_lines[line].global_char_start
}

/// Move to end of current visual line. Returns the new global cursor position.
pub fn visual_line_end(visual_lines: &[VisualLine], cursor_pos: usize) -> usize {
    let (line, _col) = cursor_to_visual_pos(visual_lines, cursor_pos);
    visual_lines[line].global_char_start + visual_lines[line].char_count
}

/// Find the nearest character boundary for a given pixel x-position.
/// `char_x_positions` has len = char_count + 1 (position 0 = left edge, position n = right edge).
pub fn find_nearest_char_boundary(click_x: f32, char_x_positions: &[f32]) -> usize {
    if char_x_positions.is_empty() {
        return 0;
    }
    let mut best = 0;
    let mut best_dist = f32::MAX;
    for (i, &x) in char_x_positions.iter().enumerate() {
        let dist = (click_x - x).abs();
        if dist < best_dist {
            best_dist = dist;
            best = i;
        }
    }
    best
}

/// Find the word boundary to the left of `pos` (for Ctrl+Left / Ctrl+Backspace).
pub fn find_word_boundary_left(text: &str, pos: usize) -> usize {
    if pos == 0 {
        return 0;
    }
    let chars: Vec<char> = text.chars().collect();
    let mut i = pos;
    // Skip whitespace to the left of cursor
    while i > 0 && chars[i - 1].is_whitespace() {
        i -= 1;
    }
    // Skip word characters to the left
    while i > 0 && !chars[i - 1].is_whitespace() {
        i -= 1;
    }
    i
}

/// Find the word boundary to the right of `pos` (for Ctrl+Right / Ctrl+Delete).
pub fn find_word_boundary_right(text: &str, pos: usize) -> usize {
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    if pos >= len {
        return len;
    }
    let mut i = pos;
    // Skip non-whitespace (current word) to the right
    while i < len && !chars[i].is_whitespace() {
        i += 1;
    }
    // Skip whitespace to the right
    while i < len && chars[i].is_whitespace() {
        i += 1;
    }
    i
}

/// Find the word boundaries (start, end) at the given character position.
/// Used for double-click word selection.
pub fn find_word_at(text: &str, pos: usize) -> (usize, usize) {
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    if len == 0 || pos >= len {
        return (pos, pos);
    }
    let is_word_char = |c: char| !c.is_whitespace();
    if !is_word_char(chars[pos]) {
        // On whitespace — select the whitespace run
        let mut start = pos;
        while start > 0 && !is_word_char(chars[start - 1]) {
            start -= 1;
        }
        let mut end = pos;
        while end < len && !is_word_char(chars[end]) {
            end += 1;
        }
        return (start, end);
    }
    // On a word char — find word boundaries
    let mut start = pos;
    while start > 0 && is_word_char(chars[start - 1]) {
        start -= 1;
    }
    let mut end = pos;
    while end < len && is_word_char(chars[end]) {
        end += 1;
    }
    (start, end)
}

/// Build the display text for rendering.
/// Returns the string that should be measured/drawn.
pub fn display_text(text: &str, placeholder: &str, is_password: bool) -> String {
    if text.is_empty() {
        return placeholder.to_string();
    }
    if is_password {
        "•".repeat(text.chars().count())
    } else {
        text.to_string()
    }
}

/// Compute x-positions for each character boundary in the display text.
/// Returns a Vec with len = char_count + 1.
/// Uses the provided measure function to measure substrings.
pub fn compute_char_x_positions(
    display_text: &str,
    font_id: u16,
    font_size: u16,
    measure_fn: &dyn Fn(&str, &crate::text::TextConfig) -> crate::math::Dimensions,
) -> Vec<f32> {
    let char_count = display_text.chars().count();
    let mut positions = Vec::with_capacity(char_count + 1);
    positions.push(0.0);

    let config = crate::text::TextConfig {
        font_id,
        font_size,
        ..Default::default()
    };

    for i in 1..=char_count {
        let byte_end = char_index_to_byte(display_text, i);
        let substr = &display_text[..byte_end];
        let dims = measure_fn(substr, &config);
        positions.push(dims.width);
    }
    positions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_char_index_to_byte_ascii() {
        let s = "Hello";
        assert_eq!(char_index_to_byte(s, 0), 0);
        assert_eq!(char_index_to_byte(s, 3), 3);
        assert_eq!(char_index_to_byte(s, 5), 5);
    }

    #[test]
    fn test_char_index_to_byte_unicode() {
        let s = "Héllo";
        assert_eq!(char_index_to_byte(s, 0), 0);
        assert_eq!(char_index_to_byte(s, 1), 1); // 'H'
        assert_eq!(char_index_to_byte(s, 2), 3); // 'é' is 2 bytes
        assert_eq!(char_index_to_byte(s, 5), 6);
    }

    #[test]
    fn test_word_boundary_left() {
        assert_eq!(find_word_boundary_left("hello world", 11), 6);
        assert_eq!(find_word_boundary_left("hello world", 6), 0); // at start of "world", skip space + "hello"
        assert_eq!(find_word_boundary_left("hello world", 5), 0);
        assert_eq!(find_word_boundary_left("hello", 0), 0);
    }

    #[test]
    fn test_word_boundary_right() {
        assert_eq!(find_word_boundary_right("hello world", 0), 6);
        assert_eq!(find_word_boundary_right("hello world", 5), 6); // from space, skip past it to start of "world"
        assert_eq!(find_word_boundary_right("hello world", 6), 11);
        assert_eq!(find_word_boundary_right("hello", 5), 5);
    }

    #[test]
    fn test_find_word_at() {
        assert_eq!(find_word_at("hello world", 2), (0, 5));
        assert_eq!(find_word_at("hello world", 7), (6, 11));
        assert_eq!(find_word_at("hello world", 5), (5, 6)); // on space
    }

    #[test]
    fn test_insert_text() {
        let mut state = TextEditState::default();
        state.insert_text("Hello", None);
        assert_eq!(state.text, "Hello");
        assert_eq!(state.cursor_pos, 5);

        state.cursor_pos = 5;
        state.insert_text(" World", None);
        assert_eq!(state.text, "Hello World");
        assert_eq!(state.cursor_pos, 11);
    }

    #[test]
    fn test_insert_text_max_length() {
        let mut state = TextEditState::default();
        state.insert_text("Hello World", Some(5));
        assert_eq!(state.text, "Hello");
        assert_eq!(state.cursor_pos, 5);

        // Already at max, no more insertion
        state.insert_text("!", Some(5));
        assert_eq!(state.text, "Hello");
    }

    #[test]
    fn test_backspace() {
        let mut state = TextEditState::default();
        state.text = "Hello".to_string();
        state.cursor_pos = 5;
        state.backspace();
        assert_eq!(state.text, "Hell");
        assert_eq!(state.cursor_pos, 4);
    }

    #[test]
    fn test_delete_forward() {
        let mut state = TextEditState::default();
        state.text = "Hello".to_string();
        state.cursor_pos = 0;
        state.delete_forward();
        assert_eq!(state.text, "ello");
        assert_eq!(state.cursor_pos, 0);
    }

    #[test]
    fn test_selection_delete() {
        let mut state = TextEditState::default();
        state.text = "Hello World".to_string();
        state.selection_anchor = Some(0);
        state.cursor_pos = 5;
        state.delete_selection();
        assert_eq!(state.text, " World");
        assert_eq!(state.cursor_pos, 0);
        assert!(state.selection_anchor.is_none());
    }

    #[test]
    fn test_select_all() {
        let mut state = TextEditState::default();
        state.text = "Hello".to_string();
        state.cursor_pos = 2;
        state.select_all();
        assert_eq!(state.selection_anchor, Some(0));
        assert_eq!(state.cursor_pos, 5);
    }

    #[test]
    fn test_move_left_right() {
        let mut state = TextEditState::default();
        state.text = "AB".to_string();
        state.cursor_pos = 1;

        state.move_left(false);
        assert_eq!(state.cursor_pos, 0);

        state.move_right(false);
        assert_eq!(state.cursor_pos, 1);
    }

    #[test]
    fn test_move_with_shift_creates_selection() {
        let mut state = TextEditState::default();
        state.text = "Hello".to_string();
        state.cursor_pos = 2;

        state.move_right(true);
        assert_eq!(state.cursor_pos, 3);
        assert_eq!(state.selection_anchor, Some(2));

        state.move_right(true);
        assert_eq!(state.cursor_pos, 4);
        assert_eq!(state.selection_anchor, Some(2));
    }

    #[test]
    fn test_display_text_normal() {
        assert_eq!(display_text("Hello", "Placeholder", false), "Hello");
    }

    #[test]
    fn test_display_text_empty() {
        assert_eq!(display_text("", "Placeholder", false), "Placeholder");
    }

    #[test]
    fn test_display_text_password() {
        assert_eq!(display_text("pass", "Placeholder", true), "••••");
    }

    #[test]
    fn test_nearest_char_boundary() {
        let positions = vec![0.0, 10.0, 20.0, 30.0];
        assert_eq!(find_nearest_char_boundary(4.0, &positions), 0);
        assert_eq!(find_nearest_char_boundary(6.0, &positions), 1);
        assert_eq!(find_nearest_char_boundary(15.0, &positions), 1); // midpoint rounds to closer
        assert_eq!(find_nearest_char_boundary(25.0, &positions), 2);
        assert_eq!(find_nearest_char_boundary(100.0, &positions), 3);
    }

    #[test]
    fn test_ensure_cursor_visible() {
        let mut state = TextEditState::default();
        state.scroll_offset = 0.0;

        // Cursor at x=150, visible_width=100 → should scroll right
        state.ensure_cursor_visible(150.0, 100.0);
        assert_eq!(state.scroll_offset, 50.0);

        // Cursor at x=30, scroll_offset=50 → 30-50 = -20 < 0 → scroll left
        state.ensure_cursor_visible(30.0, 100.0);
        assert_eq!(state.scroll_offset, 30.0);
    }

    #[test]
    fn test_backspace_word() {
        let mut state = TextEditState::default();
        state.text = "hello world".to_string();
        state.cursor_pos = 11;
        state.backspace_word();
        assert_eq!(state.text, "hello ");
        assert_eq!(state.cursor_pos, 6);
    }

    #[test]
    fn test_delete_word_forward() {
        let mut state = TextEditState::default();
        state.text = "hello world".to_string();
        state.cursor_pos = 0;
        state.delete_word_forward();
        assert_eq!(state.text, "world");
        assert_eq!(state.cursor_pos, 0);
    }

    // ── Multiline helper tests ──

    #[test]
    fn test_line_start_char_pos() {
        assert_eq!(line_start_char_pos("hello\nworld", 0), 0);
        assert_eq!(line_start_char_pos("hello\nworld", 3), 0);
        assert_eq!(line_start_char_pos("hello\nworld", 5), 0);
        assert_eq!(line_start_char_pos("hello\nworld", 6), 6); // 'w' on second line
        assert_eq!(line_start_char_pos("hello\nworld", 9), 6);
    }

    #[test]
    fn test_line_end_char_pos() {
        assert_eq!(line_end_char_pos("hello\nworld", 0), 5);
        assert_eq!(line_end_char_pos("hello\nworld", 3), 5);
        assert_eq!(line_end_char_pos("hello\nworld", 6), 11);
        assert_eq!(line_end_char_pos("hello\nworld", 9), 11);
    }

    #[test]
    fn test_line_and_column() {
        assert_eq!(line_and_column("hello\nworld", 0), (0, 0));
        assert_eq!(line_and_column("hello\nworld", 3), (0, 3));
        assert_eq!(line_and_column("hello\nworld", 5), (0, 5)); // at '\n'
        assert_eq!(line_and_column("hello\nworld", 6), (1, 0));
        assert_eq!(line_and_column("hello\nworld", 8), (1, 2));
        assert_eq!(line_and_column("hello\nworld", 11), (1, 5)); // end of text
    }

    #[test]
    fn test_line_and_column_three_lines() {
        let text = "ab\ncd\nef";
        assert_eq!(line_and_column(text, 0), (0, 0));
        assert_eq!(line_and_column(text, 2), (0, 2)); // at '\n'
        assert_eq!(line_and_column(text, 3), (1, 0));
        assert_eq!(line_and_column(text, 5), (1, 2)); // at '\n'
        assert_eq!(line_and_column(text, 6), (2, 0));
        assert_eq!(line_and_column(text, 8), (2, 2)); // end
    }

    #[test]
    fn test_char_pos_from_line_col() {
        assert_eq!(char_pos_from_line_col("hello\nworld", 0, 0), 0);
        assert_eq!(char_pos_from_line_col("hello\nworld", 0, 3), 3);
        assert_eq!(char_pos_from_line_col("hello\nworld", 1, 0), 6);
        assert_eq!(char_pos_from_line_col("hello\nworld", 1, 3), 9);
        // Column exceeds line length → clamp to end of line
        assert_eq!(char_pos_from_line_col("ab\ncd", 0, 10), 2); // line 0 ends at char 2
        assert_eq!(char_pos_from_line_col("ab\ncd", 1, 10), 5); // line 1 goes to end
    }

    #[test]
    fn test_split_lines() {
        let lines = split_lines("hello\nworld");
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], (0, "hello"));
        assert_eq!(lines[1], (6, "world"));

        let lines2 = split_lines("a\nb\nc");
        assert_eq!(lines2.len(), 3);
        assert_eq!(lines2[0], (0, "a"));
        assert_eq!(lines2[1], (2, "b"));
        assert_eq!(lines2[2], (4, "c"));

        let lines3 = split_lines("no newlines");
        assert_eq!(lines3.len(), 1);
        assert_eq!(lines3[0], (0, "no newlines"));
    }

    #[test]
    fn test_split_lines_trailing_newline() {
        let lines = split_lines("hello\n");
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], (0, "hello"));
        assert_eq!(lines[1], (6, ""));
    }

    #[test]
    fn test_move_up_down() {
        let mut state = TextEditState::default();
        state.text = "hello\nworld".to_string();
        state.cursor_pos = 8; // 'r' on line 1, col 2

        state.move_up(false);
        assert_eq!(state.cursor_pos, 2); // line 0, col 2

        state.move_down(false);
        assert_eq!(state.cursor_pos, 8); // back to line 1, col 2
    }

    #[test]
    fn test_move_up_clamps_column() {
        let mut state = TextEditState::default();
        state.text = "ab\nhello".to_string();
        state.cursor_pos = 7; // line 1, col 4 (before 'o')

        state.move_up(false);
        assert_eq!(state.cursor_pos, 2); // line 0 only has 2 chars, clamp to end
    }

    #[test]
    fn test_move_up_from_first_line() {
        let mut state = TextEditState::default();
        state.text = "hello\nworld".to_string();
        state.cursor_pos = 3;

        state.move_up(false);
        assert_eq!(state.cursor_pos, 0); // moves to start
    }

    #[test]
    fn test_move_down_from_last_line() {
        let mut state = TextEditState::default();
        state.text = "hello\nworld".to_string();
        state.cursor_pos = 8;

        state.move_down(false);
        assert_eq!(state.cursor_pos, 11); // moves to end
    }

    #[test]
    fn test_move_line_home_end() {
        let mut state = TextEditState::default();
        state.text = "hello\nworld".to_string();
        state.cursor_pos = 8; // line 1, col 2

        state.move_line_home(false);
        assert_eq!(state.cursor_pos, 6); // start of line 1

        state.move_line_end(false);
        assert_eq!(state.cursor_pos, 11); // end of line 1
    }

    #[test]
    fn test_move_up_with_shift_selects() {
        let mut state = TextEditState::default();
        state.text = "hello\nworld".to_string();
        state.cursor_pos = 8;

        state.move_up(true);
        assert_eq!(state.cursor_pos, 2);
        assert_eq!(state.selection_anchor, Some(8));
    }

    #[test]
    fn test_ensure_cursor_visible_vertical() {
        let mut state = TextEditState::default();
        state.scroll_offset_y = 0.0;

        // Cursor on line 5, line_height=20, visible_height=60
        // cursor_bottom = 5*20+20 = 120 > 60 → scroll down
        state.ensure_cursor_visible_vertical(5, 20.0, 60.0);
        assert_eq!(state.scroll_offset_y, 60.0); // 120 - 60

        // Cursor on line 1, scroll_offset_y=60 → cursor_y = 20 < 60 → scroll up
        state.ensure_cursor_visible_vertical(1, 20.0, 60.0);
        assert_eq!(state.scroll_offset_y, 20.0);
    }

    // ── Word wrapping tests ──

    /// Simple fixed-width measure: each char is 10px wide.
    fn fixed_measure(text: &str, _config: &crate::text::TextConfig) -> crate::math::Dimensions {
        crate::math::Dimensions {
            width: text.chars().count() as f32 * 10.0,
            height: 20.0,
        }
    }

    #[test]
    fn test_wrap_lines_no_wrap_needed() {
        let lines = wrap_lines("hello", 100.0, 0, 16, &fixed_measure);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text, "hello");
        assert_eq!(lines[0].global_char_start, 0);
        assert_eq!(lines[0].char_count, 5);
    }

    #[test]
    fn test_wrap_lines_hard_break() {
        let lines = wrap_lines("ab\ncd", 100.0, 0, 16, &fixed_measure);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].text, "ab");
        assert_eq!(lines[0].global_char_start, 0);
        assert_eq!(lines[1].text, "cd");
        assert_eq!(lines[1].global_char_start, 3); // after '\n'
    }

    #[test]
    fn test_wrap_lines_word_wrap() {
        // "hello world" = 11 chars × 10px = 110px, max_width=60px
        // "hello " = 6 chars = 60px fits, then "world" = 5 chars = 50px fits
        let lines = wrap_lines("hello world", 60.0, 0, 16, &fixed_measure);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].text, "hello ");
        assert_eq!(lines[0].global_char_start, 0);
        assert_eq!(lines[0].char_count, 6);
        assert_eq!(lines[1].text, "world");
        assert_eq!(lines[1].global_char_start, 6);
        assert_eq!(lines[1].char_count, 5);
    }

    #[test]
    fn test_wrap_lines_char_level_break() {
        // "abcdefghij" = 10 chars × 10px = 100px, max_width=50px
        // No spaces → character-level break at 5 chars
        let lines = wrap_lines("abcdefghij", 50.0, 0, 16, &fixed_measure);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].text, "abcde");
        assert_eq!(lines[0].char_count, 5);
        assert_eq!(lines[1].text, "fghij");
        assert_eq!(lines[1].global_char_start, 5);
    }

    #[test]
    fn test_cursor_to_visual_pos_simple() {
        let lines = vec![
            VisualLine { text: "hello ".to_string(), global_char_start: 0, char_count: 6 },
            VisualLine { text: "world".to_string(), global_char_start: 6, char_count: 5 },
        ];
        assert_eq!(cursor_to_visual_pos(&lines, 0), (0, 0));
        assert_eq!(cursor_to_visual_pos(&lines, 3), (0, 3));
        assert_eq!(cursor_to_visual_pos(&lines, 6), (1, 0)); // Wrapped → start of next line
        assert_eq!(cursor_to_visual_pos(&lines, 8), (1, 2));
        assert_eq!(cursor_to_visual_pos(&lines, 11), (1, 5));
    }

    #[test]
    fn test_cursor_to_visual_pos_hard_break() {
        // "ab\ncd" → line 0: "ab" (start=0, count=2), line 1: "cd" (start=3, count=2)
        let lines = vec![
            VisualLine { text: "ab".to_string(), global_char_start: 0, char_count: 2 },
            VisualLine { text: "cd".to_string(), global_char_start: 3, char_count: 2 },
        ];
        assert_eq!(cursor_to_visual_pos(&lines, 2), (0, 2)); // End of "ab" (before \n)
        assert_eq!(cursor_to_visual_pos(&lines, 3), (1, 0)); // Start of "cd"
    }

    #[test]
    fn test_visual_move_up_down() {
        let lines = vec![
            VisualLine { text: "hello ".to_string(), global_char_start: 0, char_count: 6 },
            VisualLine { text: "world".to_string(), global_char_start: 6, char_count: 5 },
        ];
        // From cursor at pos 8 (line 1, col 2) → move up → line 0, col 2 = pos 2
        assert_eq!(visual_move_up(&lines, 8), 2);
        // From cursor at pos 2 (line 0, col 2) → move down → line 1, col 2 = pos 8
        assert_eq!(visual_move_down(&lines, 2, 11), 8);
    }

    #[test]
    fn test_visual_line_home_end() {
        let lines = vec![
            VisualLine { text: "hello ".to_string(), global_char_start: 0, char_count: 6 },
            VisualLine { text: "world".to_string(), global_char_start: 6, char_count: 5 },
        ];
        // Cursor at pos 8 (line 1, col 2) → home = 6, end = 11
        assert_eq!(visual_line_home(&lines, 8), 6);
        assert_eq!(visual_line_end(&lines, 8), 11);
        // Cursor at pos 3 (line 0, col 3) → home = 0, end = 6
        assert_eq!(visual_line_home(&lines, 3), 0);
        assert_eq!(visual_line_end(&lines, 3), 6);
    }
}
