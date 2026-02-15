use crate::engine;

#[derive(Debug, Copy, Clone)]
pub struct Id {
    pub(crate) id: engine::ElementId,
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
        let id = engine::hash_string_with_offset(label, index, 0);
        Id { id }
    }

    #[inline]
    pub(crate) fn new_index_local_with_parent(label: &'static str, index: u32, parent_id: u32) -> Id {
        let id = engine::hash_string_with_offset(label, index, parent_id);
        Id { id }
    }
}