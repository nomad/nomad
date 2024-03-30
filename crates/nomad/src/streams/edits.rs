use core::pin::Pin;
use core::task::{Context, Poll};

use futures::Stream;

/// A [`Stream`] that yields the [`Edit`]s that are applied to a
/// [`Buffer`](crate::editor::Buffer).
pub struct Edits {}

impl Stream for Edits {
    type Item = Edit;

    #[inline]
    fn poll_next(
        self: Pin<&mut Self>,
        _ctx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        todo!()
    }
}

/// A single edit to a [`Buffer`].
#[derive(Clone)]
pub enum Edit {
    /// TODO: docs
    Insertion(Insertion),

    /// TODO: docs
    Deletion(Deletion),
}

/// TODO: docs
#[derive(Clone)]
pub struct Insertion {
    inner: cola::Insertion,
    text: String,
}

impl Insertion {
    pub(crate) fn new(inner: cola::Insertion, text: String) -> Self {
        Self { inner, text }
    }
}

/// TODO: docs
#[derive(Clone)]
pub struct Deletion {
    inner: cola::Deletion,
}

impl Deletion {
    pub(crate) fn new(inner: cola::Deletion) -> Self {
        Self { inner }
    }
}
