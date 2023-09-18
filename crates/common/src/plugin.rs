use std::error::Error;

use serde::de::DeserializeOwned;

use crate::*;

/// TODO: docs
pub trait Plugin: Default + 'static {
    /// TODO: docs
    const NAME: &'static str;

    /// TODO: docs
    type Config: DeserializeOwned + 'static;

    /// TODO: docs
    type Message: 'static;

    /// TODO: docs
    type InitError: Error + 'static;

    /// TODO: docs
    type HandleMessageError: Error + 'static;

    /// TODO: docs
    #[allow(unused_variables)]
    fn init(
        &mut self,
        sender: &Sender<Self::Message>,
    ) -> Result<(), Self::InitError> {
        Ok(())
    }

    /// TODO: docs
    #[allow(unused_variables)]
    fn init_api(builder: &mut ApiBuilder<'_, Self>) {}

    /// TODO: docs
    #[allow(unused_variables)]
    fn init_commands(builder: &mut CommandBuilder<'_, Self>) {}

    /// TODO: docs
    fn update_config(&mut self, config: Enable<Self::Config>);

    /// TODO: docs
    fn handle_message(
        &mut self,
        msg: Self::Message,
    ) -> Result<(), Self::HandleMessageError>;
}
