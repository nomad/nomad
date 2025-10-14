//! TODO: docs.

mod chunk;
mod chunks;
mod context_ext;
mod notify;
mod nvim_echo;
mod nvim_notify;
mod progress_reporter;

pub use chunk::Chunk;
pub use chunks::Chunks;
pub use context_ext::NotifyContextExt;
pub use notify::{NeovimEmitter, VimNotify, VimNotifyProvider, detect};
pub use nvim_echo::{NvimEcho, NvimEchoProgressReporter};
pub use nvim_notify::{NvimNotify, NvimNotifyProgressReporter};
pub use nvim_oxi::api::types::LogLevel as Level;
pub use progress_reporter::ProgressReporter;
