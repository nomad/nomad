//! TODO: docs.

pub mod api;
pub mod buffer;
mod convert;
pub mod cursor;
mod events;
pub mod executor;
mod neovim;
pub mod notify;
pub mod selection;
pub mod serde;
pub mod utils;
pub mod value;

pub use api::NeovimApi;
pub use neovim::Neovim;
#[doc(inline)]
pub use neovim_macros::{plugin, test};
#[doc(hidden)]
pub use nvim_oxi as oxi;
pub use nvim_oxi::mlua;
