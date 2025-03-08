use std::borrow::Cow;

use crop::Rope;
use nvimx_core::ByteOffset;
use nvimx_core::backend::Buffer;

/// TODO: docs.
pub struct TestBuffer {
    pub(crate) contents: Rope,
    pub(crate) id: TestBufferId,
    pub(crate) name: String,
}

/// TODO: docs.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct TestBufferId(pub(crate) u64);

impl TestBufferId {
    pub(crate) fn post_inc(&mut self) -> Self {
        let id = *self;
        self.0 += 1;
        id
    }
}

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
