//! TODO: docs.

#![cfg_attr(docsrs, feature(doc_auto_cfg))]

mod collab;
pub mod config;
mod convert;
mod editors;
pub mod event;
mod event_stream;
pub mod join;
pub mod leave;
mod list_ext;
pub mod project;
mod root_markers;
pub mod session;
pub mod start;
#[cfg(feature = "neovim")]
mod tcp_stream_ext;
pub mod yank;

pub use collab::Collab;
pub use collab_types::{Peer, PeerHandle, PeerId};
#[cfg(feature = "mock")]
pub use editors::mock;
pub use editors::{CollabEditor, SessionId};
