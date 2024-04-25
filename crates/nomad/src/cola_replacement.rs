use cola::{Deletion, Insertion};

/// TODO: docs
#[derive(Clone, Debug)]
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

    /// Creates a new deletion-only [`ColaReplacement`].
    #[inline]
    pub(crate) fn new_deletion(deletion: Deletion) -> Self {
        Self { deletion: Some(deletion), insertion: None }
    }

    /// Creates a new insertion-only [`ColaReplacement`].
    #[inline]
    pub(crate) fn new_insertion(insertion: Insertion) -> Self {
        Self { deletion: None, insertion: Some(insertion) }
    }

    /// Creates a new [`ColaReplacement`] representing a no-op.
    #[inline]
    pub(crate) fn new_no_op() -> Self {
        Self { deletion: None, insertion: None }
    }

    /// Sets the deletion part of this replacement.
    ///
    /// Note that this will overwrite any previously-set deletion.
    #[inline]
    pub(crate) fn with_deletion(&mut self, deletion: Deletion) {
        self.deletion = Some(deletion);
    }

    /// Sets the insertion part of this replacement.
    ///
    /// Note that this will overwrite any previously-set insertion.
    #[inline]
    pub(crate) fn with_insertion(&mut self, insertion: Insertion) {
        self.insertion = Some(insertion);
    }
}
