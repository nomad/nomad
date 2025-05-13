use abs_path::AbsPathBuf;
use ed::backend::{AgentId, Buffer, Cursor};
use ed::fs::{self, Directory, File, Fs, FsNode, Metadata};
use ed::{AsyncCtx, Shared};
use futures_util::{StreamExt, select_biased};
use fxhash::{FxBuildHasher, FxHashMap, FxHashSet};
use smallvec::{SmallVec, smallvec_inline};
use walkdir::Filter;

use crate::backend::CollabBackend;
use crate::event::{BufferEvent, CursorEvent, CursorEventKind, Event};
use crate::seq_ext::StreamableSeq;

type FxIndexMap<K, V> = indexmap::IndexMap<K, V, FxBuildHasher>;

pub(crate) struct EventStream<
    B: CollabBackend,
    F: Filter<B::Fs> = <B as CollabBackend>::ProjectFilter,
> {
    /// The `AgentId` of the `Session` that owns this `EventRx`.
    agent_id: AgentId,

    buffer_handles: FxHashMap<B::BufferId, SmallVec<[B::EventHandle; 3]>>,
    buffer_rx: flume::r#async::RecvStream<'static, BufferEvent<B>>,
    buffer_tx: flume::Sender<BufferEvent<B>>,
    #[allow(dead_code)]
    new_buffers_handle: B::EventHandle,

    cursor_handles: FxHashMap<B::CursorId, SmallVec<[B::EventHandle; 2]>>,
    cursor_rx: flume::r#async::RecvStream<'static, CursorEvent<B>>,
    cursor_tx: flume::Sender<CursorEvent<B>>,
    #[allow(dead_code)]
    new_cursors_handle: B::EventHandle,

    /// TODO: docs.
    fs_streams: FsStreams<B::Fs>,
    /// Map from a file's node ID to the ID of the corresponding buffer.
    node_to_buf_ids: FxHashMap<<B::Fs as Fs>::NodeId, B::BufferId>,
    /// A filter used to check if [`FsNode`]s created under the project root
    /// should be part of the project.
    project_filter: F,
    /// The ID of the root of the project.
    root_id: <B::Fs as Fs>::NodeId,
    /// The path to the root of the project.
    root_path: AbsPathBuf,
    /// A set of buffer IDs for buffers that have just been saved.
    saved_buffers: Shared<FxHashSet<B::BufferId>>,
}

pub(crate) struct EventStreamBuilder<Fs: fs::Fs, State = NeedsProjectFilter> {
    fs_streams: FsStreams<Fs>,
    root_id: Fs::NodeId,
    root_path: AbsPathBuf,
    state: State,
}

pub(crate) struct NeedsProjectFilter;

pub(crate) struct Done<F> {
    filter: F,
}

#[derive(cauchy::Debug, derive_more::Display, cauchy::Error)]
#[display("{_0}")]
pub(crate) enum EventRxError<B: CollabBackend, F: Filter<B::Fs>> {
    FsFilter(F::Error),
    NodeAtPath(<B::Fs as Fs>::NodeAtPathError),
}

#[derive(cauchy::Default)]
struct FsStreams<Fs: fs::Fs> {
    /// Map from a directory's node ID to its event stream.
    directories:
        FxIndexMap<Fs::NodeId, <Fs::Directory as Directory>::EventStream>,
    /// Map from a file's node ID to its event stream.
    files: FxIndexMap<Fs::NodeId, <Fs::File as File>::EventStream>,
}

impl<B: CollabBackend, F: Filter<B::Fs>> EventStream<B, F> {
    pub(crate) fn agent_id(&self) -> AgentId {
        self.agent_id
    }

    pub(crate) async fn next(
        &mut self,
        ctx: &AsyncCtx<'_, B>,
    ) -> Result<Event<B>, EventRxError<B, F>> {
        loop {
            let mut dir_stream = self.fs_streams.directories.as_stream(0);
            let mut file_stream = self.fs_streams.files.as_stream(0);

            return Ok(select_biased! {
                buffer_event = self.buffer_rx.select_next_some() => {
                    match &buffer_event {
                        BufferEvent::Created(buffer_id, _) => {
                            if !self.handle_new_buffer(buffer_id.clone(), ctx).await? {
                                continue;
                            }
                        },
                        BufferEvent::Removed(buffer_id) => {
                            self.buffer_handles.remove(buffer_id);
                        },
                        _ => {},
                    }
                    Event::Buffer(buffer_event)
                },
                cursor_event = self.cursor_rx.select_next_some() => {
                    match self.handle_cursor_event(cursor_event) {
                        Some(cursor_event) => Event::Cursor(cursor_event),
                        None => continue,
                    }
                },
                dir_event = dir_stream.select_next_some() => {
                    match self.handle_dir_event(dir_event, ctx).await? {
                        Some(dir_event) => Event::Directory(dir_event),
                        None => continue,
                    }
                },
                file_event = file_stream.select_next_some() => {
                    Event::File(file_event)
                },
            });
        }
    }

