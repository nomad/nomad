//! TODO: docs.

extern crate alloc;

mod api;
mod context;
mod editor;
mod event;
mod module;
mod module_name;
pub mod neovim;
mod spawner;
mod subscription;

pub use api::Api;
pub use context::Context;
pub use editor::Editor;
pub use event::Event;
pub use module::Module;
pub use module_name::ModuleName;
pub use nomad_macros::module_name;
pub use spawner::{JoinHandle, Spawner};
pub use subscription::{Emitter, Subscription};
