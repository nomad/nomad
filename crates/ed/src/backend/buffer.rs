use core::ops::Range;
use std::borrow::Cow;

use abs_path::AbsPath;
use smallvec::SmallVec;
use smol_str::SmolStr;

use crate::ByteOffset;
use crate::backend::{AgentId, Backend};

/// TODO: docs.
pub trait Buffer {
    /// TODO: docs.
    type Backend: Backend;

    /// TODO: docs.
    fn byte_len(&self) -> ByteOffset;

    /// TODO: docs.
    fn edit<R>(&mut self, replacements: R, agent_id: AgentId)
    where
        R: IntoIterator<Item = Replacement>;

    /// TODO: docs.
    fn id(&self) -> <Self::Backend as Backend>::BufferId;

    /// TODO: docs.
    fn focus(&mut self, agent_id: AgentId);

    /// TODO: docs.
    fn for_each_cursor<Fun>(&mut self, fun: Fun)
    where
        Fun: FnMut(<Self::Backend as Backend>::Cursor<'_>);

    /// TODO: docs.
    fn num_cursors(&mut self) -> u32 {
        let mut num_cursors = 0;
        self.for_each_cursor(|_| num_cursors += 1);
        num_cursors
    }

    /// TODO: docs.
    fn on_edited<Fun>(
        &self,
        fun: Fun,
    ) -> <Self::Backend as Backend>::EventHandle
    where
        Fun: FnMut(&<Self::Backend as Backend>::Buffer<'_>, &Edit) + 'static;

    /// TODO: docs.
    fn on_removed<Fun>(
        &self,
        fun: Fun,
    ) -> <Self::Backend as Backend>::EventHandle
    where
        Fun: FnMut(&<Self::Backend as Backend>::Buffer<'_>, AgentId) + 'static;

    /// TODO: docs.
    fn on_saved<Fun>(
        &self,
        fun: Fun,
    ) -> <Self::Backend as Backend>::EventHandle
    where
        Fun: FnMut(&<Self::Backend as Backend>::Buffer<'_>, AgentId) + 'static;

    /// TODO: docs.
    fn path(&self) -> Cow<'_, AbsPath>;
}

/// TODO: docs.
#[derive(Debug, Clone)]
pub struct Edit {
    /// TODO: docs.
    pub made_by: AgentId,

    /// TODO: docs.
    pub replacements: SmallVec<[Replacement; 1]>,
}

/// TODO: docs.
#[derive(Debug, Clone)]
pub struct Replacement {
    removed_range: Range<ByteOffset>,
    inserted_text: SmolStr,
}

impl Replacement {
    /// TODO: docs.
    #[inline]
    pub fn inserted_text(&self) -> &str {
        &self.inserted_text
    }

    /// TODO: docs.
    #[inline]
    pub fn new(
        removed_range: Range<ByteOffset>,
        inserted_text: impl Into<SmolStr>,
    ) -> Self {
        Self { removed_range, inserted_text: inserted_text.into() }
    }

    /// TODO: docs.
    #[inline]
    pub fn removed_range(&self) -> Range<ByteOffset> {
        self.removed_range.clone()
    }
}
