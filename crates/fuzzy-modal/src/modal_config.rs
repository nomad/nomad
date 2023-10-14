use common::Sender;

use crate::*;

pub(crate) type OnExit = Box<dyn FnOnce(Option<FuzzyItem>) + 'static>;
pub(crate) type OnSelect = Box<dyn FnMut(&FuzzyItem) + 'static>;
pub(crate) type OnConfirm = Box<dyn FnOnce(FuzzyItem) + 'static>;

pub struct FuzzyBuilder {
    config: FuzzyConfig,
    sender: Sender<(ModalId, Message)>,
    modal_id: ModalId,
}

/// TODO: docs
#[derive(Default)]
pub struct FuzzyConfig {
    pub(crate) results: ResultsConfig,
    pub(crate) prompt: PromptConfig,
    pub(crate) on_confirm: Option<OnConfirm>,
    pub(crate) on_cancel: Option<OnExit>,
    pub(crate) on_select: Option<OnSelect>,
}

impl FuzzyBuilder {
    /// The function that's called when the user confirms an item.
    ///
    /// The argument of the function is the item that was confirmed.
    pub fn on_confirm<F>(mut self, fun: F) -> Self
    where
        F: FnOnce(FuzzyItem) + 'static,
    {
        self.config.on_confirm = Some(Box::new(fun));
        self
    }

    /// The function that's called when the user exits the modal without
    /// confirming an item.
    ///
    /// The argument of the function is the item that was selected when the
    /// modal was exited (if there was one).
    pub fn on_cancel<F>(mut self, fun: F) -> Self
    where
        F: FnOnce(Option<FuzzyItem>) + 'static,
    {
        self.config.on_cancel = Some(Box::new(fun));
        self
    }

    /// The function that's called when the user selects an item.
    ///
    /// The argument of the function is the item that was selected.
    ///
    /// Note that selecting an item is different from confirming an item.
    /// Selecting simply means that the user has scrolled to an item and is
    /// currently hovering over it.
    pub fn on_select<F>(mut self, fun: F) -> Self
    where
        F: FnMut(&FuzzyItem) + 'static,
    {
        self.config.on_select = Some(Box::new(fun));
        self
    }

    /// TODO: docs
    pub fn open(self) -> FuzzyHandle {
        let Self { sender, config, modal_id } = self;
        sender.send((PASSTHROUGH_ID, Message::Open((config, modal_id))));
        FuzzyHandle::new(sender, modal_id)
    }

    /// TODO: docs
    pub fn open_with_selected(
        mut self,
        selected_item_idx: usize,
    ) -> FuzzyHandle {
        self.config.results.start_with_selected = Some(selected_item_idx);
        self.open()
    }

    pub(crate) fn new(
        sender: Sender<(ModalId, Message)>,
        modal_id: ModalId,
    ) -> Self {
        Self { sender, config: FuzzyConfig::default(), modal_id }
    }

    /// TODO: docs
    pub fn with_items<Item, Items>(mut self, items: Items) -> Self
    where
        Item: Into<FuzzyItem>,
        Items: IntoIterator<Item = Item>,
    {
        self.config.results.space.extend(items.into_iter().map(Into::into));
        self.config.prompt.total_results =
            self.config.results.space.len() as _;
        self
    }

    /// Set the placeholder text that's displayed in the prompt when there's no
    /// query.
    ///
    /// # Panics
    ///
    /// Panics if the text contains a newline.
    pub fn with_placeholder_text(mut self, text: impl Into<String>) -> Self {
        let text = text.into();
        assert!(!text.contains('\n'));
        self.config.prompt.placeholder_text = Some(text);
        self
    }
}
