use core::ops::Range;

use abs_path::AbsPathBuf;
use ed::ByteOffset;
use ed::backend::{Backend, Replacement};
use ed::fs::{DirectoryEvent, FileEvent};
use smallvec::SmallVec;

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
    Created(B::BufferId, AbsPathBuf),
    Edited(B::BufferId, SmallVec<[Replacement; 1]>),
    Removed(B::BufferId),
    Saved(B::BufferId),
}

pub(crate) struct CursorEvent<B: Backend> {
    pub(crate) buffer_id: B::BufferId,
    pub(crate) cursor_id: B::CursorId,
    pub(crate) kind: CursorEventKind,
}

pub(crate) enum CursorEventKind {
    Created(ByteOffset),
    Moved(ByteOffset),
    Removed,
}

pub(crate) struct SelectionEvent<B: Backend> {
    pub(crate) buffer_id: B::BufferId,
    pub(crate) selection_id: B::SelectionId,
    pub(crate) kind: SelectionEventKind,
}

pub(crate) enum SelectionEventKind {
    Created(Range<ByteOffset>),
    Moved(Range<ByteOffset>),
    Removed,
}
