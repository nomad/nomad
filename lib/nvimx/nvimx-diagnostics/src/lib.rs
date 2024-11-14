//! TODO: docs.

extern crate alloc;

mod diagnostic_message;
mod diagnostic_source;
mod highlight_group;
mod level;

pub use diagnostic_message::DiagnosticMessage;
pub use diagnostic_source::DiagnosticSource;
pub use highlight_group::HighlightGroup;
pub use level::Level;
