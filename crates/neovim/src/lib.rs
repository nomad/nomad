//! TODO: docs.

pub mod api;
pub mod buffer;
mod convert;
pub mod cursor;
mod events;
mod local_executor;
mod neovim;
pub mod notify;
pub mod serde;
pub mod utils;
pub mod value;

pub use api::NeovimApi;
pub mod executor {
    //! TODO: docs.
    pub use crate::local_executor::NeovimLocalExecutor;
}
pub use neovim::Neovim;
#[doc(inline)]
pub use neovim_macros::plugin;
#[doc(hidden)]
pub use nvim_oxi as oxi;
pub use nvim_oxi::mlua;
