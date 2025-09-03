use std::collections::hash_map;

use abs_path::AbsPathBuf;
use editor::context::{Buffer, Context, Cursor, EventHandle, Selection};
use editor::{AgentId, Buffer as _, Cursor as _, Selection as _, Shared};
use either::Either;
use fs::filter::Filter;
use fs::{Directory, File, Fs};
use futures_util::future::FusedFuture;
use futures_util::select_biased;
use futures_util::stream::StreamExt;
use fxhash::{FxBuildHasher, FxHashMap, FxHashSet};
use rand::Rng;

use crate::editors::CollabEditor;
use crate::event::{self, Event};
use crate::list_ext::List;
use crate::start::{AllButOne, ProjectFilter};

type FxIndexMap<K, V> = indexmap::IndexMap<K, V, FxBuildHasher>;

/// TODO: docs.
pub struct EventStream<Ed: CollabEditor> {
    /// The [`AgentId`] of the `Session` that owns `Self`.
    agent_id: AgentId,
    /// Map from a file's node ID to the ID of the corresponding buffer.
    buf_id_of_file_id: FxHashMap<<Ed::Fs as Fs>::NodeId, Ed::BufferId>,
    /// Streams for buffer events.
    buffer_streams: BufferStreams<Ed>,
    /// Streams for cursor events.
    cursor_streams: CursorStreams<Ed>,
    /// Streams for directory events.
    dir_streams: DirectoryStreams<Ed::Fs>,
    /// Streams for file events.
    file_streams: FileStreams<Ed::Fs>,
    /// A filter used to check if [`fs::FsNode`]s created under the project
    /// root should be part of the project.
    project_filter: ProjectFilter<Ed>,
    /// The ID of the project root.
    root_id: <Ed::Fs as Fs>::NodeId,
    /// The path to the project root.
    root_path: AbsPathBuf,
    /// Streams for selection events.
    selection_streams: SelectionStreams<Ed>,
}

/// The type of error that can occur when [`EventStream::next`] fails.
#[derive(cauchy::Debug, derive_more::Display, cauchy::Error)]
#[display("{_0}")]
pub enum EventError<Ed: CollabEditor> {
    /// The project filter returned an error.
    Filter(<Ed::ProjectFilter as Filter<Ed::Fs>>::Error),

    /// We couldn't get the node at the given path.
    NodeAtPath(<Ed::Fs as Fs>::NodeAtPathError),
}

/// A builder for [`EventStream`]s.
///
/// Unlike the [`EventStream`] it'll be built into, this type is *not* generic
/// over any [`CollabEditor`], which allows it to be `Send`.
pub(crate) struct EventStreamBuilder<Fs: fs::Fs, State = NeedsProjectFilter> {
    dir_streams: DirectoryStreams<Fs>,
    file_streams: FileStreams<Fs>,
    root_id: Fs::NodeId,
    root_path: AbsPathBuf,
    state: State,
}

/// An [`EventStreamBuilder`] typestate indicating that it won't be possible
/// to call the [`build`](EventStreamBuilder::build) method until the user
/// provides a [`Filter`].
pub(crate) struct NeedsProjectFilter;

/// An [`EventStreamBuilder`] typestate indicating that it's ready to be built.
pub(crate) struct Done<F> {
    filter: F,
}

struct BufferStreams<Ed: CollabEditor> {
    /// The receiver of buffer events.
    event_rx: flume::r#async::RecvStream<'static, event::BufferEvent<Ed>>,

    /// The sender of buffer events.
    event_tx: flume::Sender<event::BufferEvent<Ed>>,

    /// Map from a buffer's ID to the event handles corresponding to the 3
    /// types of buffer events we're interested in: edits, removals, and saves.
    handles: FxHashMap<Ed::BufferId, [EventHandle<Ed>; 3]>,

    /// The event handle corresponding to buffer creations.
    #[allow(dead_code)]
    new_buffers_handle: EventHandle<Ed>,

    /// A set of buffer IDs for buffers that have just been saved.
    ///
    /// When we receive a [`fs::FileEvent::Modification`] event, we first check
    /// if its node ID maps to a buffer ID in this set. If it does, we know the
    /// event was caused by a text buffer being saved, and we can ignore it.
    saved_buffers: Shared<FxHashSet<Ed::BufferId>>,
}

