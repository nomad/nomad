use core::pin::Pin;
use core::task::{Context, Poll};

use abs_path::{AbsPath, AbsPathBuf};
use ed::fs;
use futures_util::stream::Stream;
use walkdir::DirEntry;

use crate::event::Event;

/// TODO: docs.
#[derive(Clone)]
pub(crate) struct EventStream<Fs> {
    _fs: core::marker::PhantomData<Fs>,
}

/// TODO: docs.
pub(crate) struct EventStreamBuilder<Fs> {
    _project_root: AbsPathBuf,
    _fs: core::marker::PhantomData<Fs>,
}

/// TODO: docs.
pub(crate) enum PushError<Fs: fs::Fs> {
    Todo(core::marker::PhantomData<Fs>),
}

impl<Fs: fs::Fs> EventStream<Fs> {
    pub(crate) fn builder(project_root: &AbsPath) -> EventStreamBuilder<Fs> {
        EventStreamBuilder {
            _project_root: project_root.to_owned(),
            _fs: core::marker::PhantomData,
        }
    }
}

impl<Fs: fs::Fs> EventStreamBuilder<Fs> {
    pub(crate) fn build(self) -> EventStream<Fs> {
        EventStream { _fs: self._fs }
    }

    pub(crate) async fn push_node(
        &self,
        _dir_path: &AbsPath,
        _node: DirEntry<Fs>,
    ) -> Result<(), PushError<Fs>> {
        todo!()
    }
}

impl<Fs: fs::Fs> Stream for EventStream<Fs> {
    type Item = Event;

    fn poll_next(
        self: Pin<&mut Self>,
        _ctx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        todo!()
    }
}
