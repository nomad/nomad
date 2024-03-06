//! TODO: docs

mod module;
mod module_id;
mod module_name;

pub use macros::module_name;
pub use module::Module;
pub(crate) use module_id::ModuleId;
pub use module_name::ModuleName;
