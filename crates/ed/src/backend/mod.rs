//! TODO: docs.

mod agent_id;
mod api;
mod backend;
mod buffer;
mod executor;
mod value;

pub use agent_id::AgentId;
pub use api::Api;
pub use backend::Backend;
pub use buffer::{Buffer, Edit, Replacement};
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