struct CursorStreams<Ed: CollabEditor> {
    /// The receiver of cursor events.
    event_rx: flume::r#async::RecvStream<'static, event::CursorEvent<Ed>>,

    /// The sender of cursor events.
    event_tx: flume::Sender<event::CursorEvent<Ed>>,

    /// Map from a cursor's ID to the event handles corresponding to the 2
    /// types of cursor events we're interested in: moves and removals.
    handles: FxHashMap<Ed::CursorId, [EventHandle<Ed>; 2]>,

    /// The event handle corresponding to cursor creations.
    #[allow(dead_code)]
    new_cursors_handle: EventHandle<Ed>,
}

#[derive(cauchy::Default)]
struct DirectoryStreams<Fs: fs::Fs> {
    /// Map from a directory's node ID to its event stream.
    inner: FxIndexMap<Fs::NodeId, <Fs::Directory as Directory>::EventStream>,
}

#[derive(cauchy::Default)]
struct FileStreams<Fs: fs::Fs> {
    /// Map from a file's node ID to its event stream.
    inner: FxIndexMap<Fs::NodeId, <Fs::File as File>::EventStream>,
}

struct SelectionStreams<Ed: CollabEditor> {
    /// The receiver of selection events.
    event_rx: flume::r#async::RecvStream<'static, event::SelectionEvent<Ed>>,

    /// The sender of selection events.
    event_tx: flume::Sender<event::SelectionEvent<Ed>>,

    /// Map from a selection's ID to the event handles corresponding to the 2
    /// types of selection events we're interested in: moves and removals.
    handles: FxHashMap<Ed::SelectionId, [EventHandle<Ed>; 2]>,

    /// The event handle corresponding to selection creations.
    #[allow(dead_code)]
    new_selections_handle: EventHandle<Ed>,
}

impl<Ed: CollabEditor> EventStream<Ed> {
    pub(crate) fn agent_id(&self) -> AgentId {
        self.agent_id
    }

    /// TODO: docs.
    pub async fn next(
        &mut self,
        ctx: &mut Context<Ed>,
    ) -> Result<Event<Ed>, EventError<Ed>> {
        loop {
            let seed = ctx.with_rng(Rng::random);
            let mut dir_streams = self.dir_streams.inner.as_stream(seed);
            let mut file_streams = self.file_streams.inner.as_stream(seed);

            return Ok(select_biased! {
                buffer_event = self.buffer_streams.select_next_some() => {
                    match self.handle_buffer_event(buffer_event, ctx).await? {
                        Some(event) => Event::Buffer(event),
                        None => continue,
                    }
                },
                cursor_event = self.cursor_streams.select_next_some() => {
                    match self.handle_cursor_event(cursor_event, ctx) {
                        Some(event) => Event::Cursor(event),
                        None => continue,
                    }
                },
                dir_event = dir_streams.select_next_some() => {
                    match self.handle_dir_event(dir_event, ctx).await? {
                        Some(event) => Event::Directory(event),
                        None => continue,
                    }
                },
                file_event = file_streams.select_next_some() => {
                    match self.handle_file_event(file_event) {
                        Some(event) => Event::File(event),
                        None => continue,
                    }
                },
                selection_event = self.selection_streams.select_next_some() => {
                    match self.handle_selection_event(selection_event, ctx) {
                        Some(event) => Event::Selection(event),
                        None => continue,
                    }
                },
            });
        }
    }

    pub(crate) fn watch_buffer(
        &mut self,
        buffer: &mut Buffer<'_, Ed>,
        file_id: <Ed::Fs as fs::Fs>::NodeId,
    ) {
        self.buf_id_of_file_id.insert(file_id, buffer.id());
        self.buffer_streams.insert(buffer, self.agent_id);
    }

    pub(crate) fn watch_cursor(&mut self, cursor: &mut Cursor<'_, Ed>) {
        self.cursor_streams.insert(cursor);
    }

