use std::convert::Infallible;

use common::*;
use tracing::*;

use crate::*;

#[derive(Default)]
pub struct FuzzyModal {
    is_disabled: bool,
    config: Config,
    sender: LateInit<Sender<Message>>,
    view: LateInit<View>,
}

impl Plugin for FuzzyModal {
    const NAME: &'static str = "fuzzy_modal";

    type Message = Message;

    type Config = Config;

    type InitError = Infallible;

    type HandleMessageError = Infallible;

    fn init(
        &mut self,
        sender: &Sender<Self::Message>,
    ) -> Result<(), Infallible> {
        self.sender.init(sender.clone());
        self.view.init(View::new(sender.clone()));
        Ok(())
    }

    fn update_config(&mut self, config: Enable<Config>) {
        if !config.enable() {
            self.disable();
            return;
        }

        let window_config = config.into_inner().window;
        self.send(Message::UpdateConfig(Some(window_config)));
    }

    fn handle_message(
        &mut self,
        msg: Message,
        _: &Ctx<Self>,
    ) -> Result<(), Infallible> {
        if self.is_disabled {
            return Ok(());
        }

        match msg {
            Message::AddResults(items) => self.view.add_results(items),
            Message::Close => self.view.close(),
            Message::Open(config) => self.open(config),
            Message::HidePlaceholder => self.hide_placeholder(),
            Message::ShowPlaceholder => self.show_placeholder(),
            _ => (),
        };

        Ok(())
    }
}

impl FuzzyModal {
    /// TODO: docs
    pub fn builder(&self) -> FuzzyBuilder {
        FuzzyBuilder::new((*self.sender).clone())
    }

    fn open(&mut self, fuzzy_config: FuzzyConfig) {
        self.view.close();
        self.view.open(fuzzy_config, self.config.window.clone());
    }

    fn disable(&mut self) {
        self.is_disabled = true;
        self.view.close();
    }

    fn hide_placeholder(&mut self) {
        self.view.prompt_mut().remove_placeholder();
    }

    fn send(&mut self, msg: Message) {
        self.sender.send(msg);
    }

    fn show_placeholder(&mut self) {
        self.view.prompt_mut().show_placeholder();
    }
}
