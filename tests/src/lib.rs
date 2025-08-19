//! This crate contains the integration tests for all the crates in the
//! workspace.

#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]
#![cfg_attr(not(test), allow(dead_code, unused_imports))]

mod ed;
mod fs;
mod utils;

#[cfg(feature = "collab")]
mod collab;
#[cfg(feature = "gitignore")]
mod gitignore;
#[cfg(feature = "mock")]
mod mock;
#[cfg(feature = "neovim")]
mod neovim;
#[cfg(feature = "thread-pool")]
mod thread_pool;
