//! TODO: docs.

extern crate alloc;

mod buffer;
mod byte_offset;
mod context;
mod editor;
mod event;
mod log;
mod module;
mod module_name;
pub mod neovim;
mod nomad;
mod shared;
mod spawner;
mod subscription;
mod text;

pub use buffer::Buffer;
pub use byte_offset::ByteOffset;
pub use context::Context;
pub use editor::Editor;
pub use event::Event;
pub use module::Module;
pub use module_name::ModuleName;
pub use nomad::Nomad;
pub use nomad_macros::module_name;
pub use shared::Shared;
pub use spawner::{JoinHandle, Spawner};
pub use subscription::{Emitter, Subscription};
pub use text::Text;
