use core::fmt;
use core::iter::Sum;
use core::ops::{Add, AddAssign, Sub, SubAssign};

use str_indices::chars;

use crate::Metric;

/// TODO: docs
#[derive(Copy, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Cells(u32);

// Custom impl to make sure the output is always `Cells(..)` even when
// formatting with `{:#?}`.
impl fmt::Debug for Cells {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Cells({})", self.0)
    }
}

impl Cells {
    /// TODO: docs
    #[inline]
    pub fn as_usize(self) -> usize {
        self.0 as usize
    }

    /// TODO: docs
    #[inline]
    pub fn measure(text: &str) -> Self {
        // TODO: understand the problem better and the differences between
        // these before deciding what to do.
        //
        // https://github.com/unicode-rs/unicode-width
        // https://github.com/pascalkuthe/grapheme-width-rs
        // https://github.com/ridiculousfish/widecharwidth/
        // https://docs.rs/termwiz/latest/termwiz/cell/fn.grapheme_column_width.html
        //
        // Also see:
        // https://www.unicode.org/reports/tr11/
        // https://github.com/wez/wezterm/issues/4223
        // https://mitchellh.com/writing/grapheme-clusters-in-terminals
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

#[cfg(test)]
mod tests {
    use quickcheck::{Arbitrary, Gen, TestResult};
    use quickcheck_macros::quickcheck;

    use super::*;

    #[test]
    fn ui_cells_measure_two_cell_char() {
        // FIXME: `measure` should return the width, not the number of chars.
        assert_eq!(Cells::measure("老").as_usize(), 1); // 2
        assert_eq!(Cells::measure("虎").as_usize(), 1); // 2
        assert_eq!(Cells::measure("老虎").as_usize(), 2); // 4
    }

    #[quickcheck]
    fn ui_cells_qc_split(text: String, offset: Cells) -> TestResult {
        if offset > Cells::measure(&text) {
            return TestResult::discard();
        }

        let text = text.as_str();
        let (left, right) = offset.split(text);
        assert_eq!(Cells::measure(left), offset);
        assert_eq!(Cells::measure(right), Cells::measure(text) - offset);

        TestResult::passed()
    }

    impl Arbitrary for Cells {
        fn arbitrary(g: &mut Gen) -> Self {
            Cells(u32::arbitrary(g))
        }
    }
}
