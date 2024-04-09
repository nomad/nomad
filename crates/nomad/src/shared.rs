//! TODO: docs

use alloc::rc::Rc;
use core::cell::{Cell, UnsafeCell};
use core::panic::Location;

/// TODO: docs
#[derive(Default)]
pub struct Shared<T> {
    inner: Rc<WithCell<T>>,
}

impl<T> Clone for Shared<T> {
    #[inline]
    fn clone(&self) -> Self {
        Self { inner: Rc::clone(&self.inner) }
    }
}

impl<T> Shared<T> {
    /// Returns a copy of the value.
    #[inline]
    pub fn get(&self) -> T
    where
        T: Copy,
    {
        self.inner.get()
    }

    /// Constructs a new `Shared<T>`.
    #[inline]
    pub fn new(value: T) -> Self {
        Self { inner: Rc::new(WithCell::new(value)) }
    }

    /// TODO: docs
    #[inline]
    pub fn replace(&self, new_value: T) -> T {
        self.with_mut(|this| core::mem::replace(this, new_value))
    }

    /// TODO: docs
    #[inline]
    pub fn set(&self, new_value: T) {
        self.with_mut(|this| *this = new_value);
    }

    /// TODO: docs
    #[inline]
    pub fn take(&self) -> T
    where
        T: Default,
    {
        self.replace(T::default())
    }

    /// Tries to call a closure with a shared reference to the value, returning
    /// an error if the value is already exclusively borrowed.
    #[inline]
    pub fn try_with<R>(
        &self,
        fun: impl FnOnce(&T) -> R,
    ) -> Result<R, BorrowError> {
        self.inner.try_with(fun)
    }

    /// Tries to call a closure with an exclusive reference to the value,
    /// returning an error if the value is already borrowed.
    #[inline]
    pub fn try_with_mut<R>(
        &self,
        fun: impl FnOnce(&mut T) -> R,
    ) -> Result<R, BorrowError> {
        self.inner.try_with_mut(fun)
    }

    /// Calls a closure with a shared reference to the value, panicking if the
    /// value is already exclusively borrowed.
    ///
    /// Check out [`try_with`](Self::try_with) for a non-panicking alternative.
    #[inline]
    pub fn with<R>(&self, fun: impl FnOnce(&T) -> R) -> R {
        self.inner.with(fun)
    }

    /// Calls a closure with an exclusive reference to the value, panicking if
    /// the value is already borrowed.
    ///
    /// Check out [`try_with_mut`](Self::try_with_mut) for a non-panicking
    /// alternative.
    #[inline]
    pub fn with_mut<R>(&self, fun: impl FnOnce(&mut T) -> R) -> R {
        self.inner.with_mut(fun)
    }
}

#[derive(Default)]
struct WithCell<T> {
    borrow: Cell<Borrow>,
    value: UnsafeCell<T>,
}

impl<T> WithCell<T> {
    #[inline]
    fn get(&self) -> T
    where
        T: Copy,
    {
        // SAFETY: we don't care if the value is already borrowed because we
        // immediately return a copy of it.
        unsafe { *self.value.get() }
    }

    #[inline]
    fn new(value: T) -> Self {
        Self { borrow: Cell::new(Borrow::None), value: UnsafeCell::new(value) }
    }

    #[track_caller]
    #[inline]
    fn try_with<R>(
        &self,
        fun: impl FnOnce(&T) -> R,
    ) -> Result<R, BorrowError> {
        match self.borrow.get() {
            Borrow::None | Borrow::Shared(_) => {
                let prev = self.borrow.replace(Borrow::shared());

                // SAFETY: the value is either not borrowed or borrowed via
                // a shared reference, so creating another shared reference is
                // safe.
                let value = unsafe { &*self.value.get() };

                let res = fun(value);

                self.borrow.set(prev);

                Ok(res)
            },
            Borrow::Exclusive(excl) => Err(BorrowError::new_exclusive(excl)),
        }
    }

    #[track_caller]
    #[inline]
    fn try_with_mut<R>(
        &self,
        fun: impl FnOnce(&mut T) -> R,
    ) -> Result<R, BorrowError> {
        match self.borrow.get() {
            Borrow::None => {
                self.borrow.set(Borrow::exclusive());

                // SAFETY: the value is not borrowed, so creating an exclusive
                // reference is safe.
                let value = unsafe { &mut *self.value.get() };

                let res = fun(value);

                self.borrow.set(Borrow::None);

                Ok(res)
            },
            Borrow::Shared(shrd) => Err(BorrowError::new_shared(shrd)),
            Borrow::Exclusive(excl) => Err(BorrowError::new_exclusive(excl)),
        }
    }

    #[track_caller]
    #[inline]
    fn with<R>(&self, fun: impl FnOnce(&T) -> R) -> R {
        match self.try_with(fun) {
            Ok(result) => result,
            Err(err) => panic!("{err}"),
        }
    }

    #[track_caller]
    #[inline]
    fn with_mut<R>(&self, fun: impl FnOnce(&mut T) -> R) -> R {
        match self.try_with_mut(fun) {
            Ok(result) => result,
            Err(err) => panic!("{err}"),
        }
    }
}

/// TODO: docs
#[derive(Debug)]
pub struct BorrowError {
    is_exclusive: bool,
    #[cfg(debug_assertions)]
    location: &'static Location<'static>,
}

impl BorrowError {
    #[inline]
    fn new_exclusive(borrow: ExclusiveBorrow) -> Self {
        Self {
            is_exclusive: true,
            #[cfg(debug_assertions)]
            location: borrow.location,
        }
    }

    #[inline]
    fn new_shared(borrow: SharedBorrow) -> Self {
        Self {
            is_exclusive: false,
            #[cfg(debug_assertions)]
            location: borrow.location,
        }
    }
}

impl core::fmt::Display for BorrowError {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "value already {ty}borrowed",
            ty = if self.is_exclusive { "exclusively " } else { "" }
        )?;

        #[cfg(debug_assertions)]
        write!(f, " at {location}", location = self.location)?;

        Ok(())
    }
}

impl std::error::Error for BorrowError {}

#[derive(Copy, Clone, Default)]
enum Borrow {
    #[default]
    None,
    Shared(SharedBorrow),
    Exclusive(ExclusiveBorrow),
}

impl Borrow {
    #[track_caller]
    #[inline]
    fn exclusive() -> Self {
        Self::Exclusive(ExclusiveBorrow::new())
    }

    #[track_caller]
    #[inline]
    fn shared() -> Self {
        Self::Shared(SharedBorrow::new())
    }
}

#[derive(Copy, Clone)]
struct SharedBorrow {
    #[cfg(debug_assertions)]
    location: &'static Location<'static>,
}

impl SharedBorrow {
    #[track_caller]
    #[inline]
    fn new() -> Self {
        Self {
            #[cfg(debug_assertions)]
            location: Location::caller(),
        }
    }
}

#[derive(Copy, Clone)]
struct ExclusiveBorrow {
    #[cfg(debug_assertions)]
    location: &'static Location<'static>,
}

impl ExclusiveBorrow {
    #[track_caller]
    #[inline]
    fn new() -> Self {
        Self {
            #[cfg(debug_assertions)]
            location: Location::caller(),
        }
    }
}