    pub(crate) fn watch_buffer(
        &mut self,
        file_id: <B::Fs as fs::Fs>::NodeId,
        buffer: &B::Buffer<'_>,
    ) {
        let agent_id = self.agent_id;

        let tx = self.buffer_tx.clone();
        let edits_handle = buffer.on_edited(move |buf, edit| {
            if edit.made_by != agent_id {
                return;
            }
            let event =
                BufferEvent::Edited(buf.id(), edit.replacements.clone());
            let _ = tx.send(event);
        });

        let tx = self.buffer_tx.clone();
        let removed_handle = buffer.on_removed(move |buf, _removed_by| {
            let event = BufferEvent::Removed(buf.id());
            let _ = tx.send(event);
        });

        let saved_buffers = self.saved_buffers.clone();
        let tx = self.buffer_tx.clone();
        let saved_handle = buffer.on_saved(move |buf, saved_by| {
            saved_buffers.with_mut(|buffers| buffers.insert(buf.id()));
            if saved_by != agent_id {
                let event = BufferEvent::Saved(buf.id());
                let _ = tx.send(event);
            }
        });

        self.buffer_handles.insert(
            buffer.id(),
            smallvec_inline![edits_handle, removed_handle, saved_handle],
        );

        self.node_to_buf_ids.insert(file_id, buffer.id());
    }

    fn watch(&mut self, node: &FsNode<B::Fs>, ctx: &AsyncCtx<'_, B>) {
        self.fs_streams.watch_node(node);

        if let FsNode::File(file) = node {
            ctx.with_ctx(|ctx| {
                if let Some(buffer) = ctx.buffer_at_path(file.path()) {
                    self.watch_buffer(file.id(), &buffer);
                }
            });
        }
    }

