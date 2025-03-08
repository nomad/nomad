use fxhash::FxHashMap;
use nvimx_core::backend::{ApiValue, Backend, BufferId};
use nvimx_core::fs::{AbsPath, FsNode};
use nvimx_core::notify::MaybeResult;
use serde::{Deserialize, Serialize};

use crate::buffer::{TestBuffer, TestBufferId};
use crate::emitter::TestEmitter;
use crate::executor::TestExecutor;
use crate::fs::TestFs;
use crate::serde::{TestDeserializeError, TestSerializeError};

/// TODO: docs.
pub struct TestBackend {
    buffers: FxHashMap<TestBufferId, TestBuffer>,
    current_buffer: Option<TestBufferId>,
    emitter: TestEmitter,
    executor: TestExecutor,
    fs: TestFs,
    next_buffer_id: TestBufferId,
}

impl TestBackend {
    pub fn new(fs: TestFs) -> Self {
        Self {
            buffers: Default::default(),
            current_buffer: None,
            emitter: Default::default(),
            executor: Default::default(),
            fs,
            next_buffer_id: TestBufferId(1),
        }
    }

    fn buffer_at(&self, path: &AbsPath) -> Option<&TestBuffer> {
        self.buffers.values().find(|buf| path.as_str() == buf.name)
    }

    #[track_caller]
    fn buffer_mut(&mut self, id: TestBufferId) -> &mut TestBuffer {
        self.buffers.get_mut(&id).expect("buffer exists")
    }

    #[track_caller]
    fn open_buffer(&mut self, path: &AbsPath) -> &mut TestBuffer {
        assert!(self.buffer_at(path).is_none());

        let file =
            match self.fs.node_at_path_sync(path).expect("no file at path") {
                FsNode::File(file) => file,
                _ => todo!(),
            };

        let contents =
            str::from_utf8(&file.read_sync().expect("just got file"))
                .expect("file is not valid UTF-8")
                .into();

        let buffer = TestBuffer {
            contents,
            id: self.next_buffer_id.post_inc(),
            name: path.to_string(),
        };

        let buffer_id = buffer.id;

        self.buffers.insert(buffer.id, buffer);

        self.buffer_mut(buffer_id)
    }
}

impl Backend for TestBackend {
    const REINSTATE_PANIC_HOOK: bool = true;

    type Api = crate::api::TestApi;
    type Buffer<'a> = &'a mut TestBuffer;
    type BufferId = TestBufferId;
    type LocalExecutor = TestExecutor;
    type BackgroundExecutor = TestExecutor;
    type Fs = TestFs;
    type Emitter<'this> = &'this mut TestEmitter;
    type SerializeError = TestSerializeError;
    type DeserializeError = TestDeserializeError;

    fn buffer(&mut self, id: BufferId<Self>) -> Option<Self::Buffer<'_>> {
        self.buffers.get_mut(&id)
    }

    fn buffer_ids(&mut self) -> impl Iterator<Item = BufferId<Self>> + use<> {
        self.buffers.keys().copied().collect::<Vec<_>>().into_iter()
    }

    fn current_buffer(&mut self) -> Option<Self::Buffer<'_>> {
        self.current_buffer.map(|id| self.buffer_mut(id))
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

impl Default for TestBackend {
    fn default() -> Self {
        Self::new(TestFs::default())
    }
}
