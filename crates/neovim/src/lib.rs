//! TODO: docs.

#![cfg_attr(docsrs, feature(doc_auto_cfg))]

pub mod api;
pub mod buffer;
mod convert;
pub mod cursor;
mod decoration_provider;
mod events;
pub mod executor;
mod mode;
mod neovim;
pub mod notify;
mod option;
pub mod selection;
pub mod serde;
#[cfg(feature = "test")]
pub mod tests;
#[cfg(feature = "tracing")]
mod tracing_layer;
pub mod utils;
pub mod value;

pub use api::NeovimApi;
pub use neovim::Neovim;
#[doc(inline)]
pub use neovim_macros::plugin;
#[doc(inline)]
#[cfg(feature = "test")]
pub use neovim_macros::test;
#[doc(hidden)]
pub use nvim_oxi as oxi;
pub use nvim_oxi::mlua;
#[cfg(feature = "tracing")]
pub use tracing_layer::TracingLayer;
