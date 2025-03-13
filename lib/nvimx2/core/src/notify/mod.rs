//! TODO: docs.

mod emitter;
mod error;
mod level;
mod maybe_result;
mod message;
mod namespace;
mod nofitication;
mod notification_id;

pub use emitter::Emitter;
pub use error::Error;
pub use level::Level;
pub use maybe_result::MaybeResult;
pub use message::{Message, SpanKind};
pub use namespace::Namespace;
pub use nofitication::Notification;
pub use notification_id::NotificationId;

/// TODO: docs.
pub type Name = &'static str;
