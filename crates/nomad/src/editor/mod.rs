//! TODO: docs

mod buffer;
mod buffer_id;
mod buffer_state;

pub use buffer::{Buffer, RemoteDeletion, RemoteInsertion};
use buffer::{ByteChange, ByteOffset, Point};
pub use buffer_id::BufferId;
use buffer_state::{BufferState, Edit, LocalDeletion, LocalInsertion};
