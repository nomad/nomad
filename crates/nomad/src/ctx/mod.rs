//! TODO: docs.

mod auto_command;
mod buffer;
mod file;
mod neovim;
mod text_buffer;
mod text_file;

pub use auto_command::AutoCommandCtx;
pub use buffer::BufferCtx;
pub use file::FileCtx;
pub use neovim::NeovimCtx;
pub use text_buffer::TextBufferCtx;
pub use text_file::TextFileCtx;
