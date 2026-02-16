use crate::engine;

/// Owned string for debug/display purposes.
#[derive(Debug, Clone, Default)]
pub struct StringId {
    text: String,
}

impl StringId {
    pub fn from_str(s: &str) -> Self {
        Self {
            text: s.to_string(),
        }
    }

    pub fn empty() -> Self {
        Self::default()
    }

    /// Get the string content.
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// Returns true if the string is empty.
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }
}

#[derive(Debug, Clone, Default)]
pub struct Id {
    pub id: u32,
    pub offset: u32,
    pub base_id: u32,
    pub string_id: StringId,
}

impl Id {
    /// Creates a ply id using the `label`
    #[inline]
    pub(crate) fn new(label: &'static str) -> Id {
        Self::new_index(label, 0)
    }

    /// Creates a ply id using the `label` and the `index`
    #[inline]
    pub(crate) fn new_index(label: &'static str, index: u32) -> Id {
        engine::hash_string_with_offset(label, index, 0)
    }

    #[inline]
    pub(crate) fn new_index_local_with_parent(label: &'static str, index: u32, parent_id: u32) -> Id {
        engine::hash_string_with_offset(label, index, parent_id)
    }
}

impl From<&'static str> for Id {
    fn from(label: &'static str) -> Self {
        Id::new(label)
    }
}