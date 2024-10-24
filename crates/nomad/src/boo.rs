use core::ops::Deref;

/// Holds either a [`Borrowed`](Boo::Borrowed) or [`Owned`](Boo::Owned) value.
pub enum Boo<'a, T> {
    /// A borrowed value.
    Borrowed(&'a T),

    /// An owned value.
    Owned(T),
}

impl<T> Boo<'_, T> {
    /// TODO: docs.
    pub fn as_ref(&self) -> Boo<'_, T> {
        match self {
            Boo::Borrowed(inner) => Boo::Borrowed(inner),
            Boo::Owned(inner) => Boo::Borrowed(inner),
        }
    }

    /// TODO: docs..
    pub fn into_owned(self) -> Boo<'static, T>
    where
        T: Clone,
    {
        match self {
            Boo::Borrowed(inner) => Boo::Owned(inner.clone()),
            Boo::Owned(inner) => Boo::Owned(inner),
        }
    }
}

impl<T: Default> Default for Boo<'_, T> {
    fn default() -> Self {
        Boo::Owned(T::default())
    }
}

impl<T: Clone> Clone for Boo<'_, T> {
    fn clone(&self) -> Self {
        match self {
            Boo::Borrowed(inner) => Boo::Borrowed(inner),
            Boo::Owned(inner) => Boo::Owned(inner.clone()),
        }
    }
}

impl<T> Deref for Boo<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            Boo::Borrowed(inner) => inner,
            Boo::Owned(inner) => inner,
        }
    }
}
