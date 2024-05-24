use crate::Metric;

/// TODO: docs
#[derive(Debug, Copy, Clone)]
pub(crate) struct Point<M: Metric> {
    x: M,
    y: M,
}

impl<M: Metric> Point<M> {
    /// Creates a new [`Point`] with the given x and y values.
    #[inline]
    pub(crate) fn new(x: M, y: M) -> Self {
        Self { x, y }
    }

    /// Creates a new [`Point`] whose x and y values are both zero.
    #[inline]
    pub(crate) fn origin() -> Self {
        Self::new(M::zero(), M::zero())
    }

    /// Returns the x value of the [`Point`].
    #[inline]
    pub(crate) fn x(&self) -> M {
        self.x
    }

    /// Returns a mutable reference to the x value of the [`Point`].
    #[inline]
    pub(crate) fn x_mut(&mut self) -> &mut M {
        &mut self.x
    }

    /// Returns the y value of the [`Point`].
    #[inline]
    pub(crate) fn y(&self) -> M {
        self.y
    }

    /// Returns a mutable reference to the y value of the [`Point`].
    #[inline]
    pub(crate) fn y_mut(&mut self) -> &mut M {
        &mut self.y
    }
}
