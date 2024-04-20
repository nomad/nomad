use core::sync::atomic::{AtomicU32, Ordering};

/// TODO: docs
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct EditorId {
    /// A counter that starts at 1 and is increased every time a new `EditorId`
    /// is generated. The unknown `EditorId` has a counter of 0.
    id: u32,
}

impl EditorId {
    /// TODO: docs
    #[inline]
    pub fn generate() -> Self {
        Self { id: next_editor_id() }
    }

    /// TODO: docs
    #[inline]
    pub(crate) fn unknown() -> Self {
        Self { id: 0 }
    }
}

#[inline]
fn next_editor_id() -> u32 {
    static EDITOR_ID: AtomicU32 = AtomicU32::new(1);
    EDITOR_ID.fetch_add(1, Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests that the inner counter is increased every time a new `EditorId`
    /// is generated, and that all generated `EditorId`s differ from the
    /// unknown one.
    #[test]
    fn editor_id_next() {
        for i in 0..10 {
            let id = EditorId::generate();
            assert_eq!(id, EditorId { id: i + 1 });
            assert_ne!(id, EditorId::unknown());
        }
    }

    /// Tests that all unknown `EditorId`s are equal.
    #[test]
    fn unknown() {
        let id = EditorId::unknown();
        assert_eq!(id, EditorId::unknown());
    }
}
