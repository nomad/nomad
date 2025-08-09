use core::ops::AddAssign;

/// A generic counter that can be [pre](Self::pre_increment) and
/// [post](Self::post_increment) incremented.
#[derive(
    Debug, Copy, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct Counter<T> {
    /// The current value of the counter.
    pub value: T,
}

impl<T> Counter<T> {
    /// Creates a new `Counter` with the given initial value.
    #[inline]
    pub const fn new(value: T) -> Self {
        Self { value }
    }
}

impl<T: Copy + From<u8> + AddAssign> Counter<T> {
    /// Post-increments the counter by 1 and returns the old value.
    ///
    /// # Examples
    ///
    /// ```
    /// # use collab_types::Counter;
    /// let mut counter = Counter::new(0);
    /// assert_eq!(counter.post_increment(), 0);
    /// assert_eq!(counter.post_increment(), 1);
    /// assert_eq!(counter.value, 2);
    /// ```
    #[inline]
    pub fn post_increment(&mut self) -> T {
        let value = self.value;
        self.value += 1.into();
        value
    }

    /// Pre-increments the counter by 1 and returns the new value.
    ///
    /// # Examples
    ///
    /// ```
    /// # use collab_types::Counter;
    /// let mut counter = Counter::new(0);
    /// assert_eq!(counter.pre_increment(), 1);
    /// assert_eq!(counter.pre_increment(), 2);
    /// assert_eq!(counter.value, 2);
    /// ```
    #[inline]
    pub fn pre_increment(&mut self) -> T {
        self.value += 1.into();
        self.value
    }
}

impl<T> From<T> for Counter<T> {
    #[inline]
    fn from(value: T) -> Self {
        Self::new(value)
    }
}
