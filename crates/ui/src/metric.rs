use core::fmt::Debug;
use core::ops::{Add, AddAssign, Sub, SubAssign};

/// TODO: docs
pub trait Metric:
    Debug
    + Copy
    + Eq
    + Ord
    + Add<Self, Output = Self>
    + Sub<Self, Output = Self>
    + AddAssign<Self>
    + SubAssign<Self>
{
    /// TODO: docs
    fn zero() -> Self;

    /// TODO: docs
    #[inline]
    fn is_zero(self) -> bool {
        self == Self::zero()
    }
}

impl Metric for usize {
    #[inline]
    fn zero() -> Self {
        0
    }
}
