//! TODO: docs.

mod agent_id;
mod api;
mod backend;
mod base_backend;
mod buffer;
mod cursor;
mod selection;
mod value;

pub use agent_id::AgentId;
pub use api::Api;
pub use backend::Backend;
pub use base_backend::BaseBackend;
pub use buffer::{Buffer, Edit, Replacement};
pub use cursor::Cursor;
pub use selection::Selection;
pub use value::{Key, MapAccess, Value};

/// TODO: docs.
pub type ApiValue<B> = <<B as Backend>::Api as Api>::Value;
