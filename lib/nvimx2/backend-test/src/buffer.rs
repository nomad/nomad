use std::borrow::Cow;

use crop::Rope;
use nvimx_core::ByteOffset;
use nvimx_core::backend::Buffer;

/// TODO: docs.
pub struct TestBuffer {
    contents: Rope,
    id: TestBufferId,
    name: String,
}

/// TODO: docs.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct TestBufferId(u64);

impl Buffer for TestBuffer {
    type Id = TestBufferId;

    fn byte_len(&self) -> ByteOffset {
        self.contents.byte_len().into()
    }

    fn id(&self) -> Self::Id {
        self.id
    }

    fn name(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.name)
    }
}
