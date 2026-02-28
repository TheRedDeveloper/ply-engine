use crate::color::Color;

/// Identifies what kind of edit an undo entry was, for grouping consecutive similar edits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UndoActionKind {
    /// Inserting a single character (consecutive inserts are grouped).
    InsertChar,
    /// Inserting text from paste (not grouped).
    Paste,
    /// Deleting via Backspace (consecutive deletes are grouped).
    Backspace,
    /// Deleting via Delete key (consecutive deletes are grouped).
    Delete,
    /// Deleting a word (not grouped).
    DeleteWord,
    /// Cutting selected text (not grouped).
    Cut,
    /// Any other discrete change (not grouped).
    Other,
}

/// A snapshot of text state for undo/redo.
#[derive(Debug, Clone)]
pub struct UndoEntry {
    pub text: String,
    pub cursor_pos: usize,
    pub selection_anchor: Option<usize>,
    pub action_kind: UndoActionKind,
}

/// Maximum number of undo entries to keep.
const MAX_UNDO_STACK: usize = 200;

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
    /// Saved visual column for vertical (up/down) navigation.
    /// When set, up/down arrows try to return to this column.
    pub preferred_col: Option<usize>,
    /// When true, cursor movement skips structural style positions (`}` and empty content markers).
    /// Set from `TextInputConfig::no_styles_movement`.
    pub no_styles_movement: bool,
    /// Undo stack: previous states (newest at end).
    pub undo_stack: Vec<UndoEntry>,
    /// Redo stack: states undone (newest at end).
    pub redo_stack: Vec<UndoEntry>,
}

impl Default for TextEditState {
    fn default() -> Self {
        Self {
            text: String::new(),
            cursor_pos: 0,
            selection_anchor: None,
            scroll_offset: 0.0,
            scroll_offset_y: 0.0,
            preferred_col: None,
            no_styles_movement: false,
            cursor_blink_timer: 0.0,
            last_click_time: 0.0,
            last_click_element: 0,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
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
        let target = find_word_delete_boundary_right(&self.text, self.cursor_pos);
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

    /// Push the current state onto the undo stack before an edit.
    /// `kind` controls grouping: consecutive edits of the same kind are merged
    /// (only InsertChar, Backspace, and Delete are grouped).
    pub fn push_undo(&mut self, kind: UndoActionKind) {
        // Grouping: if the last undo entry has the same kind and it's a groupable kind,
        // don't push a new entry (the original pre-group state is already saved).
        let should_group = matches!(kind, UndoActionKind::InsertChar | UndoActionKind::Backspace | UndoActionKind::Delete);
        if should_group {
            if let Some(last) = self.undo_stack.last() {
                if last.action_kind == kind {
                    // Same groupable action — skip push, keep the original entry
                    // Clear redo stack since we're making a new edit
                    self.redo_stack.clear();
                    return;
                }
            }
        }

        self.undo_stack.push(UndoEntry {
            text: self.text.clone(),
            cursor_pos: self.cursor_pos,
            selection_anchor: self.selection_anchor,
            action_kind: kind,
        });
        // Limit stack size
        if self.undo_stack.len() > MAX_UNDO_STACK {
            self.undo_stack.remove(0);
        }
        // Any new edit clears the redo stack
        self.redo_stack.clear();
    }

    /// Undo the last edit. Returns true if undo was performed.
    pub fn undo(&mut self) -> bool {
        if let Some(entry) = self.undo_stack.pop() {
            // Save current state to redo stack
            self.redo_stack.push(UndoEntry {
                text: self.text.clone(),
                cursor_pos: self.cursor_pos,
                selection_anchor: self.selection_anchor,
                action_kind: entry.action_kind,
            });
            // Restore
            self.text = entry.text;
            self.cursor_pos = entry.cursor_pos;
            self.selection_anchor = entry.selection_anchor;
            self.reset_blink();
            true
        } else {
            false
        }
    }

    /// Redo the last undone edit. Returns true if redo was performed.
    pub fn redo(&mut self) -> bool {
        if let Some(entry) = self.redo_stack.pop() {
            // Save current state to undo stack
            self.undo_stack.push(UndoEntry {
                text: self.text.clone(),
                cursor_pos: self.cursor_pos,
                selection_anchor: self.selection_anchor,
                action_kind: entry.action_kind,
            });
            // Restore
            self.text = entry.text;
            self.cursor_pos = entry.cursor_pos;
            self.selection_anchor = entry.selection_anchor;
            self.reset_blink();
            true
        } else {
            false
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

/// When the `text-styling` feature is enabled, these methods operate on `TextEditState`
/// using visual (display) cursor positions. The internal `text` field contains raw markup,
/// but `cursor_pos` and `selection_anchor` always represent visual positions.
#[cfg(feature = "text-styling")]
impl TextEditState {
    /// Get the visual length of the text (ignoring markup).
    fn cursor_len_styled(&self) -> usize {
        styling_cursor::cursor_len(&self.text)
    }

    /// Get the selected text in visual space, returning the visible chars.
    pub fn selected_text_styled(&self) -> String {
        if let Some((start, end)) = self.selection_range() {
            let stripped = styling_cursor::strip_styling(&self.text);
            let byte_start = char_index_to_byte(&stripped, start);
            let byte_end = char_index_to_byte(&stripped, end);
            stripped[byte_start..byte_end].to_string()
        } else {
            String::new()
        }
    }

    /// Delete selection in styled mode. Returns true if a selection was deleted.
    pub fn delete_selection_styled(&mut self) -> bool {
        if let Some((start, end)) = self.selection_range() {
            if self.no_styles_movement {
                let start_cp = styling_cursor::cursor_to_content(&self.text, start);
                let end_cp = styling_cursor::cursor_to_content(&self.text, end);
                if start_cp < end_cp {
                    self.text = styling_cursor::delete_content_range(&self.text, start_cp, end_cp);
                }
                self.cursor_pos = styling_cursor::content_to_cursor(&self.text, start_cp, true);
            } else {
                self.text = styling_cursor::delete_visual_range(&self.text, start, end);
                self.cursor_pos = start;
            }
            self.selection_anchor = None;
            // Cleanup empty styles (cursor is at start)
            let (cleaned, new_pos) = styling_cursor::cleanup_empty_styles(&self.text, self.cursor_pos);
            self.text = cleaned;
            self.cursor_pos = new_pos;
            self.snap_to_content_pos();
            // If no visible content remains after deletion, clear entirely
            if styling_cursor::strip_styling(&self.text).is_empty() {
                self.text = String::new();
                self.cursor_pos = 0;
            }
            true
        } else {
            false
        }
    }

    /// Insert text at visual cursor position in styled mode, with escaping.
    /// The input `s` should already be escaped if needed.
    pub fn insert_text_styled(&mut self, s: &str, max_length: Option<usize>) {
        self.delete_selection_styled();
        let visual_count = self.cursor_len_styled();
        let insert_cursor_len = styling_cursor::cursor_len(s);
        let allowed = if let Some(max) = max_length {
            if visual_count >= max {
                0
            } else {
                insert_cursor_len.min(max - visual_count)
            }
        } else {
            insert_cursor_len
        };
        if allowed == 0 {
            return;
        }
        // If we need to truncate the insertion, work on the visual chars
        let insert_str = if allowed < insert_cursor_len {
            // Build truncated escaped string
            let stripped = styling_cursor::strip_styling(s);
            let truncated: String = stripped.chars().take(allowed).collect();
            styling_cursor::escape_str(&truncated)
        } else {
            s.to_string()
        };
        let (new_text, new_cursor) = styling_cursor::insert_at_visual(&self.text, self.cursor_pos, &insert_str);
        self.text = new_text;
        self.cursor_pos = new_cursor;
        // Clean up any empty style tags the cursor has passed
        self.cleanup_after_move();
        self.reset_blink();
    }

    /// Insert a single typed character in styled mode (auto-escapes).
    pub fn insert_char_styled(&mut self, ch: char, max_length: Option<usize>) {
        let escaped = styling_cursor::escape_char(ch);
        self.insert_text_styled(&escaped, max_length);
    }

    /// Backspace in styled mode: delete visual char before cursor.
    pub fn backspace_styled(&mut self) {
        if self.delete_selection_styled() {
            return;
        }
        if self.no_styles_movement {
            let cp = styling_cursor::cursor_to_content(&self.text, self.cursor_pos);
            if cp > 0 {
                self.text = styling_cursor::delete_content_range(&self.text, cp - 1, cp);
                self.cursor_pos = styling_cursor::content_to_cursor(&self.text, cp - 1, true);
                let (cleaned, new_pos) = styling_cursor::cleanup_empty_styles(&self.text, self.cursor_pos);
                self.text = cleaned;
                self.cursor_pos = new_pos;
                self.snap_to_content_pos();
            }
        } else if self.cursor_pos > 0 {
            self.text = styling_cursor::delete_visual_range(&self.text, self.cursor_pos - 1, self.cursor_pos);
            self.cursor_pos -= 1;
            let (cleaned, new_pos) = styling_cursor::cleanup_empty_styles(&self.text, self.cursor_pos);
            self.text = cleaned;
            self.cursor_pos = new_pos;
        }
        self.preferred_col = None;
        self.reset_blink();
    }

    /// Delete forward in styled mode: delete visual char after cursor.
    pub fn delete_forward_styled(&mut self) {
        if self.delete_selection_styled() {
            return;
        }
        if self.no_styles_movement {
            let cp = styling_cursor::cursor_to_content(&self.text, self.cursor_pos);
            let content_len = styling_cursor::strip_styling(&self.text).chars().count();
            if cp < content_len {
                self.text = styling_cursor::delete_content_range(&self.text, cp, cp + 1);
                self.cursor_pos = styling_cursor::content_to_cursor(&self.text, cp, true);
                let (cleaned, new_pos) = styling_cursor::cleanup_empty_styles(&self.text, self.cursor_pos);
                self.text = cleaned;
                self.cursor_pos = new_pos;
                self.snap_to_content_pos();
            }
        } else {
            let vis_len = self.cursor_len_styled();
            if self.cursor_pos < vis_len {
                self.text = styling_cursor::delete_visual_range(&self.text, self.cursor_pos, self.cursor_pos + 1);
                let (cleaned, new_pos) = styling_cursor::cleanup_empty_styles(&self.text, self.cursor_pos);
                self.text = cleaned;
                self.cursor_pos = new_pos;
            }
        }
        self.preferred_col = None;
        self.reset_blink();
    }

    /// Backspace word in styled mode.
    pub fn backspace_word_styled(&mut self) {
        if self.delete_selection_styled() {
            return;
        }
        if self.no_styles_movement {
            let cp = styling_cursor::cursor_to_content(&self.text, self.cursor_pos);
            let stripped = styling_cursor::strip_styling(&self.text);
            let target_cp = find_word_boundary_left(&stripped, cp);
            if target_cp < cp {
                self.text = styling_cursor::delete_content_range(&self.text, target_cp, cp);
                self.cursor_pos = styling_cursor::content_to_cursor(&self.text, target_cp, true);
                let (cleaned, new_pos) = styling_cursor::cleanup_empty_styles(&self.text, self.cursor_pos);
                self.text = cleaned;
                self.cursor_pos = new_pos;
                self.snap_to_content_pos();
            }
        } else {
            let target = styling_cursor::find_word_boundary_left_visual(&self.text, self.cursor_pos);
            self.text = styling_cursor::delete_visual_range(&self.text, target, self.cursor_pos);
            self.cursor_pos = target;
            let (cleaned, new_pos) = styling_cursor::cleanup_empty_styles(&self.text, self.cursor_pos);
            self.text = cleaned;
            self.cursor_pos = new_pos;
        }
        self.preferred_col = None;
        self.reset_blink();
    }

    /// Delete word forward in styled mode.
    pub fn delete_word_forward_styled(&mut self) {
        if self.delete_selection_styled() {
            return;
        }
        if self.no_styles_movement {
            let cp = styling_cursor::cursor_to_content(&self.text, self.cursor_pos);
            let stripped = styling_cursor::strip_styling(&self.text);
            let target_cp = find_word_delete_boundary_right(&stripped, cp);
            if target_cp > cp {
                self.text = styling_cursor::delete_content_range(&self.text, cp, target_cp);
                self.cursor_pos = styling_cursor::content_to_cursor(&self.text, cp, true);
                let (cleaned, new_pos) = styling_cursor::cleanup_empty_styles(&self.text, self.cursor_pos);
                self.text = cleaned;
                self.cursor_pos = new_pos;
                self.snap_to_content_pos();
            }
        } else {
            let target = styling_cursor::find_word_delete_boundary_right_visual(&self.text, self.cursor_pos);
            self.text = styling_cursor::delete_visual_range(&self.text, self.cursor_pos, target);
            let (cleaned, new_pos) = styling_cursor::cleanup_empty_styles(&self.text, self.cursor_pos);
            self.text = cleaned;
            self.cursor_pos = new_pos;
        }
        self.preferred_col = None;
        self.reset_blink();
    }

    /// Move left in styled mode (visual space).
    pub fn move_left_styled(&mut self, shift: bool) {
        if self.no_styles_movement {
            return self.move_left_content(shift);
        }
        if !shift {
            if let Some((start, _end)) = self.selection_range() {
                self.cursor_pos = start;
                self.selection_anchor = None;
                self.cleanup_after_move();
                return;
            }
        }
        if self.cursor_pos > 0 {
            if shift && self.selection_anchor.is_none() {
                self.selection_anchor = Some(self.cursor_pos);
            }
            self.cursor_pos -= 1;
            if shift {
                if self.selection_anchor == Some(self.cursor_pos) {
                    self.selection_anchor = None;
                }
            }
        }
        if !shift {
            self.selection_anchor = None;
        }
        self.cleanup_after_move();
    }

    /// Content-based left movement for `no_styles_movement` mode.
    /// Decrements in content space so the cursor skips structural positions.
    fn move_left_content(&mut self, shift: bool) {
        let cp = styling_cursor::cursor_to_content(&self.text, self.cursor_pos);
        if !shift {
            if let Some((start, _end)) = self.selection_range() {
                let sc = styling_cursor::cursor_to_content(&self.text, start);
                self.cursor_pos = styling_cursor::content_to_cursor(&self.text, sc, true);
                self.selection_anchor = None;
                self.cleanup_after_move();
                return;
            }
        }
        if cp > 0 {
            if shift && self.selection_anchor.is_none() {
                self.selection_anchor = Some(self.cursor_pos);
            }
            self.cursor_pos = styling_cursor::content_to_cursor(&self.text, cp - 1, true);
            if shift {
                if self.selection_anchor == Some(self.cursor_pos) {
                    self.selection_anchor = None;
                }
            }
        }
        if !shift {
            self.selection_anchor = None;
        }
        self.cleanup_after_move();
    }

    /// Move right in styled mode (visual space).
    pub fn move_right_styled(&mut self, shift: bool) {
        let vis_len = self.cursor_len_styled();
        if !shift {
            if let Some((_start, end)) = self.selection_range() {
                self.cursor_pos = end;
                self.selection_anchor = None;
                self.cleanup_after_move();
                return;
            }
        }
        if self.cursor_pos < vis_len {
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
        self.cleanup_after_move();
    }

    /// Move word left in styled mode.
    pub fn move_word_left_styled(&mut self, shift: bool) {
        if shift && self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor_pos);
        }
        self.cursor_pos = styling_cursor::find_word_boundary_left_visual(&self.text, self.cursor_pos);
        if !shift {
            self.selection_anchor = None;
        } else if self.selection_anchor == Some(self.cursor_pos) {
            self.selection_anchor = None;
        }
        self.cleanup_after_move();
    }

    /// Move word right in styled mode.
    pub fn move_word_right_styled(&mut self, shift: bool) {
        if shift && self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor_pos);
        }
        self.cursor_pos = styling_cursor::find_word_boundary_right_visual(&self.text, self.cursor_pos);
        if !shift {
            self.selection_anchor = None;
        } else if self.selection_anchor == Some(self.cursor_pos) {
            self.selection_anchor = None;
        }
        self.cleanup_after_move();
    }

    /// Move to start in styled mode.
    pub fn move_home_styled(&mut self, shift: bool) {
        if shift && self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor_pos);
        }
        self.cursor_pos = 0;
        if !shift {
            self.selection_anchor = None;
        } else if self.selection_anchor == Some(0) {
            self.selection_anchor = None;
        }
        self.cleanup_after_move();
    }

