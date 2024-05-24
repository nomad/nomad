use crate::{Cells, Cutout, Metric, SceneFragment};

/// TODO: docs
#[derive(Debug, Copy, Clone)]
pub struct ExpandRect<T: Metric> {
    pub(crate) top: T,
    pub(crate) bottom: T,
    pub(crate) left: T,
    pub(crate) right: T,
}

impl Default for ExpandRect<Cells> {
    #[inline]
    fn default() -> Self {
        Self::new(Cells::zero(), Cells::zero(), Cells::zero(), Cells::zero())
    }
}

impl<T: Metric> ExpandRect<T> {
    /// Creates a new [`ExpandRect`] with the given top, bottom, left, and
    /// right values.
    #[inline]
    pub fn new(top: T, bottom: T, left: T, right: T) -> Self {
        Self { top, bottom, left, right }
    }

    /// Sets the left and right edges of the [`ExpandRect`] to the given value.
    #[inline]
    pub fn x(mut self, expand_x_by: T) -> Self {
        self.left = expand_x_by;
        self.right = expand_x_by;
        self
    }

    /// Sets the top and bottom edges of the [`ExpandRect`] to the given value.
    #[inline]
    pub fn y(mut self, expand_y_by: T) -> Self {
        self.top = expand_y_by;
        self.bottom = expand_y_by;
        self
    }
}

impl Cutout for ExpandRect<Cells> {
    type Cutout<'a> = ExpandRectCutout<'a>;

    #[inline]
    fn cutout(
        self,
        fragment: SceneFragment,
    ) -> (SceneFragment, Self::Cutout<'_>) {
        let (top, bottom) =
            foo(self.top.into(), self.bottom.into(), fragment.height().into());

        let (left, right) =
            foo(self.left.into(), self.right.into(), fragment.width().into());

        let (top, fragment) = fragment.split_x(top.into());

        let height = fragment.height();
        let (fragment, bottom) = fragment.split_x(height - bottom.into());

        let (left, fragment) = fragment.split_y(left.into());

        let width = fragment.width();
        let (fragment, right) = fragment.split_y(width - right.into());

        let cutout = ExpandRectCutout { top, bottom, left, right };

        (fragment, cutout)
    }
}

/// Splits `total` between `lhs` and `rhs`, trying to split the total evenly
/// if `lhs` and `rhs` exceed `total`.
fn foo(lhs: u32, rhs: u32, total: u32) -> (u32, u32) {
    if lhs + rhs <= total {
        (lhs, rhs)
    } else {
        let half = total / 2;
        let to_lhs = half;
        let to_rhs = half + total % 2;
        if lhs < rhs {
            (lhs.min(to_lhs), to_rhs + to_lhs.saturating_sub(lhs))
        } else {
            (to_lhs + to_rhs.saturating_sub(rhs), rhs.min(to_rhs))
        }
    }
}

/// TODO: docs.
pub struct ExpandRectCutout<'a> {
    /// TODO: docs.
    pub top: SceneFragment<'a>,

    /// TODO: docs.
    pub bottom: SceneFragment<'a>,

    /// TODO: docs.
    pub left: SceneFragment<'a>,

    /// TODO: docs.
    pub right: SceneFragment<'a>,
}
