use core::cmp::Ordering;
use core::ops::{Add, AddAssign};

use crate::{ExpandRect, Metric};

/// TODO: docs
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Bound<T: Metric> {
    height: T,
    width: T,
}

impl<T: Metric> Default for Bound<T> {
    #[inline]
    fn default() -> Self {
        Self::empty()
    }
}

impl<T: Metric> Bound<T> {
    /// Creates a new empty `Bound`.
    #[inline]
    pub fn empty() -> Self {
        Self { height: T::zero(), width: T::zero() }
    }

    /// TODO: docs
    #[inline]
    pub fn height(&self) -> T {
        self.height
    }

    /// Returns a mutable reference to the height of the [`Bound`].
    #[inline]
    pub fn height_mut(&mut self) -> &mut T {
        &mut self.height
    }

    /// TODO: docs
    #[inline]
    pub fn intersect(self, other: Self) -> Self {
        Self {
            height: self.height.min(other.height),
            width: self.width.min(other.width),
        }
    }

    /// TODO: docs
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.height == T::zero() || self.width == T::zero()
    }

    /// Creates a new empty `Bound`.
    #[inline]
    pub fn new<H, W>(height: H, width: W) -> Self
    where
        H: Into<T>,
        W: Into<T>,
    {
        Self { height: height.into(), width: width.into() }
    }

    /// TODO: docs
    #[inline]
    pub fn width(&self) -> T {
        self.width
    }

    /// Returns a mutable reference to the width of the [`Bound`].
    #[inline]
    pub fn width_mut(&mut self) -> &mut T {
        &mut self.width
    }
}

impl<T: Metric> PartialOrd for Bound<T> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        let height_cmp = self.height.cmp(&other.height);
        let width_cmp = self.width.cmp(&other.width);

        match (height_cmp, width_cmp) {
            (Ordering::Equal, Ordering::Equal) => Some(Ordering::Equal),

            (Ordering::Less, Ordering::Less)
            | (Ordering::Equal, Ordering::Less)
            | (Ordering::Less, Ordering::Equal) => Some(Ordering::Less),

            (Ordering::Greater, Ordering::Greater)
            | (Ordering::Equal, Ordering::Greater)
            | (Ordering::Greater, Ordering::Equal) => Some(Ordering::Greater),

            _ => None,
        }
    }
}

impl<T: Metric> AddAssign<ExpandRect<T>> for Bound<T> {
    #[inline]
    fn add_assign(&mut self, expand: ExpandRect<T>) {
        self.height += expand.top + expand.bottom;
        self.width += expand.left + expand.right;
    }
}

impl<T: Metric> Add<ExpandRect<T>> for Bound<T> {
    type Output = Self;

    #[inline]
    fn add(mut self, expand: ExpandRect<T>) -> Self {
        self += expand;
        self
    }
}
