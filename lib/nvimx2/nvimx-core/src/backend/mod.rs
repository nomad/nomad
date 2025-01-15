//! TODO: docs.

mod api;
mod backend;
mod executor;
mod value;

pub use api::Api;
pub use backend::Backend;
pub use executor::{BackgroundExecutor, LocalExecutor, Task, TaskBackground};
pub use value::{Key, MapAccess, Value};

/// TODO: docs.
pub type ApiValue<B> = <<B as Backend>::Api as Api<B>>::Value;
