mod buf_leave;
mod buf_write_post;
mod buffer_created;
mod buffer_edited;
mod buffer_removed;
mod cursor_created;
mod cursor_moved;
mod event;
mod events;
mod mode_changed;
mod on_bytes;
mod option_set;
mod set_uneditable_eol;

pub(crate) use buf_leave::BufLeave;
pub(crate) use buf_write_post::BufWritePost;
pub(crate) use buffer_created::BufferCreated;
pub(crate) use buffer_edited::BufferEdited;
pub(crate) use buffer_removed::BufferRemoved;
pub(crate) use cursor_created::CursorCreated;
pub(crate) use cursor_moved::CursorMoved;
pub(crate) use event::{CallbacksContainer, Event};
pub(crate) use events::{Callbacks, EventHandle, EventKind, Events};
pub(crate) use mode_changed::ModeChanged;
pub(crate) use on_bytes::OnBytes;
pub(crate) use option_set::OptionSet;
pub(crate) use set_uneditable_eol::{
    SetUneditableEndOfLine,
    SetUneditableEolAgentIds,
};

pub(crate) type AugroupId = u32;
pub(crate) type AutocmdId = u32;
