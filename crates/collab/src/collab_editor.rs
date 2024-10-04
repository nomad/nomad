use futures_util::Stream;
use nomad::Editor;

use crate::events::{Cursor, Edit, Selection};
use crate::{Config, SessionId};

pub(crate) trait CollabEditor: Editor {
    type ConfigStream: Stream<Item = Config> + Unpin;
    type JoinStream: Stream<Item = SessionId> + Unpin;
    type StartStream: Stream<Item = ()> + Unpin;
    type EditStream: Stream<Item = Edit> + Unpin;
    type CursorStream: Stream<Item = Cursor> + Unpin;
    type SelectionStream: Stream<Item = Selection> + Unpin;
}
