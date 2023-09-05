use std::error::Error;

use nvim_oxi::Dictionary;
use serde::de::DeserializeOwned;

use crate::Enable;

/// TODO: docs
pub trait Plugin: 'static {
    /// TODO: docs
    const NAME: &'static str;

    /// TODO: docs
    type Config: DeserializeOwned + 'static;

    /// TODO: docs
    type SetupError: Error + Send + Sync + 'static;

    /// TODO: docs
    fn init() -> Self;

    /// TODO: docs
    fn api(&self) -> Dictionary;

    /// TODO: docs
    fn config(
        &mut self,
        config: Enable<Self::Config>,
    ) -> Result<(), Self::SetupError>;
}
