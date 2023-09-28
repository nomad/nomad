use std::convert::Infallible;

use common::*;

use crate::*;

pub(crate) type Sender = common::Sender<(ModalId, Message)>;

/// TODO: docs
#[derive(Default)]
pub struct FuzzyModal {
    /// TODO: docs
    config: Config,

    /// TODO: docs
    current_modal: Option<ModalId>,

    /// TODO: docs
    id_counter: ModalId,

    /// TODO: docs
    is_disabled: bool,

    /// TODO: docs
    sender: LateInit<Sender>,

    /// TODO: docs
    view: LateInit<View>,
}

impl Plugin for FuzzyModal {
    const NAME: &'static str = "fuzzy_modal";

    type Message = (ModalId, Message);

    type Config = Config;

    type InitError = Infallible;

    type HandleMessageError = Infallible;

    fn init(&mut self, sender: &Sender) -> Result<(), Infallible> {
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

        self.sender.send((
            PASSTHROUGH_ID,
            Message::UpdateConfig(Some(window_config)),
        ));
    }

    fn handle_message(
        &mut self,
        (modal_id, msg): (ModalId, Message),
        _: &Ctx<Self>,
    ) -> Result<(), Infallible> {
        if self.is_disabled {
            return Ok(());
        }

        // Filter messages that refer to old modals.
        if self.current_modal != Some(modal_id) && modal_id != PASSTHROUGH_ID {
            return Ok(());
        }

        match msg {
            Message::AddResults(items) => self.view.add_results(items),
            Message::Close => self.close(),
            Message::Closed => self.closed(),
            Message::Confirmed => self.confirm(),
            Message::DoneFiltering(matched) => self.done_filtering(matched),
            Message::HidePlaceholder => self.hide_placeholder(),
            Message::Open((config, id)) => self.open(config, id),
            Message::PromptChanged(_diff) => {},
            Message::ShowPlaceholder => self.show_placeholder(),
            Message::SelectNextItem => self.select_next(),
            Message::SelectPrevItem => self.select_prev(),
            Message::UpdateConfig(_window_config) => {},
        };

        Ok(())
    }
}

impl FuzzyModal {
    /// TODO: docs
    pub fn builder(&self) -> FuzzyBuilder {
        FuzzyBuilder::new((*self.sender).clone(), self.id_counter)
    }

    fn close(&mut self) {
        self.view.close();
        self.current_modal = None;
    }

    fn closed(&mut self) {
        self.view.closed();
        self.current_modal = None;
    }

    fn confirm(&mut self) {
        if let ConfirmResult::Confirmed = self.view.confirm() {
            self.close();
        }
    }

    fn done_filtering(&mut self, matched: u64) {
        self.view.prompt_mut().update_matched(matched);
    }

    fn open(&mut self, fuzzy_config: FuzzyConfig, modal_id: ModalId) {
        self.view.close();
        self.current_modal = Some(modal_id);
        self.view.open(fuzzy_config, self.config.window.clone(), modal_id);
        self.id_counter += 1;
    }

    fn disable(&mut self) {
        self.is_disabled = true;
        self.view.close();
    }

    fn hide_placeholder(&mut self) {
        self.view.prompt_mut().remove_placeholder();
    }

    fn select_next(&mut self) {
        self.view.results_mut().select_next();
    }

    fn select_prev(&mut self) {
        self.view.results_mut().select_prev();
    }

    fn show_placeholder(&mut self) {
        self.view.prompt_mut().show_placeholder();
    }
}
