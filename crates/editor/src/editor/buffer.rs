use core::fmt;
use core::ops::Range;
use std::borrow::Cow;

use abs_path::AbsPath;
use smallvec::SmallVec;
use smol_str::SmolStr;

use crate::AccessMut;
use crate::editor::{AgentId, ByteOffset, Editor};

/// TODO: docs.
pub trait Buffer {
    /// TODO: docs.
    type Editor: Editor;

    /// TODO: docs.
    fn byte_len(&self) -> ByteOffset;

    /// TODO: docs.
    fn get_text_range(&self, byte_range: Range<ByteOffset>) -> impl Chunks;

    /// TODO: docs.
    #[inline]
    fn get_text(&self) -> impl Chunks {
        self.get_text_range(0..self.byte_len())
    }

    /// TODO: docs.
    fn id(&self) -> <Self::Editor as Editor>::BufferId;

    /// Whether the buffer is empty, i.e. whether `byte_len()` is 0.
    #[inline]
    fn is_empty(&self) -> bool {
        self.byte_len() == 0
    }

    /// TODO: docs.
    fn for_each_cursor<Fun>(&mut self, fun: Fun)
    where
        Fun: FnMut(<Self::Editor as Editor>::Cursor<'_>);

    /// TODO: docs.
    fn on_edited<Fun>(
        &mut self,
        fun: Fun,
        editor: impl AccessMut<Self::Editor> + Clone + 'static,
    ) -> <Self::Editor as Editor>::EventHandle
    where
        Fun: FnMut(&<Self::Editor as Editor>::Buffer<'_>, &Edit) + 'static;

    /// TODO: docs.
    fn on_removed<Fun>(
        &mut self,
        fun: Fun,
        editor: impl AccessMut<Self::Editor> + Clone + 'static,
    ) -> <Self::Editor as Editor>::EventHandle
    where
        Fun: FnMut(<Self::Editor as Editor>::BufferId, AgentId) + 'static;

    /// TODO: docs.
    fn on_saved<Fun>(
        &mut self,
        fun: Fun,
        editor: impl AccessMut<Self::Editor> + Clone + 'static,
    ) -> <Self::Editor as Editor>::EventHandle
    where
        Fun: FnMut(&<Self::Editor as Editor>::Buffer<'_>, AgentId) + 'static;

    /// Returns the absolute path of the file associated with this buffer.
    fn path(&self) -> Cow<'_, AbsPath>;

    /// TODO: docs.
    #[inline]
    fn schedule_deletion(
        &mut self,
        byte_range: Range<ByteOffset>,
        agent_id: AgentId,
    ) -> impl Future<Output = ()> + 'static {
        self.schedule_replacement(Replacement::deletion(byte_range), agent_id)
    }

    /// TODO: docs.
    fn schedule_edit<R>(
        &mut self,
        replacements: R,
        agent_id: AgentId,
    ) -> impl Future<Output = ()> + 'static
    where
        R: IntoIterator<Item = Replacement>;

    /// TODO: docs.
    fn schedule_focus(
        &mut self,
        agent_id: AgentId,
    ) -> impl Future<Output = ()> + 'static;

    /// TODO: docs.
    #[inline]
    fn schedule_insertion(
        &mut self,
        byte_offset: ByteOffset,
        text: impl Into<SmolStr>,
        agent_id: AgentId,
    ) -> impl Future<Output = ()> + 'static {
        self.schedule_replacement(
            Replacement::insertion(byte_offset, text),
            agent_id,
        )
    }

    /// TODO: docs.
    #[inline]
    fn schedule_replacement(
        &mut self,
        replacement: Replacement,
        agent_id: AgentId,
    ) -> impl Future<Output = ()> + 'static {
        self.schedule_edit(core::iter::once(replacement), agent_id)
    }

    /// TODO: docs.
    fn schedule_save(
        &mut self,
        agent_id: AgentId,
    ) -> impl Future<
        Output = Result<(), <Self::Editor as Editor>::BufferSaveError>,
    > + 'static;
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
        self.replacements.iter().map(Replacement::byte_delta).sum()
    }
}

impl Replacement {
    /// Returns the net change in bytes from this [`Replacement`].
    ///
    /// Positive values indicate bytes were added, negative values indicate
    /// bytes were removed.
    #[inline]
    pub fn byte_delta(&self) -> isize {
        let num_inserted = self.inserted_text.len() as isize;
        let num_deleted = self.removed_range.len() as isize;
        num_inserted - num_deleted
    }

    /// TODO: docs.
    #[inline]
    pub fn deleted_range(&self) -> Range<ByteOffset> {
        self.removed_range.clone()
    }

    /// TODO: docs.
    #[inline]
    pub fn deletion(byte_range: Range<ByteOffset>) -> Self {
        Self::new(byte_range, "")
    }

    /// TODO: docs.
    #[inline]
    pub fn inserted_text(&self) -> &str {
        &self.inserted_text
    }

    /// TODO: docs.
    #[inline]
    pub fn insertion(at_offset: ByteOffset, text: impl Into<SmolStr>) -> Self {
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
}

impl<T: AsRef<str> + fmt::Display + fmt::Debug + for<'a> PartialEq<&'a str>>
    Chunks for T
{
    #[inline]
    fn iter(&self) -> impl Iterator<Item = impl AsRef<str>> {
        core::iter::once(self)
    }
}
