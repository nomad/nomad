use serde::Serialize;

use crate::Name;

/// TODO: docs.
pub trait Constant: Serialize + 'static {
    /// TODO: docs.
    const NAME: Name;
}