    /// Move to end in styled mode.
    pub fn move_end_styled(&mut self, shift: bool) {
        let vis_len = self.cursor_len_styled();
        if shift && self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor_pos);
        }
        self.cursor_pos = vis_len;
        if !shift {
            self.selection_anchor = None;
        } else if self.selection_anchor == Some(vis_len) {
            self.selection_anchor = None;
        }
        self.cleanup_after_move();
    }

    /// Move cursor up one visual line in styled mode.
    /// If `visual_lines` is provided (multiline with wrapping), uses them for navigation.
    /// Otherwise falls back to simple line-based movement.
    pub fn move_up_styled(&mut self, shift: bool, visual_lines: Option<&[VisualLine]>) {
        if shift && self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor_pos);
        }

        let raw_cursor = styling_cursor::cursor_to_raw_for_insertion(&self.text, self.cursor_pos);

        if let Some(vl) = visual_lines {
            let (line_idx, _raw_col) = cursor_to_visual_pos(vl, raw_cursor);

            // Compute column in content space (visible characters only)
            // so structural chars like `}` don't offset the column.
            let line_start_visual = styling_cursor::raw_to_cursor(&self.text, vl[line_idx].global_char_start);
            let content_start = styling_cursor::cursor_to_content(&self.text, line_start_visual);
            let content_current = styling_cursor::cursor_to_content(&self.text, self.cursor_pos);
            let current_col = content_current.saturating_sub(content_start);
            let col = self.preferred_col.unwrap_or(current_col);

            if line_idx == 0 {
                // Already on first line — move to start
                self.cursor_pos = 0;
            } else {
                let target = &vl[line_idx - 1];
                let target_start_visual = styling_cursor::raw_to_cursor(&self.text, target.global_char_start);
                let target_end_visual = styling_cursor::raw_to_cursor(
                    &self.text,
                    target.global_char_start + target.char_count,
                );
                let target_content_start = styling_cursor::cursor_to_content(&self.text, target_start_visual);
                let target_content_end = styling_cursor::cursor_to_content(&self.text, target_end_visual);
                let target_content_len = target_content_end - target_content_start;
                let target_col = col.min(target_content_len);
                self.cursor_pos = styling_cursor::content_to_cursor(&self.text, target_content_start + target_col, false);
            }

            self.preferred_col = Some(col);
        } else {
            // Simple line-based movement for non-multiline
            let (line, _col) = styling_cursor::line_and_column_styled(&self.text, self.cursor_pos);
            let col = self.preferred_col.unwrap_or({
                let line_start = styling_cursor::line_start_visual_styled(&self.text, line);
                let content_start = styling_cursor::cursor_to_content(&self.text, line_start);
                let content_current = styling_cursor::cursor_to_content(&self.text, self.cursor_pos);
                content_current.saturating_sub(content_start)
            });

            if line == 0 {
                self.cursor_pos = 0;
            } else {
                let target_start = styling_cursor::line_start_visual_styled(&self.text, line - 1);
                let target_end = styling_cursor::line_end_visual_styled(&self.text, line - 1);
                let target_content_start = styling_cursor::cursor_to_content(&self.text, target_start);
                let target_content_end = styling_cursor::cursor_to_content(&self.text, target_end);
                let target_content_len = target_content_end - target_content_start;
                let target_col = col.min(target_content_len);
                self.cursor_pos = styling_cursor::content_to_cursor(&self.text, target_content_start + target_col, false);
            }

            self.preferred_col = Some(col);
        }

        if !shift {
            self.selection_anchor = None;
        } else if self.selection_anchor == Some(self.cursor_pos) {
            self.selection_anchor = None;
        }
        self.reset_blink();
    }

    /// Move cursor down one visual line in styled mode.
    pub fn move_down_styled(&mut self, shift: bool, visual_lines: Option<&[VisualLine]>) {
        if shift && self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor_pos);
        }

        let vis_len = self.cursor_len_styled();
        let raw_cursor = styling_cursor::cursor_to_raw_for_insertion(&self.text, self.cursor_pos);

        if let Some(vl) = visual_lines {
            let (line_idx, _raw_col) = cursor_to_visual_pos(vl, raw_cursor);

            // Compute column in content space (visible characters only)
            let line_start_visual = styling_cursor::raw_to_cursor(&self.text, vl[line_idx].global_char_start);
            let content_start = styling_cursor::cursor_to_content(&self.text, line_start_visual);
            let content_current = styling_cursor::cursor_to_content(&self.text, self.cursor_pos);
            let current_col = content_current.saturating_sub(content_start);
            let col = self.preferred_col.unwrap_or(current_col);

            if line_idx >= vl.len() - 1 {
                // Already on last line — move to end
                self.cursor_pos = vis_len;
            } else {
                let target = &vl[line_idx + 1];
                let target_start_visual = styling_cursor::raw_to_cursor(&self.text, target.global_char_start);
                let target_end_visual = styling_cursor::raw_to_cursor(
                    &self.text,
                    target.global_char_start + target.char_count,
                );
                let target_content_start = styling_cursor::cursor_to_content(&self.text, target_start_visual);
                let target_content_end = styling_cursor::cursor_to_content(&self.text, target_end_visual);
                let target_content_len = target_content_end - target_content_start;
                let target_col = col.min(target_content_len);
                self.cursor_pos = styling_cursor::content_to_cursor(&self.text, target_content_start + target_col, false);
            }

            self.preferred_col = Some(col);
        } else {
            // Simple line-based movement
            let (line, _col) = styling_cursor::line_and_column_styled(&self.text, self.cursor_pos);
            let line_count = styling_cursor::styled_line_count(&self.text);
            let col = self.preferred_col.unwrap_or({
                let line_start = styling_cursor::line_start_visual_styled(&self.text, line);
                let content_start = styling_cursor::cursor_to_content(&self.text, line_start);
                let content_current = styling_cursor::cursor_to_content(&self.text, self.cursor_pos);
                content_current.saturating_sub(content_start)
            });

            if line >= line_count - 1 {
                self.cursor_pos = vis_len;
            } else {
                let target_start = styling_cursor::line_start_visual_styled(&self.text, line + 1);
                let target_end = styling_cursor::line_end_visual_styled(&self.text, line + 1);
                let target_content_start = styling_cursor::cursor_to_content(&self.text, target_start);
                let target_content_end = styling_cursor::cursor_to_content(&self.text, target_end);
                let target_content_len = target_content_end - target_content_start;
                let target_col = col.min(target_content_len);
                self.cursor_pos = styling_cursor::content_to_cursor(&self.text, target_content_start + target_col, false);
            }

            self.preferred_col = Some(col);
        }

        if !shift {
            self.selection_anchor = None;
        } else if self.selection_anchor == Some(self.cursor_pos) {
            self.selection_anchor = None;
        }
        self.reset_blink();
    }

    /// Select all in styled mode.
    pub fn select_all_styled(&mut self) {
        let vis_len = self.cursor_len_styled();
        if vis_len > 0 {
            self.selection_anchor = Some(0);
            self.cursor_pos = vis_len;
            self.snap_to_content_pos();
        }
        self.reset_blink();
    }

    /// Click to cursor in styled mode.
    /// `click_visual_pos` is the visual character position determined from click x.
    pub fn click_to_cursor_styled(&mut self, click_visual_pos: usize, shift: bool) {
        if shift {
            if self.selection_anchor.is_none() {
                self.selection_anchor = Some(self.cursor_pos);
            }
        } else {
            self.selection_anchor = None;
        }
        self.cursor_pos = click_visual_pos;
        if shift {
            if self.selection_anchor == Some(self.cursor_pos) {
                self.selection_anchor = None;
            }
        }
        self.cleanup_after_move();
    }

    /// Select word at visual position in styled mode.
    pub fn select_word_at_styled(&mut self, visual_pos: usize) {
        let (start, end) = styling_cursor::find_word_at_visual(&self.text, visual_pos);
        if start != end {
            self.selection_anchor = Some(start);
            self.cursor_pos = end;
            self.snap_to_content_pos();
        }
        self.reset_blink();
    }

    /// Snap `cursor_pos` (and `selection_anchor`) so they only land on
    /// visible-character boundaries, skipping `}` and empty-content markers.
    /// No-op when `no_styles_movement` is false.
    fn snap_to_content_pos(&mut self) {
        if !self.no_styles_movement { return; }
        let cp = styling_cursor::cursor_to_content(&self.text, self.cursor_pos);
        self.cursor_pos = styling_cursor::content_to_cursor(&self.text, cp, true);
        if let Some(anchor) = self.selection_anchor {
            let ac = styling_cursor::cursor_to_content(&self.text, anchor);
            self.selection_anchor = Some(
                styling_cursor::content_to_cursor(&self.text, ac, true),
            );
            if self.selection_anchor == Some(self.cursor_pos) {
                self.selection_anchor = None;
            }
        }
    }

    /// Cleanup empty styles after a cursor movement.
    /// Called after any movement that doesn't modify text.
    fn cleanup_after_move(&mut self) {
        // Snap first so the cursor moves away from structural positions;
        // this lets cleanup_empty_styles remove tags the cursor isn't inside.
        self.snap_to_content_pos();
        let (cleaned, new_pos) = styling_cursor::cleanup_empty_styles(&self.text, self.cursor_pos);
        self.text = cleaned;
        self.cursor_pos = new_pos;
        // Re-snap after cleanup since the text may have changed.
        self.snap_to_content_pos();
        self.preferred_col = None;
        self.reset_blink();
    }

    /// Convert the visual cursor_pos to a raw position for rendering.
    /// Enters empty style tags at the cursor boundary.
    pub fn cursor_pos_raw(&self) -> usize {
        styling_cursor::cursor_to_raw_for_insertion(&self.text, self.cursor_pos)
    }

    /// Convert the visual selection_anchor to a raw position for rendering.
    pub fn selection_anchor_raw(&self) -> Option<usize> {
        self.selection_anchor.map(|a| styling_cursor::cursor_to_raw(&self.text, a))
    }

    /// Get the selection range in raw positions for rendering.
    pub fn selection_range_raw(&self) -> Option<(usize, usize)> {
        self.selection_anchor.map(|anchor| {
            let raw_anchor = styling_cursor::cursor_to_raw(&self.text, anchor);
            let raw_cursor = styling_cursor::cursor_to_raw(&self.text, self.cursor_pos);
            let start = raw_anchor.min(raw_cursor);
            let end = raw_anchor.max(raw_cursor);
            (start, end)
        })
    }
}

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
    /// Override line height in pixels. When 0 (default), the natural font height is used.
    pub line_height: u16,
    /// When true, cursor movement skips over `}` and empty content style positions.
    pub no_styles_movement: bool,
    /// The font asset to use. Resolved by the renderer.
    pub font_asset: Option<&'static crate::renderer::FontAsset>,
}

