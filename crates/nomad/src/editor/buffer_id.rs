/// TODO: docs
#[derive(Debug, Clone, Copy)]
pub struct BufferId(nvim::BufHandle);

impl BufferId {
    /// TODO: docs
    #[inline]
    pub fn current() -> Self {
        (&nvim::api::Buffer::current()).into()
    }
}

impl From<&nvim::api::Buffer> for BufferId {
    #[inline]
    fn from(buf: &nvim::api::Buffer) -> Self {
        Self(unsafe { core::mem::transmute_copy(buf) })
    }
}

impl From<BufferId> for nvim::api::Buffer {
    #[inline]
    fn from(buf: BufferId) -> Self {
        unsafe { core::mem::transmute(buf) }
    }
}
