use std::ops::{Add, Index, Sub};

use common::*;
use nvim::api::Buffer;

use crate::{Sender, *};

#[derive(Default)]
pub(crate) struct ResultsConfig {
    /// The result space from which we filter results based on the current
    /// query.
    pub space: ResultSpace,

    /// TODO: docs
    pub start_with_selected: Option<usize>,
}

pub(crate) struct Results {
    /// TODO: docs
    config: ResultsConfig,

    /// The current contents of the prompt, which is used to filter the
    /// results.
    query: String,

    /// The results that are currently being displayed.
    displayed_results: DisplayedResults,

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
    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    pub fn close(&mut self) -> impl Iterator<Item = FuzzyItem> + '_ {
        self.query.clear();
        self.displayed_results.clear();
        self.clear_buffer();
        self.config.space.drain(..)
    }

    pub fn closed(&mut self) -> impl Iterator<Item = FuzzyItem> + '_ {
        self.close()
    }

    /// TODO: docs
    fn clear_buffer(&mut self) {
        self.buffer
            .set_lines(.., true, std::iter::empty::<nvim::String>())
            .unwrap();
    }

    pub fn displayed(&self, idx: usize) -> &FuzzyItem {
        let idx = DisplayedIdx(idx);
        &self.config.space[self.displayed_results[idx]]
    }

    pub fn displayed_to_result(&self, idx: usize) -> ResultIdx {
        let idx = DisplayedIdx(idx);
        self.displayed_results[idx]
    }

    pub fn extend(&mut self, items: impl IntoIterator<Item = FuzzyItem>) {
        self.config.space.extend(items);
        // TODO: rank the new items and update the displayed results.
    }

    fn is_displayed_first(&self, idx: DisplayedIdx) -> bool {
        self.displayed_results.is_first(idx)
    }

    fn is_displayed_last(&self, idx: DisplayedIdx) -> bool {
        self.displayed_results.is_last(idx)
    }

    pub fn new(_sender: Sender) -> Self {
        Self {
            config: ResultsConfig::default(),
            query: String::new(),
            displayed_results: DisplayedResults::default(),
            buffer: nvim::api::create_buf(false, true).unwrap(),
        }
    }

    pub fn num_total(&self) -> u64 {
        self.config.space.len() as _
    }

    pub fn open(&mut self, config: ResultsConfig, _modal_id: ModalId) {
        self.config = config;

        self.displayed_results =
            (0..self.config.space.len()).map(ResultIdx).collect();

        self.populate_buffer();
    }

    fn populate_buffer(&mut self) {
        let lines = self
            .displayed_results
            .iter()
            .map(|idx| self.config.space[idx].text.as_str());

        self.buffer.set_lines(.., true, lines).unwrap();
    }
}

#[derive(Default)]
pub(crate) struct ResultSpace {
    items: Vec<FuzzyItem>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ResultIdx(pub usize);

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

    pub fn extend(&mut self, items: impl IntoIterator<Item = FuzzyItem>) {
        self.items.extend(items);
        // TODO: rank the new results.
    }

    fn filter(&self, query: &str) -> Vec<ResultIdx> {
        todo!();
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
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

impl FromIterator<ResultIdx> for DisplayedResults {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = ResultIdx>,
    {
        Self { items: iter.into_iter().collect() }
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

    fn iter(&self) -> impl Iterator<Item = ResultIdx> + '_ {
        self.items.iter().copied()
    }
}
