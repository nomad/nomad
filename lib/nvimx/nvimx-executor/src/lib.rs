//! TODO: docs

extern crate alloc;

mod executor;
mod join_handle;

pub use executor::Executor;
pub use join_handle::JoinHandle;
