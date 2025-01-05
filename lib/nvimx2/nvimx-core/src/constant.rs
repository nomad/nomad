use serde::Serialize;

use crate::ActionName;

/// TODO: docs.
pub trait Constant: Serialize + 'static {
    /// TODO: docs.
    const NAME: ActionName;
}

