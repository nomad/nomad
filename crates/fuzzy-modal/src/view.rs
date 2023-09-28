use common::WindowConfig;

use crate::*;

pub(crate) struct View {
    prompt: Prompt,
    results: Results,
    on_confirm: Option<OnConfirm>,
    on_cancel: Option<OnExit>,
}

/// TODO: docs
pub(crate) enum ConfirmResult {
    /// TODO: docs
    Confirmed,

    /// TODO: docs
    Ignored,
}

impl View {
    pub fn add_results(&mut self, new_results: Vec<FuzzyItem>) {
        self.results.extend(new_results);
        let total = self.results.num_total();
        self.prompt.update_total(total);
    }

    pub fn close(&mut self) {
        self.prompt.close();

        let maybe_selected = self.results.close();

        if let Some(on_cancel) = self.on_cancel.take() {
            on_cancel(maybe_selected);
        }
    }

    pub fn closed(&mut self) {
        self.prompt.closed();

        let maybe_selected = self.results.closed();

        if let Some(on_cancel) = self.on_cancel.take() {
            on_cancel(maybe_selected);
        }
    }

    /// TODO: docs
    pub fn confirm(&mut self) -> ConfirmResult {
        if let Some(selected_result) = self.results.take_selected() {
            if let Some(on_confirm) = self.on_confirm.take() {
                on_confirm(selected_result);
            }
            ConfirmResult::Confirmed
        } else {
            ConfirmResult::Ignored
        }
    }

    pub fn new(sender: Sender) -> Self {
        Self {
            prompt: Prompt::new(sender.clone()),
            results: Results::new(sender),
            on_confirm: None,
            on_cancel: None,
        }
    }

    pub fn open(
        &mut self,
        FuzzyConfig { prompt, results, on_confirm, on_cancel }: FuzzyConfig,
        window_config: WindowConfig,
        modal_id: ModalId,
    ) {
        let (prompt_window_config, results_window_config) =
            window_config.bisect_vertical(1);

        self.prompt.open(prompt, &prompt_window_config, modal_id);
        self.results.open(results, &results_window_config, modal_id);
        self.on_confirm = on_confirm;
        self.on_cancel = on_cancel;
    }

    pub fn prompt_mut(&mut self) -> &mut Prompt {
        &mut self.prompt
    }

    pub fn results_mut(&mut self) -> &mut Results {
        &mut self.results
    }
}
