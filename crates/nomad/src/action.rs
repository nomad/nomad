use serde::de::DeserializeOwned;

use crate::prelude::{ActionName, SetCtx};

/// TODO: docs
pub trait Action: 'static {
    /// TODO: docs
    const NAME: ActionName;

    /// TODO: docs
    type Args: DeserializeOwned;

    /// TODO: docs
    fn execute(&self, args: Self::Args, ctx: &mut SetCtx);
}
