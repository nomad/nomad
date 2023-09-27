use std::ops::{Add, Index, Sub};

use common::*;
use nvim::api::{Buffer, Window};

use crate::*;

pub(crate) struct Results {
    /// The current contents of the prompt, which is used to filter the
    /// results.
    query: String,

    /// The result space from which we filter results based on the current
    /// query.
    space: ResultSpace,

    /// The results that are currently being displayed.
    displayed_results: DisplayedResults,

    /// The index of the currently selected result **in the list of displayed
    /// results**.
    ///
    /// Do not use this to index into `self.items`. Instead, use this to index
    /// into `self.displayed_results`, and then use the result of that to index
    /// into `self.items`.
    selected_result: Option<DisplayedIdx>,

    /// If true, then selecting the previous result when the first result is
    /// selected will select the last result, and selecting the next result
    /// when the last result is selected will select the first result.
    rollover: bool,

    window: Option<Window>,

    buffer: Buffer,
}

impl Results {
    pub fn close(&mut self) {}

    pub fn extend(&mut self, items: impl IntoIterator<Item = FuzzyItem>) {
        self.space.extend(items);
        // TODO: rank the new items and update the displayed results.
    }

    fn is_displayed_first(&self, idx: DisplayedIdx) -> bool {
        self.displayed_results.is_first(idx)
    }

    fn is_displayed_last(&self, idx: DisplayedIdx) -> bool {
        self.displayed_results.is_first(idx)
    }

    pub fn new(sender: Sender<Message>) -> Self {
        Self {
            query: String::new(),
            space: ResultSpace::default(),
            displayed_results: DisplayedResults::default(),
            selected_result: None,
            rollover: false,
            window: None,
            buffer: nvim::api::create_buf(false, true).unwrap(),
        }
    }

    pub fn num_total(&self) -> u64 {
        self.space.items.len() as _
    }

    /// Returns the currently selected item in the results list, if there is
    /// one.
    pub fn selected(&self) -> Option<&FuzzyItem> {
        self.displayed_results.selected().map(|idx| &self.space[idx])
    }

    /// # Panics
    ///
    /// Panics if the [`DisplayedResults`] are empty.
    fn select_first(&mut self) {
        self.select_idx(DisplayedIdx(0));
    }

    /// # Panics
    ///
    /// Panics if the [`DisplayedResults`] are empty.
    fn select_last(&mut self) {
        self.select_idx(DisplayedIdx(self.displayed_results.len() - 1));
    }

    /// Selects the next result.
    ///
    /// If the last result is currently selected, then this will select the
    /// first result if `rollover` is true.
    ///
    /// If there are no results, then this does nothing.
    pub fn select_next(&mut self) {
        if self.displayed_results.is_empty() {
            return;
        }

        if let Some(selected_idx) = self.selected_result {
            if !self.is_displayed_last(selected_idx) {
                self.select_idx(selected_idx + 1);
            } else if self.rollover {
                self.select_first();
            }
        } else {
            self.select_first();
        }
    }

    /// Selects the previous result.
    ///
    /// If the first result is currently selected, then this will select the
    /// last result if `rollover` is true.
    ///
    /// If there are no results, then this does nothing.
    pub fn select_prev(&mut self) {
        if self.displayed_results.is_empty() {
            return;
        }

        if let Some(selected_idx) = self.selected_result {
            if !self.is_displayed_first(selected_idx) {
                self.select_idx(selected_idx - 1)
            } else if self.rollover {
                self.select_last();
            }
        } else {
            self.select_last();
        }
    }

    fn select_idx(&mut self, idx: DisplayedIdx) {
        assert!(idx.0 < self.displayed_results.len());
        if let Some(window) = &mut self.window {
            window.set_cursor(idx.0, 0).unwrap();
        }
        self.selected_result = Some(idx);
    }
}

#[derive(Default)]
struct ResultSpace {
    items: Vec<FuzzyItem>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ResultIdx(usize);

impl Index<ResultIdx> for ResultSpace {
    type Output = FuzzyItem;

    fn index(&self, idx: ResultIdx) -> &Self::Output {
        &self.items[idx.0]
    }
}

impl ResultSpace {
    fn extend(
        &mut self,
        items: impl IntoIterator<Item = FuzzyItem>,
    ) -> Vec<ResultIdx> {
        self.items.extend(items);
        todo!();
    }

    fn filter(&self, query: &str) -> Vec<ResultIdx> {
        todo!();
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

    selected_item: Option<DisplayedIdx>,
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

    fn selected(&self) -> Option<ResultIdx> {
        self.selected_item.map(|idx| self[idx])
    }
}
