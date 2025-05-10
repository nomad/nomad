//! TODO: docs.

mod agent_id;
mod api;
mod backend;
mod base_backend;
mod buffer;
mod cursor;
mod executor;
mod value;

pub use agent_id::AgentId;
pub use api::Api;
pub use backend::Backend;
pub use base_backend::BaseBackend;
pub use buffer::{Buffer, Edit, Replacement};
pub use cursor::Cursor;
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
