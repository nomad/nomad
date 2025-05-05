use serde::Serialize;

use crate::notify::Name;

/// TODO: docs.
pub trait Constant: Serialize + 'static {
    /// TODO: docs.
    const NAME: Name;
}
