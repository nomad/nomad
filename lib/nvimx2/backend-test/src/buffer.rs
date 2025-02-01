use std::borrow::Cow;

use crop::Rope;
use nvimx_core::ByteOffset;
use nvimx_core::backend::Buffer;

use crate::TestBackend;

/// TODO: docs.
pub struct TestBuffer {
    contents: Rope,
    id: TestBufferId,
    name: String,
}

/// TODO: docs.
#[derive(Debug, Copy, Clone)]
pub struct TestBufferId;

impl Buffer<TestBackend> for TestBuffer {
    fn byte_len(&self) -> ByteOffset {
        self.contents.byte_len().into()
    }

    fn id(&self) -> TestBufferId {
        self.id
    }

    fn name(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.name)
    }
}
