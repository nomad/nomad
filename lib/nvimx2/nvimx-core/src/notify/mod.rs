//! TODO: docs.

mod emitter;
mod error;
mod level;
mod maybe_result;
mod message;
mod module_path;
mod nofitication;
mod notification_id;
mod source;

pub use emitter::Emitter;
pub use error::Error;
pub use level::Level;
pub use maybe_result::MaybeResult;
pub use message::{Message, SpanKind};
pub use module_path::ModulePath;
pub use nofitication::Notification;
pub use notification_id::NotificationId;
pub use source::Source;

/// TODO: docs.
pub type Name = &'static str;
