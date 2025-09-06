//! TODO: docs.

#![cfg_attr(docsrs, feature(doc_auto_cfg))]

mod async_once_lock;
mod auth;
mod auth_state;
mod config;
mod credential_store;
mod editors;
#[cfg(feature = "github")]
pub mod github;
pub mod login;
pub mod logout;

pub use auth::Auth;
pub use auth_state::{AuthInfos, AuthState};
pub use config::Config;
pub use editors::AuthEditor;
#[cfg(feature = "mock")]
pub use editors::mock;
