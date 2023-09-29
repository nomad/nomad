use std::convert::Infallible;

use common::*;
use nvim::api::{types::*, Window};

use crate::*;

pub(crate) type Sender = common::Sender<(ModalId, Message)>;

/// TODO: docs
#[derive(Default)]
pub struct FuzzyModal {
    /// TODO: docs
    config: Config,

    /// TODO: docs
    current_modal: Option<Modal>,

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

    fn build_keymaps(&self, builder: &mut KeymapBuilder<'_, Self>) {
        let prompt_buffer = self.view.prompt().buffer();

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

        let current_id = self.current_modal.as_ref().map(Modal::id);

        // Filter messages that refer to old modals.
        if current_id != Some(modal_id) && modal_id != PASSTHROUGH_ID {
            return Ok(());
        }

        match msg {
            Message::AddResults(items) => self.view.add_results(items),
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
        FuzzyBuilder::new((*self.sender).clone(), self.id_counter)
    }

    fn close(&mut self) {
        self.view.close();

        if let Some(modal) = self.current_modal.take() {
            modal.close()
        }
    }

    fn closed(&mut self) {
        self.view.closed();

        if let Some(modal) = self.current_modal.take() {
            modal.close()
        }
    }

    fn confirm(&mut self) {
        if let ConfirmResult::Confirmed = self.view.confirm() {
            self.close();
        }
    }

    fn disable(&mut self) {
        self.is_disabled = true;
        self.view.close();
    }

    fn done_filtering(&mut self, matched: u64) {
        self.view.prompt_mut().update_matched(matched);
    }

    fn hide_placeholder(&mut self) {
        self.view.prompt_mut().remove_placeholder();
    }

    fn open(&mut self, fuzzy_config: FuzzyConfig, modal_id: ModalId) {
        self.view.close();
        self.current_modal = Some(Modal::open(modal_id));
        self.view.open(fuzzy_config, self.config.window.clone(), modal_id);
        let _ = nvim::api::command("startinsert");
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

/// TODO: docs
struct Modal {
    /// TODO: docs
    id: ModalId,

    /// TODO: docs
    parent_window: Window,

    /// TODO: docs
    opened_in_mode: Mode,

    /// TODO: docs
    opened_at_position: Option<(usize, usize)>,
}

impl Modal {
    /// TODO: docs
    fn close(mut self) {
        if self.opened_in_mode.is_insert() {
            return;
        }

        let _ = nvim::api::command("stopinsert");

        if let Some((line, col)) = self.opened_at_position {
            // I'm not really sure why it's necessary to add 1 to the original
            // column for the cursor to be placed at its original position if
            // the modal was opened while in normal mode.
            let _ = self.parent_window.set_cursor(line, col + 1);
        }
    }

    fn id(&self) -> ModalId {
        self.id
    }

    /// TODO: docs
    fn open(id: ModalId) -> Self {
        let current_mode =
            if let Ok(GotMode { mode, .. }) = nvim::api::get_mode() {
                mode
            } else {
                Mode::Normal
            };

        let parent_window = nvim::api::Window::current();

        let current_pos = parent_window.get_cursor().ok();

        Self {
            id,
            parent_window,
            opened_in_mode: current_mode,
            opened_at_position: current_pos,
        }
    }
}
