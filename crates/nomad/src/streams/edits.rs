use core::pin::Pin;
use core::task::{Context, Poll};

use async_broadcast::Receiver;
use futures::Stream;
use pin_project_lite::pin_project;

use crate::editor::EditorId;

pin_project! {
    /// A [`Stream`] that yields the [`Edit`]s that are applied to a
    /// [`Buffer`](crate::editor::Buffer).
    pub struct Edits {
        #[pin]
        inner: Receiver<AppliedEdit>,
    }
}

impl Edits {
    #[inline]
    pub(crate) fn new(inner: Receiver<AppliedEdit>) -> Self {
        Self { inner }
    }
}

impl Stream for Edits {
    type Item = AppliedEdit;

    #[inline]
    fn poll_next(
        self: Pin<&mut Self>,
        ctx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        self.project().inner.poll_next(ctx)
    }
}

/// A single edit to a [`Buffer`].
#[derive(Debug, Clone)]
pub enum AppliedEdit {
    /// TODO: docs
    Insertion(AppliedInsertion),

    /// TODO: docs
    Deletion(AppliedDeletion),
}

impl AppliedEdit {
    /// TODO: docs
    #[inline]
    pub fn created_by(&self) -> EditorId {
        todo!();
    }
}

/// TODO: docs
#[derive(Debug, Clone)]
pub struct AppliedInsertion {
    /// TODO: docs
    pub inner: cola::Insertion,

    /// TODO: docs
    pub text: String,
}

impl AppliedInsertion {
    #[inline]
    pub(crate) fn anchor(&self) -> cola::Anchor {
        todo!();
    }

    #[inline]
    pub(crate) fn new(inner: cola::Insertion, text: String) -> Self {
        Self { inner, text }
    }

    #[inline]
    pub(crate) fn text(&self) -> &str {
        &self.text
    }
}

/// TODO: docs
#[derive(Debug, Clone)]
pub struct AppliedDeletion {
    /// TODO: docs
    pub inner: cola::Deletion,
}

impl AppliedDeletion {
    #[inline]
    pub(crate) fn end(&self) -> cola::Anchor {
        todo!();
    }

    #[inline]
    pub(crate) fn new(inner: cola::Deletion) -> Self {
        Self { inner }
    }

    #[inline]
    pub(crate) fn start(&self) -> cola::Anchor {
        todo!();
    }
}
