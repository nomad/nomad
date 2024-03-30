/// TODO: docs
#[derive(Debug, Clone, Copy)]
pub struct BufferId(nvim::BufHandle);

impl From<&nvim::api::Buffer> for BufferId {
    #[inline]
    fn from(buf: &nvim::api::Buffer) -> Self {
        Self(unsafe { core::mem::transmute_copy(buf) })
    }
}
