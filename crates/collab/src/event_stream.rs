use std::sync::Mutex;

use abs_path::AbsPath;
use ed::AsyncCtx;
use ed::fs::{self, Directory, DirectoryEvent, FsNode, Symlink};
use futures_util::select;
use futures_util::stream::{SelectAll, StreamExt};

use crate::CollabBackend;
use crate::event::Event;

type DirEventStream<Fs> =
    <<Fs as fs::Fs>::Directory as Directory>::EventStream;

/// TODO: docs.
pub(crate) struct EventStream<
    B: CollabBackend,
    FsFilter = <B as CollabBackend>::FsFilter,
> {
    directory_streams: SelectAll<DirEventStream<B::Fs>>,
    fs_filter: FsFilter,
}

/// TODO: docs.
pub(crate) struct EventStreamBuilder<B: CollabBackend> {
    stream: Mutex<EventStream<B, ()>>,
}

/// TODO: docs.
pub(crate) enum PushError<Fs: fs::Fs> {
    FollowSymlink(<Fs::Symlink as Symlink>::FollowError),
}

impl<B: CollabBackend> EventStream<B> {
    pub(crate) fn builder(_project_root: &AbsPath) -> EventStreamBuilder<B> {
        EventStreamBuilder {
            stream: Mutex::new(EventStream {
                directory_streams: SelectAll::new(),
                fs_filter: (),
            }),
        }
    }

    pub(crate) async fn next(
        &mut self,
        ctx: &mut AsyncCtx<'_, B>,
    ) -> Result<Event<B>, PushError<B::Fs>> {
        select! {
            dir_event = self.directory_streams.select_next_some() => {
                self.on_directory_event(&dir_event, ctx).await?;
                Ok(Event::Directory(dir_event))
            },
        }
    }

    async fn on_directory_event(
        &mut self,
        _dir_event: &DirectoryEvent<<B::Fs as fs::Fs>::Directory>,
        _ctx: &mut AsyncCtx<'_, B>,
    ) -> Result<(), PushError<B::Fs>> {
        // If the event is a move, we're probably about to receive a bunch of
        // notifications about the children being moved too. Worse yet, the
        // children events are not even guaranteed to be emitted after the root
        // move event.
        //
        // This might be the most difficult notification to handle. Basically,
        // we want to buffer the events for a while, wait for all of them to be
        // emitted, and then emit the root move once we timer has elapsed.
        //
        // How we do this is:
        //
        // - when we get a move event, we add it to some queue and start a
        // timer for some hard-coded duration (e.g. 100ms);
        //
        // - if we don't get another move event for that duration, we emit the
        // move event;
        //
        // - if we do, we reset the timer and start waiting again. Once the
        // timer finally expires, we check the queue and group all the entries
        // into their respective moves (the exact algorithm is tbd at this
        // point, but basically try to match related moves by looking at the
        // parent-child relationships in the paths before and after the move);
        todo!()
    }
}

impl<B: CollabBackend> EventStreamBuilder<B> {
    pub(crate) fn build(self, fs_filter: B::FsFilter) -> EventStream<B> {
        let mut stream = self.stream.into_inner().expect("poisoned");
        let EventStream { directory_streams, .. } = stream;
        EventStream { directory_streams, fs_filter }
    }

    pub(crate) async fn push_node(
        &self,
        node: &FsNode<B::Fs>,
        ctx: &AsyncCtx<'_, B>,
    ) -> Result<(), PushError<B::Fs>> {
        match node {
            FsNode::Directory(dir) => self.push_directory(dir).await,
            FsNode::File(file) => self.push_file(file, ctx).await,
            FsNode::Symlink(symlink) => self.push_symlink(symlink, ctx).await,
        }
    }

    async fn push_directory(
        &self,
        dir: &<B::Fs as fs::Fs>::Directory,
    ) -> Result<(), PushError<B::Fs>> {
        // Do we even need to do anything w/ the stream besides pushing it into
        // a `SelectAll`?
        //
        // Watching a directory for changes will yield events whose variants
        // are:
        //
        // 1: the directory is deleted (guaranteed to be the last event,
        //    followed by a None);
        // 2: the directory is renamed;
        // 3: the directory is moved to a new location (need to de-dup all the
        //    events about the children being moved w/ it);
        // 4: a child node is created;

        let stream = dir.watch().await;
        self.stream.lock().expect("poisoned").directory_streams.push(stream);
        Ok(())
    }

    async fn push_file(
        &self,
        _file: &<B::Fs as fs::Fs>::File,
        _ctx: &AsyncCtx<'_, B>,
    ) -> Result<(), PushError<B::Fs>> {
        todo!()
    }

    async fn push_symlink(
        &self,
        symlink: &<B::Fs as fs::Fs>::Symlink,
        ctx: &AsyncCtx<'_, B>,
    ) -> Result<(), PushError<B::Fs>> {
        // FIXME: we should add a watcher on the symlink itself to react to its
        // deletion.

        let Some(node) = symlink
            .follow_recursively()
            .await
            .map_err(PushError::FollowSymlink)?
        else {
            return Ok(());
        };

        match node {
            FsNode::Directory(dir) => self.push_directory(&dir).await,
            FsNode::File(file) => self.push_file(&file, ctx).await,
            FsNode::Symlink(_) => unreachable!(
                "following recursively must resolve to a File or Directory"
            ),
        }
    }
}
