use cola::{Deletion, Insertion};

/// TODO: docs
pub struct ColaReplacement {
    deletion: Option<Deletion>,
    insertion: Option<Insertion>,
}

impl ColaReplacement {
    /// TODO: docs
    #[inline]
    pub fn deletion(&self) -> Option<&Deletion> {
        self.deletion.as_ref()
    }

    /// TODO: docs
    #[inline]
    pub fn insertion(&self) -> Option<&Insertion> {
        self.insertion.as_ref()
    }

    /// TODO: docs
    #[inline]
    pub(crate) fn new(deletion: Deletion, insertion: Insertion) -> Self {
        Self { deletion: Some(deletion), insertion: Some(insertion) }
    }

    /// Creates a new deletion-only [`ColaReplacement`].
    ///
    /// This is just a convenience method over [`ColaReplacement::new`].
    #[inline]
    pub(crate) fn new_deletion(deletion: Deletion) -> Self {
        Self { deletion: Some(deletion), insertion: None }
    }

    /// Creates a new insertion-only [`ColaReplacement`].
    ///
    /// This is just a convenience method over [`ColaReplacement::new`].
    #[inline]
    pub(crate) fn new_insertion(insertion: Insertion) -> Self {
        Self { deletion: None, insertion: Some(insertion) }
    }
}
