//! TODO: docs.

extern crate alloc;

mod api;
mod context;
mod editor;
mod event;
mod module;
mod module_name;
mod neovim;
mod subscription;

pub use api::Api;
pub use context::Context;
pub use editor::Editor;
pub use event::Event;
pub use module::Module;
pub use module_name::ModuleName;
pub use neovim::Neovim;
pub use nomad_macros::module_name;
pub use subscription::{Emitter, Subscription};
