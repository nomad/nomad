#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

#[cfg(all(test, feature = "collab"))]
mod collab;
#[cfg(all(test, any(feature = "mock", feature = "neovim")))]
mod ed;
#[cfg(all(test, feature = "mock"))]
mod mock;
#[cfg(all(test, feature = "neovim"))]
mod neovim;
#[cfg(all(test, feature = "walkdir"))]
mod walkdir;
