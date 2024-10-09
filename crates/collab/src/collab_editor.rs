use core::fmt::{Debug, Display};
use core::hash::Hash;
use core::ops::Range;
use std::borrow::Cow;

use collab_fs::AbsUtf8Path;
use futures_util::Stream;
use nomad::{ActorId, ByteOffset};

use crate::events::cursor::Cursor;
use crate::events::edit::{Edit, Hunk};
use crate::events::selection::Selection;
use crate::{Config, SessionId};

pub(crate) trait CollabEditor: Sized {
    /// TODO: docs.
    type CursorId: Clone + Eq + Hash + Debug;

    /// TODO: docs.
    type FileId: Clone + Eq + Hash;

    /// TODO: docs.
    type SelectionId: Clone + Eq + Hash + Debug;

    /// TODO: docs.
    type OpenFiles: Stream<Item = Self::FileId> + Unpin;

    /// TODO: docs.
    type CloseFiles: Stream<Item = Self::FileId> + Unpin;

    /// TODO: docs.
    type Cursors: Stream<Item = Cursor<Self>> + Unpin;

    /// TODO: docs.
    type Edits: Stream<Item = Edit<Self>> + Unpin;

    /// TODO: docs.
    type Selections: Stream<Item = Selection<Self>> + Unpin;

    /// TODO: docs.
    fn open_files(&mut self) -> Self::OpenFiles;

    /// TODO: docs.
    fn close_files(&mut self) -> Self::CloseFiles;

    /// TODO: docs.
    fn cursors(&mut self, file_id: &Self::FileId) -> Self::Cursors;

    /// TODO: docs.
    fn edits(&mut self, file_id: &Self::FileId) -> Self::Edits;

    /// TODO: docs.
    fn is_text_file(&mut self, file_id: &Self::FileId) -> bool;

    /// TODO: docs.
    fn path<'ed>(
        &'ed mut self,
        file_id: &Self::FileId,
    ) -> Cow<'ed, AbsUtf8Path>;

    /// TODO: docs.
    fn selections(&mut self, file_id: &Self::FileId) -> Self::Selections;

    /// TODO: docs.
    fn apply_hunks<I>(
        &mut self,
        file_id: &Self::FileId,
        hunks: I,
        actor_id: ActorId,
    ) where
        I: Iterator<Item = Hunk>;

    /// TODO: docs.
    type Tooltip;

    /// TODO: docs.
    fn create_tooltip<T>(
        &mut self,
        file_id: &Self::FileId,
        create_at: ByteOffset,
        label: T,
    ) -> Self::Tooltip
    where
        T: Display;

    /// TODO: docs.
    fn move_tooltip(
        &mut self,
        tooltip: &mut Self::Tooltip,
        move_to: ByteOffset,
    );

    /// TODO: docs.
    fn remove_tooltip(&mut self, tooltip: Self::Tooltip);

    /// TODO: docs.
    type Highlight;

    /// TODO: docs.
    fn create_highlight(
        &mut self,
        file_id: &Self::FileId,
        range: Range<ByteOffset>,
        color: (u8, u8, u8),
    ) -> Self::Highlight;

    /// TODO: docs.
    fn move_highlight(
        &mut self,
        highlight: &mut Self::Highlight,
        range: Range<ByteOffset>,
    );

    /// TODO: docs.
    fn remove_highlight(&mut self, highlight: Self::Highlight);

    type ConfigStream: Stream<Item = Config> + Unpin;
    type JoinStream: Stream<Item = SessionId> + Unpin;
    type StartStream: Stream<Item = ()> + Unpin;
}
