//! TODO: docs.

mod async_once_lock;
mod auth;
mod auth_infos;
mod backend;
mod credential_store;
pub mod login;
pub mod logout;

pub use auth::Auth;
pub use auth_infos::AuthInfos;
pub use backend::AuthBackend;
#[cfg(feature = "mock")]
pub use backend::mock;
