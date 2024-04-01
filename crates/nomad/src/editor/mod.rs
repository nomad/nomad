//! TODO: docs

mod autocmd_id;
mod buffer;
mod buffer_id;
mod buffer_snapshot;
mod editor_id;

pub(crate) use autocmd_id::AutocmdId;
pub use buffer::{Buffer, RemoteDeletion, RemoteInsertion};
pub use buffer_id::BufferId;
pub use buffer_snapshot::BufferSnapshot;
pub use editor_id::EditorId;
