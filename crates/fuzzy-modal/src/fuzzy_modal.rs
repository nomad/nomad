use common::nvim;

use crate::{FuzzyItem, Prompt};

type OnExit = Box<dyn FnOnce(Option<FuzzyItem>) + 'static>;
type OnSelect = Box<dyn FnMut(&FuzzyItem) + 'static>;
type OnConfirm = Box<dyn FnOnce(FuzzyItem) + 'static>;

/// TODO: docs
#[derive(Default)]
pub struct FuzzyModal {
    items: Vec<FuzzyItem>,
    on_confirm: Option<OnConfirm>,
    on_exit: Option<OnExit>,
    on_select: Option<OnSelect>,
    prompt: Option<Prompt>,
    selected_item_idx: Option<usize>,
    starting_text: Option<String>,
}

impl FuzzyModal {
    /// TODO: docs
    pub fn builder() -> FuzzyModalBuilder {
        FuzzyModalBuilder { modal: Self::default() }
    }

    /// Closes the modal.
    ///
    /// Note that calling this will trigger the `on_exit` callback, if one was
    /// set.
    pub fn close(self) {}

    /// TODO: docs
    fn open(&mut self) {
        let len = self.items.len();

        let prompt = Prompt::new(
            self.starting_text.clone(),
            self.items.len() as _,
            move |query| {
                nvim::print!("new query is {query}");
                len as _
            },
        );

        self.prompt = Some(prompt);
    }
}

pub struct FuzzyModalBuilder {
    modal: FuzzyModal,
}

impl FuzzyModalBuilder {
    /// The function that's called when the user confirms an item.
    ///
    /// The argument of the function is the item that was confirmed.
    pub fn on_confirm<F>(mut self, fun: F) -> Self
    where
        F: FnOnce(FuzzyItem) + 'static,
    {
        self.modal.on_confirm = Some(Box::new(fun));
        self
    }

    /// The function that's called when the user exits the modal without
    /// confirming an item.
    ///
    /// The argument of the function is the item that was selected when the
    /// modal was exited (if there was one).
    pub fn on_exit<F>(mut self, fun: F) -> Self
    where
        F: FnOnce(Option<FuzzyItem>) + 'static,
    {
        self.modal.on_exit = Some(Box::new(fun));
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
        self.modal.on_select = Some(Box::new(fun));
        self
    }

    /// TODO: docs
    pub fn open(mut self) -> FuzzyModal {
        self.modal.open();
        self.modal
    }

    /// TODO: docs
    pub fn open_with_selected(
        mut self,
        selected_item_idx: usize,
    ) -> FuzzyModal {
        self.modal.selected_item_idx = Some(selected_item_idx);
        self.modal.open();
        self.modal
    }

    /// TODO: docs
    pub fn with_items<Item, Items>(mut self, items: Items) -> Self
    where
        Item: Into<FuzzyItem>,
        Items: IntoIterator<Item = Item>,
    {
        self.modal.items.extend(items.into_iter().map(Into::into));
        self
    }

    /// Set the default text that's displayed on the query line when there's no
    /// query.
    ///
    /// # Panics
    ///
    /// Panics if the text contains a newline.
    pub fn with_starting_text(mut self, text: impl Into<String>) -> Self {
        let text = text.into();
        assert!(!text.contains('\n'));
        self.modal.starting_text = Some(text);
        self
    }
}
