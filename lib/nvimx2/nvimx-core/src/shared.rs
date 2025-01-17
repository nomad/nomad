use core::cell::{Cell, UnsafeCell};
use core::fmt;
use core::panic::AssertUnwindSafe;
#[cfg(debug_assertions)]
use core::panic::Location;
use std::panic;
use std::rc::Rc;

/// TODO: docs
#[derive(Default)]
pub struct Shared<T> {
    inner: Rc<WithCell<T>>,
}

impl<T> Clone for Shared<T> {
    fn clone(&self) -> Self {
        Self { inner: Rc::clone(&self.inner) }
    }
}

impl<T> Shared<T> {
    /// Returns a copy of the value.
    pub fn get(&self) -> T
    where
        T: Copy,
    {
        self.inner.get()
    }

    /// Constructs a new `Shared<T>`.
    pub fn new(value: T) -> Self {
        Self { inner: Rc::new(WithCell::new(value)) }
    }

    /// TODO: docs
    #[track_caller]
    pub fn replace(&self, new_value: T) -> T {
        self.with_mut(|this| core::mem::replace(this, new_value))
    }

    /// TODO: docs
    #[track_caller]
    pub fn set(&self, new_value: T) {
        self.with_mut(|this| *this = new_value);
    }

    /// TODO: docs
    #[track_caller]
    pub fn take(&self) -> T
    where
        T: Default,
    {
        self.replace(T::default())
    }

    /// Tries to call a closure with a shared reference to the value, returning
    /// an error if the value is already exclusively borrowed.
    #[track_caller]
    pub fn try_with<R>(
        &self,
        fun: impl FnOnce(&T) -> R,
    ) -> Result<R, BorrowError> {
        self.inner.try_with(fun)
    }

    /// Tries to call a closure with an exclusive reference to the value,
    /// returning an error if the value is already borrowed.
    #[track_caller]
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
    #[track_caller]
    pub fn with<R>(&self, fun: impl FnOnce(&T) -> R) -> R {
        self.inner.with(fun)
    }

    /// Calls a closure with an exclusive reference to the value, panicking if
    /// the value is already borrowed.
    ///
    /// Check out [`try_with_mut`](Self::try_with_mut) for a non-panicking
    /// alternative.
    #[track_caller]
    pub fn with_mut<R>(&self, fun: impl FnOnce(&mut T) -> R) -> R {
        self.inner.with_mut(fun)
    }
}

impl<T: fmt::Debug> fmt::Debug for Shared<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.with(|field| f.debug_tuple("Shared").field(&field).finish())
    }
}

#[derive(Default)]
struct WithCell<T> {
    borrow: Cell<Borrow>,
    value: UnsafeCell<T>,
}

impl<T> WithCell<T> {
    fn get(&self) -> T
    where
        T: Copy,
    {
        // SAFETY: we don't care if the value is already borrowed because we
        // immediately return a copy of it.
        unsafe { *self.value.get() }
    }

    fn new(value: T) -> Self {
        Self { borrow: Cell::new(Borrow::None), value: UnsafeCell::new(value) }
    }

    #[track_caller]
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

                let res = panic::catch_unwind(AssertUnwindSafe(|| fun(value)));
                self.borrow.set(prev);
                res.map_err(|payload| panic::resume_unwind(payload))
            },
            Borrow::Exclusive(excl) => Err(BorrowError::new_exclusive(excl)),
        }
    }

    #[track_caller]
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

                let res = panic::catch_unwind(AssertUnwindSafe(|| fun(value)));
                self.borrow.set(Borrow::None);
                res.map_err(|payload| panic::resume_unwind(payload))
            },
            Borrow::Shared(shrd) => Err(BorrowError::new_shared(shrd)),
            Borrow::Exclusive(excl) => Err(BorrowError::new_exclusive(excl)),
        }
    }

    #[track_caller]
    fn with<R>(&self, fun: impl FnOnce(&T) -> R) -> R {
        match self.try_with(fun) {
            Ok(result) => result,
            Err(err) => panic!("{err}"),
        }
    }

    #[track_caller]
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
    fn new_exclusive(_borrow: ExclusiveBorrow) -> Self {
        Self {
            is_exclusive: true,
            #[cfg(debug_assertions)]
            location: _borrow.location,
        }
    }

    fn new_shared(_borrow: SharedBorrow) -> Self {
        Self {
            is_exclusive: false,
            #[cfg(debug_assertions)]
            location: _borrow.location,
        }
    }
}

