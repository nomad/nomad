//! TODO: docs.

#![cfg_attr(docsrs, feature(doc_auto_cfg))]

mod collab;
pub mod config;
mod convert;
pub mod copy_id;
pub mod editors;
pub mod event;
mod event_stream;
pub mod join;
pub mod jump;
pub mod leave;
mod list_ext;
mod pausable_stream;
pub mod pause;
pub mod peers;
pub mod progress;
pub mod project;
pub mod resume;
mod root_markers;
pub mod session;
pub mod start;
#[cfg(feature = "neovim")]
mod tcp_stream_ext;

pub use collab::Collab;
pub use collab_types::{Peer, PeerHandle, PeerId};
pub use editors::{CollabEditor, SessionId};
