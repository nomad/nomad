use crate::adapters::*;
use crate::{Cells, ExpandRect, RequestedBound, SceneFragment};

/// TODO: docs
pub trait Render: 'static {
    /// TODO: docs
    fn layout(&self) -> RequestedBound<Cells>;

    /// TODO: docs
    fn paint(&self, scene_fragment: &mut SceneFragment);

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
