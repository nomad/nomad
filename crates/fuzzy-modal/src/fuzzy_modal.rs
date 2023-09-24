use std::convert::Infallible;

use common::*;

use crate::*;

#[derive(Default)]
pub struct FuzzyModal {
    is_disabled: bool,
    config: Config,
    sender: LateInit<Sender<Message>>,
    view: Option<View>,
    id_counter: ModalId,
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
            Message::Close => self.close_view(),
            Message::Open(config) => self.open(config),
            _ => todo!(),
        };

        Ok(())
    }
}

impl FuzzyModal {
    /// TODO: docs
    pub fn builder(&self) -> FuzzyBuilder {
        FuzzyBuilder::new((*self.sender).clone())
    }

    fn open(&mut self, config: FuzzyConfig) {
        self.close_view();
        self.view = Some(View::new(config));
    }

    fn close_view(&mut self) {
        if let Some(view) = self.view.take() {
            view.close();
        }
    }

    fn disable(&mut self) {
        self.is_disabled = true;
        self.close_view();
    }

    fn send(&mut self, msg: Message) {
        self.sender.send(msg);
    }
}
