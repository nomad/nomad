use std::ops::{Add, Index, Sub};

use common::*;
use nvim::api::{Buffer, Window};

use crate::{Sender, *};

#[derive(Default)]
pub(crate) struct ResultsConfig {
    /// The result space from which we filter results based on the current
    /// query.
    pub space: ResultSpace,
    pub start_with_selected: Option<usize>,
    pub on_select: Option<OnSelect>,
}

pub(crate) struct Results {
    /// TODO: docs
    config: ResultsConfig,

    /// The current contents of the prompt, which is used to filter the
    /// results.
    query: String,

    /// The results that are currently being displayed.
    displayed_results: DisplayedResults,

    /// The index of the currently selected result in the displayed results.
    selected_result: Option<DisplayedIdx>,

    /// If true, then selecting the previous result when the first result is
    /// selected will select the last result, and selecting the next result
    /// when the last result is selected will select the first result.
    rollover: bool,

    /// TODO: docs
    window: Option<Window>,

    /// TODO: docs
    buffer: Buffer,
}

/// TODO: docs
enum SelectResult {
    /// TODO: docs
    Changed,

    /// TODO: docs
    Unchanged,
}

impl Results {
    pub fn close(&mut self) -> Option<FuzzyItem> {
        self.query.clear();
        self.displayed_results.clear();
        self.close_window();
        self.clear_buffer();
        self.take_selected()
    }

    pub fn closed(&mut self) -> Option<FuzzyItem> {
        self.close()
    }

    /// TODO: docs
    fn clear_buffer(&mut self) {
        self.buffer
            .set_lines(.., true, std::iter::empty::<nvim::String>())
            .unwrap();
    }

    /// TODO: docs
    fn close_window(&mut self) {
        if let Some(window) = self.window.take() {
            // This fails if the window is already closed.
            let _ = window.close(true);
        }
    }

    fn execute_on_select(&mut self) {
        // I'm afraid we have to copy `Self::selected`'s body here if we want
        // to avoid cloning the selected item (because of a double borrow).
        let selected = self.selected_result.map(|displayed_idx| {
            let result_idx = self.displayed_results[displayed_idx];
            &self.config.space[result_idx]
        });

        if let Some(selected) = selected {
            if let Some(on_select) = &mut self.config.on_select {
                on_select(selected);
            }
        }
    }

    pub fn extend(&mut self, items: impl IntoIterator<Item = FuzzyItem>) {
        self.config.space.extend(items);
        // TODO: rank the new items and update the displayed results.
    }

    fn inner_select_prev(&mut self) -> SelectResult {
        if self.displayed_results.is_empty() {
            return SelectResult::Unchanged;
        }

        if let Some(selected_idx) = self.selected_result {
            if !self.is_displayed_first(selected_idx) {
                self.select_idx(selected_idx - 1)
            } else if self.rollover {
                self.select_last()
            } else {
                SelectResult::Unchanged
            }
        } else {
            self.select_last()
        }
    }

    fn inner_select_next(&mut self) -> SelectResult {
        if self.displayed_results.is_empty() {
            return SelectResult::Unchanged;
        }

        if let Some(selected_idx) = self.selected_result {
            if !self.is_displayed_last(selected_idx) {
                self.select_idx(selected_idx + 1)
            } else if self.rollover {
                self.select_first()
            } else {
                SelectResult::Unchanged
            }
        } else {
            self.select_first()
        }
    }

    fn is_displayed_first(&self, idx: DisplayedIdx) -> bool {
        self.displayed_results.is_first(idx)
    }

    fn is_displayed_last(&self, idx: DisplayedIdx) -> bool {
        self.displayed_results.is_first(idx)
    }

    pub fn new(sender: Sender) -> Self {
        Self {
            config: ResultsConfig::default(),
            query: String::new(),
            displayed_results: DisplayedResults::default(),
            selected_result: None,
            rollover: false,
            window: None,
            buffer: nvim::api::create_buf(false, true).unwrap(),
        }
    }

    pub fn num_total(&self) -> u64 {
        self.config.space.len() as _
    }

    pub fn open(
        &mut self,
        config: ResultsConfig,
        window_config: &WindowConfig,
        modal_id: ModalId,
    ) {
    }

