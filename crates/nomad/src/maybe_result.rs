//! TODO: docs

use std::error::Error as StdError;

/// TODO: docs
pub trait MaybeResult<T> {}

impl<T> MaybeResult<T> for T {}

impl<T, E> MaybeResult<T> for Result<T, E> where E: StdError {}
