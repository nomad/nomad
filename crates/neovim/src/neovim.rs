use ::serde::{Deserialize, Serialize};
use ed::fs::os::OsFs;
use ed::fs::{self, AbsPath, Fs};
use ed::notify::Namespace;
use ed::plugin::Plugin;
use ed::{AgentId, BaseEditor, BorrowState, Buffer, Context, Editor, Shared};

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
    buffers_state: BuffersState,
    emitter: notify::NeovimEmitter,
    events: Shared<Events>,
    executor: executor::NeovimExecutor,
    reinstate_panic_hook: bool,
}

/// TODO: docs.
#[derive(Debug)]
pub struct CreateBufferError {
    inner: fs::ReadFileToStringError<OsFs>,
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

        self.events.with_mut(|events| {
            if events.contains(&events::BufReadPost) {
                events.agent_ids.created_buffer.insert(buffer_id, agent_id);
            }
            if events.contains(&events::BufEnter) {
                events.agent_ids.focused_buffer.insert(buffer_id, agent_id);
            }
        });

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
    pub fn set_emitter(&mut self, emitter: impl Into<notify::NeovimEmitter>) {
        self.emitter = emitter.into();
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

    /// Same as [`buffer`](Self::buffer), but it doesn't need an exclusive
    /// reference.
    #[inline]
    fn buffer_inner(&self, buf_id: BufferId) -> Option<NeovimBuffer<'_>> {
        buf_id.is_valid().then_some(NeovimBuffer::new(
            buf_id,
            &self.events,
            &self.buffers_state,
        ))
    }

    #[inline]
    fn new_inner(augroup_name: &str, reinstate_panic_hook: bool) -> Self {
        let decoration_provider = DecorationProvider::new(augroup_name);
        let buffers_state = BuffersState::new(decoration_provider);
        Self {
            buffers_state: buffers_state.clone(),
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
    type Fs = fs::os::OsFs;
    type Emitter<'this> = &'this mut notify::NeovimEmitter;
    type Executor = executor::NeovimExecutor;
    type EventHandle = EventHandle;
    type Selection<'a> = NeovimSelection<'a>;
    type SelectionId = BufferId;

    type BufferSaveError = oxi::api::Error;
    type CreateBufferError = CreateBufferError;
    type SerializeError = serde::NeovimSerializeError;
    type DeserializeError = serde::NeovimDeserializeError;

    #[inline]
    fn buffer(&mut self, buf_id: Self::BufferId) -> Option<Self::Buffer<'_>> {
        self.buffer_inner(buf_id)
    }

    #[inline]
    fn buffer_at_path(&mut self, path: &AbsPath) -> Option<Self::Buffer<'_>> {
        self.buffer_ids()
            .flat_map(|buf_id| self.buffer_inner(buf_id))
            .find(|buf| &*buf.path() == path)
    }

    #[inline]
    fn buffer_ids(&mut self) -> impl Iterator<Item = BufferId> + use<> {
        oxi::api::list_bufs().filter(|buf| buf.is_loaded()).map(BufferId::new)
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
    async fn create_buffer(
        file_path: &AbsPath,
        agent_id: AgentId,
        ctx: &mut Context<Self, impl BorrowState>,
    ) -> Result<Self::BufferId, Self::CreateBufferError> {
        <Self as BaseEditor>::create_buffer(file_path, agent_id, ctx).await
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
    fn on_buffer_created<Fun>(&mut self, mut fun: Fun) -> Self::EventHandle
    where
        Fun: FnMut(&Self::Buffer<'_>, AgentId) + 'static,
    {
        Events::insert(
            self.events.clone(),
            events::BufReadPost,
            move |(buf, created_by)| fun(buf, created_by),
        )
    }

    #[inline]
    fn on_cursor_created<Fun>(&mut self, mut fun: Fun) -> Self::EventHandle
    where
        Fun: FnMut(&Self::Cursor<'_>, AgentId) + 'static,
    {
        Events::insert(
            self.events.clone(),
            events::BufEnter,
            move |(&buf, focused_by)| fun(&NeovimCursor::new(buf), focused_by),
        )
    }

    #[inline]
    fn on_selection_created<Fun>(&mut self, mut fun: Fun) -> Self::EventHandle
    where
        Fun: FnMut(&Self::Selection<'_>, AgentId) + 'static,
    {
        Events::insert(
            self.events.clone(),
            events::ModeChanged,
            move |(buf, old_mode, new_mode, changed_by)| {
                if new_mode.has_selected_range()
                    // A selection is only created if the old mode wasn't
                    // already displaying a selected range.
                    && !old_mode.has_selected_range()
                {
                    fun(&NeovimSelection::new(buf), changed_by);
                }
            },
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

impl BaseEditor for Neovim {
    #[inline]
    async fn create_buffer(
        file_path: &AbsPath,
        agent_id: AgentId,
        ctx: &mut Context<impl AsMut<Self> + Editor, impl BorrowState>,
    ) -> Result<Self::BufferId, Self::CreateBufferError> {
        let contents = match ctx
            .with_editor(|ed| ed.as_mut().fs())
            .read_file_to_string(file_path)
            .await
        {
            Ok(contents) => contents,

            Err(fs::ReadFileToStringError::ReadFile(
                fs::ReadFileError::NoNodeAtPath(_),
            )) => String::default(),

            Err(other) => return Err(CreateBufferError { inner: other }),
        };

        ctx.with_editor(|ed| {
            let this = ed.as_mut();

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
}

impl AsMut<Self> for Neovim {
    #[inline]
    fn as_mut(&mut self) -> &mut Self {
        self
    }
}

impl ed::notify::Error for CreateBufferError {
    fn to_message(&self) -> (ed::notify::Level, ed::notify::Message) {
        (
            ed::notify::Level::Error,
            ed::notify::Message::from_display(&self.inner),
        )
    }
}