    /// Returns the currently selected item in the results list, if there is
    /// one.
    pub fn selected(&self) -> Option<&FuzzyItem> {
        self.selected_result.map(|displayed_idx| {
            let result_idx = self.displayed_results[displayed_idx];
            &self.config.space[result_idx]
        })
    }

    /// TODO: docs
    fn selected_idx(&self) -> Option<DisplayedIdx> {
        self.selected_result
    }

    /// # Panics
    ///
    /// Panics if the [`DisplayedResults`] are empty.
    fn select_first(&mut self) -> SelectResult {
        self.select_idx(DisplayedIdx(0))
    }

    /// # Panics
    ///
    /// Panics if the [`DisplayedResults`] are empty.
    fn select_last(&mut self) -> SelectResult {
        self.select_idx(DisplayedIdx(self.displayed_results.len() - 1))
    }

    /// Selects the next result.
    ///
    /// If the last result is currently selected, then this will select the
    /// first result if `rollover` is true.
    ///
    /// If there are no results, then this does nothing.
    pub fn select_next(&mut self) {
        if let SelectResult::Changed = self.inner_select_next() {
            self.execute_on_select();
        }
    }

    /// Selects the previous result.
    ///
    /// If the first result is currently selected, then this will select the
    /// last result if `rollover` is true.
    ///
    /// If there are no results, then this does nothing.
    pub fn select_prev(&mut self) {
        if let SelectResult::Changed = self.inner_select_prev() {
            self.execute_on_select();
        }
    }

    fn select_idx(&mut self, idx: DisplayedIdx) -> SelectResult {
        assert!(idx.0 < self.displayed_results.len());

        let old_selected = self.selected_result;

        if let Some(window) = &mut self.window {
            window.set_cursor(idx.0, 0).unwrap();
        }

        self.selected_result = Some(idx);

        if old_selected != self.selected_result {
            SelectResult::Changed
        } else {
            SelectResult::Unchanged
        }
    }

    /// TODO: docs
    pub fn take_selected(&mut self) -> Option<FuzzyItem> {
        let selected_idx = self.selected_result.take()?;
        self.config.space.drain(..).nth(selected_idx.0)
    }
}

#[derive(Default)]
pub(crate) struct ResultSpace {
    items: Vec<FuzzyItem>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ResultIdx(usize);

impl Index<ResultIdx> for ResultSpace {
    type Output = FuzzyItem;

    fn index(&self, idx: ResultIdx) -> &Self::Output {
        &self.items[idx.0]
    }
}

impl ResultSpace {
    fn drain<R>(&mut self, range: R) -> impl Iterator<Item = FuzzyItem> + '_
    where
        R: std::ops::RangeBounds<usize>,
    {
        self.items.drain(range)
    }

    pub fn extend(
        &mut self,
        items: impl IntoIterator<Item = FuzzyItem>,
    ) -> Vec<ResultIdx> {
        self.items.extend(items);
        todo!();
    }

    fn filter(&self, query: &str) -> Vec<ResultIdx> {
        todo!();
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }
}

#[derive(Default)]
struct DisplayedResults {
    /// The indices of the results that are currently being displayed.
    ///
    /// The order of the items in this vector is the order in which they are
    /// displayed, i.e. the first item in this vector is the index of the first
    /// result in the results list.
    ///
    /// TODO: use a Btree to store these instead of a Vec.
    items: Vec<ResultIdx>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DisplayedIdx(usize);

impl Add<usize> for DisplayedIdx {
    type Output = Self;

    fn add(self, rhs: usize) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl Sub<usize> for DisplayedIdx {
    type Output = Self;

    fn sub(self, rhs: usize) -> Self::Output {
        Self(self.0 - rhs)
    }
}

impl Index<DisplayedIdx> for DisplayedResults {
    type Output = ResultIdx;

    fn index(&self, idx: DisplayedIdx) -> &Self::Output {
        &self.items[idx.0]
    }
}

impl DisplayedResults {
    fn clear(&mut self) {
        self.items.clear();
    }

    fn len(&self) -> usize {
        self.items.len()
    }

    fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    fn is_first(&self, idx: DisplayedIdx) -> bool {
        !self.is_empty() && (idx.0 == 0)
    }

    fn is_last(&self, idx: DisplayedIdx) -> bool {
        !self.is_empty() && (idx.0 == self.len() - 1)
    }
}
