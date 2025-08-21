use core::marker::PhantomData;
use core::ops::{Deref, DerefMut};

/// TODO: docs.
pub trait Access<T: ?Sized> {
    /// TODO: docs.
    fn with<R>(&self, fun: impl FnOnce(&T) -> R) -> R;

    /// Maps `Self` into an `Access<U>` by using the given closure.
    fn map<F, U>(self, fun: F) -> MapAccess<Self, F, T>
    where
        Self: Sized,
        F: Fn(&T) -> &U,
    {
        MapAccess { inner: self, fun, _mapped: PhantomData }
    }
}

/// TODO: docs.
pub trait AccessMut<T: ?Sized>: Access<T> {
    /// TODO: docs.
    fn with_mut<R>(&mut self, fun: impl FnOnce(&mut T) -> R) -> R;

    /// Maps `Self` into an `AccessMut<U>` by using two closures that provide
    /// shared and mutable access to the inner value.
    fn map_mut<FnAccess, FnAccessMut, U>(
        self,
        fun_access: FnAccess,
        fun_access_mut: FnAccessMut,
    ) -> MapAccessMut<Self, FnAccess, FnAccessMut, T>
    where
        Self: Sized,
        FnAccess: Fn(&T) -> &U,
        FnAccessMut: FnMut(&mut T) -> &mut U,
    {
        MapAccessMut {
            inner: self,
            fun_access,
            fun_access_mut,
            _mapped: PhantomData,
        }
    }
}

/// An [`Access`] adapter that maps an `Access<T>` to an `Access<U>`.
#[derive(cauchy::Clone)]
pub struct MapAccess<A, F, T: ?Sized> {
    inner: A,
    fun: F,
    _mapped: PhantomData<T>,
}

/// An [`AccessMut`] adapter that maps an `AccessMut<T>` to an `AccessMut<U>`.
#[derive(cauchy::Clone)]
pub struct MapAccessMut<A, FnAccess, FnAccessMut, T: ?Sized> {
    inner: A,
    fun_access: FnAccess,
    fun_access_mut: FnAccessMut,
    _mapped: PhantomData<T>,
}

impl<A, F, T, U> Access<U> for MapAccess<A, F, T>
where
    A: Access<T>,
    F: Fn(&T) -> &U,
{
    #[inline]
    fn with<R>(&self, fun: impl FnOnce(&U) -> R) -> R {
        self.inner.with(|t| fun((self.fun)(t)))
    }
}

impl<A, FnAccess, FnAccessMut, T, U> Access<U>
    for MapAccessMut<A, FnAccess, FnAccessMut, T>
where
    A: AccessMut<T>,
    FnAccess: Fn(&T) -> &U,
    FnAccessMut: FnMut(&mut T) -> &mut U,
{
    #[inline]
    fn with<R>(&self, fun: impl FnOnce(&U) -> R) -> R {
        self.inner.with(|t| fun((self.fun_access)(t)))
    }
}

impl<A, FnAccess, FnAccessMut, T, U> AccessMut<U>
    for MapAccessMut<A, FnAccess, FnAccessMut, T>
where
    A: AccessMut<T>,
    FnAccess: Fn(&T) -> &U,
    FnAccessMut: FnMut(&mut T) -> &mut U,
{
    #[inline]
    fn with_mut<R>(&mut self, fun: impl FnOnce(&mut U) -> R) -> R {
        self.inner.with_mut(|t| fun((self.fun_access_mut)(t)))
    }
}

impl<T: Deref> Access<T::Target> for T {
    #[inline]
    fn with<R>(&self, fun: impl FnOnce(&T::Target) -> R) -> R {
        fun(self.deref())
    }
}

impl<T: DerefMut> AccessMut<T::Target> for T {
    #[inline]
    fn with_mut<R>(&mut self, fun: impl FnOnce(&mut T::Target) -> R) -> R {
        fun(self.deref_mut())
    }
}
