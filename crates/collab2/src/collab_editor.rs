use futures_util::stream::Select;
use futures_util::{Stream, StreamExt};
use nomad2::neovim::{
    command,
    function,
    CommandEvent,
    FunctionEvent,
    ModuleApi,
    Neovim,
};
use nomad2::{Editor, Subscription};

use crate::events::{
    Cursor,
    CursorEvent,
    Edit,
    EditEvent,
    JoinSession,
    Selection,
    SelectionEvent,
    StartSession,
};
use crate::SessionId;

pub(crate) trait CollabEditor: Editor {
    type JoinStream: Stream<Item = SessionId> + Unpin;
    type StartStream: Stream<Item = ()> + Unpin;
    type EditStream: Stream<Item = Edit> + Unpin;
    type CursorStream: Stream<Item = Cursor> + Unpin;
    type SelectionStream: Stream<Item = Selection> + Unpin;
}

impl CollabEditor for Neovim {
    type JoinStream = Select<
        Subscription<CommandEvent<JoinSession>, Neovim>,
        Subscription<FunctionEvent<JoinSession>, Neovim>,
    >;
    type StartStream = Select<
        Subscription<CommandEvent<StartSession>, Neovim>,
        Subscription<FunctionEvent<StartSession>, Neovim>,
    >;
    type EditStream = Subscription<EditEvent, Neovim>;
    type CursorStream = Subscription<CursorEvent, Neovim>;
    type SelectionStream = Subscription<SelectionEvent, Neovim>;
}
