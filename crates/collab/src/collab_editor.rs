use core::fmt::Debug;
use core::hash::Hash;
use std::borrow::Cow;

use collab_fs::AbsUtf8Path;
use futures_util::Stream;

use crate::events::cursor::Cursor;
use crate::events::{Edit, Selection};
use crate::{Config, SessionId};

pub(crate) trait CollabEditor: Sized {
    /// TODO: docs.
    type FileId: Clone + Eq + Hash;

    /// TODO: docs.
    type CursorId: Clone + Eq + Hash + Debug;

    /// TODO: docs.
    type Cursors: Stream<Item = Cursor<Self>> + Unpin;

    /// TODO: docs.
    type Edits: Stream<Item = Edit<Self>> + Unpin;

    /// TODO: docs.
    type Selections: Stream<Item = Selection> + Unpin;

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

    type ConfigStream: Stream<Item = Config> + Unpin;
    type JoinStream: Stream<Item = SessionId> + Unpin;
    type StartStream: Stream<Item = ()> + Unpin;
}
