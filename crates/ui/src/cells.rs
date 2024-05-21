use core::iter::Sum;
use core::ops::{Add, AddAssign, Sub, SubAssign};

use str_indices::chars;

use crate::Metric;

/// TODO: docs
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Cells(u32);

impl Cells {
    /// TODO: docs
    #[inline]
    pub fn as_usize(self) -> usize {
        self.0 as usize
    }

    /// TODO: docs
    #[inline]
    pub fn measure(text: &str) -> Self {
        Self(chars::count(text) as u32)
    }

    /// Splits the given string slice into two slices, with the left one
    /// measuring `self` cells.
    ///
    /// # Panics
    ///
    /// Panics if `self` is greater than the number of cells in the given text.
    #[inline]
    pub fn split(self, text: &str) -> (&str, &str) {
        let byte_offset = chars::to_byte_idx(text, self.0 as usize);
        text.split_at(byte_offset)
    }
}

impl From<u32> for Cells {
    #[inline]
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl From<Cells> for u32 {
    #[inline]
    fn from(cells: Cells) -> Self {
        cells.0
    }
}

impl Add for Cells {
    type Output = Self;

    #[inline]
    fn add(mut self, rhs: Self) -> Self {
        self += rhs;
        self
    }
}

impl AddAssign for Cells {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

impl Sub for Cells {
    type Output = Self;

    #[inline]
    fn sub(mut self, rhs: Self) -> Self {
        self -= rhs;
        self
    }
}

impl SubAssign for Cells {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
    }
}

impl Sum for Cells {
    #[inline]
    fn sum<I>(iter: I) -> Self
    where
        I: Iterator<Item = Self>,
    {
        iter.fold(Self::zero(), Add::add)
    }
}

impl Metric for Cells {
    #[inline]
    fn zero() -> Self {
        Self(0)
    }
}
