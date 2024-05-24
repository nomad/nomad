use core::cell::UnsafeCell;

/// TODO: docs
#[derive(Debug)]
pub struct Memoize<T> {
    memoized: UnsafeCell<Option<T>>,
}

impl<T> Default for Memoize<T> {
    #[inline]
    fn default() -> Self {
        Self { memoized: UnsafeCell::new(None) }
    }
}

impl<T> Memoize<T> {
    /// Returns a reference to the memoized value.
    ///
    /// If there's already a memoized value, it will be returned. Otherwise,
    /// the given closure will be called to produce a new value, which will be
    /// memoized and returned.
    #[inline]
    pub fn get<F: FnOnce() -> T>(&self, f: F) -> &T {
        // SAFETY: we only access the mutable reference when there's no
        // memoized value.
        unsafe { self.memoized_mut().get_or_insert_with(f) }
    }

    #[inline]
    #[allow(clippy::mut_from_ref)]
    unsafe fn memoized_mut(&self) -> &mut Option<T> {
        &mut *self.memoized.get()
    }

    /// Creates a new [`Memoize`] in its default state.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Takes the memoized value out of the [`Memoize`], leaving it in its
    /// default state.
    #[inline]
    pub fn take(&mut self) -> Option<T> {
        self.memoized.get_mut().take()
    }
}
