//! TODO: docs.

#![feature(precise_capturing_in_traits)]

mod auth;
mod auth_infos;
mod backend;
pub mod login;
pub mod logout;

pub use auth::Auth;
pub use auth_infos::AuthInfos;
pub use backend::AuthBackend;
#[cfg(feature = "mock")]
pub use backend::mock;
