use abs_path::AbsPath;
use ed::backend::{AgentId, ApiValue, Backend, BaseBackend, Edit};
use ed::executor::BackgroundSpawner;
use ed::notify::{self, MaybeResult};
use ed::shared::Shared;
use ed::{BorrowState, Context, fs};
use fxhash::FxHashMap;
use serde::{Deserialize, Serialize};
use slotmap::{DefaultKey, SlotMap};

use crate::api::Api;
use crate::buffer::{
    Buffer,
    BufferId,
    BufferInner,
    Cursor,
    CursorId,
    Selection,
    SelectionId,
};
use crate::emitter::Emitter;
use crate::executor::{Executor, Spawner};
use crate::fs::MockFs;
use crate::serde::{DeserializeError, SerializeError};

/// TODO: docs.
pub struct Mock<Fs = MockFs, BgSpawner = Spawner> {
    buffers: FxHashMap<BufferId, BufferInner>,
    callbacks: Callbacks,
    current_buffer: Option<BufferId>,
    emitter: Emitter,
    executor: Executor<BgSpawner>,
    fs: Fs,
    next_buffer_id: BufferId,
}

pub struct EventHandle {
    key: slotmap::DefaultKey,
    callbacks: Callbacks,
}

#[derive(cauchy::Debug, cauchy::PartialEq, cauchy::Eq)]
pub struct CreateBufferError<Fs: fs::Fs> {
    inner: fs::ReadFileToStringError<Fs>,
}

#[derive(Default)]
pub(crate) struct Callbacks {
    inner: Shared<SlotMap<DefaultKey, CallbackKind>>,
}

