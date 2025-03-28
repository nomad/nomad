use core::ops::Range;

use ed::ByteOffset;
use ed::backend::Backend;
use ed::fs::{DirectoryEvent, FileEvent, Fs};
use smallvec::SmallVec;
use smol_str::SmolStr;

/// TODO: docs.
pub(crate) enum Event<B: Backend> {
    /// TODO: docs.
    Buffer(BufferEvent<B>),

    /// TODO: docs.
    Cursor(CursorEvent<B>),

    /// TODO: docs.
    Directory(DirectoryEvent<B::Fs>),

    /// TODO: docs.
    File(FileEvent<B::Fs>),

    /// TODO: docs.
    Selection(SelectionEvent<B>),
}

pub(crate) enum BufferEvent<B: Backend> {
    Edited(BufferEdit<B>),
    Saved(BufferSave<B>),
}

pub(crate) struct BufferEdit<B: Backend> {
    pub(crate) buffer_id: B::BufferId,
    pub(crate) edit: SmallVec<[Replacement; 1]>,
}

pub(crate) struct BufferSave<B: Backend> {
    pub(crate) buffer_id: B::BufferId,
    pub(crate) saved_at: <B::Fs as Fs>::Timestamp,
}

pub(crate) enum CursorEvent<B: Backend> {
    /// A new cursor with the given ID was created.
    Created(B::CursorId),

    /// The cursor with the given ID was moved to a different location.
    Relocated(B::CursorId),

    /// The cursor with the given ID was removed.
    Removed(B::CursorId),
}

pub(crate) struct Replacement {
    pub(crate) deleted_range: Range<ByteOffset>,
    pub(crate) inserted_text: SmolStr,
}

pub(crate) enum SelectionEvent<B: Backend> {
    /// A new selection with the given ID was created.
    Created(B::SelectionId),

    /// TODO: docs.
    Relocated(B::SelectionId),

    /// The selection with the given ID was removed.
    Removed(B::SelectionId),
}
