use crate::render::*;
use crate::{Cells, ExpandRect, Render};

/// TODO: docs
pub trait Component {
    /// TODO: docs
    fn compose(&self) -> impl Render;

    /// TODO: docs
    #[inline]
    fn margin<R>(self, expand_rect: R) -> Margin<Self>
    where
        Self: Sized,
        R: Into<ExpandRect<Cells>>,
    {
        Margin::new(self, expand_rect.into())
    }

    /// A convenience method for setting the margin on the x-axis.
    #[inline]
    fn margin_x<C>(self, cells: C) -> Margin<Self>
    where
        Self: Sized,
        C: Into<Cells>,
    {
        self.margin(ExpandRect::default().x(cells.into()))
    }

    /// A convenience method for setting the margin on the y-axis.
    #[inline]
    fn margin_y<C>(self, cells: C) -> Margin<Self>
    where
        Self: Sized,
        C: Into<Cells>,
    {
        self.margin(ExpandRect::default().y(cells.into()))
    }
}
