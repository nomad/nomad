//! TODO: docs.

// Needed to bound the future returned by an `AsyncFnOnce` to `Send`.
#![feature(async_fn_traits)]
#![feature(unboxed_closures)]

mod filter;
mod fs_ext;
#[cfg(feature = "gitignore")]
mod gitignore;
mod walkdir;

pub use filter::{And, Either, Filter, Filtered};
pub use fs_ext::{FsExt, Walker};
#[cfg(feature = "gitignore")]
pub use gitignore::GitIgnore;
pub use walkdir::{WalkDir, WalkError};
