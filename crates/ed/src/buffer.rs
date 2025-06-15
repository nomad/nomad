use core::fmt;
use core::ops::Range;
use std::borrow::Cow;

use abs_path::AbsPath;
use smallvec::SmallVec;
use smol_str::SmolStr;

use crate::{AgentId, ByteOffset, Editor};

/// TODO: docs.
pub trait Buffer {
    /// TODO: docs.
    type Editor: Editor;

    /// TODO: docs.
    fn byte_len(&self) -> ByteOffset;

    /// TODO: docs.
    fn edit<R>(&mut self, replacements: R, agent_id: AgentId)
    where
        R: IntoIterator<Item = Replacement>;

    /// TODO: docs.
    fn get_text(&self, byte_range: Range<ByteOffset>) -> impl Chunks;

    /// TODO: docs.
    fn id(&self) -> <Self::Editor as Editor>::BufferId;

    /// Whether the buffer is empty, i.e. whether `byte_len()` is 0.
    #[inline]
    fn is_empty(&self) -> bool {
        self.byte_len() == 0
    }

    /// TODO: docs.
    fn focus(&mut self, agent_id: AgentId);

    /// TODO: docs.
    fn for_each_cursor<Fun>(&mut self, fun: Fun)
    where
        Fun: FnMut(<Self::Editor as Editor>::Cursor<'_>);

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
    ) -> <Self::Editor as Editor>::EventHandle
    where
        Fun: FnMut(&<Self::Editor as Editor>::Buffer<'_>, &Edit) + 'static;

    /// TODO: docs.
    fn on_removed<Fun>(
        &self,
        fun: Fun,
    ) -> <Self::Editor as Editor>::EventHandle
    where
        Fun: FnMut(<Self::Editor as Editor>::BufferId, AgentId) + 'static;

    /// TODO: docs.
    fn on_saved<Fun>(&self, fun: Fun) -> <Self::Editor as Editor>::EventHandle
    where
        Fun: FnMut(&<Self::Editor as Editor>::Buffer<'_>, AgentId) + 'static;

    /// TODO: docs.
    fn path(&self) -> Cow<'_, AbsPath>;
}

/// TODO: docs.
pub trait Chunks:
    fmt::Display + fmt::Debug + for<'a> PartialEq<&'a str>
{
    /// TODO: docs.
    fn iter(&self) -> impl Iterator<Item = impl AsRef<str>>;
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Replacement {
    removed_range: Range<ByteOffset>,
    inserted_text: SmolStr,
}

impl Edit {
    /// Returns the net change in bytes from all [`Replacement`]s in this edit.
    ///
    /// Positive values indicate bytes were added, negative values indicate
    /// bytes were removed.
    #[inline]
    pub fn byte_delta(&self) -> isize {
        self.replacements
            .iter()
            .map(|replacement| {
                let num_inserted = replacement.inserted_text.len() as isize;
                let num_deleted = replacement.removed_range.len() as isize;
                num_inserted - num_deleted
            })
            .sum()
    }
}

impl Replacement {
    /// TODO: docs.
    #[inline]
    pub fn inserted_text(&self) -> &str {
        &self.inserted_text
    }

    /// TODO: docs.
    #[inline]
    pub fn insertion(
        at_offset: ByteOffset,
        text: impl Into<SmolStr>,
    ) -> Self {
        Self::new(at_offset..at_offset, text)
    }

    /// Returns whether this replacement is a no-op, i.e. whether it removes no
    /// text and inserts no text.
    #[inline]
    pub fn is_no_op(&self) -> bool {
        self.removed_range.is_empty() && self.inserted_text.is_empty()
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
    pub fn removal(byte_range: Range<ByteOffset>) -> Self {
        Self::new(byte_range, "")
    }

    /// TODO: docs.
    #[inline]
    pub fn removed_range(&self) -> Range<ByteOffset> {
        self.removed_range.clone()
    }
}

impl<T: AsRef<str> + fmt::Display + fmt::Debug + for<'a> PartialEq<&'a str>>
    Chunks for T
{
    #[inline]
    fn iter(&self) -> impl Iterator<Item = impl AsRef<str>> {
        core::iter::once(self)
    }
}
