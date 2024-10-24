use core::fmt;
use core::hash::{Hash, Hasher};

use nvim_oxi::api::Buffer as NvimBuffer;

type BufHandle = i32;

/// TODO: docs.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BufferId {
    handle: BufHandle,
}

impl BufferId {
    /// TODO: docs.
    pub fn current() -> Self {
        Self::new(NvimBuffer::current())
    }

    pub(crate) fn as_nvim(&self) -> NvimBuffer {
        self.handle.into()
    }

    pub(crate) fn new(nvim_buffer: NvimBuffer) -> Self {
        Self { handle: nvim_buffer.handle() }
    }
}

impl fmt::Debug for BufferId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("BufferId").field(&self.handle).finish()
    }
}

impl Hash for BufferId {
    #[inline]
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        hasher.write_i32(self.handle);
    }
}

impl nohash::IsEnabled for BufferId {}
