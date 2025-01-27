use core::error::Error;

use crate::fs;

/// TODO: docs.
pub trait Watcher<Fs: fs::Fs + ?Sized> {
    /// TODO: docs.
    type Error: Error;

    /// TODO: docs.
    fn register_handler<F>(&mut self, callback: F)
    where
        F: FnMut(Result<FsEvent<Fs>, Self::Error>) -> bool + 'static;

    /// TODO: docs.
    fn watched_path(&self) -> &fs::AbsPath;

    /// TODO: docs.
    fn event_stream(&mut self) -> EventStream<Fs> {
        todo!();
    }
}

/// TODO: docs.
#[derive(Debug)]
pub struct FsEvent<Fs: fs::Fs + ?Sized> {
    /// TODO: docs.
    pub kind: FsEventKind,

    /// TODO: docs.
    pub path: fs::AbsPathBuf,

    /// TODO: docs.
    pub timestamp: Fs::Timestamp,
}

/// TODO: docs.
#[derive(Debug)]
pub enum FsEventKind {
    /// TODO: docs.
    CreatedDir,
}

/// TODO: docs.
#[derive(Debug)]
pub struct EventStream<Fs: fs::Fs + ?Sized> {
    _fs: core::marker::PhantomData<Fs>,
}
