//! TODO: docs.

#![feature(min_specialization)]

mod backend;
mod collab;
mod config;
mod session;
mod sessions;
mod start;
mod yank;

pub use backend::CollabBackend;
pub use collab::Collab;
