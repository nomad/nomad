use common::*;

use crate::*;

pub(crate) struct View {
    prompt: Prompt,
    results: Results,
}

impl View {
    pub fn add_results(&mut self, new_results: Vec<FuzzyItem>) {
        self.results.extend(new_results);
        let total = self.results.num_total();
        self.prompt.update_total(total);
    }

    pub fn close(&mut self) {
        self.prompt.close();
        self.results.close();
    }

    pub fn new(sender: Sender<Message>) -> Self {
        Self {
            prompt: Prompt::new(sender.clone()),
            results: Results::new(sender),
        }
    }

    pub fn open(&mut self, config: FuzzyConfig, window_config: WindowConfig) {
        let FuzzyConfig { prompt, .. } = config;

        let (prompt_window_config, _results_window_config) =
            window_config.bisect_vertical(1);

        self.prompt.open(prompt, &prompt_window_config);
    }

    pub fn prompt_mut(&mut self) -> &mut Prompt {
        &mut self.prompt
    }
}
