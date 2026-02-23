use crate::engine;

/// Owned string for debug/display purposes.
#[derive(Debug, Clone, Default, PartialEq)]
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

/// A hashed identifier used to uniquely address UI elements across frames.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Id {
    pub id: u32,
    pub offset: u32,
    pub base_id: u32,
    pub string_id: StringId,
}

impl Id {
    /// Creates a ply id using the `label`
    #[inline]
    pub fn new(label: &'static str) -> Id {
        Self::new_index(label, 0)
    }

    /// Creates a ply id using the `label` and the `index`
    #[inline]
    pub fn new_index(label: &'static str, index: u32) -> Id {
        engine::hash_string_with_offset(label, index, 0)
    }

    #[inline]
    pub fn new_index_seed(label: &'static str, index: u32, seed: u32) -> Id {
        engine::hash_string_with_offset(label, index, seed)
    }
}

impl From<&'static str> for Id {
    fn from(label: &'static str) -> Self {
        Id::new(label)
    }
}

impl From<(&str, u32)> for Id {
    fn from((label, offset): (&str, u32)) -> Self {
        engine::hash_string_with_offset(label, offset, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_str_matches_new() {
        let a = Id::from("hello");
        let b = Id::new("hello");
        assert_eq!(a.id, b.id);
        assert_eq!(a.base_id, b.base_id);
    }

    #[test]
    fn from_tuple_matches_new_index() {
        let a = Id::from(("my_button", 3));
        let b = Id::new_index("my_button", 3);
        assert_eq!(a.id, b.id);
        assert_eq!(a.offset, b.offset);
        assert_eq!(a.base_id, b.base_id);
    }

    #[test]
    fn from_tuple_zero_offset_matches_from_str() {
        let a = Id::from(("test", 0));
        let b = Id::from("test");
        assert_eq!(a.id, b.id);
    }

    #[test]
    fn different_offsets_produce_different_ids() {
        let a = Id::from(("item", 0));
        let b = Id::from(("item", 1));
        assert_ne!(a.id, b.id);
        // But base_id should be the same (pre-offset hash)
        assert_eq!(a.base_id, b.base_id);
    }
}