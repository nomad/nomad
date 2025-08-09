//! TODO: docs.

use core::error::Error;
use core::fmt::Debug;
use core::hash::Hash;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::GitHubHandle;

/// TODO: docs.
pub trait Protocol {
    /// TODO: docs.
    const MAX_FRAGMENT_LEN: u32;

    /// TODO: docs.
    type AuthenticateInfos: Send
        + Sync
        + Serialize
        + DeserializeOwned
        + AsRef<GitHubHandle>;

    /// TODO: docs.
    type AuthenticateError: Send + Serialize + DeserializeOwned + Error;

    /// TODO: docs.
    type SessionId: Debug
        + Copy
        + Clone
        + Eq
        + Hash
        + Send
        + Sync
        + Serialize
        + DeserializeOwned
        + 'static;
}