impl core::fmt::Display for BorrowError {
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
    fn exclusive() -> Self {
        Self::Exclusive(ExclusiveBorrow::new())
    }

    #[track_caller]
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
    fn new() -> Self {
        Self {
            #[cfg(debug_assertions)]
            location: Location::caller(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests that `Shared::with()` can be called inside the body of an outer
    /// `Shared::with()`.
    #[test]
    fn shared_with_nested() {
        let shared = Shared::new(0);

        shared.with(|_value| {
            shared.with(|_also_value| {});
        });
    }

    /// Tests that `Shared::with()` and `Shared::with_mut()` can both be called
    /// if neither is called inside the body of the other.
    #[test]
    fn shared_with_and_with_mut_alternating() {
        let shared = Shared::new(0);

        for idx in 0..10 {
            if idx % 2 == 0 {
                shared.with(|_value| {});
            } else {
                shared.with_mut(|_value| {});
            }
        }
    }

    /// Tests that calling `Shared::try_with_mut()` inside the body of an outer
    /// `Shared::with()` returns an error.
    #[test]
    fn shared_try_with_mut_inside_with() {
        let shared = Shared::new(0);

        shared.with(|_value| {
            let res = shared.try_with_mut(|_also_value| {});
            assert!(res.is_err());
        });
    }

    /// Tests that calling `Shared::try_with()` inside the body of an outer
    /// `Shared::with_mut()` returns an error.
    #[test]
    fn shared_try_with_inside_with_mut() {
        let shared = Shared::new(0);

        shared.with_mut(|_value| {
            let res = shared.try_with(|_also_value| {});
            assert!(res.is_err());
        });
    }

    /// Tests that calling `Shared::try_with_mut()` inside the body of an outer
    /// `Shared::with_mut()` returns an error.
    #[test]
    fn shared_try_with_mut_inside_with_mut() {
        let shared = Shared::new(0);

        shared.with_mut(|_value| {
            let res = shared.try_with_mut(|_also_value| {});
            assert!(res.is_err());
        });
    }

    /// Tests that calling `Shared::with_mut()` inside the body of an outer
    /// `Shared::with()` panics.
    #[should_panic]
    #[test]
    fn shared_with_mut_inside_with() {
        let shared = Shared::new(0);

        shared.with(|_value| {
            shared.with_mut(|_also_value| {});
        });
    }

    /// Tests that calling `Shared::with()` inside the body of an outer
    /// `Shared::with_mut()` panics.
    #[should_panic]
    #[test]
    fn shared_with_inside_with_mut() {
        let shared = Shared::new(0);

        shared.with_mut(|_value| {
            shared.with(|_also_value| {});
        });
    }

    /// Tests that calling `Shared::with_mut()` inside the body of an outer
    /// `Shared::with_mut()` panics.
    #[should_panic]
    #[test]
    fn shared_with_mut_inside_with_mut() {
        let shared = Shared::new(0);

        shared.with_mut(|_value| {
            shared.with_mut(|_also_value| {});
        });
    }

    #[test]
    fn borrow_is_reset_even_if_closure_panics() {
        let shared = Shared::new(0);
        let _ = panic::catch_unwind(AssertUnwindSafe(|| {
            shared.with_mut(|_| panic!())
        }));
        assert!(shared.try_with_mut(|_| ()).is_ok());
    }
}
