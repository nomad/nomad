use ::serde::{Deserialize, Serialize};
use abs_path::AbsPath;
use editor::notify::Namespace;
use editor::plugin::Plugin;
use editor::{AccessMut, AgentId, Buffer, Editor, Shared};
use fs::Fs;

use crate::buffer::{
    BufferId,
    BuffersState,
    HighlightRange,
    HighlightRangeHandle,
    NeovimBuffer,
    Point,
};
use crate::cursor::NeovimCursor;
use crate::decoration_provider::DecorationProvider;
use crate::events::{self, EventHandle, Events};
use crate::selection::NeovimSelection;
use crate::{api, executor, notify, oxi, serde, value};

/// TODO: docs.
pub struct Neovim {
    pub(crate) buffers_state: BuffersState,
    emitter: notify::NeovimEmitter,
    pub(crate) events: Shared<Events>,
    pub(crate) events2: Events,
    executor: executor::NeovimExecutor,
    reinstate_panic_hook: bool,
}

impl Neovim {
    /// Same as [`oxi::api::create_buf`], but keeps track of the [`AgentId`]
    /// that created the buffer.
    #[inline]
    pub fn create_buf(
        &mut self,
        is_listed: bool,
        is_scratch: bool,
        agent_id: AgentId,
    ) -> BufferId {
        let buffer_id = oxi::api::create_buf(is_listed, is_scratch)
            .expect("couldn't create buffer")
            .into();

        if self.events2.contains(&events::BufReadPost) {
            self.events2.agent_ids.created_buffer.insert(buffer_id, agent_id);
        }

        if self.events2.contains(&events::BufEnter) {
            self.events2.agent_ids.focused_buffer.insert(buffer_id, agent_id);
        }

        buffer_id
    }

    /// TODO: docs.
    #[inline]
    pub fn highlight_range<'a>(
        &'a self,
        handle: &'a HighlightRangeHandle,
    ) -> Option<HighlightRange<'a>> {
        self.buffer_inner(handle.buffer_id())
            .map(|buffer| HighlightRange::new(buffer, handle))
    }

    /// TODO: docs.
    #[inline]
    pub fn set_notifier(&mut self, emitter: impl Into<notify::NeovimEmitter>) {
        self.emitter = emitter.into();
    }

    /// Returns a new instance of the [`TracingLayer`](crate::TracingLayer).
    #[cfg(feature = "tracing")]
    #[inline]
    pub fn tracing_layer<S>(&mut self) -> crate::TracingLayer<S> {
        crate::TracingLayer::new(self)
    }

    /// Same as [`buffer`](Self::buffer), but it doesn't need an exclusive
    /// reference.
    #[inline]
    pub(crate) fn buffer_inner(
        &self,
        buf_id: BufferId,
    ) -> Option<NeovimBuffer<'_>> {
        NeovimBuffer::new(buf_id, &self.events, &self.buffers_state)
    }

    /// Should only be called by the `#[neovim::plugin]` macro.
    #[doc(hidden)]
    #[inline]
    pub fn new_plugin(augroup_name: &str) -> Self {
        Self::new_inner(augroup_name, false)
    }

    #[cfg(feature = "test")]
    pub(crate) fn new_test(augroup_name: &str) -> Self {
        Self::new_inner(augroup_name, true)
    }

    #[inline]
    fn new_inner(augroup_name: &str, reinstate_panic_hook: bool) -> Self {
        let decoration_provider = DecorationProvider::new(augroup_name);
        let buffers_state = BuffersState::new(decoration_provider);
        Self {
            buffers_state: buffers_state.clone(),
            events2: Events::new(augroup_name, buffers_state.clone()),
            events: Shared::new(Events::new(augroup_name, buffers_state)),
            emitter: Default::default(),
            executor: Default::default(),
            reinstate_panic_hook,
        }
    }
}

impl Editor for Neovim {
    type Api = api::NeovimApi;
    type Buffer<'a> = NeovimBuffer<'a>;
    type BufferId = BufferId;
    type Cursor<'a> = NeovimCursor<'a>;
    type CursorId = BufferId;
    type Fs = real_fs::RealFs;
    type Emitter<'this> = &'this mut notify::NeovimEmitter;
    type Executor = executor::NeovimExecutor;
    type EventHandle = EventHandle;
    type Selection<'a> = NeovimSelection<'a>;
    type SelectionId = BufferId;

    type BufferSaveError = oxi::api::Error;
    type CreateBufferError = fs::ReadFileToStringError<real_fs::RealFs>;
    type SerializeError = serde::NeovimSerializeError;
    type DeserializeError = serde::NeovimDeserializeError;

