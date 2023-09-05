use common::oxi;
use common::{Enable, Plugin};

pub struct Seph;

impl Plugin for Seph {
    const NAME: &'static str = "seph";

    type Config = ();

    type SetupError = std::convert::Infallible;

    fn init() -> Self {
        Self
    }

    fn api(&self) -> oxi::Dictionary {
        oxi::Dictionary::new()
    }

    fn config(
        &mut self,
        config: Enable<Self::Config>,
    ) -> Result<(), Self::SetupError> {
        if config.enable() {
            oxi::print!("seph enabled");
        }
        Ok(())
    }
}
