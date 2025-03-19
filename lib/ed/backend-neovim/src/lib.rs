//! TODO: docs.

#![feature(precise_capturing_in_traits)]

pub mod api;
mod background_executor;
mod buffer;
mod convert;
mod local_executor;
mod neovim;
pub mod notify;
pub mod serde;
pub mod utils;
pub mod value;

pub use api::NeovimApi;
pub mod executor {
    //! TODO: docs.
    pub use crate::background_executor::NeovimBackgroundExecutor;
    pub use crate::local_executor::NeovimLocalExecutor;
}
pub use buffer::NeovimBuffer;
pub use neovim::Neovim;
#[doc(inline)]
pub use neovim_macros::plugin;
#[doc(hidden)]
pub use nvim_oxi as oxi;
pub use nvim_oxi::mlua;

/// TODO: docs.
pub type NeovimFs = ed_core::fs::os::OsFs;
