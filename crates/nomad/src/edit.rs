use core::ops::Range;

use cola::{Deletion, Insertion};
use smallvec::SmallVec;
use smol_str::SmolStr;

use crate::{ByteOffset, CrdtReplacement, EditorId, Replacement};

/// TODO: docs
#[derive(Debug, Clone)]
pub struct Edit {
    applied_by: EditorId,
    crdt_replacement: CrdtReplacement,
    replacements: SmallVec<[Replacement<ByteOffset>; 1]>,
}

impl Edit {
    /// TODO: docs
    #[inline]
    pub fn applied_by(&self) -> EditorId {
        self.applied_by
    }

    /// TODO: docs
    #[inline]
    pub fn crdt_replacement(&self) -> &CrdtReplacement {
        &self.crdt_replacement
    }

    /// TODO: docs
    #[inline]
    pub(crate) fn local(
        text: Replacement<ByteOffset>,
        crdt: CrdtReplacement,
    ) -> Self {
        Self {
            applied_by: EditorId::unknown(),
            crdt_replacement: crdt,
            replacements: SmallVec::from_elem(text, 1),
        }
    }

    /// TODO: docs
    #[inline]
    pub(crate) fn no_op() -> Self {
        Self {
            applied_by: EditorId::unknown(),
            crdt_replacement: CrdtReplacement::new_no_op(),
            replacements: SmallVec::new(),
        }
    }

    /// TODO: docs
    #[inline]
    pub(crate) fn remote_insertion(
        offset: ByteOffset,
        text: impl Into<SmolStr>,
        crdt: Insertion,
    ) -> Self {
        let replacement = Replacement::insertion(offset, text);

        Self {
            applied_by: EditorId::unknown(),
            crdt_replacement: CrdtReplacement::new_insertion(crdt),
            replacements: SmallVec::from_elem(replacement, 1),
        }
    }

    /// TODO: docs
    #[inline]
    pub(crate) fn remote_deletion(
        ranges: impl IntoIterator<Item = Range<ByteOffset>>,
        crdt: Deletion,
    ) -> Self {
        let replacements =
            ranges.into_iter().map(Replacement::deletion).collect();

        Self {
            applied_by: EditorId::unknown(),
            crdt_replacement: CrdtReplacement::new_deletion(crdt),
            replacements,
        }
    }

    /// TODO: docs
    #[inline]
    pub fn replacements(
        &self,
    ) -> impl ExactSizeIterator<Item = &Replacement<ByteOffset>> + '_ {
        self.replacements.iter()
    }

    #[inline]
    pub(crate) fn with_editor(mut self, id: EditorId) -> Self {
        self.applied_by = id;
        self
    }
}