impl Default for TextInputConfig {
    fn default() -> Self {
        Self {
            placeholder: String::new(),
            max_length: None,
            is_password: false,
            is_multiline: false,
            font_size: 0,
            text_color: Color::rgba(255.0, 255.0, 255.0, 255.0),
            placeholder_color: Color::rgba(128.0, 128.0, 128.0, 255.0),
            cursor_color: Color::rgba(255.0, 255.0, 255.0, 255.0),
            selection_color: Color::rgba(69.0, 130.0, 181.0, 128.0),
            line_height: 0,
            no_styles_movement: false,
            font_asset: None,
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

    /// Sets the font to use for this text input.
    ///
    /// The font is loaded asynchronously during rendering.
    #[inline]
    pub fn font(&mut self, asset: &'static crate::renderer::FontAsset) -> &mut Self {
        self.config.font_asset = Some(asset);
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

    /// Sets the line height in pixels for multiline inputs.
    ///
    /// When set to a value greater than 0, this overrides the natural font
    /// height for spacing between lines. Text is vertically centred within
    /// each line slot. A value of 0 (default) uses the natural font height.
    #[inline]
    pub fn line_height(&mut self, height: u16) -> &mut Self {
        self.config.line_height = height;
        self
    }

    /// Enables no-styles movement mode.
    /// When enabled, cursor navigation skips over `}` exit positions and
    /// empty content markers, so the cursor only stops at visible character
    /// boundaries. Useful for live-highlighted text inputs where the user
    /// should not navigate through invisible style markup.
    #[inline]
    pub fn no_styles_movement(&mut self) -> &mut Self {
        self.config.no_styles_movement = true;
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

/// Convert a character index to a byte index in the string.
pub fn char_index_to_byte(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(byte_pos, _)| byte_pos)
        .unwrap_or(s.len())
}

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
    font_asset: Option<&'static crate::renderer::FontAsset>,
    font_size: u16,
    measure_fn: &dyn Fn(&str, &crate::text::TextConfig) -> crate::math::Dimensions,
) -> Vec<VisualLine> {
    let config = crate::text::TextConfig {
        font_asset,
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

            #[cfg(feature = "text-styling")]
            {
                // Styling-aware measurement: skip calling measure_fn for substrings that
                // end inside a tag header ({name|) to avoid warnings from incomplete tags.
                // Tag header chars have zero visible width, so advancing fit_count is safe.
                let mut in_tag_hdr = false;
                let mut escaped = false;
                for i in 1..=(total_chars - line_char_start) {
                    let ch = chars[line_char_start + i - 1];
                    if escaped {
                        escaped = false;
                        // Escaped char is visible: measure
                        let substr: String = chars[line_char_start..line_char_start + i].iter().collect();
                        let w = measure_fn(&substr, &config).width;
                        if w > max_width { break; }
                        fit_count = i;
                        continue;
                    }
                    match ch {
                        '\\' => { escaped = true; /* don't update fit_count: \ and next char are atomic */ }
                        '{' => { in_tag_hdr = true; fit_count = i; }
                        '|' if in_tag_hdr => { in_tag_hdr = false; fit_count = i; }
                        '}' => { fit_count = i; }
                        _ if in_tag_hdr => { fit_count = i; }
                        _ => {
                            // Visible char: measure the substring
                            let substr: String = chars[line_char_start..line_char_start + i].iter().collect();
                            let w = measure_fn(&substr, &config).width;
                            if w > max_width { break; }
                            fit_count = i;
                        }
                    }
                }
            }

            #[cfg(not(feature = "text-styling"))]
            {
                for i in 1..=(total_chars - line_char_start) {
                    let substr: String = chars[line_char_start..line_char_start + i].iter().collect();
                    let w = measure_fn(&substr, &config).width;
                    if w > max_width {
                        break;
                    }
                    fit_count = i;
                }
            }

            if fit_count == 0 {
                // Even a single character doesn't fit; force at least one visible unit.
                // If the first char is a backslash (escape), include the next char too
                // so we never split an escape sequence across lines.
                #[cfg(feature = "text-styling")]
                if chars[line_char_start] == '\\' && line_char_start + 2 <= total_chars {
                    fit_count = 2;
                } else {
                    fit_count = 1;
                }
                #[cfg(not(feature = "text-styling"))]
                {
                    fit_count = 1;
                }
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
                #[allow(unused_mut)]
                let mut wrap_count = if found_space { break_at } else { fit_count };
                // Never split an escape sequence (\{, \}, etc.) across lines
                #[cfg(feature = "text-styling")]
                if wrap_count > 0
                    && chars[line_char_start + wrap_count - 1] == '\\'
                    && line_char_start + wrap_count < total_chars
                {
                    if wrap_count > 1 {
                        wrap_count -= 1; // back up before the backslash
                    } else {
                        wrap_count = 2.min(total_chars - line_char_start); // include the escape pair
                    }
                }
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
    let mut i = pos.min(chars.len());
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
/// Skips whitespace first, then stops at the end of the next word.
pub fn find_word_boundary_right(text: &str, pos: usize) -> usize {
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    if pos >= len {
        return len;
    }
    let mut i = pos;
    // Skip whitespace to the right
    while i < len && chars[i].is_whitespace() {
        i += 1;
    }
    // Skip non-whitespace (word) to the right
    while i < len && !chars[i].is_whitespace() {
        i += 1;
    }
    i
}

/// Find the delete boundary to the right of `pos` (for Ctrl+Delete).
/// Deletes the current word AND trailing whitespace (skips word → skips spaces).
pub fn find_word_delete_boundary_right(text: &str, pos: usize) -> usize {
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

/// When text-styling is enabled, the raw string contains markup like `{red|...}` and
/// escape sequences like `\{`. The user-visible "visual" positions ignore all markup.
///
/// These helpers convert between visual positions (what the user sees / cursor_pos)
/// and raw char positions (byte-level indices into the raw string).
///
/// Terminology:
/// - "raw position" = char index into the full raw string (including markup)
/// - "visual position" = char index into the displayed (stripped) text
#[cfg(feature = "text-styling")]
pub mod styling_cursor {
    /// Escape a character that would be interpreted as styling markup.
    /// Characters `{`, `}`, `|`, and `\` are prefixed with `\`.
    pub fn escape_char(ch: char) -> String {
        match ch {
            '{' | '}' | '|' | '\\' => format!("\\{}", ch),
            _ => ch.to_string(),
        }
    }

    /// Escape all styling-significant characters in a string.
    pub fn escape_str(s: &str) -> String {
        let mut result = String::with_capacity(s.len());
        for ch in s.chars() {
            match ch {
                '{' | '}' | '|' | '\\' => {
                    result.push('\\');
                    result.push(ch);
                }
                _ => result.push(ch),
            }
        }
        result
    }

    /// Convert a visual (display) cursor position to a raw char position.
    ///
    /// Visual elements include:
    /// - Visible characters (escaped chars like `\{` count as one)
    /// - `}` closing a style tag (the "exit tag" position)
    /// - Empty content area of an empty `{name|}` tag
    ///
    /// `{name|` headers are transparent and occupy no visual positions.
    ///
    /// Position 0 always maps to raw 0.
    /// For visible char positions, the result is advanced past any following
    /// `{name|` headers so the cursor lands inside the tag content.
    pub fn cursor_to_raw(raw: &str, visual_pos: usize) -> usize {
        if visual_pos == 0 {
            return 0;
        }

        let chars: Vec<char> = raw.chars().collect();
        let len = chars.len();
        let mut visual = 0usize;
        let mut raw_idx = 0usize;
        let mut escaped = false;
        let mut in_style_def = false;

        while raw_idx < len {
            let c = chars[raw_idx];

            if escaped {
                // Escaped char is a visible element
                visual += 1;
                escaped = false;
                raw_idx += 1;
                if visual == visual_pos {
                    return skip_tag_headers(&chars, raw_idx);
                }
                continue;
            }

            match c {
                '\\' => {
                    escaped = true;
                    raw_idx += 1;
                }
                '{' if !in_style_def => {
                    in_style_def = true;
                    raw_idx += 1;
                }
                '|' if in_style_def => {
                    in_style_def = false;
                    // Check for empty content: if next char is `}`
                    if raw_idx + 1 < len && chars[raw_idx + 1] == '}' {
                        // Empty content marker — counts as a visual element
                        visual += 1;
                        raw_idx += 1; // now at `}`
                        if visual == visual_pos {
                            return raw_idx; // position at `}`, i.e. between `|` and `}`
                        }
                        // The `}` itself will be processed in the next iteration
                    } else {
                        raw_idx += 1;
                    }
                }
                '}' if !in_style_def => {
                    // Closing brace counts as a visual element (exit tag position)
                    visual += 1;
                    raw_idx += 1;
                    if visual == visual_pos {
                        // After `}` — DON'T skip tag headers; cursor is outside the tag
                        return raw_idx;
                    }
                }
                _ if in_style_def => {
                    raw_idx += 1;
                }
                _ => {
                    // Regular visible character
                    visual += 1;
                    raw_idx += 1;
                    if visual == visual_pos {
                        // After visible char — skip past any following `{name|` headers
                        // so the cursor lands inside the next tag's content area
                        return skip_tag_headers(&chars, raw_idx);
                    }
                }
            }
        }

        len
    }

    /// Skip past `{name|` tag headers starting at `pos`.
    /// Returns the position after all consecutive tag headers.
    fn skip_tag_headers(chars: &[char], pos: usize) -> usize {
        let len = chars.len();
        let mut p = pos;
        while p < len && chars[p] == '{' {
            let mut j = p + 1;
            while j < len && chars[j] != '|' && chars[j] != '}' {
                j += 1;
            }
            if j < len && chars[j] == '|' {
                p = j + 1; // skip past the `|`
            } else {
                break; // Not a valid tag header
            }
        }
        p
    }

    /// Convert a raw char position to a visual (display) cursor position.
    /// Accounts for `}` and empty content positions.
    pub fn raw_to_cursor(raw: &str, raw_pos: usize) -> usize {
        let chars: Vec<char> = raw.chars().collect();
        let len = chars.len();
        let mut visual = 0usize;
        let mut raw_idx = 0usize;
        let mut escaped = false;
        let mut in_style_def = false;

        while raw_idx < len && raw_idx < raw_pos {
            let c = chars[raw_idx];

            if escaped {
                visual += 1;
                escaped = false;
                raw_idx += 1;
                continue;
            }

            match c {
                '\\' => {
                    escaped = true;
                    raw_idx += 1;
                }
                '{' if !in_style_def => {
                    in_style_def = true;
                    raw_idx += 1;
                }
                '|' if in_style_def => {
                    in_style_def = false;
                    // Check for empty content
                    if raw_idx + 1 < len && chars[raw_idx + 1] == '}' {
                        visual += 1; // empty content position
                        raw_idx += 1; // now at `}`
                        // Don't increment raw_idx again; `}` will be processed next
                    } else {
                        raw_idx += 1;
                    }
                }
                '}' if !in_style_def => {
                    visual += 1; // exit tag position
                    raw_idx += 1;
                }
                _ if in_style_def => {
                    raw_idx += 1;
                }
                _ => {
                    visual += 1;
                    raw_idx += 1;
                }
            }
        }

        visual
    }

    /// Count the total number of visual positions in a raw styled string.
    /// Includes visible chars, `}` exit positions, and empty content positions.
    pub fn cursor_len(raw: &str) -> usize {
        raw_to_cursor(raw, raw.chars().count())
    }

    /// If `pos` (a raw char index) points to the start of an empty `{name|}` tag,
    /// advance into it (returning the position between `|` and `}`).
    /// Handles consecutive empty tags by entering each one.
    fn enter_empty_tags_at(chars: &[char], pos: usize) -> usize {
        let len = chars.len();
        let mut p = pos;
        while p < len && chars[p] == '{' {
            // Scan for `|`
            let mut j = p + 1;
            while j < len && chars[j] != '|' && chars[j] != '}' {
                j += 1;
            }
            if j < len && chars[j] == '|' {
                // Found `{name|`, check if content is empty (next char is `}`)
                if j + 1 < len && chars[j + 1] == '}' {
                    // Empty tag! Enter it (position between `|` and `}`)
                    p = j + 1;
                } else {
                    break; // Non-empty tag — don't enter
                }
            } else {
                break; // Not a valid style tag
            }
        }
        p
    }

    /// Like `cursor_to_raw`, but also enters empty style tags at the boundary
    /// when the basic position lands right before one.
    /// Use this for cursor positioning and single-point insertion.
    pub fn cursor_to_raw_for_insertion(raw: &str, visual_pos: usize) -> usize {
        let pos = cursor_to_raw(raw, visual_pos);
        let chars: Vec<char> = raw.chars().collect();
        // If pos lands right at the start of an empty {name|} tag, enter it
        enter_empty_tags_at(&chars, pos)
    }

    /// Insert a (pre-escaped) string at the given visual position in the raw text.
    /// Returns the new raw string and the new visual cursor position after insertion.
    /// Enters empty style tags at the cursor boundary so typed text goes inside them.
    pub fn insert_at_visual(raw: &str, visual_pos: usize, insert: &str) -> (String, usize) {
        let raw_pos = cursor_to_raw_for_insertion(raw, visual_pos);
        let byte_pos = super::char_index_to_byte(raw, raw_pos);
        let mut new_raw = String::with_capacity(raw.len() + insert.len());
        new_raw.push_str(&raw[..byte_pos]);
        new_raw.push_str(insert);
        new_raw.push_str(&raw[byte_pos..]);
        let inserted_visual = cursor_len(insert);
        (new_raw, visual_pos + inserted_visual)
    }

    /// Delete visible characters in the visual range `[visual_start, visual_end)`.
    /// Preserves all style tag structure (`{name|`, `}`) and only removes the content
    /// characters that fall within the visual range.
    pub fn delete_visual_range(raw: &str, visual_start: usize, visual_end: usize) -> String {
        if visual_start >= visual_end {
            return raw.to_string();
        }

        let chars: Vec<char> = raw.chars().collect();
        let len = chars.len();
        let mut result = String::with_capacity(raw.len());
        let mut visual = 0usize;
        let mut i = 0;
        let mut in_style_def = false;

        while i < len {
            let c = chars[i];

            match c {
                '\\' if !in_style_def && i + 1 < len => {
                    // Escaped pair `\X` counts as one visible char
                    let in_range = visual >= visual_start && visual < visual_end;
                    if !in_range {
                        result.push(c);
                        result.push(chars[i + 1]);
                    }
                    visual += 1;
                    i += 2;
                }
                '{' if !in_style_def => {
                    in_style_def = true;
                    result.push(c); // Always keep tag structure
                    i += 1;
                }
                '|' if in_style_def => {
                    in_style_def = false;
                    result.push(c);
                    // Check for empty content
                    if i + 1 < len && chars[i + 1] == '}' {
                        visual += 1; // Empty content has a visual position but is structural
                    }
                    i += 1;
                }
                '}' if !in_style_def => {
                    result.push(c); // Always keep `}`
                    visual += 1; // `}` has a visual position but is structural
                    i += 1;
                }
                _ if in_style_def => {
                    result.push(c); // Tag name chars — always keep
                    i += 1;
                }
                _ => {
                    let in_range = visual >= visual_start && visual < visual_end;
                    if !in_range {
                        result.push(c);
                    }
                    visual += 1;
                    i += 1;
                }
            }
        }

        result
    }

    /// Remove empty style tags (`{style|}`) from the raw string,
    /// EXCEPT those that contain the cursor. A cursor is "inside" an empty
    /// style tag if its visual position equals the visual position of that tag's content area.
    ///
    /// Returns the new raw string and the (possibly adjusted) visual cursor position.
    pub fn cleanup_empty_styles(raw: &str, cursor_visual_pos: usize) -> (String, usize) {
        let chars: Vec<char> = raw.chars().collect();
        let len = chars.len();
        let mut result = String::with_capacity(raw.len());
        let mut i = 0;
        let mut visual = 0usize;
        let mut escaped = false;
        let mut cursor_adj = cursor_visual_pos;

        // We need to track style nesting to correctly identify empty tags
        while i < len {
            let c = chars[i];

            if escaped {
                result.push(c);
                visual += 1;
                escaped = false;
                i += 1;
                continue;
            }

            match c {
                '\\' => {
                    escaped = true;
                    result.push(c);
                    i += 1;
                }
                '{' => {
                    // Look ahead: find the matching `|`, then check if there's content
                    // before the closing `}`. Pattern: `{...| <content> }`
                    // Find the `|` that ends this style definition
                    let mut j = i + 1;
                    let mut style_escaped = false;
                    let mut found_pipe = false;
                    while j < len {
                        if style_escaped {
                            style_escaped = false;
                            j += 1;
                            continue;
                        }
                        if chars[j] == '\\' {
                            style_escaped = true;
                            j += 1;
                            continue;
                        }
                        if chars[j] == '|' {
                            found_pipe = true;
                            j += 1; // j now points to first char after `|`
                            break;
                        }
                        if chars[j] == '{' {
                            // Nested `{` inside style def — not valid but push through
                            j += 1;
                            continue;
                        }
                        j += 1;
                    }

                    if !found_pipe {
                        // Malformed — just push as-is
                        result.push(c);
                        i += 1;
                        continue;
                    }

                    // j points to first char after `|`
                    // Now scan for the closing `}` and check if there's any visible content
                    let _content_start_raw = j;
                    let mut k = j;
                    let mut content_escaped = false;
                    let mut has_visible_content = false;
                    let mut nesting = 1; // Track nested style tags
                    while k < len && nesting > 0 {
                        if content_escaped {
                            has_visible_content = true;
                            content_escaped = false;
                            k += 1;
                            continue;
                        }
                        match chars[k] {
                            '\\' => {
                                content_escaped = true;
                                k += 1;
                            }
                            '{' => {
                                // Nested style opening
                                nesting += 1;
                                k += 1;
                            }
                            '}' => {
                                nesting -= 1;
                                if nesting == 0 {
                                    break; // k points to the closing `}`
                                }
                                k += 1;
                            }
                            '|' => {
                                // Could be pipe inside nested style def
                                k += 1;
                            }
                            _ => {
                                has_visible_content = true;
                                k += 1;
                            }
                        }
                    }

                    if !has_visible_content && nesting == 0 {
                        // This is an empty style tag: `{style| <possibly nested empty tags> }`
                        // In the new model, empty content is at visual position `visual`
                        // and `}` is at visual position `visual + 1`.
                        // Keep the tag if cursor is at either position.
                        let cursor_is_inside = cursor_visual_pos == visual
                            || cursor_visual_pos == visual + 1;
                        if cursor_is_inside {
                            // Keep the tag — push everything from i to k (inclusive)
                            for idx in i..=k {
                                result.push(chars[idx]);
                            }
                            visual += 2; // empty content + }
                        } else {
                            // Remove the entire tag
                            // Adjust cursor if it was after this tag
                            if cursor_adj > visual {
                                cursor_adj = cursor_adj.saturating_sub(2);
                            }
                        }
                        i = k + 1;
                    } else {
                        // Non-empty style tag — keep the entire header `{...|`
                        // j points to the first char after `|`
                        for idx in i..j {
                            result.push(chars[idx]);
                        }
                        // Header is transparent — no visual increment
                        i = j;
                    }
                }
                '}' => {
                    result.push(c);
                    visual += 1; // } has a visual position in the new model
                    i += 1;
                }
                _ => {
                    result.push(c);
                    visual += 1;
                    i += 1;
                }
            }
        }

        (result, cursor_adj)
    }

    /// Get the visual character at a given visual position, or None if past end.
    pub fn visual_char_at(raw: &str, visual_pos: usize) -> Option<char> {
        let raw_pos = cursor_to_raw(raw, visual_pos);
        let chars: Vec<char> = raw.chars().collect();
        if raw_pos >= chars.len() {
            return None;
        }
        // If the char at raw_pos is `\`, the visible char is the next one
        if chars[raw_pos] == '\\' && raw_pos + 1 < chars.len() {
            Some(chars[raw_pos + 1])
        } else {
            Some(chars[raw_pos])
        }
    }

    /// Strip all styling markup from a raw string, returning only visible text.
    pub fn strip_styling(raw: &str) -> String {
        let mut result = String::new();
        let mut escaped = false;
        let mut in_style_def = false;
        for c in raw.chars() {
            if escaped {
                result.push(c);
                escaped = false;
                continue;
            }
            match c {
                '\\' => { escaped = true; }
                '{' if !in_style_def => { in_style_def = true; }
                '|' if in_style_def => { in_style_def = false; }
                '}' if !in_style_def => { /* closing tag, skip */ }
                _ if in_style_def => { /* inside style def, skip */ }
                _ => { result.push(c); }
            }
        }
        result
    }

    /// Convert a "structural visual" position (includes } and empty content markers)
    /// to a "content position" (just visible chars, matching strip_styling output).
    /// Content position is clamped to stripped text length.
    pub fn cursor_to_content(raw: &str, cursor_pos: usize) -> usize {
        let chars: Vec<char> = raw.chars().collect();
        let len = chars.len();
        let mut visual = 0usize;
        let mut content = 0usize;
        let mut escaped = false;
        let mut in_style_def = false;

        for i in 0..len {
            if visual >= cursor_pos {
                break;
            }
            let c = chars[i];

            if escaped {
                visual += 1;
                content += 1;
                escaped = false;
                continue;
            }

            match c {
                '\\' => { escaped = true; }
                '{' if !in_style_def => { in_style_def = true; }
                '|' if in_style_def => {
                    in_style_def = false;
                    if i + 1 < len && chars[i + 1] == '}' {
                        visual += 1; // empty content position (not a real content char)
                    }
                }
                '}' if !in_style_def => {
                    visual += 1; // } position (not a real content char)
                }
                _ if in_style_def => {}
                _ => {
                    visual += 1;
                    content += 1;
                }
            }
        }

        content
    }

    /// Convert a "content position" (from strip_styling output) back to a
    /// "structural visual" position (includes } and empty content markers).
    ///
    /// When `skip_structural` is true, returns the visual position immediately
    /// before the `content_pos`-th visible character — or at the end of the
    /// visual text when `content_pos` equals the content length.  This means
    /// the cursor only ever lands on visible-character boundaries (used by
    /// `no_styles_movement`).
    pub fn content_to_cursor(raw: &str, content_pos: usize, snap_to_content: bool) -> usize {
        let chars: Vec<char> = raw.chars().collect();
        let len = chars.len();
        let mut visual = 0usize;
        let mut content = 0usize;
        let mut escaped = false;
        let mut in_style_def = false;

        if snap_to_content {
            // No-structural mode: check `content >= content_pos` BEFORE advancing
            for i in 0..len {
                let c = chars[i];

                if escaped {
                    if content >= content_pos {
                        return visual;
                    }
                    visual += 1;
                    content += 1;
                    escaped = false;
                    continue;
                }

                match c {
                    '\\' => { escaped = true; }
                    '{' if !in_style_def => { in_style_def = true; }
                    '|' if in_style_def => {
                        in_style_def = false;
                        if i + 1 < len && chars[i + 1] == '}' {
                            visual += 1; // empty content marker — skip
                        }
                    }
                    '}' if !in_style_def => {
                        visual += 1; // } exit marker — skip
                    }
                    _ if in_style_def => {}
                    _ => {
                        if content >= content_pos {
                            return visual;
                        }
                        visual += 1;
                        content += 1;
                    }
                }
            }
        } else {
            // Structural mode: break when `content >= content_pos` at top of loop
            for i in 0..len {
                if content >= content_pos {
                    break;
                }
                let c = chars[i];

                if escaped {
                    visual += 1;
                    content += 1;
                    escaped = false;
                    continue;
                }

                match c {
                    '\\' => { escaped = true; }
                    '{' if !in_style_def => { in_style_def = true; }
                    '|' if in_style_def => {
                        in_style_def = false;
                        if i + 1 < len && chars[i + 1] == '}' {
                            visual += 1; // empty content
                        }
                    }
                    '}' if !in_style_def => {
                        visual += 1; // } position
                    }
                    _ if in_style_def => {}
                    _ => {
                        visual += 1;
                        content += 1;
                    }
                }
            }
        }

        visual
    }

    /// Delete content characters in `[content_start, content_end)` from the
    /// raw styled string, preserving all structural/tag characters.
    pub fn delete_content_range(raw: &str, content_start: usize, content_end: usize) -> String {
        if content_start >= content_end {
            return raw.to_string();
        }

        let chars: Vec<char> = raw.chars().collect();
        let len = chars.len();
        let mut result = String::with_capacity(raw.len());
        let mut content = 0usize;
        let mut i = 0;
        let mut in_style_def = false;

        while i < len {
            let c = chars[i];

            match c {
                '\\' if !in_style_def && i + 1 < len => {
                    let in_range = content >= content_start && content < content_end;
                    if !in_range {
                        result.push(c);
                        result.push(chars[i + 1]);
                    }
                    content += 1;
                    i += 2;
                }
                '{' if !in_style_def => {
                    in_style_def = true;
                    result.push(c);
                    i += 1;
                }
                '|' if in_style_def => {
                    in_style_def = false;
                    result.push(c);
                    i += 1;
                }
                '}' if !in_style_def => {
                    result.push(c);
                    i += 1;
                }
                _ if in_style_def => {
                    result.push(c);
                    i += 1;
                }
                _ => {
                    let in_range = content >= content_start && content < content_end;
                    if !in_range {
                        result.push(c);
                    }
                    content += 1;
                    i += 1;
                }
            }
        }

        result
    }

    /// Find word boundary left in visual space.
    /// Returns a visual position.
    pub fn find_word_boundary_left_visual(raw: &str, visual_pos: usize) -> usize {
        let cp = cursor_to_content(raw, visual_pos);
        let stripped = strip_styling(raw);
        let boundary = super::find_word_boundary_left(&stripped, cp);
        content_to_cursor(raw, boundary, false)
    }

    /// Find word boundary right in visual space.
    /// Returns a visual position.
    pub fn find_word_boundary_right_visual(raw: &str, visual_pos: usize) -> usize {
        let cp = cursor_to_content(raw, visual_pos);
        let stripped = strip_styling(raw);
        let boundary = super::find_word_boundary_right(&stripped, cp);
        content_to_cursor(raw, boundary, false)
    }

    /// Find word delete boundary right in visual space (skips word then spaces).
    /// Used for Ctrl+Delete to delete word + trailing whitespace.
    pub fn find_word_delete_boundary_right_visual(raw: &str, visual_pos: usize) -> usize {
        let cp = cursor_to_content(raw, visual_pos);
        let stripped = strip_styling(raw);
        let boundary = super::find_word_delete_boundary_right(&stripped, cp);
        content_to_cursor(raw, boundary, false)
    }

    /// Find word at a visual position (for double-click selection).
    /// Returns (start, end) in visual positions.
    pub fn find_word_at_visual(raw: &str, visual_pos: usize) -> (usize, usize) {
        let cp = cursor_to_content(raw, visual_pos);
        let stripped = strip_styling(raw);
        let (s, e) = super::find_word_at(&stripped, cp);
        (content_to_cursor(raw, s, false), content_to_cursor(raw, e, false))
    }

    /// Count the number of hard lines (\n-separated) in a styled raw string.
    pub fn styled_line_count(raw: &str) -> usize {
        // Newlines in the raw text map 1:1 to hard lines regardless of styling
        raw.chars().filter(|&c| c == '\n').count() + 1
    }

    /// Return (line_index, visual_column) for a visual cursor position in styled text.
    /// Lines are \n-separated. Visual column is the visual offset from line start.
    pub fn line_and_column_styled(raw: &str, visual_pos: usize) -> (usize, usize) {
        // Walk through the raw text, tracking visual position and line number.
        let chars: Vec<char> = raw.chars().collect();
        let len = chars.len();
        let mut visual = 0usize;
        let mut line = 0usize;
        let mut line_start_visual = 0usize;
        let mut escaped = false;
        let mut in_style_def = false;

        for i in 0..len {
            if visual >= visual_pos {
                break;
            }
            let c = chars[i];

            if escaped {
                visual += 1;
                escaped = false;
                continue;
            }

            match c {
                '\\' => { escaped = true; }
                '\n' => {
                    visual += 1; // newline is a visible character
                    line += 1;
                    line_start_visual = visual;
                }
                '{' if !in_style_def => { in_style_def = true; }
                '|' if in_style_def => {
                    in_style_def = false;
                    if i + 1 < len && chars[i + 1] == '}' {
                        visual += 1; // empty content position
                    }
                }
                '}' if !in_style_def => {
                    visual += 1;
                }
                _ if in_style_def => {}
                _ => {
                    visual += 1;
                }
            }
        }

        (line, visual_pos.saturating_sub(line_start_visual))
    }

    /// Return the visual position of the start of line `line_idx` (0-based).
    pub fn line_start_visual_styled(raw: &str, line_idx: usize) -> usize {
        if line_idx == 0 {
            return 0;
        }
        let chars: Vec<char> = raw.chars().collect();
        let len = chars.len();
        let mut visual = 0usize;
        let mut line = 0usize;
        let mut escaped = false;
        let mut in_style_def = false;

        for i in 0..len {
            let c = chars[i];
            if escaped {
                visual += 1;
                escaped = false;
                continue;
            }
            match c {
                '\\' => { escaped = true; }
                '\n' => {
                    visual += 1;
                    line += 1;
                    if line == line_idx {
                        return visual;
                    }
                }
                '{' if !in_style_def => { in_style_def = true; }
                '|' if in_style_def => {
                    in_style_def = false;
                    if i + 1 < len && chars[i + 1] == '}' {
                        visual += 1;
                    }
                }
                '}' if !in_style_def => { visual += 1; }
                _ if in_style_def => {}
                _ => { visual += 1; }
            }
        }
        visual // past last line
    }

    /// Return the visual position of the end of line `line_idx` (0-based).
    pub fn line_end_visual_styled(raw: &str, line_idx: usize) -> usize {
        let chars: Vec<char> = raw.chars().collect();
        let len = chars.len();
        let mut visual = 0usize;
        let mut line = 0usize;
        let mut escaped = false;
        let mut in_style_def = false;

        for i in 0..len {
            let c = chars[i];
            if escaped {
                visual += 1;
                escaped = false;
                continue;
            }
            match c {
                '\\' => { escaped = true; }
                '\n' => {
                    if line == line_idx {
                        return visual;
                    }
                    visual += 1;
                    line += 1;
                }
                '{' if !in_style_def => { in_style_def = true; }
                '|' if in_style_def => {
                    in_style_def = false;
                    if i + 1 < len && chars[i + 1] == '}' {
                        visual += 1;
                    }
                }
                '}' if !in_style_def => { visual += 1; }
                _ if in_style_def => {}
                _ => { visual += 1; }
            }
        }
        visual // last line ends at total visual length
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_escape_char() {
            assert_eq!(escape_char('a'), "a");
            assert_eq!(escape_char('{'), "\\{");
            assert_eq!(escape_char('}'), "\\}");
            assert_eq!(escape_char('|'), "\\|");
            assert_eq!(escape_char('\\'), "\\\\");
        }

        #[test]
        fn test_escape_str() {
            assert_eq!(escape_str("hello"), "hello");
            assert_eq!(escape_str("a{b}c"), "a\\{b\\}c");
            assert_eq!(escape_str("x|y\\z"), "x\\|y\\\\z");
        }

        #[test]
        fn test_cursor_to_raw_no_styling() {
            // Plain text: visual == raw
            assert_eq!(cursor_to_raw("hello", 0), 0);
            assert_eq!(cursor_to_raw("hello", 3), 3);
            assert_eq!(cursor_to_raw("hello", 5), 5);
        }

        #[test]
        fn test_cursor_to_raw_with_escape() {
            // "hel\{lo" → visual "hel{lo"
            let raw = r"hel\{lo";
            assert_eq!(cursor_to_raw(raw, 0), 0); // before h
            assert_eq!(cursor_to_raw(raw, 3), 3); // before \{  → raw pos 3 (the \)
            assert_eq!(cursor_to_raw(raw, 4), 5); // after { → raw pos 5 (l)
            assert_eq!(cursor_to_raw(raw, 5), 6); // after l → raw pos 6 (o)
            assert_eq!(cursor_to_raw(raw, 6), 7); // end
        }

        #[test]
        fn test_cursor_to_raw_with_style() {
            // "{red|world}" → visual positions: w(1) o(2) r(3) l(4) d(5) }(6)
            let raw = "{red|world}";
            assert_eq!(cursor_to_raw(raw, 0), 0);  // before tag = raw 0
            // Position 0 with skip_tag_headers: raw 0 is the { → skip {red| → raw 5
            // But visual_pos == 0 returns raw 0 directly!
            assert_eq!(cursor_to_raw(raw, 1), 6);  // after 'w' (raw 5), returns raw 6
            assert_eq!(cursor_to_raw(raw, 5), 10); // after 'd' (raw 9), returns raw 10
            assert_eq!(cursor_to_raw(raw, 6), 11); // after '}' (raw 10), returns raw 11
        }

        #[test]
        fn test_cursor_to_raw_mixed() {
            // "hel\{lo{red|world}" → visual: h(1) e(2) l(3) \{(4) l(5) o(6) w(7) o(8) r(9) l(10) d(11) }(12)
            let raw = r"hel\{lo{red|world}";
            assert_eq!(cursor_to_raw(raw, 0), 0);  // before everything
            assert_eq!(cursor_to_raw(raw, 3), 3);  // after 'l', before \{
            assert_eq!(cursor_to_raw(raw, 4), 5);  // after \{, before 'l'
            assert_eq!(cursor_to_raw(raw, 6), 12); // after 'o', skip {red| → raw 12 (before 'w')
            assert_eq!(cursor_to_raw(raw, 11), 17); // after 'd', before '}'
            assert_eq!(cursor_to_raw(raw, 12), 18); // after '}' = end
        }

        #[test]
        fn test_raw_to_cursor_no_styling() {
            assert_eq!(raw_to_cursor("hello", 0), 0);
            assert_eq!(raw_to_cursor("hello", 3), 3);
            assert_eq!(raw_to_cursor("hello", 5), 5);
        }

        #[test]
        fn test_raw_to_cursor_with_escape() {
            let raw = r"hel\{lo";
            assert_eq!(raw_to_cursor(raw, 0), 0);
            assert_eq!(raw_to_cursor(raw, 3), 3); // at the \ 
            assert_eq!(raw_to_cursor(raw, 5), 4); // at l after \{
            assert_eq!(raw_to_cursor(raw, 7), 6); // end
        }

        #[test]
        fn test_raw_to_cursor_with_style() {
            // "{red|world}" → visual: w(1) o(2) r(3) l(4) d(5) }(6)
            let raw = "{red|world}";
            assert_eq!(raw_to_cursor(raw, 0), 0);
            assert_eq!(raw_to_cursor(raw, 5), 0);  // just after {red| (before content starts)
            assert_eq!(raw_to_cursor(raw, 6), 1);  // after 'w'
            assert_eq!(raw_to_cursor(raw, 10), 5); // after 'd'
            assert_eq!(raw_to_cursor(raw, 11), 6); // after '}' — the exit tag position
        }

        #[test]
        fn test_cursor_len() {
            assert_eq!(cursor_len("hello"), 5);
            assert_eq!(cursor_len("{red|world}"), 6);  // 5 chars + 1 for }
            assert_eq!(cursor_len(r"hel\{lo{red|world}"), 12); // 11 chars + 1 for }
            assert_eq!(cursor_len(r"\\\{"), 2); // \\ → \, \{ → {
            assert_eq!(cursor_len("{red|}"), 2); // empty content + }
        }

        #[test]
        fn test_insert_at_visual() {
            let (new, pos) = insert_at_visual("{red|hello}", 3, "XY");
            // visual "hello", insert "XY" at pos 3 → "helXYlo"
            // raw: {red| + hel + XY + lo + }
            assert_eq!(new, "{red|helXYlo}");
            assert_eq!(pos, 5);
        }

        #[test]
        fn test_delete_visual_range() {
            let new = delete_visual_range("{red|hello}", 1, 3);
            // visual "hello", delete visual 1..3 → remove "el" → "hlo"
            assert_eq!(new, "{red|hlo}");
        }

        #[test]
        fn test_cleanup_empty_styles_removes_empty() {
            let (result, _) = cleanup_empty_styles("{red|}", 999);
            assert_eq!(result, ""); // cursor not inside, remove it
        }

        #[test]
        fn test_cleanup_empty_styles_keeps_if_cursor_inside() {
            // cursor at visual 0 is "inside" the empty tag at visual 0
            let (result, _) = cleanup_empty_styles("{red|}", 0);
            assert_eq!(result, "{red|}"); // cursor inside, keep it
        }

        #[test]
        fn test_cleanup_empty_styles_nonempty_kept() {
            let (result, _) = cleanup_empty_styles("{red|hello}", 999);
            assert_eq!(result, "{red|hello}");
        }

        #[test]
        fn test_cleanup_preserves_text_after_empty() {
            // "something{red|}more"
            // cursor not at visual position of the empty tag content
            let raw = "something{red|}more";
            // "something" = 9 visual chars, the tag content is at visual 9
            let (result, _) = cleanup_empty_styles(raw, 0); // cursor at 0 = not inside tag
            assert_eq!(result, "somethingmore");
        }

        #[test]
        fn test_cleanup_keeps_empty_when_cursor_at_content() {
            let raw = "something{red|}more";
            // tag content is at visual position 9
            let (result, _) = cleanup_empty_styles(raw, 9);
            assert_eq!(result, "something{red|}more");
        }

        #[test]
        fn test_cleanup_nonempty_nested_visual_counting() {
            // Regression test: cleanup_empty_styles must not inflate visual counter
            // when processing non-empty style tag headers like `{color=red|..}`
            let raw = "{color=red|hello}world";
            // Visual: h(1)e(2)l(3)l(4)o(5) }(6) w(7)o(8)r(9)l(10)d(11)
            // Cursor at 11 (end) — cleanup should return same text, cursor at 11
            let (result, new_cursor) = cleanup_empty_styles(raw, 11);
            assert_eq!(result, raw);
            assert_eq!(new_cursor, 11);

            // With empty tag after non-empty: "{color=red|hello}{blue|}"
            let raw2 = "{color=red|hello}{blue|}";
            // Visual: h(1)e(2)l(3)l(4)o(5) }(6) [empty](7) }(8)
            // Cursor at 8 (after both), empty tag should be removed
            let (result2, new_cursor2) = cleanup_empty_styles(raw2, 8);
            assert_eq!(result2, "{color=red|hello}");
            // Cursor was at 8, empty tag was at visual 6-7, cursor_adj = 8-2 = 6
            assert_eq!(new_cursor2, 6);
        }

        #[test]
        fn test_cleanup_deeply_nested_nonempty() {
            // Deeply nested non-empty tags shouldn't inflate visual counter
            let raw = "aaa{r|{g|{b|xyz}}}end";
            // Visual: a(1)a(2)a(3) x(4)y(5)z(6) }(7) }(8) }(9) e(10)n(11)d(12)
            let vl = cursor_len(raw);
            assert_eq!(vl, 12);
            let (result, new_cursor) = cleanup_empty_styles(raw, vl);
            assert_eq!(result, raw);
            assert_eq!(new_cursor, vl);
        }

        #[test]
        fn test_word_boundary_visual_nested_tags() {
            // Regression test for crash: ctrl+left on text with deeply nested tags
            // The cleanup_empty_styles visual inflation bug caused cursor_pos
            // to exceed content length, crashing find_word_boundary_left.
            let raw = "aaa{r|{r|{r|bbb}}} ccc";
            // Visual: a(1)a(2)a(3) b(4)b(5)b(6) }(7) }(8) }(9) (10)c(11)c(12)c(13)
            let vl = cursor_len(raw);
            assert_eq!(vl, 13);

            // Word boundary at end should work
            let result = find_word_boundary_left_visual(raw, vl);
            assert!(result <= vl, "word boundary should not exceed visual len");

            // Word boundary from every visual position should not panic
            for v in 0..=vl {
                let _ = find_word_boundary_left_visual(raw, v);
                let _ = find_word_boundary_right_visual(raw, v);
            }
        }

        #[test]
        fn test_word_boundary_visual_after_cleanup() {
            // Simulate the crash scenario: text with nested non-empty tags,
            // cleanup_empty_styles was inflating visual counter, then word
            // boundary was called with the resulting bad cursor position.
            let raw = "aaa{color=red|{color=red|bbb}}} ccc";
            let vl = cursor_len(raw);
            // First, do a cleanup (simulating move_word_left_styled)
            let (cleaned, cursor) = cleanup_empty_styles(raw, vl);
            let cleaned_vl = cursor_len(&cleaned);
            assert!(cursor <= cleaned_vl,
                "cursor {} should be <= cursor_len {} after cleanup",
                cursor, cleaned_vl);

            // Now call word boundary on the cleaned text
            let _ = find_word_boundary_left_visual(&cleaned, cursor);
        }

        #[test]
        fn test_roundtrip_visual_raw() {
            let raw = r"hel\{lo{red|world}";
            // cursor_len = 12 (11 visible chars + 1 for })
            for v in 0..=12 {
                let r = cursor_to_raw(raw, v);
                let v2 = raw_to_cursor(raw, r);
                assert_eq!(v, v2, "visual {} → raw {} → visual {} (expected {})", v, r, v2, v);
            }
        }

        #[test]
        fn test_cursor_to_raw_for_insertion_enters_empty_tag() {
            // "test{red|}" — visual: t(1) e(2) s(3) t(4) [empty content](5) }(6)
            let raw = "test{red|}";
            // Position 4 skips tag header: raw 4 → skip {red| → raw 9 (inside empty tag)
            assert_eq!(cursor_to_raw(raw, 4), 9);
            // Position 5 = empty content marker → raw 9
            assert_eq!(cursor_to_raw(raw, 5), 9);
            // Position 6 = after } → raw 10
            assert_eq!(cursor_to_raw(raw, 6), 10);
            // Cursor variant at pos 4: cursor_to_raw returns 9, enter_empty_tags finds nothing
            assert_eq!(cursor_to_raw_for_insertion(raw, 4), 9);
        }

        #[test]
        fn test_cursor_to_raw_for_insertion_nonempty_tag_not_entered() {
            // "test{red|x}" — visual: t(1) e(2) s(3) t(4) x(5) }(6)
            let raw = "test{red|x}";
            // Position 4 skips tag header: raw 4 → skip {red| → raw 9 (inside tag, before 'x')
            assert_eq!(cursor_to_raw(raw, 4), 9);
            assert_eq!(cursor_to_raw_for_insertion(raw, 4), 9);
        }

        #[test]
        fn test_cursor_to_raw_for_insertion_at_start() {
            // "{red|}hello" — visual: [empty content](1) }(2) h(3) e(4) l(5) l(6) o(7)
            let raw = "{red|}hello";
            // Position 0 = raw 0 (before everything)
            assert_eq!(cursor_to_raw(raw, 0), 0);
            // Cursor variant enters the empty tag at raw 0 → {red|} → raw 5
            assert_eq!(cursor_to_raw_for_insertion(raw, 0), 5);
            // Position 1 = empty content → raw 5
            assert_eq!(cursor_to_raw(raw, 1), 5);
            // Position 2 = after } → raw 6
            assert_eq!(cursor_to_raw(raw, 2), 6);
        }

        #[test]
        fn test_insert_at_visual_enters_empty_tag() {
            // Insertion at cursor position should go inside empty tag
            let raw = "test{red|}";
            let (new, pos) = insert_at_visual(raw, 4, "X");
            // X should go inside {red|}, not before it
            assert_eq!(new, "test{red|X}");
            assert_eq!(pos, 5);
        }

        #[test]
        fn test_insert_at_visual_empty_tag_middle() {
            // "hello{red|}world" — insert at visual 5 (between "hello" and "world")
            let raw = "hello{red|}world";
            let (new, pos) = insert_at_visual(raw, 5, "X");
            assert_eq!(new, "hello{red|X}world");
            assert_eq!(pos, 6);
        }

        #[test]
        fn test_user_scenario_backspace_to_empty_tag() {
            // User's example: "hel\{lo{red|world}" — visual "hel{loworld"
            // cursor at visual 11 (end), backspace 5 times to visual 6
            // Expected result: "hel\{lo{red|}" with cursor at 6 (inside empty tag)

            // Simulate: start with the full text, delete chars at visual 6..11
            let raw = r"hel\{lo{red|world}";
            // Delete visual range [6, 11) — removes "world"
            let after_delete = delete_visual_range(raw, 6, 11);
            assert_eq!(after_delete, r"hel\{lo{red|}");

            // Cursor is now at visual 6. The {red|} tag is empty.
            // cleanup_empty_styles at visual 6: tag content is at visual 6 → keep it
            let (cleaned, _) = cleanup_empty_styles(&after_delete, 6);
            assert_eq!(cleaned, r"hel\{lo{red|}");

            // Typing inside the empty tag: insert "X" at visual 6
            let (after_insert, new_pos) = insert_at_visual(&cleaned, 6, "X");
            // X should go inside {red|}
            assert_eq!(after_insert, r"hel\{lo{red|X}");
            assert_eq!(new_pos, 7);

            // Moving cursor away (to visual 5): cleanup removes empty tag
            // First make it empty again
            let empty_again = delete_visual_range(&after_insert, 6, 7);
            assert_eq!(empty_again, r"hel\{lo{red|}");
            let (after_move, _) = cleanup_empty_styles(&empty_again, 5);
            assert_eq!(after_move, r"hel\{lo");
        }
    }
}

/// Compute x-positions for each character boundary in the display text.
/// Returns a Vec with len = char_count + 1.
/// Uses the provided measure function to measure substrings.
pub fn compute_char_x_positions(
    display_text: &str,
    font_asset: Option<&'static crate::renderer::FontAsset>,
    font_size: u16,
    measure_fn: &dyn Fn(&str, &crate::text::TextConfig) -> crate::math::Dimensions,
) -> Vec<f32> {
    let char_count = display_text.chars().count();
    let mut positions = Vec::with_capacity(char_count + 1);
    positions.push(0.0);

    let config = crate::text::TextConfig {
        font_asset,
        font_size,
        ..Default::default()
    };

    #[cfg(feature = "text-styling")]
    {
        // When text-styling is enabled, the display text contains markup like {red|...}
        // and escape sequences like \{. We must avoid measuring prefixes that end inside
        // a tag header ({name|) because measure_fn warns on incomplete style definitions.
        // For non-visible chars (tag headers, braces, backslashes), reuse the last width.
        let chars: Vec<char> = display_text.chars().collect();
        let mut in_tag_header = false;
        let mut escaped = false;
        let mut last_width = 0.0f32;

        for i in 0..char_count {
            let ch = chars[i];
            if escaped {
                escaped = false;
                // Escaped char is visible: measure the prefix up to this char
                let byte_end = char_index_to_byte(display_text, i + 1);
                let substr = &display_text[..byte_end];
                let dims = measure_fn(substr, &config);
                last_width = dims.width;
                positions.push(last_width);
                continue;
            }
            match ch {
                '\\' => {
                    escaped = true;
                    // Backslash itself is not visible
                    positions.push(last_width);
                }
                '{' => {
                    in_tag_header = true;
                    positions.push(last_width);
                }
                '|' if in_tag_header => {
                    in_tag_header = false;
                    positions.push(last_width);
                }
                '}' => {
                    // Closing brace is not visible
                    positions.push(last_width);
                }
                _ if in_tag_header => {
                    // Inside tag name — not visible
                    positions.push(last_width);
                }
                _ => {
                    // Visible character: measure the prefix
                    let byte_end = char_index_to_byte(display_text, i + 1);
                    let substr = &display_text[..byte_end];
                    let dims = measure_fn(substr, &config);
                    last_width = dims.width;
                    positions.push(last_width);
                }
            }
        }
    }

    #[cfg(not(feature = "text-styling"))]
    {
        for i in 1..=char_count {
            let byte_end = char_index_to_byte(display_text, i);
            let substr = &display_text[..byte_end];
            let dims = measure_fn(substr, &config);
            positions.push(dims.width);
        }
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
        assert_eq!(find_word_boundary_right("hello world", 0), 5);  // end of "hello"
        assert_eq!(find_word_boundary_right("hello world", 5), 11); // skip space, end of "world"
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
        let lines = wrap_lines("hello", 100.0, None, 16, &fixed_measure);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].text, "hello");
        assert_eq!(lines[0].global_char_start, 0);
        assert_eq!(lines[0].char_count, 5);
    }

    #[test]
    fn test_wrap_lines_hard_break() {
        let lines = wrap_lines("ab\ncd", 100.0, None, 16, &fixed_measure);
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
        let lines = wrap_lines("hello world", 60.0, None, 16, &fixed_measure);
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
        let lines = wrap_lines("abcdefghij", 50.0, None, 16, &fixed_measure);
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

    #[test]
    fn test_undo_basic() {
        let mut state = TextEditState::default();
        state.text = "hello".to_string();
        state.cursor_pos = 5;

        // Push undo, then modify
        state.push_undo(UndoActionKind::Paste);
        state.insert_text(" world", None);
        assert_eq!(state.text, "hello world");

        // Undo should restore
        assert!(state.undo());
        assert_eq!(state.text, "hello");
        assert_eq!(state.cursor_pos, 5);

        // Redo should restore the edit
        assert!(state.redo());
        assert_eq!(state.text, "hello world");
        assert_eq!(state.cursor_pos, 11);
    }

    #[test]
    fn test_undo_grouping_insert_char() {
        let mut state = TextEditState::default();

        // Simulate typing "abc" one char at a time
        state.push_undo(UndoActionKind::InsertChar);
        state.insert_text("a", None);
        state.push_undo(UndoActionKind::InsertChar);
        state.insert_text("b", None);
        state.push_undo(UndoActionKind::InsertChar);
        state.insert_text("c", None);
        assert_eq!(state.text, "abc");

        // Should undo all at once (grouped)
        assert!(state.undo());
        assert_eq!(state.text, "");
        assert_eq!(state.cursor_pos, 0);

        // No more undos
        assert!(!state.undo());
    }

    #[test]
    fn test_undo_grouping_backspace() {
        let mut state = TextEditState::default();
        state.text = "hello".to_string();
        state.cursor_pos = 5;

        // Backspace 3 times
        state.push_undo(UndoActionKind::Backspace);
        state.backspace();
        state.push_undo(UndoActionKind::Backspace);
        state.backspace();
        state.push_undo(UndoActionKind::Backspace);
        state.backspace();
        assert_eq!(state.text, "he");

        // Should undo all backspaces at once
        assert!(state.undo());
        assert_eq!(state.text, "hello");
    }

    #[test]
    fn test_undo_different_kinds_not_grouped() {
        let mut state = TextEditState::default();

        // Type then delete — different kinds, not grouped
        state.push_undo(UndoActionKind::InsertChar);
        state.insert_text("abc", None);
        state.push_undo(UndoActionKind::Backspace);
        state.backspace();
        assert_eq!(state.text, "ab");

        // First undo restores before backspace
        assert!(state.undo());
        assert_eq!(state.text, "abc");

        // Second undo restores before insert
        assert!(state.undo());
        assert_eq!(state.text, "");
    }

    #[test]
    fn test_redo_cleared_on_new_edit() {
        let mut state = TextEditState::default();

        state.push_undo(UndoActionKind::Paste);
        state.insert_text("hello", None);
        state.undo();
        assert_eq!(state.text, "");
        assert!(!state.redo_stack.is_empty());

        // New edit should clear redo
        state.push_undo(UndoActionKind::Paste);
        state.insert_text("world", None);
        assert!(state.redo_stack.is_empty());
    }

    #[test]
    fn test_undo_empty_stack() {
        let mut state = TextEditState::default();
        assert!(!state.undo());
        assert!(!state.redo());
    }

    #[cfg(feature = "text-styling")]
    fn make_no_styles_state(raw: &str) -> TextEditState {
        let mut s = TextEditState::default();
        s.text = raw.to_string();
        s.no_styles_movement = true;
        // Snap the cursor to a content boundary at position 0
        s.cursor_pos = 0;
        s
    }

    #[test]
    #[cfg(feature = "text-styling")]
    fn test_content_to_cursor_no_structural_basic() {
        use crate::text_input::styling_cursor::content_to_cursor;
        // "a{red|}b" — visual: a@0, empty@1, }@2, b@3. content: a,b
        assert_eq!(content_to_cursor("a{red|}b", 0, true), 0);  // before 'a'
        assert_eq!(content_to_cursor("a{red|}b", 1, true), 3);  // before 'b' (skip empty + })
        assert_eq!(content_to_cursor("a{red|}b", 2, true), 4);  // after 'b'
    }

    #[test]
    #[cfg(feature = "text-styling")]
    fn test_content_to_cursor_no_structural_nested() {
        use crate::text_input::styling_cursor::content_to_cursor;
        // "a{red|b}{blue|c}" — visual: a@0, b@1, }@2, c@3, }@4. content: a,b,c
        assert_eq!(content_to_cursor("a{red|b}{blue|c}", 0, true), 0);
        assert_eq!(content_to_cursor("a{red|b}{blue|c}", 1, true), 1);  // before 'b'
        assert_eq!(content_to_cursor("a{red|b}{blue|c}", 2, true), 3);  // before 'c' (skip } + header)
        assert_eq!(content_to_cursor("a{red|b}{blue|c}", 3, true), 5);  // after last } (end of text)
    }

    #[test]
    #[cfg(feature = "text-styling")]
    fn test_delete_content_range() {
        use crate::text_input::styling_cursor::delete_content_range;
        // Delete 'b' from "a{red|b}c" → "a{red|}c"
        assert_eq!(delete_content_range("a{red|b}c", 1, 2), "a{red|}c");
        // Delete 'a' from "a{red|b}c" → "{red|b}c"
        assert_eq!(delete_content_range("a{red|b}c", 0, 1), "{red|b}c");
        // Delete all content
        assert_eq!(delete_content_range("a{red|b}c", 0, 3), "{red|}");
        // No-op
        assert_eq!(delete_content_range("abc", 1, 1), "abc");
    }

    #[test]
    #[cfg(feature = "text-styling")]
    fn test_no_styles_move_right() {
        let mut s = make_no_styles_state("a{red|}b");
        // cursor at 0 (before 'a'), move right.
        // Because cursor moves away from {red|}, the empty tag gets cleaned up.
        // Text becomes "ab" and cursor lands at content 1 (between a and b).
        s.move_right_styled(false);
        assert_eq!(s.text, "ab");
        assert_eq!(s.cursor_pos, 1);
        // move right again → after 'b' (end)
        s.move_right_styled(false);
        assert_eq!(s.cursor_pos, 2);
        assert_eq!(styling_cursor::cursor_to_content(&s.text, s.cursor_pos), 2);
    }

    #[test]
    #[cfg(feature = "text-styling")]
    fn test_no_styles_move_left() {
        let mut s = make_no_styles_state("a{red|}b");
        // Put cursor at end — the empty {red|} tag will be cleaned up since
        // cursor at end is not inside it. Text becomes "ab", cursor at 2.
        s.cursor_pos = styling_cursor::content_to_cursor(&s.text, 2, true);
        // Trigger cleanup to normalise
        s.move_end_styled(false);
        assert_eq!(s.text, "ab");
        assert_eq!(s.cursor_pos, 2);
        // move left → before 'b' (content 1)
        s.move_left_styled(false);
        let cp = styling_cursor::cursor_to_content(&s.text, s.cursor_pos);
        assert_eq!(cp, 1);
        // move left → before 'a' (content 0)
        s.move_left_styled(false);
        assert_eq!(s.cursor_pos, 0);
    }

    #[test]
    #[cfg(feature = "text-styling")]
    fn test_no_styles_move_left_skips_closing_brace() {
        let mut s = make_no_styles_state("a{red|b}c");
        // visual: a@0, b@1, }@2, c@3. Set cursor before 'c' (content 2 → visual 3).
        s.cursor_pos = styling_cursor::content_to_cursor(&s.text, 2, true);
        // 'b' is at visual 1, '}' at 2, 'c' at 3.  cursor should be at visual 3.
        assert_eq!(s.cursor_pos, 3);
        // Move left → before 'b' (content 1, visual 1)
        s.move_left_styled(false);
        let cp = styling_cursor::cursor_to_content(&s.text, s.cursor_pos);
        assert_eq!(cp, 1);
    }

    #[test]
    #[cfg(feature = "text-styling")]
    fn test_no_styles_backspace() {
        let mut s = make_no_styles_state("a{red|b}c");
        // Put cursor at content 2 (before 'c')
        s.cursor_pos = styling_cursor::content_to_cursor(&s.text, 2, true);
        s.backspace_styled();
        // Should delete 'b', leaving "a...c"
        let stripped = styling_cursor::strip_styling(&s.text);
        assert_eq!(stripped, "ac");
        // Cursor should be at content 1 (between a and c)
        let cp = styling_cursor::cursor_to_content(&s.text, s.cursor_pos);
        assert_eq!(cp, 1);
    }

    #[test]
    #[cfg(feature = "text-styling")]
    fn test_no_styles_delete_forward() {
        let mut s = make_no_styles_state("{red|abc}");
        // Put cursor at content 1 (before 'b')
        s.cursor_pos = styling_cursor::content_to_cursor(&s.text, 1, true);
        s.delete_forward_styled();
        let stripped = styling_cursor::strip_styling(&s.text);
        assert_eq!(stripped, "ac");
        let cp = styling_cursor::cursor_to_content(&s.text, s.cursor_pos);
        assert_eq!(cp, 1);
    }

    #[test]
    #[cfg(feature = "text-styling")]
    fn test_no_styles_home_end() {
        let mut s = make_no_styles_state("{red|}hello{blue|}");
        // Home — should be at the first content char
        s.move_home_styled(false);
        let cp = styling_cursor::cursor_to_content(&s.text, s.cursor_pos);
        assert_eq!(cp, 0);
        // End
        s.move_end_styled(false);
        let cp = styling_cursor::cursor_to_content(&s.text, s.cursor_pos);
        let content_len = styling_cursor::strip_styling(&s.text).chars().count();
        assert_eq!(cp, content_len);
    }

    #[test]
    #[cfg(feature = "text-styling")]
    fn test_no_styles_select_all_and_delete() {
        let mut s = make_no_styles_state("a{red|b}c");
        s.select_all_styled();
        assert!(s.selection_anchor.is_some());
        // Delete selection
        s.delete_selection_styled();
        let stripped = styling_cursor::strip_styling(&s.text);
        assert!(stripped.is_empty());
    }
}
