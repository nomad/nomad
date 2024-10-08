use core::fmt::Debug;
use core::hash::Hash;

use futures_util::Stream;
use nomad::{Context, Editor};

use crate::events::cursor::Cursor;
use crate::events::{Edit, Selection};
use crate::{Config, SessionId};

pub(crate) trait CollabEditor: Editor {
    /// TODO: docs.
    type FileId: Hash + Clone;

    /// TODO: docs.
    type CursorId: Clone + Eq + Hash + Debug;

    /// TODO: docs.
    type Cursors: Stream<Item = Cursor<Self>> + Unpin;

    /// TODO: docs.
    fn cursors(ctx: &Context<Self>, file_id: Self::FileId) -> Self::Cursors;

    type ConfigStream: Stream<Item = Config> + Unpin;
    type JoinStream: Stream<Item = SessionId> + Unpin;
    type StartStream: Stream<Item = ()> + Unpin;
    type EditStream: Stream<Item = Edit> + Unpin;
    type SelectionStream: Stream<Item = Selection> + Unpin;
}
