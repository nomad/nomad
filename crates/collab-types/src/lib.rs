//! This crate contains all the types that need to be shared between the
//! Collab machinery running on the client and the [Collab
//! server](https://github.com/nomad/collab-server).

pub mod annotation;
pub mod binary;
mod counter;
pub mod fs;
mod github_handle;
pub mod lamport;
mod message;
mod peer;
mod peer_id;
mod project_request;
mod project_response;
mod protocol;
pub mod text;

pub use counter::Counter;
pub use github_handle::GitHubHandle;
pub use message::Message;
pub use peer::Peer;
pub use peer_id::PeerId;
pub use project_request::ProjectRequest;
pub use project_response::ProjectResponse;
pub use protocol::Protocol;

/// TODO: docs.
pub type Peers = smallvec::SmallVec<[Peer; 8]>;

pub use {bytes, cola, crop, puff, smallvec, smol_str};
