use common::*;
use nvim::api::Buffer;

use crate::FuzzyItem;

pub type DynLayout = Box<dyn Layout + 'static>;

/// TODO: docs
pub trait Layout {
    /// TODO: docs
    fn open(
        &mut self,
        prompt_buffer: &Buffer,
        results_buffer: &Buffer,
        inside: Rectangle,
    ) -> nvim::Result<()>;

    /// TODO: docs
    fn resize(&mut self, inside: Rectangle) -> nvim::Result<()>;

    /// TODO: docs
    fn close(&mut self) -> nvim::Result<Option<usize>>;

    /// TODO: docs
    fn select_next(&mut self) -> Option<usize>;

    /// TODO: docs
    fn select_prev(&mut self) -> Option<usize>;
}
