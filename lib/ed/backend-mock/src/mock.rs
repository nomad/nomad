use abs_path::AbsPath;
use ed_core::backend::{AgentId, ApiValue, Backend, Edit};
use ed_core::fs::{Fs, FsNode};
use ed_core::notify::MaybeResult;
use ed_core::shared::Shared;
use fxhash::FxHashMap;
use serde::{Deserialize, Serialize};
use slotmap::{DefaultKey, SlotMap};

use crate::api::Api;
use crate::buffer::{Buffer, BufferId, BufferInner};
use crate::emitter::Emitter;
use crate::executor::Executor;
use crate::fs::MockFs;
use crate::serde::{DeserializeError, SerializeError};

/// TODO: docs.
pub struct Mock {
    buffers: FxHashMap<BufferId, BufferInner>,
    callbacks: Callbacks,
    current_buffer: Option<BufferId>,
    emitter: Emitter,
    executor: Executor,
    fs: MockFs,
    next_buffer_id: BufferId,
}

pub struct EventHandle {
    key: slotmap::DefaultKey,
    callbacks: Callbacks,
}

#[derive(Clone, Default)]
pub(crate) struct Callbacks {
    inner: Shared<SlotMap<DefaultKey, CallbackKind>>,
}

pub(crate) enum CallbackKind {
    OnBufferCreated(Box<dyn FnMut(&Buffer<'_>) + 'static>),
    OnBufferEdited(Box<dyn FnMut(&Buffer<'_>, &Edit) + 'static>),
    OnBufferRemoved(Box<dyn FnMut(&Buffer<'_>, AgentId) + 'static>),
    OnBufferSaved(Box<dyn FnMut(&Buffer<'_>, AgentId) + 'static>),
}

impl Mock {
    pub fn new(fs: MockFs) -> Self {
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

    fn callbacks(&self) -> &Callbacks {
        &self.callbacks
    }

    fn buffer_at(&self, path: &AbsPath) -> Option<&BufferInner> {
        self.buffers.values().find(|buf| path.as_str() == buf.name)
    }

    #[track_caller]
    fn buffer_mut(&mut self, id: BufferId) -> Buffer<'_> {
        Buffer {
            inner: self.buffers.get_mut(&id).expect("buffer exists"),
            callbacks: self.callbacks.clone(),
        }
    }

    #[track_caller]
    fn open_buffer(&mut self, path: &AbsPath) -> Buffer<'_> {
        assert!(self.buffer_at(path).is_none());

        let file =
            match futures_lite::future::block_on(self.fs.node_at_path(path))
                .expect("infallible")
                .expect("no file at path")
            {
                FsNode::File(file) => file,
                _ => todo!(),
            };

        let contents =
            str::from_utf8(&file.read_sync().expect("just got file"))
                .expect("file is not valid UTF-8")
                .into();

        let buffer = BufferInner {
            contents,
            id: self.next_buffer_id.post_inc(),
            name: path.to_string(),
        };

        let buffer_id = buffer.id;

        self.buffers.insert(buffer.id, buffer);

        self.buffer_mut(buffer_id)
    }
}

impl Callbacks {
    pub(crate) fn insert(&mut self, kind: CallbackKind) -> EventHandle {
        EventHandle {
            key: self.inner.with_mut(|map| map.insert(kind)),
            callbacks: self.clone(),
        }
    }
}

impl Backend for Mock {
    const REINSTATE_PANIC_HOOK: bool = true;

    type Api = Api;
    type Buffer<'a> = Buffer<'a>;
    type BufferId = BufferId;
    type Cursor<'a> = ();
    type CursorId = ();
    type EventHandle = EventHandle;
    type LocalExecutor = Executor;
    type BackgroundExecutor = Executor;
    type Fs = MockFs;
    type Emitter<'this> = &'this mut Emitter;
    type Selection<'a> = ();
    type SelectionId = ();

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

    fn buffer_ids(&mut self) -> impl Iterator<Item = BufferId> + use<> {
        self.buffers.keys().copied().collect::<Vec<_>>().into_iter()
    }

    fn current_buffer(&mut self) -> Option<Self::Buffer<'_>> {
        self.current_buffer.map(|id| self.buffer_mut(id))
    }

    fn cursor(&mut self, _id: Self::CursorId) -> Option<Self::Cursor<'_>> {
        todo!()
    }

    fn fs(&mut self) -> Self::Fs {
        self.fs.clone()
    }

    fn emitter(&mut self) -> Self::Emitter<'_> {
        &mut self.emitter
    }

    fn local_executor(&mut self) -> &mut Self::LocalExecutor {
        &mut self.executor
    }

    fn focus_buffer_at(&mut self, path: &AbsPath) -> Option<Self::Buffer<'_>> {
        let buf_id = self
            .buffer_at(path)
            .map(|buf| buf.id)
            .unwrap_or_else(|| self.open_buffer(path).id);
        self.current_buffer = Some(buf_id);
        Some(self.buffer_mut(buf_id))
    }

    fn background_executor(&mut self) -> &mut Self::BackgroundExecutor {
        &mut self.executor
    }

    fn on_buffer_created<Fun>(&mut self, fun: Fun) -> Self::EventHandle
    where
        Fun: FnMut(&Self::Buffer<'_>) + 'static,
    {
        self.callbacks.insert(CallbackKind::OnBufferCreated(Box::new(fun)))
    }

    fn selection(
        &mut self,
        _id: Self::SelectionId,
    ) -> Option<Self::Selection<'_>> {
        todo!()
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

impl Default for Mock {
    fn default() -> Self {
        Self::new(MockFs::default())
    }
}

impl Drop for EventHandle {
    fn drop(&mut self) {
        self.callbacks.inner.with_mut(|map| map.remove(self.key));
    }
}
