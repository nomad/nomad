use core::cmp::Ordering;
use core::ops::{Add, AddAssign, Sub, SubAssign};

/// A byte offset in a buffer.
#[derive(
    Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
pub struct ByteOffset(usize);

impl ByteOffset {
    /// Converts the [`ByteOffset`] into a `u64`.
    pub fn into_u64(self) -> u64 {
        self.0.try_into().expect("too big to fail")
    }

    /// Creates a new [`ByteOffset`] from the given offset.
    pub fn new(offset: usize) -> Self {
        Self(offset)
    }
}

impl Add<Self> for ByteOffset {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0)
    }
}

impl AddAssign<Self> for ByteOffset {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

impl Add<usize> for ByteOffset {
    type Output = Self;

    #[inline]
    fn add(self, rhs: usize) -> Self {
        Self(self.0 + rhs)
    }
}

impl Sub<Self> for ByteOffset {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self(self.0 - rhs.0)
    }
}

impl SubAssign<Self> for ByteOffset {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
    }
}

impl Sub<usize> for ByteOffset {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: usize) -> Self {
        Self(self.0 - rhs)
    }
}

impl From<usize> for ByteOffset {
    #[inline]
    fn from(offset: usize) -> Self {
        Self::new(offset)
    }
}

impl From<u64> for ByteOffset {
    #[inline]
    fn from(offset: u64) -> Self {
        Self::new(offset.try_into().expect("too big to fail"))
    }
}

impl From<ByteOffset> for usize {
    #[inline]
    fn from(offset: ByteOffset) -> usize {
        offset.0
    }
}

impl From<ByteOffset> for nvim_oxi::Object {
    #[inline]
    fn from(offset: ByteOffset) -> Self {
        (offset.0 as nvim_oxi::Integer).into()
    }
}

impl PartialEq<usize> for ByteOffset {
    #[inline]
    fn eq(&self, other: &usize) -> bool {
        self.0 == *other
    }
}

impl PartialEq<ByteOffset> for usize {
    #[inline]
    fn eq(&self, other: &ByteOffset) -> bool {
        *self == other.0
    }
}

impl PartialOrd<usize> for ByteOffset {
    #[inline]
    fn partial_cmp(&self, other: &usize) -> Option<Ordering> {
        self.0.partial_cmp(other)
    }
}

impl PartialOrd<ByteOffset> for usize {
    #[inline]
    fn partial_cmp(&self, other: &ByteOffset) -> Option<Ordering> {
        self.partial_cmp(&other.0)
    }
}