    pub(crate) fn watch_selection(
        &mut self,
        selection: &mut Selection<'_, Ed>,
    ) {
        self.selection_streams.insert(selection);
    }

    async fn handle_buffer_event(
        &mut self,
        event: event::BufferEvent<Ed>,
        ctx: &mut Context<Ed>,
    ) -> Result<Option<event::BufferEvent<Ed>>, EventError<Ed>> {
        match &event {
            event::BufferEvent::Created(buffer_id, _) => {
                let Some(buffer_path) = ctx.with_borrowed(|ctx| {
                    let buf = ctx.buffer(buffer_id.clone())?;
                    Some(buf.path().into_owned())
                }) else {
                    return Ok(None);
                };

                if !buffer_path.starts_with(&self.root_path) {
                    return Ok(None);
                }

                let Some(node) = ctx
                    .fs()
                    .node_at_path(buffer_path)
                    .await
                    .map_err(EventError::NodeAtPath)?
                else {
                    return Ok(None);
                };

                if !self.should_watch_node(&node).await? {
                    return Ok(None);
                }

                let fs::Node::File(file) = node else { return Ok(None) };

                let is_watched = ctx.with_borrowed(|ctx| {
                    if let Some(mut buffer) = ctx.buffer(buffer_id.clone()) {
                        self.watch_buffer(&mut buffer, file.id());
                        true
                    } else {
                        false
                    }
                });

                if !is_watched {
                    return Ok(None);
                }
            },

            event::BufferEvent::Removed(buffer_id) => {
                self.buffer_streams.remove(buffer_id);
            },

            _ => {},
        }

        Ok(Some(event))
    }

    fn handle_cursor_event(
        &mut self,
        mut event: event::CursorEvent<Ed>,
        ctx: &mut Context<Ed>,
    ) -> Option<event::CursorEvent<Ed>> {
        match &mut event.kind {
            event::CursorEventKind::Created(buffer_id, offset) => {
                // We only care about cursors in buffers that are part of the
                // project.
                if !self.buffer_streams.is_watched(buffer_id) {
                    return None;
                }

                ctx.with_borrowed(|ctx| {
                    if let Some(mut cursor) =
                        ctx.cursor(event.cursor_id.clone())
                    {
                        // The cursor's position may have changed since the
                        // event was sent, so update the event's offset to the
                        // current one.
                        *offset = cursor.byte_offset();
                        self.watch_cursor(&mut cursor);
                    }
                });
            },
            event::CursorEventKind::Moved(_) => (),
            event::CursorEventKind::Removed => {
                self.cursor_streams.remove(&event.cursor_id);
            },
        }

        Some(event)
    }

