use core::mem;

use ::serde::{Deserialize, Serialize};
use abs_path::AbsPath;
use editor::notify::Namespace;
use editor::plugin::Plugin;
use editor::{AccessMut, AgentId, Buffer, Editor};
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
    /// TODO: docs.
    pub(crate) buffers_state: BuffersState,

    /// TODO: docs.
    pub(crate) decoration_provider: DecorationProvider,

    /// TODO: docs.
    pub(crate) events: Events,

    /// TODO: docs.
    emitter: notify::NeovimEmitter,

    /// TODO: docs.
    executor: executor::NeovimExecutor,

    /// TODO: docs.
    reinstate_panic_hook: bool,

    #[cfg(feature = "test")]
    pub(crate) scratch_buffer_count: u32,
}

impl Neovim {
    /// TODO: docs.
    #[inline]
    pub fn highlight_range<'a>(
        &'a mut self,
        handle: &'a HighlightRangeHandle,
    ) -> Option<HighlightRange<'a>> {
        self.buffer(handle.buffer_id())
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

    /// Should only be called by the `#[neovim::plugin]` macro.
    #[doc(hidden)]
    #[inline]
    pub fn new_plugin(augroup_name: &str) -> Self {
        Self::new_inner(augroup_name, false)
    }

    #[inline]
    pub(crate) fn create_buffer(
        &mut self,
        file_path: &AbsPath,
        agent_id: AgentId,
    ) -> NeovimBuffer<'_> {
        let mut buffer =
            oxi::api::create_buf(true, false).expect("couldn't create buffer");

        buffer.set_name(file_path).expect("couldn't set name");

        let buffer_id = BufferId::new(buffer);

        if self.events.contains(&events::BufReadPost) {
            self.events.agent_ids.created_buffer.insert(buffer_id, agent_id);
        }

        if self.events.contains(&events::BufEnter) {
            self.events.agent_ids.focused_buffer.insert(buffer_id, agent_id);
        }

        self.buffer(buffer_id).expect("just created the buffer")
    }

    #[cfg(feature = "test")]
    pub(crate) fn new_test(augroup_name: &str) -> Self {
        Self::new_inner(augroup_name, true)
    }

    #[inline]
    fn new_inner(augroup_name: &str, reinstate_panic_hook: bool) -> Self {
        Self {
            buffers_state: BuffersState::default(),
            decoration_provider: DecorationProvider::new(augroup_name),
            events: Events::new(augroup_name),
            emitter: Default::default(),
            executor: Default::default(),
            reinstate_panic_hook,
            #[cfg(feature = "test")]
            scratch_buffer_count: 0,
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
        NeovimBuffer::new(buf_id, self)
    }

    #[inline]
    fn buffer_at_path(&mut self, path: &AbsPath) -> Option<Self::Buffer<'_>> {
        for buf_id in oxi::api::list_bufs().map(BufferId::new) {
            let Some(buffer) = self.buffer(buf_id) else { continue };
            if &*buffer.path() == path {
                // SAFETY: Rust is dumb.
                let buffer = unsafe {
                    mem::transmute::<Self::Buffer<'_>, Self::Buffer<'_>>(
                        buffer,
                    )
                };
                return Some(buffer);
            }
        }
        None
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
            if let Some(buffer) = self.buffer(buf_id) {
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
            let mut buffer = this.create_buffer(file_path, agent_id);

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
        Fun: FnMut(&mut Self::Buffer<'_>, AgentId) + 'static,
    {
        self.events.insert(
            events::BufReadPost,
            move |(mut buf, created_by)| fun(&mut buf, created_by),
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
        Fun: FnMut(&mut Self::Cursor<'_>, AgentId) + 'static,
    {
        self.events.insert(
            events::BufEnter,
            move |(buf, focused_by)| {
                fun(&mut NeovimCursor::new(buf), focused_by)
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
        Fun: FnMut(&mut Self::Selection<'_>, AgentId) + 'static,
    {
        self.events.insert(
            events::ModeChanged,
            move |(buf, old_mode, new_mode, changed_by)| {
                if new_mode.has_selected_range()
                    // A selection is only created if the old mode wasn't
                    // already displaying a selected range.
                    && !old_mode.has_selected_range()
                {
                    fun(&mut NeovimSelection::new(buf), changed_by);
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
    fn remove_event(&mut self, event_handle: Self::EventHandle) {
        self.events.remove_event(event_handle);
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
