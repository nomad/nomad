#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

//! This crate contains all of the workspace's tests.
//!
//! # Neovim
//!
//! Because the Neovim tests have to be compiled into a dynamic library before
//! they can be run, all the code paths touched by them have to always be
//! present, even when not in `cfg(test)`.

#[cfg(all(test, feature = "collab"))]
mod collab;
#[cfg(all(test, feature = "gitignore"))]
mod gitignore;
#[cfg(all(test, feature = "mock"))]
mod mock;
#[cfg(feature = "neovim")]
mod neovim;
#[cfg(all(test, feature = "thread-pool"))]
mod thread_pool;
#[cfg(all(test, feature = "walkdir"))]
mod walkdir;

#[cfg(any(all(test, feature = "__editor"), feature = "neovim"))]
mod ed;
#[cfg(any(all(test, feature = "__any"), feature = "neovim"))]
mod utils;