    #[allow(clippy::too_many_lines)]
    async fn handle_dir_event(
        &mut self,
        event: fs::DirectoryEvent<Ed::Fs>,
        ctx: &mut Context<Ed>,
    ) -> Result<Option<fs::DirectoryEvent<Ed::Fs>>, EventError<Ed>> {
        Ok(match event {
            fs::DirectoryEvent::Creation(ref creation) => {
                let Some(node) = ctx
                    .fs()
                    .node_at_path(&creation.node_path)
                    .await
                    .map_err(EventError::NodeAtPath)?
                else {
                    // The node must've already been deleted.
                    return Ok(None);
                };

                if self.should_watch_node(&node).await? {
                    self.watch_node(&node, ctx);
                    Some(event)
                } else {
                    None
                }
            },

            fs::DirectoryEvent::Deletion(ref deletion) => {
                if let Some(buf_id) =
                    self.buf_id_of_file_id.get(&deletion.node_id)
                {
                    self.buffer_streams.remove(buf_id);
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
                    if self.file_streams.remove(&r#move.node_id) {
                        if let Some(buf_id) =
                            self.buf_id_of_file_id.get(&r#move.node_id)
                        {
                            self.buffer_streams.remove(buf_id);
                        }
                    } else {
                        self.dir_streams.remove(&r#move.node_id);
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

    fn handle_file_event(
        &self,
        event: fs::FileEvent<Ed::Fs>,
    ) -> Option<fs::FileEvent<Ed::Fs>> {
        if let fs::FileEvent::Modification(modif) = &event
            && let Some(buf_id) = self.buf_id_of_file_id.get(&modif.file_id)
            && self.buffer_streams.has_buffer_been_saved(buf_id)
        {
            None
        } else {
            Some(event)
        }
    }

    fn handle_selection_event(
        &mut self,
        mut event: event::SelectionEvent<Ed>,
        ctx: &mut Context<Ed>,
    ) -> Option<event::SelectionEvent<Ed>> {
        match &mut event.kind {
            event::SelectionEventKind::Created(buffer_id, range) => {
                // We only care about selections in buffers that are part of
                // the project.
                if self.buffer_streams.is_watched(buffer_id) {
                    return None;
                }

                ctx.with_borrowed(|ctx| {
                    if let Some(mut selection) =
                        ctx.selection(event.selection_id.clone())
                    {
                        // The selected range may have changed since the event
                        // was sent, so update the event's range to the
                        // current one.
                        *range = selection.byte_range();
                        self.watch_selection(&mut selection);
                    }
                });
            },
            event::SelectionEventKind::Moved(_) => (),
            event::SelectionEventKind::Removed => {
                self.selection_streams.remove(&event.selection_id);
            },
        }

        Some(event)
    }

    /// Returns whether the given node should be watched.
    ///
    /// # Panics
    ///
    /// Panics if the node is not in the root's subtree.
    async fn should_watch_node(
        &self,
        node: &fs::Node<Ed::Fs>,
    ) -> Result<bool, EventError<Ed>> {
        debug_assert!(node.path().starts_with(&self.root_path));

        let Some(parent_path) = node.path().parent() else { return Ok(false) };

        self.project_filter
            .should_filter(parent_path, &node.meta())
            .await
            .map(|should_filter| !should_filter)
            .map_err(|err| match err {
                Either::Left(err) => EventError::Filter(err),
            })
    }

    fn watch_node(&mut self, node: &fs::Node<Ed::Fs>, ctx: &mut Context<Ed>) {
        match node {
            fs::Node::Directory(dir) => self.dir_streams.insert(dir),
            fs::Node::File(file) => {
                self.file_streams.insert(file);
                ctx.with_borrowed(|ctx| {
                    if let Some(mut buffer) = ctx.buffer_at_path(file.path()) {
                        self.watch_buffer(&mut buffer, file.id());
                    }
                });
            },
            fs::Node::Symlink(_) => {},
        }
    }
}

impl<Fs: fs::Fs, State> EventStreamBuilder<Fs, State> {
    pub(crate) fn push_directory(&mut self, dir: &Fs::Directory) {
        self.dir_streams.insert(dir);
    }

    pub(crate) fn push_file(&mut self, file: &Fs::File) {
        self.file_streams.insert(file);
    }
}

impl<Fs: fs::Fs> EventStreamBuilder<Fs, NeedsProjectFilter> {
    pub(crate) fn new(project_root: &Fs::Directory) -> Self {
        Self {
            dir_streams: Default::default(),
            file_streams: Default::default(),
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
            dir_streams: self.dir_streams,
            file_streams: self.file_streams,
            root_id: self.root_id,
            root_path: self.root_path,
            state: Done { filter },
        }
    }
}

impl<Fs, Fi> EventStreamBuilder<Fs, Done<Either<Fi, AllButOne<Fs>>>>
where
    Fs: fs::Fs,
    Fi: Filter<Fs>,
{
    pub(crate) fn build<Ed>(self, ctx: &mut Context<Ed>) -> EventStream<Ed>
    where
        Ed: CollabEditor<Fs = Fs, ProjectFilter = Fi>,
    {
        EventStream {
            agent_id: ctx.new_agent_id(),
            buffer_streams: BufferStreams::new(ctx),
            cursor_streams: CursorStreams::new(ctx),
            dir_streams: self.dir_streams,
            file_streams: self.file_streams,
            selection_streams: SelectionStreams::new(ctx),
            project_filter: self.state.filter,
            buf_id_of_file_id: Default::default(),
            root_id: self.root_id,
            root_path: self.root_path,
        }
    }
}

impl<Ed: CollabEditor> BufferStreams<Ed> {
    /// Starts receiving [`event::BufferEvent`]s on the given buffer.
    fn insert(&mut self, buffer: &mut Buffer<'_, Ed>, agent_id: AgentId) {
        let buffer_handles = match self.handles.entry(buffer.id()) {
            hash_map::Entry::Occupied(_) => {
                panic!(
                    "already receiving events for buffer at {:?}",
                    buffer.path()
                );
            },
            hash_map::Entry::Vacant(vacant) => vacant,
        };

        let event_tx = self.event_tx.clone();
        let edits_handle = buffer.on_edited(move |buf, edit| {
            if edit.made_by == agent_id {
                return;
            }
            let _ = event_tx.send(event::BufferEvent::Edited(
                buf.id(),
                edit.replacements.clone(),
            ));
        });

        let event_tx = self.event_tx.clone();
        let removed_handle = buffer.on_removed(move |buf_id, _removed_by| {
            let _ = event_tx.send(event::BufferEvent::Removed(buf_id));
        });

        let event_tx = self.event_tx.clone();
        let saved_buffers = self.saved_buffers.clone();
        let saved_handle = buffer.on_saved(move |buf, saved_by| {
            saved_buffers.with_mut(|buffers| buffers.insert(buf.id()));
            if saved_by != agent_id {
                let _ = event_tx.send(event::BufferEvent::Saved(buf.id()));
            }
        });

        buffer_handles.insert([edits_handle, removed_handle, saved_handle]);
    }

    /// Returns whether the buffer with the given ID is currently being
    /// watched.
    fn is_watched(&self, buffer_id: &Ed::BufferId) -> bool {
        self.handles.contains_key(buffer_id)
    }

    /// Returns whether the buffer with the given ID has just been saved.
    fn has_buffer_been_saved(&self, buffer_id: &Ed::BufferId) -> bool {
        self.saved_buffers.with_mut(|buffer_ids| buffer_ids.remove(buffer_id))
    }

    fn new(ctx: &mut Context<Ed>) -> Self {
        let (event_tx, event_rx) = flume::unbounded();

        let new_buffers_handle = {
            let event_tx = event_tx.clone();
            ctx.on_buffer_created(move |buf, _created_by| {
                let _ = event_tx.send(event::BufferEvent::Created(
                    buf.id(),
                    buf.path().into_owned(),
                ));
            })
        };

        Self {
            event_rx: event_rx.into_stream(),
            event_tx,
            handles: Default::default(),
            new_buffers_handle,
            saved_buffers: Default::default(),
        }
    }

    /// Removes the event handles corresponding to the buffer with the given
    /// ID.
    fn remove(&mut self, buffer_id: &Ed::BufferId) {
        self.handles.remove(buffer_id);
    }

    fn select_next_some(
        &mut self,
    ) -> impl FusedFuture<Output = event::BufferEvent<Ed>> {
        StreamExt::select_next_some(&mut self.event_rx)
    }
}

impl<Ed: CollabEditor> CursorStreams<Ed> {
    /// Starts receiving [`event::CursorEvent`]s on the given cursor.
    fn insert(&mut self, cursor: &mut Cursor<'_, Ed>) {
        let cursor_handles = match self.handles.entry(cursor.id()) {
            hash_map::Entry::Occupied(_) => {
                panic!("already receiving cursor events for {:?}", cursor.id())
            },
            hash_map::Entry::Vacant(vacant) => vacant,
        };

        let event_tx = self.event_tx.clone();
        let moved_handle = cursor.on_moved(move |cursor, _moved_by| {
            let _ = event_tx.send(event::CursorEvent {
                cursor_id: cursor.id(),
                kind: event::CursorEventKind::Moved(cursor.byte_offset()),
            });
        });

        let event_tx = self.event_tx.clone();
        let removed_handle =
            cursor.on_removed(move |cursor_id, _removed_by| {
                let event = event::CursorEvent {
                    cursor_id,
                    kind: event::CursorEventKind::Removed,
                };
                let _ = event_tx.send(event);
            });

        cursor_handles.insert([moved_handle, removed_handle]);
    }

    fn new(ctx: &mut Context<Ed>) -> Self {
        let (event_tx, event_rx) = flume::unbounded();

        let new_cursors_handle = {
            let event_tx = event_tx.clone();
            ctx.on_cursor_created(move |cursor, _created_by| {
                let _ = event_tx.send(event::CursorEvent {
                    cursor_id: cursor.id(),
                    kind: event::CursorEventKind::Created(
                        cursor.buffer_id(),
                        cursor.byte_offset(),
                    ),
                });
            })
        };

        Self {
            event_rx: event_rx.into_stream(),
            event_tx,
            handles: Default::default(),
            new_cursors_handle,
        }
    }

    /// Removes the event handles corresponding to the cursor with the given
    /// ID.
    fn remove(&mut self, cursor_id: &Ed::CursorId) {
        self.handles.remove(cursor_id);
    }

    fn select_next_some(
        &mut self,
    ) -> impl FusedFuture<Output = event::CursorEvent<Ed>> {
        StreamExt::select_next_some(&mut self.event_rx)
    }
}

impl<Fs: fs::Fs> DirectoryStreams<Fs> {
    /// Starts receiving [`fs::DirectoryEvent`]s on the given dir.
    fn insert(&mut self, dir: &Fs::Directory) {
        self.inner.insert(dir.id(), dir.watch());
    }

    /// Removes the event stream corresponding to the directory with the given
    /// ID.
    fn remove(&mut self, dir_id: &Fs::NodeId) {
        self.inner.swap_remove(dir_id);
    }
}

impl<Fs: fs::Fs> FileStreams<Fs> {
    /// Starts receiving [`fs::FileEvent`]s on the given file.
    fn insert(&mut self, file: &Fs::File) {
        self.inner.insert(file.id(), file.watch());
    }

    /// Removes the event stream corresponding to the file with the given ID,
    /// returning whether it was present.
    fn remove(&mut self, file_id: &Fs::NodeId) -> bool {
        self.inner.swap_remove(file_id).is_some()
    }
}

impl<Ed: CollabEditor> SelectionStreams<Ed> {
    /// Starts receiving [`event::SelectionEvent`]s on the given selection.
    fn insert(&mut self, selection: &mut Selection<'_, Ed>) {
        let selection_handles = match self.handles.entry(selection.id()) {
            hash_map::Entry::Occupied(_) => {
                panic!(
                    "already receiving selection events for {:?}",
                    selection.id()
                );
            },
            hash_map::Entry::Vacant(vacant) => vacant,
        };

        let event_tx = self.event_tx.clone();
        let moved_handle = selection.on_moved(move |selection, _moved_by| {
            let _ = event_tx.send(event::SelectionEvent {
                selection_id: selection.id(),
                kind: event::SelectionEventKind::Moved(selection.byte_range()),
            });
        });

        let event_tx = self.event_tx.clone();
        let removed_handle =
            selection.on_removed(move |selection_id, _removed_by| {
                let event = event::SelectionEvent {
                    selection_id,
                    kind: event::SelectionEventKind::Removed,
                };
                let _ = event_tx.send(event);
            });

        selection_handles.insert([moved_handle, removed_handle]);
    }

    fn new(ctx: &mut Context<Ed>) -> Self {
        let (event_tx, event_rx) = flume::unbounded();

        let new_selections_handle = {
            let event_tx = event_tx.clone();
            ctx.on_selection_created(move |selection, _created_by| {
                let _ = event_tx.send(event::SelectionEvent {
                    selection_id: selection.id(),
                    kind: event::SelectionEventKind::Created(
                        selection.buffer_id(),
                        selection.byte_range(),
                    ),
                });
            })
        };

        Self {
            event_rx: event_rx.into_stream(),
            event_tx,
            handles: Default::default(),
            new_selections_handle,
        }
    }

    /// Removes the event handles corresponding to the selection with the given
    /// ID.
    fn remove(&mut self, selection_id: &Ed::SelectionId) {
        self.handles.remove(selection_id);
    }

    fn select_next_some(
        &mut self,
    ) -> impl FusedFuture<Output = event::SelectionEvent<Ed>> {
        StreamExt::select_next_some(&mut self.event_rx)
    }
}
