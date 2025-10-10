//! This crate contains the [`Params`][NomadParams] used by Nomad's collab
//! server running at `collab.nomad.foo`.

mod auth_error;
mod auth_infos;
mod nomad_params;

pub use auth_error::AuthError;
pub use auth_infos::AuthInfos;
pub use nomad_params::NomadParams;
pub use {auth_types, ulid};

/// TODO: docs.
pub const API_VERSION: u32 = 1;
