use ::serde::{Deserialize, Serialize};
use ed::backend::{AgentId, Backend, BaseBackend, Buffer};
use ed::fs::os::OsFs;
use ed::fs::{self, AbsPath, Fs};
use ed::notify::Namespace;
use ed::plugin::Plugin;
use ed::{BorrowState, Context, Shared};

use crate::buffer::{BufferId, NeovimBuffer, Point};
use crate::cursor::NeovimCursor;
use crate::events::{self, EventHandle, Events};
use crate::selection::NeovimSelection;
use crate::{api, executor, notify, oxi, serde, value};

/// TODO: docs.
pub struct Neovim {
    emitter: notify::NeovimEmitter,
    events: Shared<Events>,
    executor: executor::NeovimExecutor,
}

/// TODO: docs.
#[derive(Debug)]
pub struct CreateBufferError {
    inner: fs::ReadFileToStringError<OsFs>,
}

impl Neovim {
    /// TODO: docs.
    #[inline]
    pub fn init(augroup_name: &str) -> Self {
        Self {
            events: Shared::new(Events::new(augroup_name)),
            emitter: Default::default(),
            executor: Default::default(),
        }
    }

    /// TODO: docs.
    #[inline]
    pub fn set_emitter(&mut self, emitter: impl Into<notify::NeovimEmitter>) {
        self.emitter = emitter.into();
    }
}

impl Backend for Neovim {
    const REINSTATE_PANIC_HOOK: bool = false;

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

    type CreateBufferError = CreateBufferError;
    type SerializeError = serde::NeovimSerializeError;
    type DeserializeError = serde::NeovimDeserializeError;

    #[inline]
    fn buffer(&mut self, buf_id: Self::BufferId) -> Option<Self::Buffer<'_>> {
        buf_id.is_valid().then_some(NeovimBuffer::new(buf_id, &self.events))
    }

    #[inline]
    fn buffer_at_path(&mut self, path: &AbsPath) -> Option<Self::Buffer<'_>> {
        self.buffer_ids()
            .map(|buf_id| NeovimBuffer::new(buf_id, &self.events))
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
        Some(NeovimBuffer::current(&self.events))
    }

    #[inline]
    async fn create_buffer(
        file_path: &AbsPath,
        agent_id: AgentId,
        ctx: &mut Context<Self, impl BorrowState>,
    ) -> Result<Self::BufferId, Self::CreateBufferError> {
        <Self as BaseBackend>::create_buffer(file_path, agent_id, ctx).await
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
    fn on_selection_created<Fun>(&mut self, _fun: Fun) -> Self::EventHandle
    where
        Fun: FnMut(&Self::Selection<'_>, AgentId) + 'static,
    {
        todo!();
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

impl BaseBackend for Neovim {
    #[inline]
    async fn create_buffer(
        file_path: &AbsPath,
        agent_id: AgentId,
        ctx: &mut Context<impl AsMut<Self> + Backend, impl BorrowState>,
    ) -> Result<Self::BufferId, Self::CreateBufferError> {
        let contents = ctx
            .with_editor(|ed| ed.as_mut().fs())
            .read_to_string(file_path)
            .await
            .map_err(|inner| CreateBufferError { inner })?;

        let buf_id: BufferId = oxi::api::create_buf(true, false)
            .expect("couldn't create buf")
            .into();

        ctx.with_editor(|ed| {
            let this = ed.as_mut();

            this.events.with_mut(|events| {
                let ids = &mut events.agent_ids.created_buffer;
                let maybe_prev = ids.insert(buf_id, agent_id);
                debug_assert!(maybe_prev.is_none());
            });

            let buffer = NeovimBuffer::new(buf_id, &this.events);

            buffer.replace_text_in_point_range(
                Point::zero()..Point::zero(),
                &contents,
            );

            buffer.inner().set_name(file_path).expect("couldn't set name");

            // TODO: do we have to set the buffer's filename?
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
