use std::sync::Mutex;

use abs_path::AbsPath;
use ed::AsyncCtx;
use ed::fs::{self, Directory, FsNode, Symlink};
use futures_util::select;
use futures_util::stream::{SelectAll, StreamExt};
use walkdir::Filter;

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
pub(crate) enum EventStreamError<B: CollabBackend> {
    FollowSymlink(<<B::Fs as fs::Fs>::Symlink as Symlink>::FollowError),
    FsFilter(<B::FsFilter as Filter<B::Fs>>::Error),
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
    ) -> Result<Event<B>, EventStreamError<B>> {
        select! {
            dir_event = self.directory_streams.select_next_some() => {
                self.on_directory_event(&dir_event, ctx).await?;
                Ok(Event::Directory(dir_event))
            },
        }
    }

    async fn on_directory_event(
        &mut self,
        event: &fs::DirectoryEvent<<B::Fs as fs::Fs>::Directory>,
        ctx: &mut AsyncCtx<'_, B>,
    ) -> Result<(), EventStreamError<B>> {
        match event {
            fs::DirectoryEvent::Creation(node_creation) => {
                self.on_node_creation(node_creation, ctx).await
            },
            fs::DirectoryEvent::Deletion(directory_deletion) => {
                self.on_directory_deletion(directory_deletion, ctx).await;
                Ok(())
            },
            fs::DirectoryEvent::Move(directory_move) => {
                self.on_directory_move(directory_move, ctx).await;
                Ok(())
            },
        }
    }

    async fn on_directory_deletion(
        &mut self,
        _deletion: &fs::DirectoryDeletion,
        _ctx: &mut AsyncCtx<'_, B>,
    ) {
        // Many of the things discussed in `on_directory_move` apply here too.
        todo!()
    }

    async fn on_directory_move(
        &mut self,
        _move: &fs::DirectoryMove<<B::Fs as fs::Fs>::Directory>,
        _ctx: &mut AsyncCtx<'_, B>,
    ) {
        todo!()
    }

    async fn on_node_creation(
        &mut self,
        creation: &fs::NodeCreation<B::Fs>,
        _ctx: &mut AsyncCtx<'_, B>,
    ) -> Result<(), EventStreamError<B>> {
        if self
            .fs_filter
            .should_filter(creation.parent.path(), &creation.child)
            .await
            .map_err(EventStreamError::FsFilter)?
        {
            return Ok(());
        }

        match &creation.child {
            FsNode::File(file) => todo!(),
            FsNode::Directory(dir) => {
                self.directory_streams.push(dir.watch().await);
                Ok(())
            },
            FsNode::Symlink(symlink) => todo!(),
        }
    }
}

impl<B: CollabBackend> EventStreamBuilder<B> {
    pub(crate) fn build(self, fs_filter: B::FsFilter) -> EventStream<B> {
        let stream = self.stream.into_inner().expect("poisoned");
        let EventStream { directory_streams, .. } = stream;
        EventStream { directory_streams, fs_filter }
    }

    pub(crate) async fn push_node(
        &self,
        node: &FsNode<B::Fs>,
        ctx: &AsyncCtx<'_, B>,
    ) -> Result<(), EventStreamError<B>> {
        match node {
            FsNode::Directory(dir) => {
                self.push_directory(dir).await;
                Ok(())
            },
            FsNode::File(file) => self.push_file(file, ctx).await,
            FsNode::Symlink(symlink) => self.push_symlink(symlink, ctx).await,
        }
    }

    async fn push_directory(&self, dir: &<B::Fs as fs::Fs>::Directory) {
        let stream = dir.watch().await;
        self.stream.lock().expect("poisoned").directory_streams.push(stream);
    }

    async fn push_file(
        &self,
        _file: &<B::Fs as fs::Fs>::File,
        _ctx: &AsyncCtx<'_, B>,
    ) -> Result<(), EventStreamError<B>> {
        todo!()
    }

    async fn push_symlink(
        &self,
        symlink: &<B::Fs as fs::Fs>::Symlink,
        ctx: &AsyncCtx<'_, B>,
    ) -> Result<(), EventStreamError<B>> {
        // FIXME: we should add a watcher on the symlink itself to react to its
        // deletion.

        let Some(node) = symlink
            .follow_recursively()
            .await
            .map_err(EventStreamError::FollowSymlink)?
        else {
            return Ok(());
        };

        match node {
            FsNode::Directory(dir) => {
                self.push_directory(&dir).await;
                Ok(())
            },
            FsNode::File(file) => self.push_file(&file, ctx).await,
            FsNode::Symlink(_) => unreachable!(
                "following recursively must resolve to a File or Directory"
            ),
        }
    }
}
