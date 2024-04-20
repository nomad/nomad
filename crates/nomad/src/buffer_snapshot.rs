use cola::Replica;
use crop::Rope;

/// TODO: docs
pub struct BufferSnapshot {
    replica: Replica,
    text: Rope,
}

impl BufferSnapshot {
    #[inline]
    pub(crate) fn new(replica: Replica, text: Rope) -> Self {
        Self { replica, text }
    }

    /// TODO: docs
    #[inline]
    pub fn replica(&self) -> &Replica {
        &self.replica
    }

    /// TODO: docs
    #[inline]
    pub fn text(&self) -> &Rope {
        &self.text
    }
}
