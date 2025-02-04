//! TODO: docs.

#[cfg(feature = "__neovim")]
pub use backend_neovim as neovim;
#[cfg(feature = "tests")]
pub use backend_test as tests;
#[doc(inline)]
pub use nvimx_core::*;