    #[inline]
    fn buffer(&mut self, buf_id: Self::BufferId) -> Option<Self::Buffer<'_>> {
        self.buffer_inner(buf_id)
    }

    #[inline]
    fn buffer_at_path(&mut self, path: &AbsPath) -> Option<Self::Buffer<'_>> {
        oxi::api::list_bufs().find_map(|buf| {
            let id = BufferId::new(buf);
            let buffer = self.buffer_inner(id)?;
            (&*buffer.path() == path).then_some(buffer)
        })
    }

    #[inline]
    fn fs(&mut self) -> Self::Fs {
        Self::Fs::default()
    }

    #[inline]
    fn current_buffer(&mut self) -> Option<Self::Buffer<'_>> {
        self.buffer(BufferId::of_focused())
    }

    #[inline]
    fn for_each_buffer<Fun>(&mut self, mut fun: Fun)
    where
        Fun: FnMut(Self::Buffer<'_>),
    {
        for buf_id in oxi::api::list_bufs().map(BufferId::new) {
            if let Some(buffer) = self.buffer_inner(buf_id) {
                fun(buffer);
            }
        }
    }

    #[inline]
    async fn create_buffer(
        mut this: impl AccessMut<Self>,
        file_path: &AbsPath,
        agent_id: AgentId,
    ) -> Result<Self::BufferId, Self::CreateBufferError> {
        let contents = match this
            .with_mut(|this| this.fs())
            .read_file_to_string(file_path)
            .await
        {
            Ok(contents) => contents,

            Err(fs::ReadFileToStringError::ReadFile(
                fs::ReadFileError::NoNodeAtPath(_),
            )) => String::default(),

            Err(other) => return Err(other),
        };

        this.with_mut(|this| {
            let buf_id = this.create_buf(true, false, agent_id);

            let buffer = this.buffer_inner(buf_id).expect("just created");

            // 'eol' is turned on by default, so avoid inserting the file's
            // trailing newline or we'll get an extra line.
            let contents = if contents.ends_with('\n') {
                &contents[..contents.len() - 1]
            } else {
                &contents
            };

            if !contents.is_empty() {
                buffer.replace_text_in_point_range(
                    Point::zero()..Point::zero(),
                    contents,
                    agent_id,
                );
            }

            buffer.inner().set_name(file_path).expect("couldn't set name");

            // TODO: do we have to set the buffer's filetype?
            // vim.filetype.match(buffer.inner())

            Ok(buffer.id())
        })
    }

    #[inline]
    fn cursor(&mut self, buf_id: Self::CursorId) -> Option<Self::Cursor<'_>> {
        let buffer = self.buffer(buf_id)?;
        buffer.is_focused().then_some(NeovimCursor::new(buffer))
    }

    #[inline]
    fn emitter(&mut self) -> Self::Emitter<'_> {
        &mut self.emitter
    }

    #[inline]
    fn executor(&mut self) -> &mut Self::Executor {
        &mut self.executor
    }

    #[inline]
    fn selection(
        &mut self,
        buf_id: Self::SelectionId,
    ) -> Option<Self::Selection<'_>> {
        let buffer = self.buffer(buf_id)?;
        buffer.selection().is_some().then_some(NeovimSelection::new(buffer))
    }

    #[inline]
    fn serialize<T: ?Sized + Serialize>(
        &mut self,
        value: &T,
    ) -> Result<value::NeovimValue, serde::NeovimSerializeError> {
        serde::serialize(value)
    }

    #[inline]
    fn deserialize<'de, T: Deserialize<'de>>(
        &mut self,
        value: value::NeovimValue,
    ) -> Result<T, serde::NeovimDeserializeError> {
        serde::deserialize(value)
    }

    #[inline]
    fn on_buffer_created<Fun>(
        &mut self,
        mut fun: Fun,
        this: impl AccessMut<Self> + Clone + 'static,
    ) -> Self::EventHandle
    where
        Fun: FnMut(&Self::Buffer<'_>, AgentId) + 'static,
    {
        self.events2.insert2(
            events::BufReadPost,
            move |(buf, created_by)| fun(buf, created_by),
            this,
        )
    }

    #[inline]
    fn on_cursor_created<Fun>(
        &mut self,
        mut fun: Fun,
        this: impl AccessMut<Self> + Clone + 'static,
    ) -> Self::EventHandle
    where
        Fun: FnMut(&Self::Cursor<'_>, AgentId) + 'static,
    {
        self.events2.insert2(
            events::BufEnter,
            move |(buf, focused_by)| {
                fun(&NeovimCursor::new(buf.clone()), focused_by)
            },
            this,
        )
    }

    #[inline]
    fn on_selection_created<Fun>(
        &mut self,
        mut fun: Fun,
        this: impl AccessMut<Self> + Clone + 'static,
    ) -> Self::EventHandle
    where
        Fun: FnMut(&Self::Selection<'_>, AgentId) + 'static,
    {
        self.events2.insert2(
            events::ModeChanged,
            move |(buf, old_mode, new_mode, changed_by)| {
                if new_mode.has_selected_range()
                    // A selection is only created if the old mode wasn't
                    // already displaying a selected range.
                    && !old_mode.has_selected_range()
                {
                    fun(&NeovimSelection::new(buf.clone()), changed_by);
                }
            },
            this,
        )
    }

    #[inline]
    fn reinstate_panic_hook(&self) -> bool {
        self.reinstate_panic_hook
    }

    #[inline]
    fn emit_deserialize_error_in_config<P: Plugin<Self>>(
        &mut self,
        config_path: &Namespace,
        namespace: &Namespace,
        mut err: Self::DeserializeError,
    ) {
        err.set_config_path(config_path.clone());
        self.emit_err(namespace, err);
    }
}
