//! TODO: docs.

mod api;
mod backend;
mod buffer;
mod executor;
mod value;

pub use api::Api;
pub use backend::Backend;
pub use buffer::Buffer;
pub use executor::{
    BackgroundExecutor,
    LocalExecutor,
    Task,
    TaskBackground,
    TaskLocal,
};
pub use value::{Key, MapAccess, Value};

/// TODO: docs.
pub type ApiValue<B> = <<B as Backend>::Api as Api>::Value;

/// TODO: docs.
pub type BufferId<B> = <B as Backend>::BufferId;