    #[allow(clippy::too_many_lines)]
    async fn handle_dir_event(
        &mut self,
        event: fs::DirectoryEvent<B::Fs>,
        ctx: &AsyncCtx<'_, B>,
    ) -> Result<Option<fs::DirectoryEvent<B::Fs>>, EventRxError<B, F>> {
        Ok(match event {
            fs::DirectoryEvent::Creation(ref creation) => {
                let Some(node) = ctx
                    .fs()
                    .node_at_path(&creation.node_path)
                    .await
                    .map_err(EventRxError::NodeAtPath)?
                else {
                    // The node must've already been deleted.
                    return Ok(None);
                };

                if self.should_watch(&node).await? {
                    self.watch(&node, ctx);
                    Some(event)
                } else {
                    None
                }
            },

            fs::DirectoryEvent::Deletion(ref deletion) => {
                if let Some(buf_id) =
                    self.node_to_buf_ids.get(&deletion.node_id)
                {
                    self.buffer_handles.remove(buf_id);
                }

                if deletion.node_id != deletion.deletion_root_id {
                    // This event was caused by an ancestor of the node being
                    // deleted. We should ignore it, unless it's about the
                    // root.
                    (deletion.node_id == self.root_id).then_some(event)
                } else {
                    Some(event)
                }
            },

            fs::DirectoryEvent::Move(r#move) => {
                if r#move.node_id != r#move.move_root_id {
                    // This event was caused by an ancestor of the node being
                    // moved. We should ignore it, unless it's about the root.
                    if r#move.node_id == self.root_id {
                        self.root_path = r#move.new_path.clone();
                        return Ok(Some(fs::DirectoryEvent::Move(r#move)));
                    } else {
                        return Ok(None);
                    }
                }

                if r#move.new_path.starts_with(&self.root_path) {
                    Some(fs::DirectoryEvent::Move(r#move))
                } else {
                    // The node was moved outside the root's subtree, which is
                    // effectively the same as it being deleted.

                    // We don't know if the node was a file or a directory, so
                    // try them both.
                    if self
                        .fs_streams
                        .files
                        .swap_remove(&r#move.node_id)
                        .is_some()
                    {
                        if let Some(buf_id) =
                            self.node_to_buf_ids.get(&r#move.node_id)
                        {
                            self.buffer_handles.remove(buf_id);
                        }
                    } else {
                        self.fs_streams
                            .directories
                            .swap_remove(&r#move.node_id);
                    }

                    Some(fs::DirectoryEvent::Deletion(fs::NodeDeletion {
                        node_id: r#move.node_id,
                        node_path: r#move.old_path,
                        deletion_root_id: r#move.move_root_id,
                    }))
                }
            },
        })
    }

    fn handle_cursor_event(
        &mut self,
        event: CursorEvent<B>,
    ) -> Option<CursorEvent<B>> {
        match &event.kind {
            CursorEventKind::Created(_) => self
                .buffer_handles
                .contains_key(&event.buffer_id)
                .then_some(event),

            CursorEventKind::Moved(_) => Some(event),

            CursorEventKind::Removed => {
                self.cursor_handles.remove(&event.cursor_id);
                Some(event)
            },
        }
    }

    async fn handle_new_buffer(
        &mut self,
        buffer_id: B::BufferId,
        ctx: &AsyncCtx<'_, B>,
    ) -> Result<bool, EventRxError<B, F>> {
        let Some(buffer_path) = ctx.with_ctx(|ctx| {
            ctx.buffer(buffer_id.clone()).map(|buf| buf.path().into_owned())
        }) else {
            return Ok(false);
        };

        if !buffer_path.starts_with(&self.root_path) {
            return Ok(false);
        }

        let Some(node) = ctx
            .fs()
            .node_at_path(buffer_path)
            .await
            .map_err(EventRxError::NodeAtPath)?
        else {
            return Ok(false);
        };

        if !self.should_watch(&node).await? {
            return Ok(false);
        }

        let FsNode::File(file) = node else { return Ok(false) };

        Ok(ctx.with_ctx(|ctx| {
            if let Some(buffer) = ctx.buffer(buffer_id) {
                self.watch_buffer(file.id(), &buffer);
                true
            } else {
                false
            }
        }))
    }

    /// Returns whether this `EventRx` should watch the given `FsNode`.
    ///
    /// # Panics
    ///
    /// Panics if the node is not in the root's subtree.
    async fn should_watch(
        &self,
        node: &FsNode<B::Fs>,
    ) -> Result<bool, EventRxError<B, F>> {
        debug_assert!(node.path().starts_with(&self.root_path));

        let Some(parent_path) = node.path().parent() else { return Ok(false) };
        let meta = node.meta();
        Ok(!meta.node_kind().is_symlink()
            && !self
                .project_filter
                .should_filter(parent_path, &meta)
                .await
                .map_err(EventRxError::FsFilter)?)
    }
}

impl<Fs: fs::Fs, State> EventStreamBuilder<Fs, State> {
    pub(crate) fn push_directory(&mut self, dir: &Fs::Directory) {
        self.fs_streams.watch_directory(dir);
    }

    pub(crate) fn push_file(&mut self, file: &Fs::File) {
        self.fs_streams.watch_file(file);
    }

    pub(crate) fn push_node(&mut self, node: &FsNode<Fs>) {
        self.fs_streams.watch_node(node);
    }
}

impl<Fs: fs::Fs> EventStreamBuilder<Fs, NeedsProjectFilter> {
    pub(crate) fn new(project_root: &Fs::Directory) -> Self {
        Self {
            fs_streams: Default::default(),
            root_id: project_root.id(),
            root_path: project_root.path().to_owned(),
            state: NeedsProjectFilter,
        }
    }

    pub(crate) fn push_filter<F: Filter<Fs>>(
        self,
        filter: F,
    ) -> EventStreamBuilder<Fs, Done<F>> {
        EventStreamBuilder {
            fs_streams: self.fs_streams,
            root_id: self.root_id,
            root_path: self.root_path,
            state: Done { filter },
        }
    }
}

impl<Fs: fs::Fs, F: Filter<Fs>> EventStreamBuilder<Fs, Done<F>> {
    pub(crate) fn build<B>(self, ctx: &mut AsyncCtx<B>) -> EventStream<B, F>
    where
        B: CollabBackend<Fs = Fs>,
    {
        let (buffer_tx, buffer_rx) = flume::unbounded();

        let also_buffer_tx = buffer_tx.clone();

        let new_buffers_handle = ctx.with_ctx(|ctx| {
            ctx.on_buffer_created(move |buf, _created_by| {
                let event =
                    BufferEvent::Created(buf.id(), buf.path().into_owned());
                let _ = also_buffer_tx.send(event);
            })
        });

        let (cursor_tx, cursor_rx) = flume::unbounded();

        let also_cursor_tx = cursor_tx.clone();

        let new_cursors_handle = ctx.with_ctx(|ctx| {
            ctx.on_cursor_created(move |cursor, _created_by| {
                let event = CursorEvent {
                    buffer_id: cursor.buffer_id(),
                    cursor_id: cursor.id(),
                    kind: CursorEventKind::Created(cursor.byte_offset()),
                };
                let _ = also_cursor_tx.send(event);
            })
        });

        EventStream {
            agent_id: ctx.new_agent_id(),
            buffer_handles: Default::default(),
            buffer_rx: buffer_rx.into_stream(),
            buffer_tx,
            new_buffers_handle,
            cursor_handles: Default::default(),
            cursor_rx: cursor_rx.into_stream(),
            cursor_tx,
            new_cursors_handle,
            fs_streams: self.fs_streams,
            project_filter: self.state.filter,
            node_to_buf_ids: Default::default(),
            root_id: self.root_id,
            root_path: self.root_path,
            saved_buffers: Default::default(),
        }
    }
}

impl<Fs: fs::Fs> FsStreams<Fs> {
    fn watch_directory(&mut self, dir: &Fs::Directory) {
        self.directories.insert(dir.id(), dir.watch());
    }

    fn watch_file(&mut self, file: &Fs::File) {
        self.files.insert(file.id(), file.watch());
    }

    fn watch_node(&mut self, node: &FsNode<Fs>) {
        match node {
            FsNode::Directory(dir) => self.watch_directory(dir),
            FsNode::File(file) => self.watch_file(file),
            FsNode::Symlink(_) => {},
        }
    }
}
