use core::error::Error;
use core::pin::Pin;
use core::task::{Context, Poll};

use futures_lite::Stream;

use crate::fs;

/// TODO: docs.
pub trait Watcher<Fs: fs::Fs<Watcher = Self>> {
    /// TODO: docs.
    type Error: Error + 'static;

    /// TODO: docs.
    fn register_handler<F>(&mut self, callback: F)
    where
        F: FnMut(Result<FsEvent<Fs>, Self::Error>) -> bool + 'static;

    /// TODO: docs.
    fn watched_path(&self) -> &fs::AbsPath;

    /// TODO: docs.
    fn event_stream(&mut self, capacity: Option<usize>) -> EventStream<Fs> {
        let (tx, rx) = match capacity {
            Some(cap) => flume::bounded(cap),
            None => flume::unbounded(),
        };
        self.register_handler(move |res| tx.send(res).is_err());
        EventStream { inner: rx.into_stream::<'static>() }
    }
}

pin_project_lite::pin_project! {
    /// TODO: docs.
    #[derive(Debug)]
    pub struct EventStream<Fs: fs::Fs> {
        #[pin]
        inner: flume::r#async::RecvStream<
            'static,
            Result<FsEvent<Fs>, <Fs::Watcher as Watcher<Fs>>::Error>,
        >,
    }
}

impl<Fs: fs::Fs> Stream for EventStream<Fs> {
    type Item = Result<FsEvent<Fs>, <Fs::Watcher as Watcher<Fs>>::Error>;

    #[inline]
    fn poll_next(
        self: Pin<&mut Self>,
        ctx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        self.project().inner.poll_next(ctx)
    }
}
