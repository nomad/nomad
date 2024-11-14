use core::fmt;
use core::hash::{Hash, Hasher};

use nvim_oxi::api::{self, Buffer as NvimBuffer};

type BufHandle = i32;

/// TODO: docs.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BufferId {
    handle: BufHandle,
}

impl BufferId {
    /// Returns the [`BufferId`] of the currently focused buffer.
    pub fn current() -> Self {
        Self::new(NvimBuffer::current())
    }

    /// Returns the [`BufferId`] of the buffer with the given name.
    pub fn of_name<T: AsRef<str>>(name: T) -> Option<Self> {
        api::call_function::<_, i32>("bufnr", (name.as_ref(),))
            .ok()
            .and_then(|handle| (handle != -1).then_some(Self { handle }))
    }

    /// Returns an iterator of the [`BufferId`]s of all the currently opened
    /// buffers.
    pub fn opened() -> impl ExactSizeIterator<Item = Self> {
        api::list_bufs().map(Self::new)
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
