use std::sync::Mutex;

use abs_path::{AbsPath, AbsPathBuf};
use ed::AsyncCtx;
use ed::fs::{
    self,
    Directory,
    Fs,
    FsNode,
    NodeCreation,
    NodeMetadataError,
    Symlink,
};
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
    root_id: <B::Fs as fs::Fs>::NodeId,
    root_path: AbsPathBuf,
}

/// TODO: docs.
pub(crate) struct EventStreamBuilder<B: CollabBackend> {
    stream: Mutex<EventStream<B, ()>>,
}

/// TODO: docs.
pub(crate) enum EventStreamError<B: CollabBackend> {
    FollowSymlink(<<B::Fs as fs::Fs>::Symlink as Symlink>::FollowError),
    FsFilter(<B::FsFilter as Filter<B::Fs>>::Error),
    Metadata(NodeMetadataError<B::Fs>),
    NodeAtPath(<B::Fs as fs::Fs>::NodeAtPathError),
}

impl<B: CollabBackend> EventStream<B> {
    pub(crate) fn builder(project_root: AbsPathBuf) -> EventStreamBuilder<B> {
        EventStreamBuilder {
            stream: Mutex::new(EventStream {
                directory_streams: SelectAll::new(),
                fs_filter: (),
                root_id: todo!(),
                root_path: project_root,
            }),
        }
    }

    pub(crate) async fn next(
        &mut self,
        ctx: &mut AsyncCtx<'_, B>,
    ) -> Result<Event<B>, EventStreamError<B>> {
        loop {
            select! {
                dir_event = self.directory_streams.select_next_some() => {
                    if self.on_directory_event(&dir_event, ctx).await? {
                        return Ok(Event::Directory(dir_event))
                    }
                },
            }
        }
    }

    async fn on_directory_event(
        &mut self,
        event: &fs::DirectoryEvent<B::Fs>,
        ctx: &mut AsyncCtx<'_, B>,
    ) -> Result<bool, EventStreamError<B>> {
        Ok(match event {
            fs::DirectoryEvent::Deletion(node_deletion) => {
                node_deletion.dir_id == node_deletion.deletion_root_id
                    || node_deletion.dir_id == self.root_id
            },
            fs::DirectoryEvent::Move(node_move) => {
                if !node_move.new_path.starts_with(&self.root_path) {
                    // The directory was moved outside the project, so we can
                    // drop its event stream.
                    todo!("drop dir's stream");
                }
                node_move.dir_id == node_move.move_root_id
                    || node_move.dir_id == self.root_id
            },
            fs::DirectoryEvent::Creation(node_creation) => {
                self.on_node_creation(node_creation, ctx).await?;
                true
            },
        })
    }

    async fn on_node_creation(
        &mut self,
        creation: &fs::NodeCreation<B::Fs>,
        ctx: &mut AsyncCtx<'_, B>,
    ) -> Result<(), EventStreamError<B>> {
        let Some(node) = ctx
            .fs()
            .node_at_path(&creation.node_path)
            .await
            .map_err(EventStreamError::NodeAtPath)?
        else {
            // The node must've already been deleted.
            return Ok(());
        };

        let meta = node.meta().await.map_err(EventStreamError::Metadata)?;

        let parent_path = creation.node_path.parent().expect("has a parent");

        if self
            .fs_filter
            .should_filter(parent_path, &meta)
            .await
            .map_err(EventStreamError::FsFilter)?
        {
            return Ok(());
        }

        match node {
            FsNode::File(file) => todo!(),
            FsNode::Directory(dir) => {
                self.directory_streams.push(dir.watch().await);
            },
            FsNode::Symlink(_) => {},
        }

        Ok(())
    }
}

impl<B: CollabBackend> EventStreamBuilder<B> {
    pub(crate) fn build(self, fs_filter: B::FsFilter) -> EventStream<B> {
        let stream = self.stream.into_inner().expect("poisoned");
        let EventStream { directory_streams, .. } = stream;
        todo!();
        // EventStream { directory_streams, fs_filter }
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
