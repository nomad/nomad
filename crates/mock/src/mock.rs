use abs_path::AbsPath;
use ed::backend::{
    AgentId,
    ApiValue,
    Backend,
    BackgroundExecutor,
    Buffer as _,
    Edit,
    LocalExecutor,
};
use ed::fs::{self, File, FsNode};
use ed::notify::MaybeResult;
use ed::shared::Shared;
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
use crate::executor::Executor;
use crate::fs::MockFs;
use crate::serde::{DeserializeError, SerializeError};

/// TODO: docs.
pub struct Mock<Fs = MockFs, LocalEx = Executor, BackgroundEx = Executor> {
    background_executor: BackgroundEx,
    buffers: FxHashMap<BufferId, BufferInner>,
    callbacks: Callbacks,
    current_buffer: Option<BufferId>,
    emitter: Emitter,
    fs: Fs,
    local_executor: LocalEx,
    next_buffer_id: BufferId,
}

pub struct EventHandle {
    key: slotmap::DefaultKey,
    callbacks: Callbacks,
}

#[derive(Default)]
pub(crate) struct Callbacks {
    inner: Shared<SlotMap<DefaultKey, CallbackKind>>,
}

#[allow(clippy::type_complexity)]
pub(crate) enum CallbackKind {
    OnBufferCreated(Box<dyn FnMut(&Buffer<'_>) + 'static>),
    OnBufferEdited(BufferId, Box<dyn FnMut(&Buffer<'_>, &Edit) + 'static>),
    OnBufferRemoved(BufferId, Box<dyn FnMut(&Buffer<'_>, AgentId) + 'static>),
    OnBufferSaved(BufferId, Box<dyn FnMut(&Buffer<'_>, AgentId) + 'static>),
}

impl<Fs: fs::Fs> Mock<Fs> {
    pub fn new(fs: impl Into<Fs>) -> Self {
        let local_executor = Executor::default();
        Self {
            background_executor: local_executor.clone(),
            buffers: Default::default(),
            callbacks: Default::default(),
            current_buffer: None,
            emitter: Default::default(),
            fs: fs.into(),
            local_executor,
            next_buffer_id: BufferId(1),
        }
    }
}

impl<Fs, LocalEx, BackgroundEx> Mock<Fs, LocalEx, BackgroundEx>
where
    Fs: fs::Fs,
    LocalEx: LocalExecutor + 'static,
    BackgroundEx: BackgroundExecutor,
{
    pub fn with_background_executor<NewBackgroundEx>(
        self,
        background_executor: NewBackgroundEx,
    ) -> Mock<Fs, LocalEx, NewBackgroundEx> {
        Mock {
            background_executor,
            buffers: self.buffers,
            callbacks: self.callbacks,
            current_buffer: self.current_buffer,
            emitter: self.emitter,
            fs: self.fs,
            local_executor: self.local_executor,
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
        }
    }

    #[track_caller]
    fn open_buffer(&mut self, path: &AbsPath) -> Buffer<'_> {
        assert!(self.buffer_at(path).is_none());

        let contents = futures_lite::future::block_on(async {
            let file = match self
                .fs()
                .node_at_path(path)
                .await
                .expect("infallible")
                .expect("no file at path")
            {
                FsNode::File(file) => file,
                _ => todo!(),
            };

            str::from_utf8(&file.read().await.expect("just got file"))
                .expect("file is not valid UTF-8")
                .into()
        });

        let buffer = BufferInner::new(
            self.next_buffer_id.post_inc(),
            path.to_string(),
            contents,
        );

        let buffer_id = buffer.id;

        self.buffers.insert(buffer.id, buffer);

        self.buffer_mut(buffer_id)
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

impl<Fs, LocalEx, BackgroundEx> Backend for Mock<Fs, LocalEx, BackgroundEx>
where
    Fs: fs::Fs,
    LocalEx: LocalExecutor + 'static,
    BackgroundEx: BackgroundExecutor,
{
    const REINSTATE_PANIC_HOOK: bool = true;

    type Api = Api;
    type Buffer<'a> = Buffer<'a>;
    type BufferId = BufferId;
    type Cursor<'a> = Cursor<'a>;
    type CursorId = CursorId;
    type EventHandle = EventHandle;
    type LocalExecutor = LocalEx;
    type BackgroundExecutor = BackgroundEx;
    type Fs = Fs;
    type Emitter<'this> = &'this mut Emitter;
    type Selection<'a> = Selection<'a>;
    type SelectionId = SelectionId;

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
    ) -> impl Iterator<Item = BufferId> + use<Fs, LocalEx, BackgroundEx> {
        self.buffers.keys().copied().collect::<Vec<_>>().into_iter()
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

    fn local_executor(&mut self) -> &mut Self::LocalExecutor {
        &mut self.local_executor
    }

    fn focus_buffer_at(&mut self, path: &AbsPath) -> Option<Self::Buffer<'_>> {
        let buf_id = self
            .buffer_at(path)
            .map(|buf| buf.id)
            .unwrap_or_else(|| self.open_buffer(path).id());
        self.current_buffer = Some(buf_id);
        Some(self.buffer_mut(buf_id))
    }

    fn background_executor(&mut self) -> &mut Self::BackgroundExecutor {
        &mut self.background_executor
    }

    fn on_buffer_created<Fun>(&mut self, fun: Fun) -> Self::EventHandle
    where
        Fun: FnMut(&Self::Buffer<'_>) + 'static,
    {
        self.callbacks.insert(CallbackKind::OnBufferCreated(Box::new(fun)))
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

impl<Fs: fs::Fs + Default> Default for Mock<Fs> {
    fn default() -> Self {
        Self::new(Fs::default())
    }
}

impl Drop for EventHandle {
    fn drop(&mut self) {
        self.callbacks.inner.with_mut(|map| map.remove(self.key));
    }
}
