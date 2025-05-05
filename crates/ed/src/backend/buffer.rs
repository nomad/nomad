use core::ops::Range;
use std::borrow::Cow;

use smallvec::SmallVec;
use smol_str::SmolStr;

use super::Backend;
use crate::ByteOffset;
use crate::backend::AgentId;

/// TODO: docs.
pub trait Buffer {
    /// TODO: docs.
    type EventHandle;

    /// TODO: docs.
    type Id: Clone;

    /// TODO: docs.
    type Backend: Backend<BufferId = Self::Id>;

    /// TODO: docs.
    fn byte_len(&self) -> ByteOffset;

    /// TODO: docs.
    fn id(&self) -> Self::Id;

    /// TODO: docs.
    fn edit<R>(&mut self, replacements: R, agent_id: AgentId)
    where
        R: IntoIterator<Item = Replacement>;

    /// TODO: docs.
    fn name(&self) -> Cow<'_, str>;

    /// TODO: docs.
    fn on_edited<Fun>(&mut self, fun: Fun) -> Self::EventHandle
    where
        Fun: FnMut(&<Self::Backend as Backend>::Buffer<'_>, &Edit) + 'static;

    /// TODO: docs.
    fn on_removed<Fun>(&mut self, fun: Fun) -> Self::EventHandle
    where
        Fun: FnMut(&<Self::Backend as Backend>::Buffer<'_>, AgentId) + 'static;

    /// TODO: docs.
    fn on_saved<Fun>(&mut self, fun: Fun) -> Self::EventHandle
    where
        Fun: FnMut(&<Self::Backend as Backend>::Buffer<'_>, AgentId) + 'static;
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