#[allow(clippy::type_complexity)]
#[allow(dead_code)]
pub(crate) enum CallbackKind {
    BufferCreated(Box<dyn FnMut(&Buffer<'_>, AgentId) + 'static>),
    BufferEdited(BufferId, Box<dyn FnMut(&Buffer<'_>, &Edit) + 'static>),
    #[allow(dead_code)]
    BufferRemoved(BufferId, Box<dyn FnMut(&Buffer<'_>, AgentId) + 'static>),
    #[allow(dead_code)]
    BufferSaved(BufferId, Box<dyn FnMut(&Buffer<'_>, AgentId) + 'static>),
    #[allow(dead_code)]
    CursorCreated(Box<dyn FnMut(&Cursor<'_>, AgentId) + 'static>),
    CursorMoved(CursorId, Box<dyn FnMut(&Cursor<'_>, AgentId) + 'static>),
    #[allow(dead_code)]
    CursorRemoved(CursorId, Box<dyn FnMut(&Cursor<'_>, AgentId) + 'static>),
    #[allow(dead_code)]
    SelectionCreated(Box<dyn FnMut(&Selection<'_>, AgentId) + 'static>),
    #[allow(dead_code)]
    SelectionMoved(
        SelectionId,
        Box<dyn FnMut(&Selection<'_>, AgentId) + 'static>,
    ),
    #[allow(dead_code)]
    SelectionRemoved(
        SelectionId,
        Box<dyn FnMut(&Selection<'_>, AgentId) + 'static>,
    ),
}

impl<Fs> Mock<Fs> {
    pub fn new(fs: Fs) -> Self {
        Self {
            buffers: Default::default(),
            callbacks: Default::default(),
            current_buffer: None,
            emitter: Default::default(),
            executor: Default::default(),
            fs,
            next_buffer_id: BufferId(1),
        }
    }
}

impl<Fs, BgSpawner> Mock<Fs, BgSpawner>
where
    Fs: fs::Fs,
    BgSpawner: BackgroundSpawner,
{
    pub fn with_background_spawner<NewBgSpawner>(
        self,
        spawner: NewBgSpawner,
    ) -> Mock<Fs, NewBgSpawner> {
        Mock {
            buffers: self.buffers,
            callbacks: self.callbacks,
            current_buffer: self.current_buffer,
            emitter: self.emitter,
            executor: self.executor.with_background_spawner(spawner),
            fs: self.fs,
            next_buffer_id: self.next_buffer_id,
        }
    }

    fn buffer_at(&self, path: &AbsPath) -> Option<&BufferInner> {
        self.buffers.values().find(|buf| path.as_str() == buf.name)
    }

    #[track_caller]
    fn buffer_mut(&mut self, id: BufferId) -> Buffer<'_> {
        Buffer {
            inner: self.buffers.get_mut(&id).expect("buffer exists"),
            callbacks: &self.callbacks,
            current_buffer: &mut self.current_buffer,
        }
    }
}

impl Callbacks {
    pub(crate) fn insert(&self, kind: CallbackKind) -> EventHandle {
        EventHandle {
            key: self.inner.with_mut(|map| map.insert(kind)),
            callbacks: Self { inner: self.inner.clone() },
        }
    }

    pub(crate) fn with_mut<R>(
        &self,
        f: impl FnOnce(&mut SlotMap<DefaultKey, CallbackKind>) -> R,
    ) -> R {
        self.inner.with_mut(f)
    }
}

impl<Fs, BgSpawner> Backend for Mock<Fs, BgSpawner>
where
    Fs: fs::Fs,
    BgSpawner: BackgroundSpawner,
{
    const REINSTATE_PANIC_HOOK: bool = true;

    type Api = Api;
    type Buffer<'a> = Buffer<'a>;
    type BufferId = BufferId;
    type Cursor<'a> = Cursor<'a>;
    type CursorId = CursorId;
    type EventHandle = EventHandle;
    type Executor = Executor<BgSpawner>;
    type Fs = Fs;
    type Emitter<'this> = &'this mut Emitter;
    type Selection<'a> = Selection<'a>;
    type SelectionId = SelectionId;

    type CreateBufferError = CreateBufferError<Fs>;
    type SerializeError = SerializeError;
    type DeserializeError = DeserializeError;

    fn buffer(&mut self, id: BufferId) -> Option<Self::Buffer<'_>> {
        self.buffers.contains_key(&id).then_some(self.buffer_mut(id))
    }

    fn buffer_at_path(&mut self, path: &AbsPath) -> Option<Self::Buffer<'_>> {
        self.buffer_at(path)
            .map(|buffer| buffer.id)
            .map(|id| self.buffer_mut(id))
    }

    fn buffer_ids(
        &mut self,
    ) -> impl Iterator<Item = BufferId> + use<Fs, BgSpawner> {
        self.buffers.keys().copied().collect::<Vec<_>>().into_iter()
    }

    async fn create_buffer(
        file_path: &AbsPath,
        agent_id: AgentId,
        ctx: &mut Context<Self, impl BorrowState>,
    ) -> Result<Self::BufferId, Self::CreateBufferError> {
        <Self as BaseBackend>::create_buffer(file_path, agent_id, ctx).await
    }

    fn current_buffer(&mut self) -> Option<Self::Buffer<'_>> {
        self.current_buffer.map(|id| self.buffer_mut(id))
    }

    fn cursor(
        &mut self,
        cursor_id: Self::CursorId,
    ) -> Option<Self::Cursor<'_>> {
        self.buffer(cursor_id.buffer_id())
            .and_then(|buf| buf.into_cursor(cursor_id))
    }

    fn fs(&mut self) -> Self::Fs {
        self.fs.clone()
    }

    fn emitter(&mut self) -> Self::Emitter<'_> {
        &mut self.emitter
    }

    fn executor(&mut self) -> &mut Self::Executor {
        &mut self.executor
    }

    fn on_buffer_created<Fun>(&mut self, fun: Fun) -> Self::EventHandle
    where
        Fun: FnMut(&Self::Buffer<'_>, AgentId) + 'static,
    {
        self.callbacks.insert(CallbackKind::BufferCreated(Box::new(fun)))
    }

    fn on_cursor_created<Fun>(&mut self, fun: Fun) -> Self::EventHandle
    where
        Fun: FnMut(&Self::Cursor<'_>, AgentId) + 'static,
    {
        let cb_kind = CallbackKind::CursorCreated(Box::new(fun));
        self.callbacks.insert(cb_kind)
    }

    fn on_selection_created<Fun>(&mut self, fun: Fun) -> Self::EventHandle
    where
        Fun: FnMut(&Self::Selection<'_>, AgentId) + 'static,
    {
        let cb_kind = CallbackKind::SelectionCreated(Box::new(fun));
        self.callbacks.insert(cb_kind)
    }

    fn selection(
        &mut self,
        selection_id: Self::SelectionId,
    ) -> Option<Self::Selection<'_>> {
        self.buffer(selection_id.buffer_id())
            .and_then(|buf| buf.into_selection(selection_id))
    }

    fn serialize<T>(
        &mut self,
        value: &T,
    ) -> impl MaybeResult<ApiValue<Self>, Error = Self::SerializeError>
    where
        T: ?Sized + Serialize,
    {
        crate::serde::serialize(value)
    }

    fn deserialize<'de, T>(
        &mut self,
        value: ApiValue<Self>,
    ) -> impl MaybeResult<T, Error = Self::DeserializeError>
    where
        T: Deserialize<'de>,
    {
        crate::serde::deserialize(value)
    }
}

impl<Fs, BgSpawner> BaseBackend for Mock<Fs, BgSpawner>
where
    Fs: fs::Fs,
    BgSpawner: BackgroundSpawner,
{
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

        ctx.with_editor(|ed| {
            let this = ed.as_mut();

            let buffer_id = this.next_buffer_id.post_inc();

            this.buffers.insert(
                buffer_id,
                BufferInner::new(buffer_id, file_path.to_string(), contents),
            );

            let buffer = this.buffer_mut(buffer_id);

            buffer.callbacks.with_mut(|callbacks| {
                for cb_kind in callbacks.values_mut() {
                    if let CallbackKind::BufferCreated(fun) = cb_kind {
                        fun(&buffer, agent_id);
                    }
                }
            });

            Ok(buffer_id)
        })
    }
}

impl<Fs: Default> Default for Mock<Fs> {
    fn default() -> Self {
        Self::new(Fs::default())
    }
}

impl<Fs, BgSpawner> AsMut<Self> for Mock<Fs, BgSpawner> {
    fn as_mut(&mut self) -> &mut Self {
        self
    }
}

impl Drop for EventHandle {
    fn drop(&mut self) {
        self.callbacks.inner.with_mut(|map| map.remove(self.key));
    }
}

impl<Fs: fs::Fs> notify::Error for CreateBufferError<Fs> {
    fn to_message(&self) -> (notify::Level, notify::Message) {
        (notify::Level::Error, notify::Message::from_display(&self.inner))
    }
}
