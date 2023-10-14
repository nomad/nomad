use std::convert::Infallible;

use common::*;
use nvim::api::types::Mode;

use crate::*;

pub(crate) type Sender = common::Sender<(ModalId, Message)>;

/// TODO: docs
#[derive(Default)]
pub struct FuzzyModal {
    /// TODO: docs
    modal: LateInit<Modal>,

    /// TODO: docs
    config: Config,

    /// TODO: docs
    next_id: ModalId,

    /// TODO: docs
    is_disabled: bool,

    /// TODO: docs
    sender: LateInit<Sender>,
}

impl Plugin for FuzzyModal {
    const NAME: &'static str = "fuzzy_modal";

    type Message = (ModalId, Message);

    type Config = Config;

    type InitError = Infallible;

    type HandleMessageError = Infallible;

    fn init(&mut self, sender: &Sender) -> Result<(), Infallible> {
        self.sender.init(sender.clone());

        let layout = Box::new(layouts::PromptOnTop::default());

        self.modal.init(Modal::new(layout, sender.clone()));

        Ok(())
    }

    fn build_keymaps(&self, builder: &mut KeymapBuilder<'_, Self>) {
        let prompt_buffer = self.modal.prompt().buffer();

        builder
            .in_mode(Mode::Insert)
            .in_buffer(prompt_buffer.clone())
            .map("<CR>")
            .to(|| passthrough(Message::Confirm))
            .build();

        builder
            .in_mode(Mode::Insert)
            .in_buffer(prompt_buffer.clone())
            .map("<Esc>")
            .to(|| passthrough(Message::Close))
            .build();

        builder
            .in_mode(Mode::Insert)
            .in_buffer(prompt_buffer.clone())
            .map("<Up>")
            .to(|| passthrough(Message::SelectPrev))
            .build();

        builder
            .in_mode(Mode::Insert)
            .in_buffer(prompt_buffer.clone())
            .map("<Down>")
            .to(|| passthrough(Message::SelectNext))
            .build();
    }

    fn update_config(&mut self, config: Enable<Config>) {
        if !config.enable() {
            self.disable();
            return;
        }

        let window_config = config.into_inner().window;

        let msg = Message::UpdateConfig(Some(window_config));

        self.sender.send(passthrough(msg));
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
        if self.modal.id() != Some(modal_id) && modal_id != PASSTHROUGH_ID {
            return Ok(());
        }

        match msg {
            Message::AddResults(items) => self.modal.add_results(items),
            Message::Close => self.close(),
            Message::Closed => self.closed(),
            Message::Confirm => self.confirm(),
            Message::DoneFiltering(matched) => self.done_filtering(matched),
            Message::HidePlaceholder => self.hide_placeholder(),
            Message::Open((config, id)) => self.open(config, id),
            Message::PromptChanged(_diff) => {},
            Message::ShowPlaceholder => self.show_placeholder(),
            Message::SelectNext => self.select_next(),
            Message::SelectPrev => self.select_prev(),
            Message::UpdateConfig(_window_config) => {},
        };

        Ok(())
    }
}

impl FuzzyModal {
    /// TODO: docs
    pub fn builder(&self) -> FuzzyBuilder {
        FuzzyBuilder::new((*self.sender).clone(), self.next_id)
    }

    fn close(&mut self) {
        self.modal.close();
    }

    fn closed(&mut self) {
        self.modal.closed();
    }

    fn confirm(&mut self) {
        if let ConfirmResult::Confirmed = self.modal.confirm() {
            self.close();
        }
    }

    fn disable(&mut self) {
        self.is_disabled = true;
        self.modal.close();
    }

    fn done_filtering(&mut self, matched: u64) {
        self.modal.prompt_mut().update_matched(matched);
    }

    fn hide_placeholder(&mut self) {
        self.modal.prompt_mut().remove_placeholder();
    }

    fn open(&mut self, fuzzy_config: FuzzyConfig, modal_id: ModalId) {
        self.modal.open(fuzzy_config, self.config.window, modal_id);
    }

    fn select_next(&mut self) {
        self.modal.select_next();
    }

    fn select_prev(&mut self) {
        self.modal.select_prev();
    }

    fn show_placeholder(&mut self) {
        self.modal.prompt_mut().show_placeholder();
    }
}
